cmake_minimum_required(VERSION 3.24)
project(arib_caption_bridge LANGUAGES C CXX)

add_subdirectory(${CMAKE_CURRENT_SOURCE_DIR}/../../third_party/libaribcaption libaribcaption)

add_library(arib_caption_bridge STATIC src/bridge.cpp)
target_compile_features(arib_caption_bridge PUBLIC cxx_std_17)
target_include_directories(arib_caption_bridge PUBLIC include)
target_link_libraries(arib_caption_bridge PRIVATE aribcaption)

install(TARGETS arib_caption_bridge aribcaption ARCHIVE DESTINATION lib)
install(FILES include/arib_caption_bridge.h DESTINATION include)
