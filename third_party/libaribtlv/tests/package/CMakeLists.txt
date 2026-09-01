cmake_minimum_required(VERSION 3.16)
project(aribtlv-package-consumer LANGUAGES C CXX)

find_package(aribtlv CONFIG REQUIRED)

add_executable(aribtlv-package-consumer main.cpp)
target_link_libraries(aribtlv-package-consumer PRIVATE aribtlv::aribtlv)

add_executable(aribtlv-c-package-consumer main.c)
target_link_libraries(aribtlv-c-package-consumer PRIVATE aribtlv::aribtlv)
