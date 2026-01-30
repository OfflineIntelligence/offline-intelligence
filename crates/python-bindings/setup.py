from setuptools import setup, find_packages
from pyo3_setuptools import build_ext
import os

# Read the README file
with open("README.md", "r", encoding="utf-8") as fh:
    long_description = fh.read()

# Read requirements
with open("requirements.txt", "r", encoding="utf-8") as fh:
    requirements = [line.strip() for line in fh if line.strip() and not line.startswith("#")]

setup(
    name="offline-intelligence",
    version="0.1.1",
    author="Offline Intelligence Team",
    author_email="intelligencedevelopment.io@gmail.com",
    description="High-performance library for offline AI inference with context management and memory optimization",
    long_description=long_description,
    long_description_content_type="text/markdown",
    url="https://github.com/offline-intelligence/library",
    project_urls={
        "Bug Tracker": "https://github.com/offline-intelligence/library/issues",
        "Documentation": "https://github.com/offline-intelligence/library/wiki",
        "Source Code": "https://github.com/offline-intelligence/library",
    },
    packages=find_packages(),
    classifiers=[
        "Development Status :: 4 - Beta",
        "Intended Audience :: Developers",
        "Intended Audience :: Science/Research",
        "License :: OSI Approved :: Apache Software License",
        "Operating System :: OS Independent",
        "Programming Language :: Python :: 3",
        "Programming Language :: Python :: 3.8",
        "Programming Language :: Python :: 3.9",
        "Programming Language :: Python :: 3.10",
        "Programming Language :: Python :: 3.11",
        "Programming Language :: Python :: 3.12",
        "Programming Language :: Rust",
        "Topic :: Scientific/Engineering :: Artificial Intelligence",
        "Topic :: Software Development :: Libraries :: Python Modules",
    ],
    python_requires=">=3.8",
    install_requires=requirements,
    setup_requires=["setuptools", "wheel", "pyo3-setuptools"],
    cmdclass={"build_ext": build_ext},
    zip_safe=False,
    include_package_data=True,
    package_data={
        "": ["*.so", "*.dll", "*.dylib"],
    },
    keywords=[
        "ai", "llm", "offline", "context-management", "memory-search",
        "machine-learning", "natural-language-processing", "rust"
    ],
)