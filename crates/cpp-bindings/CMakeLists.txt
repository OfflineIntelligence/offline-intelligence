cmake_minimum_required(VERSION 3.12)
project(offline_intelligence_cpp VERSION 0.1.1 LANGUAGES CXX)

# Project information
set(PROJECT_DESCRIPTION "High-performance library for offline AI inference with context management and memory optimization")
set(PROJECT_HOMEPAGE_URL "https://github.com/offline-intelligence/library")
set(PROJECT_LICENSE "Apache-2.0")

# Set C++ standard
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# Compiler-specific options
if(MSVC)
    add_compile_options(/W4)
else()
    add_compile_options(-Wall -Wextra -pedantic)
endif()

# Find required packages
find_package(PkgConfig REQUIRED)

# Add library target
add_library(offline_intelligence_cpp INTERFACE)

# Set target properties
target_include_directories(offline_intelligence_cpp INTERFACE
    $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
    $<INSTALL_INTERFACE:include>
)

target_compile_features(offline_intelligence_cpp INTERFACE cxx_std_17)

# Add alias for imported target
add_library(offline_intelligence::offline_intelligence_cpp ALIAS offline_intelligence_cpp)

# Install headers
install(DIRECTORY include/ 
    DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}
    COMPONENT Devel
)

# Install library target
install(TARGETS offline_intelligence_cpp
    EXPORT offline_intelligence_cppTargets
    LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}
    ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR}
    RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR}
    INCLUDES DESTINATION ${CMAKE_INSTALL_INCLUDEDIR}
    COMPONENT Devel
)

# Install export set
install(EXPORT offline_intelligence_cppTargets
    FILE offline_intelligence_cppTargets.cmake
    NAMESPACE offline_intelligence::
    DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/offline_intelligence_cpp
    COMPONENT Devel
)

# Create package config files
include(CMakePackageConfigHelpers)

# Configure the config file
configure_package_config_file(
    "${CMAKE_CURRENT_SOURCE_DIR}/cmake/offline_intelligence_cppConfig.cmake.in"
    "${CMAKE_CURRENT_BINARY_DIR}/offline_intelligence_cppConfig.cmake"
    INSTALL_DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/offline_intelligence_cpp
)

# Create version file
write_basic_package_version_file(
    "${CMAKE_CURRENT_BINARY_DIR}/offline_intelligence_cppConfigVersion.cmake"
    VERSION ${PROJECT_VERSION}
    COMPATIBILITY SameMajorVersion
)

# Install config files
install(FILES
    "${CMAKE_CURRENT_BINARY_DIR}/offline_intelligence_cppConfig.cmake"
    "${CMAKE_CURRENT_BINARY_DIR}/offline_intelligence_cppConfigVersion.cmake"
    DESTINATION ${CMAKE_INSTALL_LIBDIR}/cmake/offline_intelligence_cpp
    COMPONENT Devel
)

# CPack configuration for packaging
set(CPACK_PACKAGE_NAME "offline-intelligence-cpp")
set(CPACK_PACKAGE_VENDOR "Offline Intelligence Team")
set(CPACK_PACKAGE_DESCRIPTION_SUMMARY "${PROJECT_DESCRIPTION}")
set(CPACK_PACKAGE_VERSION "${PROJECT_VERSION}")
set(CPACK_PACKAGE_CONTACT "intelligencedevelopment.io@gmail.com")
set(CPACK_RESOURCE_FILE_LICENSE "${CMAKE_CURRENT_SOURCE_DIR}/LICENSE")
set(CPACK_RESOURCE_FILE_README "${CMAKE_CURRENT_SOURCE_DIR}/README.md")

# Component installation
set(CPACK_COMPONENTS_ALL Devel)
set(CPACK_COMPONENT_DEVEL_DISPLAY_NAME "C++ Development Files")
set(CPACK_COMPONENT_DEVEL_DESCRIPTION "Header files and CMake config for development")

include(CPack)