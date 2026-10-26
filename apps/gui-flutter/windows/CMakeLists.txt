# Project-level configuration.
cmake_minimum_required(VERSION 3.14)
project(Mixar LANGUAGES CXX)

# The name of the executable created for the application. Change this to change
# the on-disk name of your application.
set(BINARY_NAME "Mixar")

# Explicitly opt in to modern CMake behaviors to avoid warnings with recent
# versions of CMake.
cmake_policy(VERSION 3.14...3.25)

# Define build configuration option.
get_property(IS_MULTICONFIG GLOBAL PROPERTY GENERATOR_IS_MULTI_CONFIG)
if(IS_MULTICONFIG)
  set(CMAKE_CONFIGURATION_TYPES "Debug;Profile;Release"
    CACHE STRING "" FORCE)
else()
  if(NOT CMAKE_BUILD_TYPE AND NOT CMAKE_CONFIGURATION_TYPES)
    set(CMAKE_BUILD_TYPE "Debug" CACHE
      STRING "Flutter build mode" FORCE)
    set_property(CACHE CMAKE_BUILD_TYPE PROPERTY STRINGS
      "Debug" "Profile" "Release")
  endif()
endif()
# Define settings for the Profile build mode.
set(CMAKE_EXE_LINKER_FLAGS_PROFILE "${CMAKE_EXE_LINKER_FLAGS_RELEASE}")
set(CMAKE_SHARED_LINKER_FLAGS_PROFILE "${CMAKE_SHARED_LINKER_FLAGS_RELEASE}")
set(CMAKE_C_FLAGS_PROFILE "${CMAKE_C_FLAGS_RELEASE}")
set(CMAKE_CXX_FLAGS_PROFILE "${CMAKE_CXX_FLAGS_RELEASE}")

# Use Unicode for all projects.
add_definitions(-DUNICODE -D_UNICODE)

# Compilation settings that should be applied to most targets.
#
# Be cautious about adding new options here, as plugins use this function by
# default. In most cases, you should add new options to specific targets instead
# of modifying this function.
function(APPLY_STANDARD_SETTINGS TARGET)
  target_compile_features(${TARGET} PUBLIC cxx_std_17)
  target_compile_options(${TARGET} PRIVATE /W4 /WX /wd"4100")
  target_compile_options(${TARGET} PRIVATE /EHsc)
  target_compile_definitions(${TARGET} PRIVATE "_HAS_EXCEPTIONS=0")
  target_compile_definitions(${TARGET} PRIVATE "$<$<CONFIG:Debug>:_DEBUG>")
endfunction()

# Flutter library and tool build rules.
set(FLUTTER_MANAGED_DIR "${CMAKE_CURRENT_SOURCE_DIR}/flutter")
add_subdirectory(${FLUTTER_MANAGED_DIR})

# Application build; see runner/CMakeLists.txt.
add_subdirectory("runner")


# Generated plugin build rules, which manage building the plugins and adding
# them to the application.
include(flutter/generated_plugins.cmake)


# === Installation ===
# Support files are copied into place next to the executable, so that it can
# run in place. This is done instead of making a separate bundle (as on Linux)
# so that building and running from within Visual Studio will work.
set(BUILD_BUNDLE_DIR "$<TARGET_FILE_DIR:${BINARY_NAME}>")
# Make the "install" step default, as it's required to run.
set(CMAKE_VS_INCLUDE_INSTALL_TO_DEFAULT_BUILD 1)
if(CMAKE_INSTALL_PREFIX_INITIALIZED_TO_DEFAULT)
  set(CMAKE_INSTALL_PREFIX "${BUILD_BUNDLE_DIR}" CACHE PATH "..." FORCE)
endif()

set(INSTALL_BUNDLE_DATA_DIR "${CMAKE_INSTALL_PREFIX}/data")
set(INSTALL_BUNDLE_LIB_DIR "${CMAKE_INSTALL_PREFIX}")

install(TARGETS ${BINARY_NAME} RUNTIME DESTINATION "${CMAKE_INSTALL_PREFIX}"
  COMPONENT Runtime)

install(FILES "${FLUTTER_ICU_DATA_FILE}" DESTINATION "${INSTALL_BUNDLE_DATA_DIR}"
  COMPONENT Runtime)

install(FILES "${FLUTTER_LIBRARY}" DESTINATION "${INSTALL_BUNDLE_LIB_DIR}"
  COMPONENT Runtime)

if(PLUGIN_BUNDLED_LIBRARIES)
  # A plugin that lists a sidecar it never produced (e.g. the ORT WebGPU Dawn
  # DLL) otherwise fails the whole build in the INSTALL project, where CMake's
  # output is swallowed and MSBuild only reports "exited with code 1". Check at
  # install time — paths carry $<CONFIG> and only resolve then — and name the
  # offenders instead of shipping a bundle with a missing runtime dependency.
  string(REPLACE ";" "|" _bundled_libraries "${PLUGIN_BUNDLED_LIBRARIES}")
  install(CODE "
    string(REPLACE \"|\" \";\" _bundled \"${_bundled_libraries}\")
    set(_missing \"\")
    foreach(_library IN LISTS _bundled)
      if(NOT EXISTS \"\${_library}\")
        string(APPEND _missing \"\n  \${_library}\")
      endif()
    endforeach()
    if(NOT _missing STREQUAL \"\")
      message(FATAL_ERROR \"PLUGIN_BUNDLED_LIBRARIES entries do not exist:\${_missing}\")
    endif()
    " COMPONENT Runtime)

  install(FILES "${PLUGIN_BUNDLED_LIBRARIES}"
    DESTINATION "${INSTALL_BUNDLE_LIB_DIR}"
    COMPONENT Runtime)
endif()

# Copy the native assets provided by the build.dart from all packages.
set(NATIVE_ASSETS_DIR "${PROJECT_BUILD_DIR}native_assets/windows/")
install(DIRECTORY "${NATIVE_ASSETS_DIR}"
   DESTINATION "${INSTALL_BUNDLE_LIB_DIR}"
   COMPONENT Runtime)

# Fully re-copy the assets directory on each build to avoid having stale files
# from a previous install.
set(FLUTTER_ASSET_DIR_NAME "flutter_assets")
install(CODE "
  file(REMOVE_RECURSE \"${INSTALL_BUNDLE_DATA_DIR}/${FLUTTER_ASSET_DIR_NAME}\")
  " COMPONENT Runtime)
install(DIRECTORY "${PROJECT_BUILD_DIR}/${FLUTTER_ASSET_DIR_NAME}"
  DESTINATION "${INSTALL_BUNDLE_DATA_DIR}" COMPONENT Runtime)

# Install the AOT library on non-Debug builds only.
install(FILES "${AOT_LIBRARY}" DESTINATION "${INSTALL_BUNDLE_DATA_DIR}"
  CONFIGURATIONS Profile;Release
  COMPONENT Runtime)

# === Visual C++ runtime, application-local ===
# Every C++ module in the bundle imports the Visual C++ runtime: the runner,
# flutter_windows.dll, native plugins, and the dynamic CRT that rustc links into
# the host_flutter cdylib by default. Windows only carries those DLLs when the
# "Microsoft Visual C++ 2015-2022 Redistributable" is installed, so a clean
# machine dies at launch with "The code execution cannot proceed because
# MSVCP140.dll was not found". Flutter's Windows packaging docs prescribe
# application-local deployment: the DLLs beside the exe are what the loader
# resolves first. install() lands them in the bundle dir, which is exactly what
# the fastforge/Inno Setup step packs into the installer.
#
# Taken from the MSVC redistributable that ships with the Visual Studio instance
# doing the build — VC/Redist/MSVC/<toolset>/x64/Microsoft.VC14x.CRT. The whole
# directory is installed rather than a hardcoded name list: the folder and file
# set drift across toolsets (VC143 for MSVC 14.3x/14.4x, VC145 for 14.5x) and a
# stale list would silently drop a DLL and reintroduce this crash.
set(MIXAR_MSVC_CRT_DIR "" CACHE PATH
  "x64 MSVC CRT redistributable directory to ship beside Mixar.exe. Defaults to the newest one under the Visual Studio install.")
if(NOT MIXAR_MSVC_CRT_DIR)
  if(NOT CMAKE_GENERATOR_INSTANCE)
    message(FATAL_ERROR
      "Cannot locate the MSVC redistributable: this generator has no Visual Studio instance. "
      "Pass -DMIXAR_MSVC_CRT_DIR=<VS>/VC/Redist/MSVC/<toolset>/x64/Microsoft.VC14x.CRT so the "
      "bundle ships the Visual C++ runtime app-locally.")
  endif()
  file(GLOB _mixar_msvc_crt_dirs
    "${CMAKE_GENERATOR_INSTANCE}/VC/Redist/MSVC/*/x64/Microsoft.VC14*.CRT")
  if(NOT _mixar_msvc_crt_dirs)
    message(FATAL_ERROR
      "No MSVC CRT redistributable under `${CMAKE_GENERATOR_INSTANCE}/VC/Redist/MSVC`. "
      "Install the Visual Studio \"Desktop development with C++\" workload, or pass "
      "-DMIXAR_MSVC_CRT_DIR=<VS>/VC/Redist/MSVC/<toolset>/x64/Microsoft.VC14x.CRT.")
  endif()
  list(SORT _mixar_msvc_crt_dirs)
  list(GET _mixar_msvc_crt_dirs -1 MIXAR_MSVC_CRT_DIR)
endif()
foreach(_mixar_crt_required msvcp140.dll vcruntime140.dll vcruntime140_1.dll)
  if(NOT EXISTS "${MIXAR_MSVC_CRT_DIR}/${_mixar_crt_required}")
    message(FATAL_ERROR
      "`${MIXAR_MSVC_CRT_DIR}` lacks ${_mixar_crt_required}. Flutter's application-local "
      "deployment needs msvcp140.dll, vcruntime140.dll and vcruntime140_1.dll beside Mixar.exe; "
      "point MIXAR_MSVC_CRT_DIR at the x64/Microsoft.VC14x.CRT redistributable directory.")
  endif()
endforeach()
file(GLOB _mixar_msvc_crt_dlls "${MIXAR_MSVC_CRT_DIR}/*.dll")
install(FILES ${_mixar_msvc_crt_dlls} DESTINATION "${INSTALL_BUNDLE_LIB_DIR}"
  COMPONENT Runtime)
