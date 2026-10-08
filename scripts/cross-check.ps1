# Type-checks the workspace for macOS, Linux and the browser from a Windows
# machine (no linking, no Apple SDK needed). Usage: scripts\cross-check.ps1
# Requires: rustup target add aarch64-apple-darwin x86_64-apple-darwin x86_64-unknown-linux-gnu wasm32-unknown-unknown
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
$failed = @()
try {
    foreach ($t in @("aarch64-apple-darwin", "x86_64-apple-darwin")) {
        Write-Host "== $t"
        cargo check --workspace --target $t
        if ($LASTEXITCODE -ne 0) { $failed += $t }
    }
    # alsa-sys / libudev-sys ask pkg-config for the Linux libraries: answer with a stub
    Write-Host "== x86_64-unknown-linux-gnu"
    $env:PKG_CONFIG = Join-Path $PSScriptRoot "fake-pkg-config.cmd"
    $env:PKG_CONFIG_ALLOW_CROSS = "1"
    cargo check --workspace --target x86_64-unknown-linux-gnu
    if ($LASTEXITCODE -ne 0) { $failed += "linux" }
    Remove-Item Env:PKG_CONFIG, Env:PKG_CONFIG_ALLOW_CROSS
    Write-Host "== wasm32-unknown-unknown"
    cargo check -p gm-runner -p gm-ide --target wasm32-unknown-unknown
    if ($LASTEXITCODE -ne 0) { $failed += "wasm32" }
} finally {
    Pop-Location
}
if ($failed.Count -gt 0) { Write-Error "cross-check failed: $($failed -join ', ')" }
Write-Host "cross-check OK"
