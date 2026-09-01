cmake_minimum_required(VERSION 3.24)
project(arib_caption_bridge LANGUAGES C CXX)

add_subdirectory(${CMAKE_CURRENT_SOURCE_DIR}/../../third_party/libaribcaption libaribcaption)

add_library(arib_caption_bridge STATIC src/bridge.cpp)
target_compile_features(arib_caption_bridge PUBLIC cxx_std_17)
target_include_directories(arib_caption_bridge PUBLIC include)
# The bridge is consumed as a static archive by Cargo.  Keep aribcaption and
# its platform font/runtime dependencies on the consumer link interface;
# PRIVATE would hide the transitive libraries on Linux and macOS.
target_link_libraries(arib_caption_bridge PUBLIC aribcaption)

install(TARGETS arib_caption_bridge aribcaption ARCHIVE DESTINATION lib)
install(FILES include/arib_caption_bridge.h DESTINATION include)
