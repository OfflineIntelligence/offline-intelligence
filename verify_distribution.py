#!/usr/bin/env python3
"""
Verification script for Offline Intelligence Library distribution
Checks that all components are properly structured for multi-language distribution
"""

import os
import sys
from pathlib import Path

def check_directory_structure():
    """Verify the directory structure is correct"""
    print("🔍 Checking directory structure...")
    
    required_paths = [
        "crates/offline-intelligence/src/lib.rs",
        "crates/offline-intelligence/src/main.rs",
        "crates/offline-intelligence/Cargo.toml",
        "Cargo.toml",
        "README.md",
        "bindings/python/setup.py",
        "bindings/cpp/CMakeLists.txt",
        "bindings/javascript/package.json",
        "bindings/java/pom.xml"
    ]
    
    missing_paths = []
    for path in required_paths:
        if not Path(path).exists():
            missing_paths.append(path)
    
    if missing_paths:
        print(f"❌ Missing required paths:")
        for path in missing_paths:
            print(f"   - {path}")
        return False
    else:
        print("✅ All required paths present")
        return True

def check_lib_exports():
    """Check that lib.rs properly exports only core components"""
    print("\n🔍 Checking library exports...")
    
    lib_path = Path("crates/offline-intelligence/src/lib.rs")
    if not lib_path.exists():
        print("❌ lib.rs not found")
        return False
    
    content = lib_path.read_text(encoding='utf-8')
    
    private_components = ["context_engine", "cache_management"]
    private_found = []
    
    for component in private_components:
        if f"pub mod {component}" in content and not f"// pub mod {component}" in content:
            private_found.append(component)
    
    if private_found:
        print(f"⚠️  Found public exports of private components: {private_found}")
        print("   These should be commented out for the open-source release")
        return False
    else:
        print("✅ Private components properly hidden")
    
    core_components = ["config", "llm_integration", "metrics", "proxy", "admin"]
    missing_exports = []
    
    for component in core_components:
        if not f"pub mod {component}" in content:
            missing_exports.append(component)
    
    if missing_exports:
        print(f"❌ Missing exports of core components: {missing_exports}")
        return False
    else:
        print("✅ Core components properly exported")
    
    return True

def check_cargo_config():
    """Check Cargo configuration for library distribution"""
    print("\n🔍 Checking Cargo configuration...")
    
    cargo_path = Path("Cargo.toml")
    if not cargo_path.exists():
        print("❌ Workspace Cargo.toml not found")
        return False
    
    content = cargo_path.read_text(encoding='utf-8')
    
    required_sections = [
        "[profile.release]",
        "strip = \"symbols\"",
        "lto = \"thin\"",
        "codegen-units = 1"
    ]
    
    missing_sections = []
    for section in required_sections:
        if section not in content:
            missing_sections.append(section)
    
    if missing_sections:
        print(f"❌ Missing required Cargo configuration: {missing_sections}")
        return False
    else:
        print("✅ Cargo configuration properly set for distribution")
        return True

def check_bindings_structure():
    """Check that all binding directories exist with basic files"""
    print("\n🔍 Checking language binding structures...")
    
    bindings = {
        "python": ["setup.py", "README.md"],
        "cpp": ["CMakeLists.txt"],
        "javascript": ["package.json"],
        "java": ["pom.xml"]
    }
    
    missing_bindings = []
    
    for lang, required_files in bindings.items():
        lang_path = Path(f"bindings/{lang}")
        if not lang_path.exists():
            missing_bindings.append(lang)
            continue
            
        for file in required_files:
            if not (lang_path / file).exists():
                missing_bindings.append(f"{lang}/{file}")
    
    if missing_bindings:
        print(f"❌ Missing binding components: {missing_bindings}")
        return False
    else:
        print("✅ All binding structures present")
        return True

def check_build_scripts():
    """Check that build scripts exist"""
    print("\n🔍 Checking build scripts...")
    
    build_scripts = ["build.sh", "build.bat"]
    missing_scripts = []
    
    for script in build_scripts:
        if not Path(script).exists():
            missing_scripts.append(script)
    
    if missing_scripts:
        print(f"❌ Missing build scripts: {missing_scripts}")
        return False
    else:
        print("✅ Build scripts present")
        return True

def main():
    """Main verification function"""
    print("🧪 Offline Intelligence Library Verification")
    print("=" * 50)
    
    checks = [
        check_directory_structure,
        check_lib_exports,
        check_cargo_config,
        check_bindings_structure,
        check_build_scripts
    ]
    
    passed = 0
    total = len(checks)
    
    for check in checks:
        if check():
            passed += 1
    
    print("\n" + "=" * 50)
    print(f"📊 Verification Results: {passed}/{total} checks passed")
    
    if passed == total:
        print("🎉 All checks passed! Library is ready for distribution.")
        return 0
    else:
        print("❌ Some checks failed. Please review the issues above.")
        return 1

if __name__ == "__main__":
    sys.exit(main())
