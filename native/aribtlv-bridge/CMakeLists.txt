cmake_minimum_required(VERSION 3.24)
project(resub_aribtlv_bridge LANGUAGES C CXX)

set(RESUB_VENDORED_SOURCE_DIR ${CMAKE_CURRENT_BINARY_DIR}/vendored-source)
file(COPY ${CMAKE_CURRENT_SOURCE_DIR}/../../third_party/zlib DESTINATION ${RESUB_VENDORED_SOURCE_DIR})
file(COPY ${CMAKE_CURRENT_SOURCE_DIR}/../../third_party/libaribtlv DESTINATION ${RESUB_VENDORED_SOURCE_DIR})

set(RESUB_ZLIB_SOURCE_DIR ${RESUB_VENDORED_SOURCE_DIR}/zlib)
set(RESUB_ZLIB_CONFIG_HEADER ${RESUB_ZLIB_SOURCE_DIR}/zconf.h)
if(NOT EXISTS ${RESUB_ZLIB_CONFIG_HEADER})
    configure_file(
        ${RESUB_ZLIB_SOURCE_DIR}/zconf.h.in
        ${RESUB_ZLIB_CONFIG_HEADER}
        COPYONLY
    )
endif()

set(ZLIB_BUILD_SHARED OFF CACHE BOOL "Build only the pinned static Zlib" FORCE)
set(ZLIB_BUILD_STATIC ON CACHE BOOL "Build the pinned static Zlib" FORCE)
set(ZLIB_BUILD_TESTING OFF CACHE BOOL "Do not build Zlib examples" FORCE)
set(ZLIB_INSTALL OFF CACHE BOOL "Install Zlib through the bridge" FORCE)
add_subdirectory(${RESUB_ZLIB_SOURCE_DIR} zlib EXCLUDE_FROM_ALL)
set_target_properties(zlibstatic PROPERTIES
    OUTPUT_NAME resub_zlib
    POSITION_INDEPENDENT_CODE ON
)
add_library(ZLIB::ZLIB ALIAS zlibstatic)

list(PREPEND CMAKE_MODULE_PATH ${CMAKE_CURRENT_SOURCE_DIR}/cmake)
set(BUILD_SHARED_LIBS OFF CACHE BOOL "Build libaribtlv statically" FORCE)
set(BUILD_TESTING OFF CACHE BOOL "Do not build libaribtlv tests" FORCE)
add_subdirectory(
    ${RESUB_VENDORED_SOURCE_DIR}/libaribtlv
    libaribtlv
    EXCLUDE_FROM_ALL
)
set_target_properties(aribtlv PROPERTIES POSITION_INDEPENDENT_CODE ON)
if(MSVC)
    # libaribtlv 0.6.1 uses std::any_of/std::none_of without directly
    # including <algorithm>; force the standard header for the pinned PoC.
    # The cmake crate also supplies CXX flags that omit MSVC unwinding.
    target_compile_options(aribtlv PRIVATE /FIalgorithm /EHsc)
endif()

add_library(resub_aribtlv_bridge STATIC src/bridge.cpp)
target_compile_features(resub_aribtlv_bridge PUBLIC cxx_std_20)
if(MSVC)
    target_compile_options(resub_aribtlv_bridge PRIVATE /EHsc)
endif()
target_include_directories(resub_aribtlv_bridge PUBLIC include)
target_link_libraries(resub_aribtlv_bridge PUBLIC aribtlv)

install(TARGETS resub_aribtlv_bridge ARCHIVE DESTINATION lib)
# libaribtlv is added through an EXCLUDE_FROM_ALL subdirectory, so its own
# install rules are excluded as well. Install the archive explicitly for the
# cmake crate's isolated Cargo output directory.
install(TARGETS aribtlv ARCHIVE DESTINATION lib)
install(TARGETS zlibstatic EXPORT aribtlvTargets ARCHIVE DESTINATION lib)
install(FILES include/resub_aribtlv_bridge.h DESTINATION include)
