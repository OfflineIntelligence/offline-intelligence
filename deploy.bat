@echo off
setlocal enabledelayedexpansion

REM Offline Intelligence - Multi-Ecosystem Deployment Script
REM Deploys to: crates.io, PyPI, npm, Maven Central

echo [DEPLOY] Starting Offline Intelligence multi-ecosystem deployment...
echo [DEPLOY] Repository: https://github.com/OfflineIntelligence/offline-intelligence

REM Configuration
set PROJECT_ROOT=%~dp0
set CRATE_DIR=%PROJECT_ROOT%crates\offline-intelligence
set DIST_DIR=%PROJECT_ROOT%dist

REM Create distribution directory
echo [DEPLOY] Creating distribution directory...
if exist "%DIST_DIR%" rmdir /s /q "%DIST_DIR%"
mkdir "%DIST_DIR%"
mkdir "%DIST_DIR%\logs"

REM Check prerequisites
echo [DEPLOY] Checking deployment prerequisites...
set MISSING_TOOLS=

where cargo >nul 2>&1 || set MISSING_TOOLS=%MISSING_TOOLS% cargo
where python >nul 2>&1 || set MISSING_TOOLS=%MISSING_TOOLS% python
where npm >nul 2>&1 || set MISSING_TOOLS=%MISSING_TOOLS% npm
where mvn >nul 2>&1 || set MISSING_TOOLS=%MISSING_TOOLS% maven
where twine >nul 2>&1 || set MISSING_TOOLS=%MISSING_TOOLS% twine

if defined MISSING_TOOLS (
    echo [ERROR] Missing required deployment tools: %MISSING_TOOLS%
    echo [ERROR] Please install missing dependencies:
    echo [ERROR] - cargo (Rust)
    echo [ERROR] - python + twine (Python)
    echo [ERROR] - npm (JavaScript)
    echo [ERROR] - maven (Java)
    exit /b 1
)

echo [SUCCESS] All deployment tools available

REM Build all components first
echo [DEPLOY] Building all components...
call "%PROJECT_ROOT%build.bat"
if errorlevel 1 (
    echo [ERROR] Build failed, deployment aborted
    exit /b 1
)

REM Deploy to Crates.io (Rust)
echo [DEPLOY] Deploying Rust crate to crates.io...
cd /d "%CRATE_DIR%"
cargo publish --dry-run --allow-dirty > "%DIST_DIR%\logs\crates_io_dry_run.log" 2>&1
if errorlevel 1 (
    echo [ERROR] Crates.io dry run failed, check logs
    type "%DIST_DIR%\logs\crates_io_dry_run.log"
    exit /b 1
)

REM Uncomment the next line for actual deployment
REM cargo publish --allow-dirty

echo [SUCCESS] Rust crate ready for crates.io deployment

REM Deploy Python package to PyPI
echo [DEPLOY] Deploying Python package to PyPI...
cd /d "%PROJECT_ROOT%bindings\python"
if exist build rmdir /s /q build
if exist dist rmdir /s /q dist

python setup.py sdist bdist_wheel > "%DIST_DIR%\logs\python_build.log" 2>&1
if errorlevel 1 (
    echo [ERROR] Python package build failed
    type "%DIST_DIR%\logs\python_build.log"
    exit /b 1
)

twine check dist/* > "%DIST_DIR%\logs\python_check.log" 2>&1
if errorlevel 1 (
    echo [ERROR] Python package validation failed
    type "%DIST_DIR%\logs\python_check.log"
    exit /b 1
)

REM Uncomment for actual PyPI deployment
REM twine upload dist/*

echo [SUCCESS] Python package ready for PyPI deployment

REM Deploy JavaScript package to npm
echo [DEPLOY] Deploying JavaScript package to npm...
cd /d "%PROJECT_ROOT%bindings\javascript"

npm pack --dry-run > "%DIST_DIR%\logs\npm_pack.log" 2>&1
if errorlevel 1 (
    echo [ERROR] npm package validation failed
    type "%DIST_DIR%\logs\npm_pack.log"
    exit /b 1
)

REM Uncomment for actual npm deployment
REM npm publish

echo [SUCCESS] JavaScript package ready for npm deployment

REM Deploy Java package to Maven Central
echo [DEPLOY] Deploying Java package to Maven Central...
cd /d "%PROJECT_ROOT%bindings\java"

mvn clean compile test > "%DIST_DIR%\logs\java_build.log" 2>&1
if errorlevel 1 (
    echo [ERROR] Java package build failed
    type "%DIST_DIR%\logs\java_build.log"
    exit /b 1
)

mvn package > "%DIST_DIR%\logs\java_package.log" 2>&1
if errorlevel 1 (
    echo [ERROR] Java package creation failed
    type "%DIST_DIR%\logs\java_package.log"
    exit /b 1
)

REM Uncomment for actual Maven deployment
REM mvn clean deploy -P release

echo [SUCCESS] Java package ready for Maven Central deployment

REM Create cross-platform distribution
echo [DEPLOY] Creating cross-platform distribution packages...

REM Copy Rust artifacts
mkdir "%DIST_DIR%\rust" 2>nul
xcopy "%CRATE_DIR%\target\release\offline-intelligence.exe" "%DIST_DIR%\rust\" /Y 2>nul
xcopy "%CRATE_DIR%\target\release\liboffline_intelligence.rlib" "%DIST_DIR%\rust\" /Y 2>nul

REM Copy Python artifacts
mkdir "%DIST_DIR%\python" 2>nul
if exist "%PROJECT_ROOT%bindings\python\dist\*" (
    xcopy "%PROJECT_ROOT%bindings\python\dist\*" "%DIST_DIR%\python\" /E /I /Y
)

REM Copy JavaScript artifacts
mkdir "%DIST_DIR%\javascript" 2>nul
npm pack > "%DIST_DIR%\logs\npm_pack_final.log" 2>nul
if exist "*.tgz" (
    move "*.tgz" "%DIST_DIR%\javascript\"
)

REM Copy Java artifacts
mkdir "%DIST_DIR%\java" 2>nul
if exist "%PROJECT_ROOT%bindings\java\target\*.jar" (
    xcopy "%PROJECT_ROOT%bindings\java\target\*.jar" "%DIST_DIR%\java\" /Y
)

REM Create deployment manifest
(
echo {
echo   "name": "offline-intelligence",
echo   "version": "0.1.1",
echo   "repository": "https://github.com/OfflineIntelligence/offline-intelligence",
echo   "timestamp": "%date% %time%",
echo   "deployment_status": {
echo     "rust": "ready_for_crates_io",
echo     "python": "ready_for_pypi",
echo     "javascript": "ready_for_npm",
echo     "java": "ready_for_maven_central"
echo   },
echo   "artifacts": {
echo     "rust": {
echo       "executable": "rust/offline-intelligence.exe",
echo       "library": "rust/liboffline_intelligence.rlib"
echo     },
echo     "python": {
echo       "wheels": "python/*.whl",
echo       "source": "python/*.tar.gz"
echo     },
echo     "javascript": {
echo       "package": "javascript/*.tgz"
echo     },
echo     "java": {
echo       "jars": "java/*.jar"
echo     }
echo   }
echo }
) > "%DIST_DIR%\DEPLOYMENT_MANIFEST.json"

REM Create deployment summary
(
echo OFFLINE INTELLIGENCE - DEPLOYMENT SUMMARY
echo ========================================
echo Repository: https://github.com/OfflineIntelligence/offline-intelligence
echo Version: 0.1.1
echo Timestamp: %date% %time%
echo.
echo DEPLOYMENT STATUS:
echo - Rust (crates.io): READY - Run 'cargo publish' in crates/offline-intelligence
echo - Python (PyPI): READY - Run 'twine upload dist/*' in bindings/python
echo - JavaScript (npm): READY - Run 'npm publish' in bindings/javascript
echo - Java (Maven Central): READY - Run 'mvn clean deploy -P release' in bindings/java
echo.
echo DISTRIBUTION ARTIFACTS:
echo - Executables: %DIST_DIR%\rust\
echo - Python packages: %DIST_DIR%\python\
echo - JavaScript packages: %DIST_DIR%\javascript\
echo - Java packages: %DIST_DIR%\java\
echo.
echo LOG FILES:
echo - Build logs: %DIST_DIR%\logs\
echo - Deployment manifest: %DIST_DIR%\DEPLOYMENT_MANIFEST.json
) > "%DIST_DIR%\DEPLOYMENT_SUMMARY.txt"

echo [SUCCESS] Multi-ecosystem deployment preparation completed!
echo [DEPLOY] Distribution created at: %DIST_DIR%
echo [DEPLOY] See DEPLOYMENT_SUMMARY.txt for detailed instructions
echo [DEPLOY] Manual deployment commands are commented out - uncomment to deploy