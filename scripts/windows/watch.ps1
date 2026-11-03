param(
    [switch]$CheckOnly
)

$Host.UI.RawUI.WindowTitle = "Poly Live Dev Server"
$root = Resolve-Path (Join-Path $PSScriptRoot "../..")
Set-Location $root

Write-Host "==========================================" -ForegroundColor Cyan
Write-Host "  Poly Fast Live Dev Server" -ForegroundColor Cyan
Write-Host "  Engine: rust-lld + incremental" -ForegroundColor DarkCyan
Write-Host "==========================================" -ForegroundColor Cyan
Write-Host ""

if ($CheckOnly) {
    Write-Host "[Poly] Running fast typecheck..." -ForegroundColor Yellow
    cargo check --package poly
    exit $LASTEXITCODE
}

$watchDirs = @(
    (Join-Path $root "crates"),
    (Join-Path $root "assets"),
    (Join-Path $root "Cargo.toml")
)

function Start-PolyProcess {
    Write-Host "`n[Poly Live] Starting app..." -ForegroundColor Green
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = "cargo"
    $psi.Arguments = "run --package poly"
    $psi.WorkingDirectory = $root
    $psi.UseShellExecute = $false
    return [System.Diagnostics.Process]::Start($psi)
}

function Stop-PolyProcess {
    param($proc)
    if ($proc -and -not $proc.HasExited) {
        Write-Host "[Poly Live] Closing active app instance..." -ForegroundColor Yellow
        try {
            Get-Process poly -ErrorAction SilentlyContinue | Stop-Process -Force
            if (-not $proc.HasExited) {
                $proc.Kill()
            }
        } catch {}
    }
}

$currentProc = Start-PolyProcess

$watcher = New-Object System.IO.FileSystemWatcher
$watcher.Path = (Join-Path $root "crates")
$watcher.IncludeSubdirectories = $true
$watcher.EnableRaisingEvents = $true
$watcher.NotifyFilter = [System.IO.NotifyFilters]::LastWrite -bor [System.IO.NotifyFilters]::FileName

$lastChange = [DateTime]::MinValue

Write-Host "`n[Poly Live] Watching for code changes in crates/ and assets/..." -ForegroundColor DarkGray
Write-Host "[Poly Live] Save any file to automatically recompile and relaunch (Ctrl+C to exit)." -ForegroundColor DarkGray

try {
    while ($true) {
        $change = $watcher.WaitForChanged([System.IO.WatcherChangeTypes]::Changed -bor [System.IO.WatcherChangeTypes]::Created, 1000)
        if (-not $change.TimedOut) {
            $ext = [System.IO.Path]::GetExtension($change.Name)
            if ($ext -in @(".rs", ".ftl", ".toml", ".svg", ".png", ".ico")) {
                $now = [DateTime]::UtcNow
                if (($now - $lastChange).TotalMilliseconds -gt 800) {
                    $lastChange = $now
                    Write-Host "`n[Poly Live] Detected change in: $($change.Name)" -ForegroundColor Magenta
                    Stop-PolyProcess -proc $currentProc
                    Start-Sleep -Milliseconds 200
                    $currentProc = Start-PolyProcess
                }
            }
        }
    }
} finally {
    $watcher.Dispose()
    Stop-PolyProcess -proc $currentProc
}
