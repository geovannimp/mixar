SET(cargokit_cmake_root "${CMAKE_CURRENT_LIST_DIR}/..")

# Workaround for https://github.com/dart-lang/pub/issues/4010
get_filename_component(cargokit_cmake_root "${cargokit_cmake_root}" REALPATH)

if(WIN32)
    # REALPATH does not properly resolve symlinks on windows :-/
    execute_process(COMMAND powershell -ExecutionPolicy Bypass -File "${CMAKE_CURRENT_LIST_DIR}/resolve_symlinks.ps1" "${cargokit_cmake_root}" OUTPUT_VARIABLE cargokit_cmake_root OUTPUT_STRIP_TRAILING_WHITESPACE)
endif()

# Arguments
# - target: CMAKE target to which rust library is linked
# - manifest_dir: relative path from current folder to directory containing cargo manifest
# - lib_name: cargo package name
# - any_symbol_name: name of any exported symbol from the library.
#                    used on windows to force linking with library.
function(apply_cargokit target manifest_dir lib_name any_symbol_name)

    set(CARGOKIT_LIB_NAME "${lib_name}")
    set(CARGOKIT_LIB_FULL_NAME "${CMAKE_SHARED_MODULE_PREFIX}${CARGOKIT_LIB_NAME}${CMAKE_SHARED_MODULE_SUFFIX}")
    if (CMAKE_CONFIGURATION_TYPES)
        set(CARGOKIT_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}/$<CONFIG>")
        set(OUTPUT_LIB "${CMAKE_CURRENT_BINARY_DIR}/$<CONFIG>/${CARGOKIT_LIB_FULL_NAME}")
    else()
        set(CARGOKIT_OUTPUT_DIR "${CMAKE_CURRENT_BINARY_DIR}")
        set(OUTPUT_LIB "${CMAKE_CURRENT_BINARY_DIR}/${CARGOKIT_LIB_FULL_NAME}")
    endif()
    set(CARGOKIT_TEMP_DIR "${CMAKE_CURRENT_BINARY_DIR}/cargokit_build")

    if (FLUTTER_TARGET_PLATFORM)
        set(CARGOKIT_TARGET_PLATFORM "${FLUTTER_TARGET_PLATFORM}")
    else()
        set(CARGOKIT_TARGET_PLATFORM "windows-x64")
    endif()

    # Flutter builds via .plugin_symlinks. Relative ../../../../crates/... only
    # works after resolving that link to rust_builder. CMake ABSOLUTE is lexical
    # (v0.0.7 Linux). Windows often leaves the junction unresolved (v0.0.8:
    # src still under .plugin_symlinks → windows/flutter/crates/...). Prefer
    # resolved+relative, then walk up for crates/<lib_name>/Cargo.toml.
    set(_cargokit_src_dir "${CMAKE_CURRENT_SOURCE_DIR}")
    if (WIN32)
        execute_process(
            COMMAND powershell -ExecutionPolicy Bypass -File "${cargokit_cmake_root}/cmake/resolve_symlinks.ps1" "${CMAKE_CURRENT_SOURCE_DIR}"
            OUTPUT_VARIABLE _cargokit_resolved
            OUTPUT_STRIP_TRAILING_WHITESPACE
            RESULT_VARIABLE _cargokit_resolve_rc
        )
        if (_cargokit_resolve_rc EQUAL 0 AND NOT _cargokit_resolved STREQUAL "")
            set(_cargokit_src_dir "${_cargokit_resolved}")
        endif()
    else()
        get_filename_component(_cargokit_src_dir "${CMAKE_CURRENT_SOURCE_DIR}" REALPATH)
    endif()
    if (IS_ABSOLUTE "${manifest_dir}")
        set(_cargokit_manifest_dir "${manifest_dir}")
    else()
        set(_cargokit_manifest_dir "${_cargokit_src_dir}/${manifest_dir}")
    endif()
    get_filename_component(_cargokit_manifest_dir "${_cargokit_manifest_dir}" ABSOLUTE)
    if (NOT EXISTS "${_cargokit_manifest_dir}/Cargo.toml")
        # Package dir is crates/host-flutter; lib_name is host_flutter (cdylib).
        get_filename_component(_cargokit_crate_dirname "${manifest_dir}" NAME)
        set(_cargokit_walk "${CMAKE_CURRENT_SOURCE_DIR}")
        set(_cargokit_manifest_dir "")
        foreach(_cargokit_i RANGE 0 16)
            set(_cargokit_candidate "${_cargokit_walk}/crates/${_cargokit_crate_dirname}")
            if (EXISTS "${_cargokit_candidate}/Cargo.toml")
                get_filename_component(_cargokit_manifest_dir "${_cargokit_candidate}" ABSOLUTE)
                break()
            endif()
            get_filename_component(_cargokit_walk "${_cargokit_walk}/.." ABSOLUTE)
        endforeach()
        if (_cargokit_manifest_dir STREQUAL "")
            message(FATAL_ERROR "cargokit: could not find crates/${_cargokit_crate_dirname}/Cargo.toml walking up from ${CMAKE_CURRENT_SOURCE_DIR}")
        endif()
    endif()
    message(STATUS "cargokit: CARGOKIT_MANIFEST_DIR=${_cargokit_manifest_dir}")

    set(CARGOKIT_ENV
        "CARGOKIT_CMAKE=${CMAKE_COMMAND}"
        "CARGOKIT_CONFIGURATION=$<CONFIG>"
        "CARGOKIT_MANIFEST_DIR=${_cargokit_manifest_dir}"
        "CARGOKIT_TARGET_TEMP_DIR=${CARGOKIT_TEMP_DIR}"
        "CARGOKIT_OUTPUT_DIR=${CARGOKIT_OUTPUT_DIR}"
        "CARGOKIT_TARGET_PLATFORM=${CARGOKIT_TARGET_PLATFORM}"
        "CARGOKIT_TOOL_TEMP_DIR=${CARGOKIT_TEMP_DIR}/tool"
        "CARGOKIT_ROOT_PROJECT_DIR=${CMAKE_SOURCE_DIR}"
    )

    if (WIN32)
        set(SCRIPT_EXTENSION ".cmd")
        set(IMPORT_LIB_EXTENSION ".lib")
    else()
        set(SCRIPT_EXTENSION ".sh")
        set(IMPORT_LIB_EXTENSION "")
        execute_process(COMMAND chmod +x "${cargokit_cmake_root}/run_build_tool${SCRIPT_EXTENSION}")
    endif()

    # Using generators in custom command is only supported in CMake 3.20+
    if (CMAKE_CONFIGURATION_TYPES AND ${CMAKE_VERSION} VERSION_LESS "3.20.0")
        foreach(CONFIG IN LISTS CMAKE_CONFIGURATION_TYPES)
            add_custom_command(
                OUTPUT
                "${CMAKE_CURRENT_BINARY_DIR}/${CONFIG}/${CARGOKIT_LIB_FULL_NAME}"
                "${CMAKE_CURRENT_BINARY_DIR}/_phony_"
                COMMAND ${CMAKE_COMMAND} -E env ${CARGOKIT_ENV}
                "${cargokit_cmake_root}/run_build_tool${SCRIPT_EXTENSION}" build-cmake
                VERBATIM
            )
        endforeach()
    else()
        add_custom_command(
            OUTPUT
            ${OUTPUT_LIB}
            "${CMAKE_CURRENT_BINARY_DIR}/_phony_"
            COMMAND ${CMAKE_COMMAND} -E env ${CARGOKIT_ENV}
            "${cargokit_cmake_root}/run_build_tool${SCRIPT_EXTENSION}" build-cmake
            VERBATIM
        )
    endif()


    set_source_files_properties("${CMAKE_CURRENT_BINARY_DIR}/_phony_" PROPERTIES SYMBOLIC TRUE)

    if (TARGET ${target})
        # If we have actual cmake target provided create target and make existing
        # target depend on it
        add_custom_target("${target}_cargokit" DEPENDS ${OUTPUT_LIB})
        add_dependencies("${target}" "${target}_cargokit")
        target_link_libraries("${target}" PRIVATE "${OUTPUT_LIB}${IMPORT_LIB_EXTENSION}")
        if(WIN32)
            target_link_options(${target} PRIVATE "/INCLUDE:${any_symbol_name}")
        endif()
    else()
        # Otherwise (FFI) just use ALL to force building always
        add_custom_target("${target}_cargokit" ALL DEPENDS ${OUTPUT_LIB})
    endif()

    # Allow adding the output library to plugin bundled libraries
    set("${target}_cargokit_lib" ${OUTPUT_LIB} PARENT_SCOPE)

endfunction()
