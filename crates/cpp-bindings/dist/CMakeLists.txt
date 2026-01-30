cmake_minimum_required(VERSION 3.12)
project(offline-intelligence-cpp VERSION 0.1.2 LANGUAGES CXX)

# Add the include directory
target_include_directories(${PROJECT_NAME} INTERFACE
    ${CMAKE_CURRENT_SOURCE_DIR}/include
)

# Add the library
add_library(${PROJECT_NAME} SHARED IMPORTED)
set_target_properties(${PROJECT_NAME} PROPERTIES
    IMPORTED_LOCATION ${CMAKE_CURRENT_SOURCE_DIR}/offline_intelligence_cpp.dll
    INTERFACE_INCLUDE_DIRECTORIES ${CMAKE_CURRENT_SOURCE_DIR}/include
)

# Install targets
install(TARGETS ${PROJECT_NAME}
    LIBRARY DESTINATION lib
    RUNTIME DESTINATION bin
)
install(DIRECTORY include/ DESTINATION include)