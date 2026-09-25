# The Flutter tooling requires that developers have a version of Visual Studio
# installed that includes CMake 3.14 or later. You should not increase this
# version, as doing so will cause the plugin to fail to compile for some
# customers of the plugin.
cmake_minimum_required(VERSION 3.14)

# Project-level configuration.
set(PROJECT_NAME "host_flutter")
project(${PROJECT_NAME} LANGUAGES CXX)

include("../cargokit/cmake/cargokit.cmake")
apply_cargokit(${PROJECT_NAME} ../../../../crates/host-flutter host_flutter "")

# List of absolute paths to libraries that should be bundled with the plugin.
# This list could contain prebuilt libraries, or libraries created by an
# external build triggered from this build file.
#
# ORT WebGPU sidecars (copied by cargokit build_cmake next to the cdylib, see
# ortRuntimeSidecars in the build tool): the Dawn library, plus the DirectX
# shader compiler that Dawn's D3D12 backend opens at runtime. The Windows ORT
# WebGPU distribution is the only one that ships more than the Dawn library.
#
# Windows uses the multi-config VS generator, so cargokit writes every artifact
# into ${CMAKE_CURRENT_BINARY_DIR}/$<CONFIG> (cargokit.cmake CARGOKIT_OUTPUT_DIR).
# The $<CONFIG> is required here: without it install() looks one level too high,
# the build compiles fine, and the INSTALL project dies with a bare MSB3073.
set(host_flutter_bundled_libraries
  "${${PROJECT_NAME}_cargokit_lib}"
  "${CMAKE_CURRENT_BINARY_DIR}/$<CONFIG>/webgpu_dawn.dll"
  "${CMAKE_CURRENT_BINARY_DIR}/$<CONFIG>/dxcompiler.dll"
  "${CMAKE_CURRENT_BINARY_DIR}/$<CONFIG>/dxil.dll"
  PARENT_SCOPE
)
