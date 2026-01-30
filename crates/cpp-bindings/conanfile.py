#!/usr/bin/env python3
from conans import ConanFile, CMake, tools
import os

class OfflineIntelligenceConan(ConanFile):
    name = "offline-intelligence"
    version = "0.1.1"
    license = "Apache-2.0"
    author = "Offline Intelligence Team intelligencedevelopment.io@gmail.com"
    url = "https://github.com/offline-intelligence/library"
    description = "High-performance library for offline AI inference with context management and memory optimization"
    topics = ("ai", "llm", "offline", "context-management", "memory-search")
    settings = "os", "compiler", "build_type", "arch"
    options = {"shared": [True, False]}
    default_options = {"shared": False}
    generators = "cmake"
    exports_sources = "src/*", "include/*", "CMakeLists.txt", "cmake/*"

    def build(self):
        cmake = CMake(self)
        cmake.configure()
        cmake.build()

    def package(self):
        self.copy("*.h", dst="include", src="include")
        self.copy("*.hpp", dst="include", src="include")
        self.copy("*.lib", dst="lib", keep_path=False)
        self.copy("*.dll", dst="bin", keep_path=False)
        self.copy("*.so", dst="lib", keep_path=False)
        self.copy("*.dylib", dst="lib", keep_path=False)
        self.copy("*.a", dst="lib", keep_path=False)

    def package_info(self):
        self.cpp_info.libs = ["offline_intelligence_cpp"]
        self.cpp_info.includedirs = ["include"]


        self.cpp_info.cxxflags = ["-std=c++17"] if self.settings.compiler != "Visual Studio" else ["/std:c++17"]

