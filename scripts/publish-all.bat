@echo off
setlocal enabledelayedexpansion

REM Comprehensive publish script for all platforms (Windows)
REM This script publishes the library to PyPI, npm, crates.io, and prepares for Maven/JitPack

echo === Offline Intelligence Library - Multi-Platform Publish ===
echo.

REM Check Git status
echo [INFO] Checking Git status...
git status --porcelain | findstr . >nul
if %errorlevel% equ 0 (
    echo [WARN] Working directory has uncommitted changes
    set /p CONTINUE="Continue anyway? (y/N): "
    if /i not "%CONTINUE%"=="y" (
        echo [INFO] Publish cancelled
        exit /b 0
    )
)

REM Get version
for /f "tokens=2 delims==" %%i in ('findstr "^version = " crates\offline-intelligence\Cargo.toml') do (
    set VERSION=%%i
    goto :gotversion
)
:gotversion
set VERSION=%VERSION:"=%
set VERSION=%VERSION: =%
echo [INFO] Publishing version: %VERSION%

REM Confirm publish
echo.
echo [WARN] About to publish Offline Intelligence Library v%VERSION% to multiple platforms
echo [WARN] This will:
echo [WARN]   - Create and push Git tag v%VERSION%
echo [WARN]   - Publish to crates.io
echo [WARN]   - Publish to PyPI
echo [WARN]   - Publish to npm
echo [WARN]   - Prepare Maven/JitPack release
echo.
set /p CONFIRM="Continue? (y/N): "
if /i not "%CONFIRM%"=="y" (
    echo [INFO] Publish cancelled
    exit /b 0
)

REM Build all components
echo [INFO] Building all components...
call build.bat

REM Create Git tag
echo [INFO] Creating Git tag...
git tag -a "v%VERSION%" -m "Release version %VERSION%"
git push origin "v%VERSION%"

REM Publish to crates.io
echo [INFO] Publishing to crates.io...
cd crates\offline-intelligence
cargo publish --dry-run >nul 2>&1
if %errorlevel% equ 0 (
    cargo publish
    if %errorlevel% equ 0 (
        echo [INFO] Successfully published to crates.io
    ) else (
        echo [ERROR] Failed to publish to crates.io
    )
) else (
    echo [ERROR] crates.io dry run failed
)
cd ..\..

REM Publish to PyPI
where twine >nul 2>&1
if %errorlevel% equ 0 (
    echo [INFO] Publishing to PyPI...
    cd crates\python-bindings
    python setup.py sdist bdist_wheel
    twine upload --repository pypi dist\*
    if %errorlevel% equ 0 (
        echo [INFO] Successfully published to PyPI
    ) else (
        echo [ERROR] Failed to publish to PyPI
    )
    cd ..\..
) else (
    echo [WARN] twine not found, skipping PyPI publish
)

REM Publish to npm
where npm >nul 2>&1
if %errorlevel% equ 0 (
    echo [INFO] Publishing to npm...
    cd crates\js-bindings
    npm publish
    if %errorlevel% equ 0 (
        echo [INFO] Successfully published to npm
    ) else (
        echo [ERROR] Failed to publish to npm
    )
    cd ..\..
) else (
    echo [WARN] npm not found, skipping npm publish
)

REM Prepare Maven/JitPack
echo [INFO] Preparing Maven/JitPack release...
cd crates\java-bindings
echo [INFO] Push changes to GitHub to trigger JitPack build
cd ..\..

REM Summary
echo.
echo [INFO] === Publish Summary ===
echo [INFO] Version: %VERSION%
echo [INFO] Git tag: v%VERSION% created and pushed
echo [INFO] crates.io: Published (if successful)
echo [INFO] PyPI: Published (if successful)
echo [INFO] npm: Published (if successful)
echo [INFO] Maven/JitPack: Ready for GitHub release
echo.
echo [INFO] Next steps:
echo [INFO] 1. Create GitHub release for tag v%VERSION%
echo [INFO] 2. JitPack will automatically build Java bindings
echo [INFO] 3. Verify all packages are available
echo [INFO] 4. Update documentation if needed

pause