if(TARGET ZLIB::ZLIB)
    set(ZLIB_FOUND TRUE)
    set(ZLIB_VERSION_STRING 1.3.2)
    set(ZLIB_LIBRARIES ZLIB::ZLIB)
    get_target_property(ZLIB_INCLUDE_DIRS zlibstatic INTERFACE_INCLUDE_DIRECTORIES)
    return()
endif()

include(${CMAKE_ROOT}/Modules/FindZLIB.cmake)
