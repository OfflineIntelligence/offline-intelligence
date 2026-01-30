# Publishing Guide

This document explains how to publish the Offline Intelligence Library to various package managers and registries.

## Prerequisites

Before publishing, ensure you have:

1. **Accounts and Authentication:**
   - PyPI account and `~/.pypirc` configured
   - npm account and `npm login` completed  
   - crates.io account and `cargo login` completed
   - GitHub account with repository access

2. **Tools Installed:**
   - Rust toolchain (`rustc`, `cargo`)
   - Python 3.8+ with `pip`, `twine`
   - Node.js and npm
   - Java JDK 8+ and Maven/Gradle (for testing)
   - Git

3. **Environment Variables:**
   ```bash
   # For Maven Central (optional)
   export OSSRH_USERNAME=your_username
   export OSSRH_PASSWORD=your_password
   ```

## Version Management

All packages use the same version number defined in:
- `crates/offline-intelligence/Cargo.toml` (canonical source)
- Sync versions in binding packages before publishing

## Current Publication Status (as of v0.1.2)

✅ **Published Successfully:**
- `offline-intelligence` v0.1.2 (core library) - crates.io
- `offline_intelligence_java` v0.1.2 (Java bindings) - crates.io
- `offline_intelligence_js` v0.1.2 (JavaScript bindings) - crates.io
- `offline_intelligence_cpp` v0.1.2 (C++ bindings) - crates.io
- `offline-intelligence-sdk` v0.1.2 (JavaScript package) - npm
- Java package available via JitPack (com.github.OfflineIntelligence:offline-intelligence:v0.1.2)

❌ **Pending Publication:**
- Python bindings - Blocked (Python not installed on build system)

## Publishing Process

### Option 1: Automated Publishing (Recommended)

Run the comprehensive publish script:

**Linux/macOS:**
```bash
./scripts/publish-all.sh
```

**Windows:**
```cmd
scripts\publish-all.bat
```

This script will:
- Build all components
- Create and push Git tag
- Publish to crates.io
- Publish to PyPI (if twine available)
- Publish to npm (if npm available)
- Prepare Maven/JitPack release

### Option 2: Manual Platform-by-Platform

#### 1. Rust (crates.io)
```bash
cd crates/offline-intelligence
cargo publish
```

#### 2. Python (PyPI)
```bash
cd crates/python-bindings
python setup.py sdist bdist_wheel
twine upload dist/*
```

#### 3. JavaScript (npm)
```bash
cd crates/js-bindings
npm publish
```

#### 4. Java (Maven/JitPack)
```bash
# Push to GitHub - JitPack builds automatically
git tag v0.1.1
git push origin v0.1.1
```

Then visit: `https://jitpack.io/#offline-intelligence/library/v0.1.1`

#### 5. C++ (Manual Distribution)
For C++, distribute the header files and compiled binaries:
- Upload to GitHub Releases
- Submit to Conan Center Index
- Submit to vcpkg registry

## Post-Publish Checklist

After publishing, verify:

- [ ] All packages are searchable on their respective registries
- [ ] Documentation links work correctly
- [ ] Installation instructions work
- [ ] GitHub release is created with changelog
- [ ] Examples in documentation are updated

## Troubleshooting

### Common Issues

1. **Authentication failures:**
   - Re-login to respective services
   - Check credentials and permissions

2. **Version conflicts:**
   - Ensure version numbers are synchronized
   - Check that version doesn't already exist

3. **Build failures:**
   - Run `./build.sh` to test locally first
   - Check platform-specific requirements

4. **Publish restrictions:**
   - Some registries have rate limits
   - New accounts may need verification

### Registry-Specific Notes

- **crates.io:** 10-minute cooldown between versions
- **PyPI:** Requires 2FA for new accounts
- **npm:** Scoped packages (@offline-intelligence/) require organization setup
- **JitPack:** Builds on-demand, first build may take several minutes

## Security Considerations

- Never commit credentials to repository
- Use CI/CD for automated publishing when possible
- Sign packages where supported
- Verify package integrity before publishing

## Support

For issues with publishing:
1. Check the specific registry's documentation
2. Review error messages carefully
3. Test locally before attempting to publish
4. Contact registry support if needed