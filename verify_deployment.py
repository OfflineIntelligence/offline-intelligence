#!/usr/bin/env python3
"""
Offline Intelligence - Deployment Verification Script
Verifies that all deployment artifacts are correctly generated and ready for publishing
"""

import os
import sys
import json
import subprocess
from pathlib import Path
from datetime import datetime

def check_file_exists(filepath, description):
    """Check if a file exists and report status"""
    path = Path(filepath)
    if path.exists():
        size_kb = path.stat().st_size / 1024
        print(f"✅ {description}: {filepath} ({size_kb:.2f} KB)")
        return True
    else:
        print(f"❌ {description}: {filepath} (NOT FOUND)")
        return False

def run_command(command, description, cwd=None):
    """Run a command and check if it succeeds"""
    try:
        result = subprocess.run(
            command, 
            shell=True, 
            cwd=cwd,
            capture_output=True, 
            text=True
        )
        if result.returncode == 0:
            print(f"✅ {description}: PASSED")
            return True
        else:
            print(f"❌ {description}: FAILED")
            print(f"   Error: {result.stderr.strip()}")
            return False
    except Exception as e:
        print(f"❌ {description}: ERROR - {e}")
        return False

def verify_rust_deployment():
    """Verify Rust crate deployment readiness"""
    print("\n🦀 Verifying Rust Deployment...")
    
    checks = [
        ("Cargo.toml", "crates/offline-intelligence/Cargo.toml"),
        ("README.md", "README.md"),
        ("LICENSE", "LICENSE"),
        ("lib.rs", "crates/offline-intelligence/src/lib.rs"),
        ("main.rs", "crates/offline-intelligence/src/main.rs")
    ]
    
    all_passed = True
    for desc, filepath in checks:
        if not check_file_exists(filepath, f"Rust {desc}"):
            all_passed = False
    
    if all_passed:
        cargo_check = run_command(
            "cargo publish --dry-run --allow-dirty",
            "Cargo publish validation",
            "crates/offline-intelligence"
        )
        all_passed = all_passed and cargo_check
    
    return all_passed

def verify_python_deployment():
    """Verify Python package deployment readiness"""
    print("\n🐍 Verifying Python Deployment...")
    
    checks = [
        ("setup.py", "bindings/python/setup.py"),
        ("main.cpp", "bindings/python/src/main.cpp"),
        ("README.md", "bindings/python/README.md")
    ]
    
    all_passed = True
    for desc, filepath in checks:
        if not check_file_exists(filepath, f"Python {desc}"):
            all_passed = False
    
    if all_passed:
        build_check = run_command(
            "python setup.py sdist bdist_wheel --dry-run",
            "Python package build validation",
            "bindings/python"
        )
        all_passed = all_passed and build_check
    
    return all_passed

def verify_javascript_deployment():
    """Verify JavaScript package deployment readiness"""
    print("\n🟨 Verifying JavaScript Deployment...")
    
    checks = [
        ("package.json", "bindings/javascript/package.json"),
        ("index.js", "bindings/javascript/index.js"),
        ("index.d.ts", "bindings/javascript/index.d.ts")
    ]
    
    all_passed = True
    for desc, filepath in checks:
        if not check_file_exists(filepath, f"JavaScript {desc}"):
            all_passed = False
    
    if all_passed:
        npm_check = run_command(
            "npm pack --dry-run",
            "npm package validation",
            "bindings/javascript"
        )
        all_passed = all_passed and npm_check
    
    return all_passed

def verify_java_deployment():
    """Verify Java package deployment readiness"""
    print("\n☕ Verifying Java Deployment...")
    
    checks = [
        ("pom.xml", "bindings/java/pom.xml"),
        ("Config.java", "bindings/java/src/main/java/com/offlineintelligence/Config.java"),
        ("Server.java", "bindings/java/src/main/java/com/offlineintelligence/Server.java")
    ]
    
    all_passed = True
    for desc, filepath in checks:
        if not check_file_exists(filepath, f"Java {desc}"):
            all_passed = False
    
    if all_passed:
        maven_check = run_command(
            "mvn compile test-compile",
            "Maven build validation",
            "bindings/java"
        )
        all_passed = all_passed and maven_check
    
    return all_passed

def verify_cpp_deployment():
    """Verify C++ deployment readiness"""
    print("\n➕ Verifying C++ Deployment...")
    
    checks = [
        ("CMakeLists.txt", "bindings/cpp/CMakeLists.txt"),
        ("header file", "bindings/cpp/src/offline_intelligence.h"),
        ("source file", "bindings/cpp/src/offline_intelligence.cpp")
    ]
    
    all_passed = True
    for desc, filepath in checks:
        if not check_file_exists(filepath, f"C++ {desc}"):
            all_passed = False
    
    return all_passed

def verify_build_system():
    """Verify build system components"""
    print("\n🔨 Verifying Build System...")
    
    checks = [
        ("build script", "build.sh"),
        ("build script", "build.bat"),
        ("deploy script", "deploy.sh"),
        ("deploy script", "deploy.bat"),
        ("documentation", "DEPLOYMENT.md"),
        ("examples", "USAGE_EXAMPLES.md")
    ]
    
    all_passed = True
    for desc, filepath in checks:
        if not check_file_exists(filepath, f"Build {desc}"):
            all_passed = False
    
    return all_passed

def create_deployment_report(results):
    """Create a deployment readiness report"""
    timestamp = datetime.now().isoformat()
    
    report = {
        "project": "Offline Intelligence",
        "version": "0.1.1",
        "repository": "https://github.com/OfflineIntelligence/offline-intelligence",
        "timestamp": timestamp,
        "deployment_readiness": {
            "rust": results["rust"],
            "python": results["python"],
            "javascript": results["javascript"],
            "java": results["java"],
            "cpp": results["cpp"],
            "build_system": results["build_system"]
        },
        "overall_status": "READY" if all(results.values()) else "NOT_READY",
        "ready_for_deployment": list([k for k, v in results.items() if v]),
        "needs_attention": list([k for k, v in results.items() if not v])
    }
    
    with open("DEPLOYMENT_VERIFICATION_REPORT.json", "w", encoding="utf-8") as f:
        json.dump(report, f, indent=2)
    
    return report

def main():
    """Main verification function"""
    print("🧪 Offline Intelligence Deployment Verification")
    print("=" * 50)
    
    results = {
        "rust": verify_rust_deployment(),
        "python": verify_python_deployment(),
        "javascript": verify_javascript_deployment(),
        "java": verify_java_deployment(),
        "cpp": verify_cpp_deployment(),
        "build_system": verify_build_system()
    }
    
    report = create_deployment_report(results)
    
    print("\n" + "=" * 50)
    print("📊 DEPLOYMENT VERIFICATION SUMMARY")
    print("=" * 50)
    
    for component, status in results.items():
        status_icon = "✅" if status else "❌"
        print(f"{status_icon} {component.upper()}: {'READY' if status else 'NOT READY'}")
    
    print(f"\n📈 Overall Status: {report['overall_status']}")
    
    if report["ready_for_deployment"]:
        print(f"✅ Ready for deployment: {', '.join(report['ready_for_deployment'])}")
    
    if report["needs_attention"]:
        print(f"❌ Needs attention: {', '.join(report['needs_attention'])}")
    
    print(f"\n📄 Detailed report saved to: DEPLOYMENT_VERIFICATION_REPORT.json")
    
    return 0 if report["overall_status"] == "READY" else 1

if __name__ == "__main__":
    sys.exit(main())
