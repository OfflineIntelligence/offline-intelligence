cmake_minimum_required(VERSION 3.16)
project(offline_intelligence_cpp VERSION 0.1.4 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# ── Fetch external header-only dependencies ───────────────────────────────
include(FetchContent)

# cpp-httplib: single-header HTTP/1.1 client/server
FetchContent_Declare(
    httplib
    GIT_REPOSITORY https://github.com/yhirose/cpp-httplib.git
    GIT_TAG        v0.15.3
    GIT_SHALLOW    TRUE
)
FetchContent_MakeAvailable(httplib)

# nlohmann/json: single-header JSON
FetchContent_Declare(
    nlohmann_json
    GIT_REPOSITORY https://github.com/nlohmann/json.git
    GIT_TAG        v3.11.3
    GIT_SHALLOW    TRUE
)
FetchContent_MakeAvailable(nlohmann_json)

# ── Library target ─────────────────────────────────────────────────────

# Header-only interface library
add_library(offline_intelligence_cpp INTERFACE)
target_include_directories(offline_intelligence_cpp INTERFACE
    $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
    $<INSTALL_INTERFACE:include>
)
target_link_libraries(offline_intelligence_cpp INTERFACE
    httplib::httplib
    nlohmann_json::nlohmann_json
    Threads::Threads
)
find_package(Threads REQUIRED)

# ── Example executable ──────────────────────────────────────────────────

if(EXISTS "${CMAKE_CURRENT_SOURCE_DIR}/examples/example.cpp")
    add_executable(offline_intelligence_example
        examples/example.cpp
    )
    target_link_libraries(offline_intelligence_example PRIVATE
        offline_intelligence_cpp
    )
endif()

# ── Install ────────────────────────────────────────────────────────────

install(TARGETS offline_intelligence_cpp
    EXPORT OfflineIntelligenceTargets
)

install(
    DIRECTORY include/
    DESTINATION include
)

include(CMakePackageConfigHelpers)
write_basic_package_version_file(
    "${CMAKE_CURRENT_BINARY_DIR}/OfflineIntelligenceConfigVersion.cmake"
    VERSION ${PROJECT_VERSION}
    COMPATIBILITY SameMajorVersion
)