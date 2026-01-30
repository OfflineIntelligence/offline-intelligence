# Release v0.1.2

## What's Changed

This release introduces multi-language bindings and improved library structure while maintaining all core functionality.

## 🚀 New Features

### Multi-Language Support
- **Python Bindings**: Full Python SDK with PyO3 integration
- **JavaScript/Node.js**: N-API bindings for Node.js applications
- **Java**: JNI bindings with Maven/JitPack support
- **C++**: C FFI bindings with CMake integration

### Core Improvements
- Library-only distribution (frontend components removed)
- Enhanced API surface for external integrations
- Improved memory management and context optimization
- Better error handling and logging

## 📦 Package Availability

- **crates.io**: `offline-intelligence v0.1.2` ✅
- **npm**: `@offline-intelligence/sdk v0.1.2` ⏳ (awaiting authentication)
- **PyPI**: `offline-intelligence v0.1.2` ⏳ (awaiting Python setup)
- **Maven/JitPack**: `io.intelligencedevelopment:offline-intelligence:0.1.2` ⏳ (triggered by this release)
- **C++**: Header files and CMake configuration available

## 🛠️ Technical Changes

### Breaking Changes
- Frontend components removed from library distribution
- Resources folder excluded from git tracking
- CLI features now optional behind feature flags

### Dependencies
- Updated to latest compatible versions
- Reduced binary size by excluding unnecessary components
- Improved cross-platform compatibility

## 📖 Documentation

See [PUBLISHING.md](PUBLISHING.md) for detailed installation and usage instructions for each language.

## 🐛 Bug Fixes

- Fixed version consistency across all binding packages
- Resolved filename collision warnings
- Improved build reliability across platforms

---

**Full Changelog**: https://github.com/OfflineIntelligence/offline-intelligence/compare/v0.1.1...v0.1.2