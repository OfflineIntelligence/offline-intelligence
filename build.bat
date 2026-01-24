@echo off
setlocal enabledelayedexpansion

REM Offline Intelligence Library - Multi-Language Distribution Build Script (Windows)
REM Supports: Rust (native), Python, C++, JavaScript/Node.js, Java

echo [INFO] Starting Offline Intelligence Library build process...

REM Configuration
set PROJECT_ROOT=%~dp0
set CRATE_DIR=%PROJECT_ROOT%crates\offline-intelligence
set BINDINGS_DIR=%PROJECT_ROOT%bindings
set BUILD_DIR=%PROJECT_ROOT%build
set DIST_DIR=%PROJECT_ROOT%dist

REM Detect platform
set PLATFORM=windows
set ARCH=x86_64
echo [INFO] Detected platform: %PLATFORM%-%ARCH%

REM Clean previous builds
echo [INFO] Cleaning previous builds...
if exist "%BUILD_DIR%" rmdir /s /q "%BUILD_DIR%"
if exist "%DIST_DIR%" rmdir /s /q "%DIST_DIR%"
mkdir "%BUILD_DIR%"
mkdir "%DIST_DIR%"

REM Build Rust library
echo [INFO] Building Rust library...
cd /d "%CRATE_DIR%"
cargo build --release --target x86_64-pc-windows-msvc
if errorlevel 1 (
    echo [ERROR] Rust build failed
    exit /b 1
)
echo [SUCCESS] Rust library built successfully

REM Build Python bindings
echo [INFO] Building Python bindings...
cd /d "%BINDINGS_DIR%\python"
if exist build rmdir /s /q build
mkdir build
python setup.py build_ext --inplace
python setup.py bdist_wheel
if errorlevel 1 (
    echo [ERROR] Python bindings build failed
    exit /b 1
)
echo [SUCCESS] Python bindings built successfully

REM Build C++ bindings
echo [INFO] Building C++ bindings...
cd /d "%BINDINGS_DIR%\cpp"
if exist build rmdir /s /q build
mkdir build
cd build
cmake .. -DCMAKE_BUILD_TYPE=Release
cmake --build . --config Release
cmake --install . --prefix "%DIST_DIR%\cpp"
if errorlevel 1 (
    echo [ERROR] C++ bindings build failed
    exit /b 1
)
echo [SUCCESS] C++ bindings built successfully

REM Build JavaScript bindings
echo [INFO] Building JavaScript bindings...
cd /d "%BINDINGS_DIR%\javascript"
npm install
npm run build
if errorlevel 1 (
    echo [ERROR] JavaScript bindings build failed
    exit /b 1
)
echo [SUCCESS] JavaScript bindings built successfully

REM Build Java bindings
echo [INFO] Building Java bindings...
cd /d "%BINDINGS_DIR%\java"
mvn clean compile package
if errorlevel 1 (
    echo [ERROR] Java bindings build failed
    exit /b 1
)
echo [SUCCESS] Java bindings built successfully

REM Create distribution
echo [INFO] Creating distribution packages...
mkdir "%DIST_DIR%\rust"
mkdir "%DIST_DIR%\python"
mkdir "%DIST_DIR%\cpp-lib"
mkdir "%DIST_DIR%\javascript"
mkdir "%DIST_DIR%\java"

REM Copy Rust binaries
xcopy "%CRATE_DIR%\target\x86_64-pc-windows-msvc\release\*" "%DIST_DIR%\rust\" /E /I /Y

REM Copy Python wheels
if exist "%BINDINGS_DIR%\python\dist\*.whl" (
    copy "%BINDINGS_DIR%\python\dist\*.whl" "%DIST_DIR%\python\"
)

REM Copy C++ libraries
if exist "%DIST_DIR%\cpp\*" (
    xcopy "%DIST_DIR%\cpp\*" "%DIST_DIR%\cpp-lib\" /E /I /Y
)

REM Copy JavaScript packages
if exist "%BINDINGS_DIR%\javascript\build\*" (
    xcopy "%BINDINGS_DIR%\javascript\build\*" "%DIST_DIR%\javascript\" /E /I /Y
)

REM Copy Java JARs
if exist "%BINDINGS_DIR%\java\target\*.jar" (
    copy "%BINDINGS_DIR%\java\target\*.jar" "%DIST_DIR%\java\"
)

REM Create manifest
(
echo {
echo   "name": "offline-intelligence",
echo   "version": "0.1.0",
echo   "platforms": ["windows"],
echo   "architectures": ["x86_64"],
echo   "components": {
echo     "rust": {
echo       "type": "native-library",
echo       "path": "rust/",
echo       "description": "Native Rust library and executables"
echo     },
echo     "python": {
echo       "type": "bindings",
echo       "path": "python/",
echo       "description": "Python language bindings"
echo     },
echo     "cpp": {
echo       "type": "bindings",
echo       "path": "cpp-lib/",
echo       "description": "C++ language bindings"
echo     },
echo     "javascript": {
echo       "type": "bindings",
echo       "path": "javascript/",
echo       "description": "JavaScript/Node.js bindings"
echo     },
echo     "java": {
echo       "type": "bindings",
echo       "path": "java/",
echo       "description": "Java language bindings"
echo     }
echo   }
echo }
) > "%DIST_DIR%\MANIFEST.json"

echo [SUCCESS] Build process completed successfully!
echo [INFO] Distribution available at: %DIST_DIR%