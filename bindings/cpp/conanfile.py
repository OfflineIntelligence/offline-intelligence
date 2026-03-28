from conan import ConanFile
from conan.tools.cmake import CMakeToolchain, CMake, cmake_layout
from conan.tools.files import copy
import os


class OfflineIntelligenceConan(ConanFile):
    name = "offline-intelligence"
    version = "0.1.4"
    license = "Apache-2.0"
    author = "Offline Intelligence Team <intelligencedevelopment.io@gmail.com>"
    url = "https://github.com/OfflineIntelligence/offline-intelligence"
    homepage = "https://github.com/OfflineIntelligence/offline-intelligence"
    description = (
        "C++ HTTP client bindings for the Offline Intelligence server. "
        "Provides a header-only interface to run local AI inference via "
        "the Offline Intelligence Rust server (port 9999)."
    )
    topics = ("llm", "ai", "offline", "inference", "http-client", "local-ai")

    # Header-only — no compiled artifacts
    package_type = "header-library"
    settings = "os", "compiler", "build_type", "arch"
    exports_sources = "include/*"

    requires = (
        "cpp-httplib/0.15.3",
        "nlohmann_json/3.11.3",
    )

    def layout(self):
        cmake_layout(self)

    def package(self):
        # Copy headers from the binding's include directory
        copy(self, "*.hpp",
             src=os.path.join(self.source_folder, "include"),
             dst=os.path.join(self.package_folder, "include"),
             keep_path=True)
        # Fallback: recipe_folder (used during conan create)
        src_include = os.path.join(self.recipe_folder, "include")
        if os.path.isdir(src_include) and not os.path.samefile(
                self.source_folder, self.recipe_folder):
            copy(self, "*.hpp", src=src_include,
                 dst=os.path.join(self.package_folder, "include"),
                 keep_path=True)

    def package_info(self):
        self.cpp_info.bindirs = []
        self.cpp_info.libdirs = []
        self.cpp_info.set_property("cmake_file_name", "offline_intelligence")
        self.cpp_info.set_property("cmake_target_name", "offline_intelligence::offline_intelligence")
