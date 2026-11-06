# Gives every program and DLL of RVP's whisper.cpp build for Windows a
# version resource (Explorer's Details page; a program without one looks
# unknown to antivirus heuristics: the host's antivirus quarantined a whisper-cli.exe
# without one as Gen:Variant.Lazy on 2026-11-06). tools/whisper/build.sh
# includes this file after whisper.cpp's project() through
# CMAKE_PROJECT_whisper.cpp_INCLUDE and passes RVP_WHISPER_TAG,
# RVP_WHISPER_COMMIT and RVP_WHISPER_COPYRIGHT (the first line of
# whisper.cpp's licence that names its authors). The resources are added
# when the top directory is done, once every target exists.
include_guard(GLOBAL)
enable_language(RC)

if(NOT RVP_WHISPER_TAG MATCHES "^v([0-9]+)\\.([0-9]+)\\.([0-9]+)$")
    message(FATAL_ERROR "RVP_WHISPER_TAG ${RVP_WHISPER_TAG} is not vX.Y.Z")
endif()
set(RVP_WHISPER_NUMBERS "${CMAKE_MATCH_1},${CMAKE_MATCH_2},${CMAKE_MATCH_3},0")
set(RVP_WHISPER_VERSION "${CMAKE_MATCH_1}.${CMAKE_MATCH_2}.${CMAKE_MATCH_3}")

# The build's targets in DIR and below, in the order CMake made them.
function(rvp_targets dir out)
    get_property(found DIRECTORY "${dir}" PROPERTY BUILDSYSTEM_TARGETS)
    get_property(subdirs DIRECTORY "${dir}" PROPERTY SUBDIRECTORIES)
    foreach(sub IN LISTS subdirs)
        rvp_targets("${sub}" more)
        list(APPEND found ${more})
    endforeach()
    set(${out} ${found} PARENT_SCOPE)
endfunction()

function(rvp_version_resources)
    rvp_targets("${CMAKE_SOURCE_DIR}" targets)
    foreach(target IN LISTS targets)
        get_target_property(type ${target} TYPE)
        if(type STREQUAL "EXECUTABLE")
            set(file_type 0x1L)
            set(kind EXECUTABLE)
        elseif(type STREQUAL "SHARED_LIBRARY")
            set(file_type 0x2L)
            set(kind SHARED_LIBRARY)
        elseif(type STREQUAL "MODULE_LIBRARY")
            set(file_type 0x2L)
            set(kind SHARED_MODULE)
        else()
            continue()
        endif()
        # The file name's parts as the target's own directory has them
        # (ggml empties the prefix there).
        get_target_property(dir ${target} SOURCE_DIR)
        get_directory_property(prefix DIRECTORY "${dir}" DEFINITION CMAKE_${kind}_PREFIX)
        get_directory_property(suffix DIRECTORY "${dir}" DEFINITION CMAKE_${kind}_SUFFIX)
        get_target_property(name ${target} OUTPUT_NAME)
        if(NOT name)
            set(name ${target})
        endif()
        get_target_property(own_prefix ${target} PREFIX)
        if(NOT own_prefix STREQUAL "own_prefix-NOTFOUND")
            set(prefix "${own_prefix}")
        endif()
        if(target STREQUAL "whisper-cli")
            set(description "whisper.cpp's speech recognition command line, RVP's build")
        else()
            set(description "${name} of whisper.cpp, RVP's build")
        endif()
        set(rc "${CMAKE_BINARY_DIR}/rvp-version/${target}.rc")
        file(WRITE "${rc}" "\
1 VERSIONINFO
FILEVERSION ${RVP_WHISPER_NUMBERS}
PRODUCTVERSION ${RVP_WHISPER_NUMBERS}
FILEFLAGSMASK 0x3fL
FILEFLAGS 0x0L
FILEOS 0x40004L
FILETYPE ${file_type}
FILESUBTYPE 0x0L
BEGIN
    BLOCK \"StringFileInfo\"
    BEGIN
        BLOCK \"040904b0\"
        BEGIN
            VALUE \"CompanyName\", \"RVP\"
            VALUE \"FileDescription\", \"${description}\"
            VALUE \"FileVersion\", \"${RVP_WHISPER_VERSION}\"
            VALUE \"InternalName\", \"${name}\"
            VALUE \"LegalCopyright\", \"${RVP_WHISPER_COPYRIGHT}. MIT License.\"
            VALUE \"OriginalFilename\", \"${prefix}${name}${suffix}\"
            VALUE \"ProductName\", \"whisper.cpp\"
            VALUE \"ProductVersion\", \"${RVP_WHISPER_VERSION}\"
            VALUE \"Comments\", \"Built for RVP from whisper.cpp ${RVP_WHISPER_TAG} (${RVP_WHISPER_COMMIT}) by tools/whisper/build.sh\"
        END
    END
    BLOCK \"VarFileInfo\"
    BEGIN
        VALUE \"Translation\", 0x409, 1200
    END
END
")
        target_sources(${target} PRIVATE "${rc}")
    endforeach()
endfunction()

cmake_language(DEFER DIRECTORY "${CMAKE_SOURCE_DIR}" CALL rvp_version_resources)
