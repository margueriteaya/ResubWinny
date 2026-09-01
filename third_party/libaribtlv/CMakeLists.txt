cmake_minimum_required(VERSION 3.16)

project(aribtlv VERSION 0.6.1 LANGUAGES C CXX)

include(CTest)
include(GNUInstallDirs)
include(CMakePackageConfigHelpers)

option(BUILD_SHARED_LIBS "Build libaribtlv as a shared library" ON)

if(NOT EMSCRIPTEN)
    find_package(ZLIB REQUIRED)
endif()

add_library(aribtlv
    src/asset_layers.cpp
    src/c_api.cpp
    src/application_resources.cpp
    src/compressed_ip_parser.cpp
    src/demuxer.cpp
    src/duration_probe.cpp
    src/mmtp_media_parser.cpp
    src/mmtp_parser.cpp
    src/mmtp_signalling_parser.cpp
    src/mmtp_signalling_tables.cpp
    src/recording.cpp
    src/tlv_parser.cpp
    src/tlv_transport_parser.cpp
    src/video_presentation.cpp
)
add_library(aribtlv::aribtlv ALIAS aribtlv)

set_target_properties(aribtlv PROPERTIES
    CXX_EXTENSIONS OFF
    OUTPUT_NAME aribtlv
    VERSION ${PROJECT_VERSION}
    SOVERSION ${PROJECT_VERSION_MAJOR}
)

target_compile_features(aribtlv PUBLIC cxx_std_20)
target_compile_definitions(aribtlv PRIVATE
    ARIBTLV_BUILDING_LIBRARY
    ARIBTLV_VERSION_STRING="${PROJECT_VERSION}")
if(NOT BUILD_SHARED_LIBS)
    target_compile_definitions(aribtlv PUBLIC ARIBTLV_STATIC)
endif()
if(EMSCRIPTEN)
    target_compile_options(aribtlv PRIVATE "-sUSE_ZLIB=1")
    target_link_options(aribtlv PUBLIC "-sUSE_ZLIB=1")
else()
    target_link_libraries(aribtlv PUBLIC ZLIB::ZLIB)
endif()
target_include_directories(aribtlv
    PUBLIC
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
        $<INSTALL_INTERFACE:${CMAKE_INSTALL_INCLUDEDIR}>
    PRIVATE
        ${CMAKE_CURRENT_SOURCE_DIR}/src
)

if(MSVC)
    target_compile_options(aribtlv PRIVATE /W4 /permissive-)
else()
    target_compile_options(aribtlv PRIVATE
        -Wall -Wextra -Wpedantic -Wconversion -Wsign-conversion)
endif()

install(TARGETS aribtlv
    EXPORT aribtlvTargets
    LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}
    ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR}
    RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR}
    INCLUDES DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}
)
install(DIRECTORY include/ DESTINATION ${CMAKE_INSTALL_INCLUDEDIR})
install(FILES LICENSE DESTINATION ${CMAKE_INSTALL_DATADIR}/licenses/aribtlv)
install(EXPORT aribtlvTargets
    FILE aribtlvTargets.cmake
    NAMESPACE aribtlv::
    DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/aribtlv
)
configure_package_config_file(
    cmake/aribtlvConfig.cmake.in
    ${CMAKE_CURRENT_BINARY_DIR}/aribtlvConfig.cmake
    INSTALL_DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/aribtlv
)
write_basic_package_version_file(
    ${CMAKE_CURRENT_BINARY_DIR}/aribtlvConfigVersion.cmake
    VERSION ${PROJECT_VERSION}
    COMPATIBILITY SameMajorVersion
)
install(FILES
    ${CMAKE_CURRENT_BINARY_DIR}/aribtlvConfig.cmake
    ${CMAKE_CURRENT_BINARY_DIR}/aribtlvConfigVersion.cmake
    DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/aribtlv
)

if(APPLE)
    set(ARIBTLV_CXX_RUNTIME_LIBRARY "-lc++")
elseif(MSVC)
    set(ARIBTLV_CXX_RUNTIME_LIBRARY "")
else()
    set(ARIBTLV_CXX_RUNTIME_LIBRARY "-lstdc++")
endif()
configure_file(cmake/libaribtlv.pc.in libaribtlv.pc @ONLY)
install(FILES ${CMAKE_CURRENT_BINARY_DIR}/libaribtlv.pc
        DESTINATION ${CMAKE_INSTALL_LIBDIR}/pkgconfig)

if(BUILD_TESTING)
    add_executable(aribtlv-asset-layers-tests tests/asset_layers_tests.cpp)
    target_link_libraries(aribtlv-asset-layers-tests PRIVATE aribtlv::aribtlv)

    add_executable(aribtlv-video-color-tests tests/video_color_tests.cpp)
    target_link_libraries(aribtlv-video-color-tests PRIVATE aribtlv::aribtlv)
    add_test(NAME aribtlv-video-color-tests COMMAND aribtlv-video-color-tests)

    add_executable(aribtlv-application-resources-tests
        tests/application_resources_tests.cpp)
    target_link_libraries(aribtlv-application-resources-tests PRIVATE aribtlv::aribtlv)

    add_executable(aribtlv-demuxer-tests tests/demuxer_tests.cpp)
    target_link_libraries(aribtlv-demuxer-tests PRIVATE aribtlv::aribtlv)

    add_executable(aribtlv-timestamp-mpu-tests tests/timestamp_mpu_tests.cpp)
    target_link_libraries(aribtlv-timestamp-mpu-tests PRIVATE aribtlv::aribtlv)

    add_executable(aribtlv-tlv-transport-tests tests/tlv_transport_tests.cpp)
    target_link_libraries(aribtlv-tlv-transport-tests PRIVATE aribtlv::aribtlv)

    add_executable(aribtlv-recording-tests tests/recording_tests.cpp)
    target_link_libraries(aribtlv-recording-tests PRIVATE aribtlv::aribtlv)

    add_executable(aribtlv-hlg-sdr-tone-mapping-tests
        tests/hlg_sdr_tone_mapping_tests.cpp)
    target_link_libraries(aribtlv-hlg-sdr-tone-mapping-tests PRIVATE aribtlv::aribtlv)

    add_executable(aribtlv-package-api-tests tests/package/main.cpp)
    target_link_libraries(aribtlv-package-api-tests PRIVATE aribtlv::aribtlv)

    add_executable(aribtlv-c-api-tests tests/c_api_tests.c)
    target_link_libraries(aribtlv-c-api-tests PRIVATE aribtlv::aribtlv)

    foreach(test_target IN ITEMS
            aribtlv-asset-layers-tests
            aribtlv-application-resources-tests
            aribtlv-demuxer-tests
            aribtlv-timestamp-mpu-tests
            aribtlv-tlv-transport-tests
            aribtlv-recording-tests
            aribtlv-hlg-sdr-tone-mapping-tests
            aribtlv-package-api-tests
            aribtlv-c-api-tests)
        if(MSVC)
            target_compile_options(${test_target} PRIVATE /W4 /permissive-)
        else()
            target_compile_options(${test_target} PRIVATE -Wall -Wextra -Wpedantic)
        endif()
    endforeach()

    add_test(NAME aribtlv-asset-layers-tests COMMAND aribtlv-asset-layers-tests)
    add_test(NAME aribtlv-application-resources-tests
             COMMAND aribtlv-application-resources-tests)
    add_test(NAME aribtlv-demuxer-tests COMMAND aribtlv-demuxer-tests)
    add_test(NAME aribtlv-timestamp-mpu-tests COMMAND aribtlv-timestamp-mpu-tests)
    add_test(NAME aribtlv-tlv-transport-tests COMMAND aribtlv-tlv-transport-tests)
    add_test(NAME aribtlv-recording-tests COMMAND aribtlv-recording-tests)
    add_test(NAME aribtlv-hlg-sdr-tone-mapping-tests
             COMMAND aribtlv-hlg-sdr-tone-mapping-tests)
    add_test(NAME aribtlv-package-api-tests COMMAND aribtlv-package-api-tests)
    add_test(NAME aribtlv-c-api-tests COMMAND aribtlv-c-api-tests)
endif()
