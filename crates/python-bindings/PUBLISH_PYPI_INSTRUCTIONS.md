# Python Package Publishing Instructions

Due to system limitations on the build environment, the Python package cannot be published directly from this system. Follow these instructions to publish the Python package to PyPI.

## Prerequisites

1. A system with Python 3.8+ properly installed
2. A PyPI account (https://pypi.org/account/register/)
3. Twine installed: `pip install twine`

## Publishing Steps

### 1. Clone the Repository

```bash
git clone https://github.com/OfflineIntelligence/offline-intelligence.git
cd offline-intelligence
```

### 2. Navigate to Python Bindings

```bash
cd crates/python-bindings
```

### 3. Build the Rust Extension

```bash
# Build the Rust extension module
cargo build --release

# Copy the built extension to the Python package
# On Windows:
copy ..\..\target\release\offline_intelligence_py.dll offline_intelligence_py\
# On Linux/macOS:
cp ../../target/release/liboffline_intelligence_py.so offline_intelligence_py/
```

### 4. Create Source Distribution

```bash
python setup.py sdist
```

### 5. Create Wheel Distribution

```bash
python setup.py bdist_wheel
```

### 6. Upload to PyPI

```bash
# For Test PyPI (recommended for testing first)
twine upload --repository testpypi dist/*

# For Production PyPI
twine upload dist/*
```

You'll be prompted for your PyPI username and password (or use API token).

## Package Details

- **Package Name**: `offline-intelligence`
- **Version**: 0.1.2
- **Description**: High-performance library for offline AI inference with context management and memory optimization
- **Homepage**: https://github.com/OfflineIntelligence/offline-intelligence
- **Author**: Offline Intelligence Team
- **License**: Apache-2.0

## Verification

After publishing, verify the package is available:

```bash
pip install offline-intelligence==0.1.2
```

Test the installation with the example code from README.md.

## Troubleshooting

Common issues:
1. Missing Rust toolchain - install from https://rustup.rs/
2. Permission errors - run with appropriate privileges
3. Missing dependencies - ensure all requirements are installed
4. Build errors - check that the Rust extension compiled successfully

## Alternative: Using pyproject.toml

The package also includes a `pyproject.toml` file for modern Python packaging:

```bash
pip install build
python -m build
twine upload dist/*
```

This approach uses the newer Python packaging standards and may be preferred for future releases.