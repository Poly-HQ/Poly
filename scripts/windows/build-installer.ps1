param(
    [string]$Version = "0.0.0",
    [string]$Executable = "target/release/poly.exe",
    [string]$Output = "dist"
)

$ErrorActionPreference = "Stop"
$root = Resolve-Path (Join-Path $PSScriptRoot "../..")
$source = Resolve-Path (Join-Path $root $Executable)
$outputPath = Join-Path $root $Output
New-Item -ItemType Directory -Force $outputPath | Out-Null

$iscc = @(
    (Join-Path ${env:ProgramFiles(x86)} "Inno Setup 6/ISCC.exe")
    (Join-Path $env:ProgramFiles "Inno Setup 6/ISCC.exe")
    (Join-Path $env:LOCALAPPDATA "Programs/Inno Setup 6/ISCC.exe")
) | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $iscc) {
    throw "Inno Setup 6 is not installed"
}

$env:SONORA_VERSION = $Version.TrimStart("v")
$env:SONORA_EXE = $source.Path
$env:SONORA_DIST = (Resolve-Path $outputPath).Path
if (-not $env:SONORA_ARCH) { $env:SONORA_ARCH = "x64compatible" }
if (-not $env:SONORA_SETUP) { $env:SONORA_SETUP = "Poly-Setup" }

& $iscc (Join-Path $PSScriptRoot "sonora.iss")
if ($LASTEXITCODE -ne 0) {
    throw "Inno Setup failed with exit code $LASTEXITCODE"
}

Copy-Item (Join-Path $outputPath "$($env:SONORA_SETUP).exe") (Join-Path $outputPath "Poly-Windows.exe") -Force
