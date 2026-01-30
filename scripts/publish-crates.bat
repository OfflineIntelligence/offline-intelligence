@echo off
setlocal enabledelayedexpansion

REM Publish script for crates.io (Windows)
REM This script publishes the core library to crates.io

echo === Publishing to crates.io ===
echo.

REM Check if we're logged in to crates.io
echo [INFO] Checking crates.io login status...
cargo owner --list offline-intelligence >nul 2>&1
if %errorlevel% neq 0 (
    echo [ERROR] Not logged in to crates.io. Please run: cargo login
    exit /b 1
)

REM Check version
for /f "tokens=2 delims==" %%i in ('findstr "^version =" crates\offline-intelligence\Cargo.toml') do (
    set VERSION=%%i
    goto :gotversion
)
:gotversion
set VERSION=%VERSION:"=%
set VERSION=%VERSION: =%
echo [INFO] Publishing version: %VERSION%

REM Build and test
echo [INFO] Running tests...
cd crates\offline-intelligence
cargo test --release
if %errorlevel% neq 0 (
    echo [ERROR] Tests failed
    cd ..\..
    exit /b 1
)

REM Check for unpublished changes
echo [INFO] Checking for unpublished changes...
cargo package --list --allow-dirty >nul 2>&1
if %errorlevel% neq 0 (
    echo [ERROR] Package validation failed
    cd ..\..
    exit /b 1
)

REM Dry run first
echo [INFO] Performing dry run...
cargo publish --dry-run
if %errorlevel% neq 0 (
    echo [ERROR] Dry run failed
    cd ..\..
    exit /b 1
)

echo [INFO] Dry run successful

REM Confirm publish
echo.
echo [WARN] About to publish offline-intelligence v%VERSION% to crates.io
set /p CONFIRM="Continue? (y/N): "
if /i not "%CONFIRM%"=="y" (
    echo [INFO] Publish cancelled
    cd ..\..
    exit /b 0
)

REM Publish
echo [INFO] Publishing to crates.io...
cargo publish
if %errorlevel% equ 0 (
    echo [INFO] Successfully published offline-intelligence v%VERSION% to crates.io!
    echo Package URL: https://crates.io/crates/offline-intelligence
) else (
    echo [ERROR] Failed to publish to crates.io
    cd ..\..
    exit /b 1
)

cd ..\..
pause