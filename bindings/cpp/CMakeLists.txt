cmake_minimum_required(VERSION 3.16)
project(offline_intelligence_cpp VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Find required dependencies
find_package(PkgConfig REQUIRED)
find_package(Threads REQUIRED)

# Include directories
include_directories(${CMAKE_CURRENT_SOURCE_DIR}/../../crates/offline-intelligence/src)

# Create the library
add_library(offline_intelligence_cpp SHARED
    src/offline_intelligence.cpp
    src/offline_intelligence.h
)

# Link dependencies
target_link_libraries(offline_intelligence_cpp
    ${CMAKE_THREAD_LIBS_INIT}
)

# Install targets
install(TARGETS offline_intelligence_cpp
    LIBRARY DESTINATION lib
    ARCHIVE DESTINATION lib
    RUNTIME DESTINATION bin
)

install(FILES src/offline_intelligence.h
    DESTINATION include/offline_intelligence
)

# Export configuration
include(CMakePackageConfigHelpers)
write_basic_package_version_file(
    "${CMAKE_CURRENT_BINARY_DIR}/OfflineIntelligenceConfigVersion.cmake"
    VERSION ${PROJECT_VERSION}
    COMPATIBILITY SameMajorVersion
)

configure_package_config_file(
    "${CMAKE_CURRENT_SOURCE_DIR}/cmake/OfflineIntelligenceConfig.cmake.in"
    "${CMAKE_CURRENT_BINARY_DIR}/OfflineIntelligenceConfig.cmake"
    INSTALL_DESTINATION lib/cmake/OfflineIntelligence
)

install(FILES
    "${CMAKE_CURRENT_BINARY_DIR}/OfflineIntelligenceConfig.cmake"
    "${CMAKE_CURRENT_BINARY_DIR}/OfflineIntelligenceConfigVersion.cmake"
    DESTINATION lib/cmake/OfflineIntelligence
)