option(NO_BUILDCACHE "Disable build caching using buildcache" Off)
option(MOTIS_BUILDCACHE_BOOTSTRAP
    "Allow downloading buildcache into the build directory when not found on PATH" Off)

set(buildcache-bin ${CMAKE_CURRENT_BINARY_DIR}/buildcache/bin/buildcache)
get_property(rule-launch-set GLOBAL PROPERTY RULE_LAUNCH_COMPILE SET)

function(motis_try_enable_buildcache candidate_path)
    if (NOT EXISTS "${candidate_path}")
        return()
    endif ()

    execute_process(
        COMMAND "${candidate_path}" --version
        RESULT_VARIABLE buildcache_result
        OUTPUT_QUIET
        ERROR_VARIABLE buildcache_error
        ERROR_STRIP_TRAILING_WHITESPACE
    )

    if (buildcache_result EQUAL 0)
        message(STATUS "Using buildcache: ${candidate_path}")
        set_property(GLOBAL PROPERTY RULE_LAUNCH_COMPILE "${candidate_path}")
    else ()
        set(NO_BUILDCACHE ON CACHE BOOL "Disable build caching using buildcache" FORCE)
        message(WARNING
            "buildcache exists but is not executable (${candidate_path}). "
            "Auto-setting NO_BUILDCACHE=ON for this build directory. "
            "Runtime error: ${buildcache_error}")
    endif ()
endfunction()

if (NO_BUILDCACHE)
    message(STATUS "NO_BUILDCACHE set, buildcache disabled")
elseif (rule-launch-set)
    message(STATUS "Global property RULE_LAUNCH_COMPILE already set - skipping buildcache")
else ()
    find_program(buildcache_program buildcache
        HINTS /opt/buildcache/bin ${CMAKE_CURRENT_BINARY_DIR}/buildcache/bin)
    if (buildcache_program)
        motis_try_enable_buildcache("${buildcache_program}")
    elseif (NOT MOTIS_BUILDCACHE_BOOTSTRAP)
        set(NO_BUILDCACHE ON CACHE BOOL "Disable build caching using buildcache" FORCE)
        message(STATUS
            "buildcache not found on PATH; auto-setting NO_BUILDCACHE=ON. "
            "Set -DMOTIS_BUILDCACHE_BOOTSTRAP=ON to auto-download buildcache.")
    else ()
        message(STATUS "buildcache not found - bootstrapping in build directory")
        if (UNIX AND ${CMAKE_HOST_SYSTEM_PROCESSOR} STREQUAL "aarch64")
            set(buildcache-archive "buildcache-linux-arm64.tar.gz")
        elseif (UNIX)
            set(buildcache-archive "buildcache-linux-amd64.tar.gz")
        else ()
            message(FATAL_ERROR "This fork supports Linux-only builds; unsupported platform for buildcache bootstrap")
        endif ()

        set(buildcache-url "https://gitlab.com/bits-n-bites/buildcache/-/releases/v0.31.5/downloads/${buildcache-archive}")
        message(STATUS "Downloading buildcache binary from ${buildcache-url}")
        file(DOWNLOAD "${buildcache-url}" ${CMAKE_CURRENT_BINARY_DIR}/${buildcache-archive}
            STATUS buildcache-download-status)
        list(GET buildcache-download-status 0 buildcache-download-code)
        list(GET buildcache-download-status 1 buildcache-download-error)

        if (buildcache-download-code EQUAL 0)
            execute_process(
                COMMAND ${CMAKE_COMMAND} -E tar xf ${CMAKE_CURRENT_BINARY_DIR}/${buildcache-archive}
                WORKING_DIRECTORY ${CMAKE_CURRENT_BINARY_DIR}
                RESULT_VARIABLE extract_result
                ERROR_VARIABLE extract_error
                ERROR_STRIP_TRAILING_WHITESPACE
            )

            if (extract_result EQUAL 0)
                motis_try_enable_buildcache("${buildcache-bin}")
            else ()
                set(NO_BUILDCACHE ON CACHE BOOL "Disable build caching using buildcache" FORCE)
                message(WARNING
                    "Failed to extract downloaded buildcache archive. "
                    "Auto-setting NO_BUILDCACHE=ON for this build directory. "
                    "Extract error: ${extract_error}")
            endif ()
        else ()
            set(NO_BUILDCACHE ON CACHE BOOL "Disable build caching using buildcache" FORCE)
            message(WARNING
                "Failed to download buildcache archive from ${buildcache-url}. "
                "Auto-setting NO_BUILDCACHE=ON for this build directory. "
                "Download error: [${buildcache-download-code}] ${buildcache-download-error}")
        endif ()
    endif ()
endif ()
