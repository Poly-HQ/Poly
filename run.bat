@echo off
setlocal
cd /d "%~dp0"

if "%1"=="check" (
    powershell -NoProfile -ExecutionPolicy Bypass -File scripts\windows\watch.ps1 -CheckOnly
    goto :eof
)

if "%1"=="once" (
    cargo run --package poly
    goto :eof
)

where cargo-watch >nul 2>nul
if %ERRORLEVEL% EQU 0 (
    echo [Poly Live] Launching cargo-watch...
    cargo watch -w crates -w assets -x "run --package poly"
) else (
    powershell -NoProfile -ExecutionPolicy Bypass -File scripts\windows\watch.ps1
)
