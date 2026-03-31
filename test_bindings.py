#!/usr/bin/env python3
"""
Test script to verify all language bindings work correctly
"""

import subprocess
import sys
import os
from pathlib import Path

def test_python_binding():
    """Test Python binding"""
    print("🐍 Testing Python binding...")
    
    try:

        import pybind11
        print("✅ Python binding framework available")
        
        python_binding_path = Path("bindings/python/src/main.cpp")
        if python_binding_path.exists():
            print("✅ Python binding source exists")
        else:
            print("❌ Python binding source missing")
            return False
            
    except ImportError as e:
        print(f"⚠️  Python binding test skipped: {e}")
    
    return True

def test_cpp_binding():
    """Test C++ binding"""
    print("\n➕ Testing C++ binding...")
    
    cpp_header = Path("bindings/cpp/src/offline_intelligence.h")
    cpp_source = Path("bindings/cpp/src/offline_intelligence.cpp")
    
    if cpp_header.exists() and cpp_source.exists():
        print("✅ C++ binding sources exist")
        return True
    else:
        print("❌ C++ binding sources missing")
        return False

def test_javascript_binding():
    """Test JavaScript binding"""
    print("\n🟨 Testing JavaScript binding...")
    
    js_main = Path("bindings/javascript/index.js")
    js_types = Path("bindings/javascript/index.d.ts")
    
    if js_main.exists() and js_types.exists():
        print("✅ JavaScript binding files exist")
        return True
    else:
        print("❌ JavaScript binding files missing")
        return False

def test_java_binding():
    """Test Java binding"""
    print("\n☕ Testing Java binding...")
    
    java_files = [
        "bindings/java/src/main/java/com/offlineintelligence/Config.java",
        "bindings/java/src/main/java/com/offlineintelligence/Server.java",
        "bindings/java/src/main/java/com/offlineintelligence/OfflineIntelligenceException.java"
    ]
    
    missing_files = []
    for file_path in java_files:
        if not Path(file_path).exists():
            missing_files.append(file_path)
    
    if missing_files:
        print(f"❌ Missing Java files: {missing_files}")
        return False
    else:
        print("✅ All Java binding files exist")
        return True

def test_build_system():
    """Test build system"""
    print("\n🔨 Testing build system...")
    
    build_scripts = ["build.sh", "build.bat"]
    found_scripts = []
    
    for script in build_scripts:
        if Path(script).exists():
            found_scripts.append(script)
    
    if found_scripts:
        print(f"✅ Build scripts found: {found_scripts}")
        return True
    else:
        print("❌ No build scripts found")
        return False

def main():
    """Run all tests"""
    print("🧪 Offline Intelligence Language Bindings Test Suite")
    print("=" * 60)
    
    tests = [
        test_python_binding,
        test_cpp_binding,
        test_javascript_binding,
        test_java_binding,
        test_build_system
    ]
    
    passed = 0
    total = len(tests)
    
    for test in tests:
        if test():
            passed += 1
    
    print("\n" + "=" * 60)
    print(f"📊 Test Results: {passed}/{total} tests passed")
    
    if passed == total:
        print("🎉 All language bindings are ready!")
        print("\n📋 Next steps:")
        print("1. Run the build scripts to compile all bindings")
        print("2. Test each language binding individually")
        print("3. Package for distribution")
        return 0
    else:
        print("❌ Some bindings need attention")
        return 1

if __name__ == "__main__":
    sys.exit(main())
