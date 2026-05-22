from setuptools import setup, find_packages
import os

long_description = ""
readme_path = os.path.join(os.path.dirname(__file__), "README.md")
if os.path.exists(readme_path):
    with open(readme_path, encoding="utf-8") as f:
        long_description = f.read()

setup(
    name="offline-intelligence",
    version="0.1.6",
    author="Offline Intelligence Team",
    author_email="team@offlineintelligence.com",
    description="Python bindings for Offline Intelligence Library",
    long_description=long_description,
    long_description_content_type="text/markdown",
    url="https://github.com/offline-intelligence/offline-intelligence",
    packages=find_packages(exclude=["tests*", "examples*"]),
    python_requires=">=3.8",
    install_requires=[
        "requests>=2.28.0",
    ],
    extras_require={
        "streaming": ["sseclient-ng>=1.0.0"],
    },
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
        "Programming Language :: Python :: 3.12",
        "Topic :: Software Development :: Libraries :: Python Modules",
        "Topic :: Scientific/Engineering :: Artificial Intelligence",
    ],
    keywords=["llm", "ai", "offline", "intelligence", "local-ai", "http-client"],
    project_urls={
        "Bug Tracker": "https://github.com/offline-intelligence/offline-intelligence/issues",
        "Source": "https://github.com/offline-intelligence/offline-intelligence",
    },
)
