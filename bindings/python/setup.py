from setuptools import setup, find_packages
from pybind11.setup_helpers import Pybind11Extension, build_ext
import os

# Define the extension module
ext_modules = [
    Pybind11Extension(
        "offline_intelligence_py",
        ["src/main.cpp"],
        include_dirs=["../../crates/offline-intelligence/src"],
        cxx_std=17,
        define_macros=[("VERSION_INFO", "0.1.0")],
    ),
]

setup(
    name="offline-intelligence",
    version="0.1.0",
    author="Offline Intelligence Team",
    author_email="team@offlineintelligence.com",
    description="Python bindings for Offline Intelligence Library",
    long_description=open("README.md").read(),
    long_description_content_type="text/markdown",
    url="https://github.com/offline-intelligence/offline-intelligence",
    packages=find_packages(),
    ext_modules=ext_modules,
    cmdclass={"build_ext": build_ext},
    zip_safe=False,
    python_requires=">=3.8",
    install_requires=[
        "pybind11>=2.10.0",
    ],
    classifiers=[
        "Development Status :: 4 - Beta",
        "Intended Audience :: Developers",
        "License :: OSI Approved :: Apache Software License",
        "Operating System :: OS Independent",
        "Programming Language :: Python :: 3",
        "Programming Language :: Python :: 3.8",
        "Programming Language :: Python :: 3.9",
        "Programming Language :: Python :: 3.10",
        "Programming Language :: Python :: 3.11",
        "Programming Language :: Rust",
    ],
)