@echo off
setlocal EnableDelayedExpansion

REM ---------------------------------------------------------------------------
REM generate_certs.bat — generate a self-signed TLS certificate for testing
REM
REM Usage:
REM   generate_certs.bat [OUTPUT_DIR]
REM
REM   OUTPUT_DIR defaults to C:\ProgramData\OfflineIntelligence\tls  (server)
REM   or %LOCALAPPDATA%\OfflineIntelligence\tls                      (desktop).
REM   Override by passing a path:  generate_certs.bat C:\mycerts
REM
REM Output files:
REM   server.crt  — self-signed certificate (valid 365 days)
REM   server.key  — RSA-4096 private key
REM
REM After running this script, add these two lines to your .env:
REM   TLS_CERT_PATH=<OUTPUT_DIR>\server.crt
REM   TLS_KEY_PATH=<OUTPUT_DIR>\server.key
REM
REM Requires: OpenSSL for Windows
REM   Download : https://slproweb.com/products/Win32OpenSSL.html
REM   Or via   : winget install ShiningLight.OpenSSL
REM              choco install openssl
REM              scoop install openssl
REM
REM NOTE: Self-signed certificates are for internal testing only.
REM For production obtain a certificate from your organisation's CA.
REM ---------------------------------------------------------------------------

echo.
echo ====================================================================
echo   Offline Intelligence -- TLS Certificate Generator
echo ====================================================================

REM ── Resolve output directory ─────────────────────────────────────────────
if "%~1" NEQ "" (
    set "OUT_DIR=%~1"
) else if exist "C:\ProgramData\OfflineIntelligence" (
    set "OUT_DIR=C:\ProgramData\OfflineIntelligence\tls"
) else (
    set "OUT_DIR=%LOCALAPPDATA%\OfflineIntelligence\tls"
)

set "CERT=%OUT_DIR%\server.crt"
set "KEY=%OUT_DIR%\server.key"

echo   Output directory : %OUT_DIR%
echo   Certificate      : %CERT%
echo   Private key      : %KEY%
echo ====================================================================
echo.

REM ── Create output directory ───────────────────────────────────────────────
if not exist "%OUT_DIR%" (
    mkdir "%OUT_DIR%"
    if errorlevel 1 (
        echo ERROR: Could not create directory: %OUT_DIR%
        echo        Try running this script as Administrator.
        exit /b 1
    )
)

REM ── Check for openssl ─────────────────────────────────────────────────────
where openssl >nul 2>&1
if errorlevel 1 (
    echo ERROR: openssl.exe is not found on PATH.
    echo.
    echo Install OpenSSL for Windows:
    echo   winget install ShiningLight.OpenSSL
    echo   choco install openssl
    echo   scoop install openssl
    echo.
    echo Or download from: https://slproweb.com/products/Win32OpenSSL.html
    echo After installing, restart this terminal so PATH is updated.
    exit /b 1
)

REM ── Generate certificate ───────────────────────────────────────────────────
echo Generating RSA-4096 private key and self-signed certificate (365 days)...
echo.

openssl req ^
    -x509 ^
    -newkey rsa:4096 ^
    -keyout "%KEY%" ^
    -out    "%CERT%" ^
    -days   365 ^
    -nodes ^
    -subj   "/C=US/ST=State/L=City/O=OfflineIntelligence/CN=localhost" ^
    -addext "subjectAltName=DNS:localhost,IP:127.0.0.1,IP:::1"

if errorlevel 1 (
    echo.
    echo ERROR: Certificate generation failed.
    echo        See the openssl output above for details.
    exit /b 1
)

echo.
echo ====================================================================
echo   Done. Certificate valid for 365 days.
echo ====================================================================
echo.
echo Add these two lines to your .env:
echo.
echo   TLS_CERT_PATH=%CERT%
echo   TLS_KEY_PATH=%KEY%
echo.
echo Then restart the server. It will serve HTTPS on the configured port.
echo.
echo WARNING: Self-signed certificates will trigger browser warnings.
echo          For production, replace with a certificate from your CA.
echo.

endlocal
