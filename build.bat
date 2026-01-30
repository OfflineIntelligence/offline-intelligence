@echo off
setlocal enabledelayedexpansion

REM Build script for Offline Intelligence Library - Multi-language compilation (Windows)
REM This script builds the core library and all language bindings

echo === Offline Intelligence Library Build Script ===
echo Building core library and all language bindings
echo.

REM Check prerequisites
echo [INFO] Checking prerequisites...

REM Check Rust
rustc --version >nul 2>&1
if %errorlevel% neq 0 (
    echo [ERROR] Rust is not installed. Please install Rust from https://rustup.rs/
    exit /b 1
)

rustc --version
echo.

REM Check if we're in the right directory
if not exist "Cargo.toml" (
    echo [ERROR] Cargo.toml not found. Please run this script from the repository root.
    exit /b 1
)

REM Build core library
echo [INFO] Building core library...
cargo build --release
if %errorlevel% neq 0 (
    echo [ERROR] Failed to build core library
    exit /b 1
)

REM Build Python bindings
if exist "crates\python-bindings" (
    echo [INFO] Building Python bindings...
    cd crates\python-bindings
    cargo build --release
    if %errorlevel% neq 0 (
        echo [ERROR] Failed to build Python bindings
        cd ..\..
        exit /b 1
    )
    cd ..\..
) else (
    echo [WARN] Python bindings directory not found, skipping...
)

REM Build Java bindings
if exist "crates\java-bindings" (
    echo [INFO] Building Java bindings...
    cd crates\java-bindings
    cargo build --release
    if %errorlevel% neq 0 (
        echo [ERROR] Failed to build Java bindings
        cd ..\..
        exit /b 1
    )
    cd ..\..
) else (
    echo [WARN] Java bindings directory not found, skipping...
)

REM Build JavaScript bindings
if exist "crates\js-bindings" (
    echo [INFO] Building JavaScript bindings...
    cd crates\js-bindings
    cargo build --release
    if %errorlevel% neq 0 (
        echo [ERROR] Failed to build JavaScript bindings
        cd ..\..
        exit /b 1
    )
    cd ..\..
) else (
    echo [WARN] JavaScript bindings directory not found, skipping...
)

REM Build C++ bindings
if exist "crates\cpp-bindings" (
    echo [INFO] Building C++ bindings...
    cd crates\cpp-bindings
    cargo build --release
    if %errorlevel% neq 0 (
        echo [ERROR] Failed to build C++ bindings
        cd ..\..
        exit /b 1
    )
    cd ..\..
) else (
    echo [WARN] C++ bindings directory not found, skipping...
)

echo [INFO] Build completed successfully!
echo.
echo [INFO] Output files:
echo   Core library: target\release\offline_intelligence.dll
echo   Python bindings: crates\python-bindings\target\release\offline_intelligence_py.dll
echo   Java bindings: crates\java-bindings\target\release\offline_intelligence_java.dll
echo   JavaScript bindings: crates\js-bindings\target\release\offline_intelligence_js.node
echo   C++ bindings: crates\cpp-bindings\target\release\offline_intelligence_cpp.dll
echo.
echo [INFO] To run tests:
echo   cargo test --release

pause