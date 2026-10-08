# Builds the browser version (IDE + player) into dist/web.
#   scripts\build-web.ps1 [-Debug] [-Out dist\web]
# Needs: rustup target add wasm32-unknown-unknown
#        cargo install wasm-bindgen-cli --version <the wasm-bindgen version in Cargo.lock>
# Serve with e.g.: python -m http.server -d dist/web 8000   → http://localhost:8000/
param([switch]$Debug, [string]$Out = "dist\web")
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Push-Location $root
try {
    $prof = if ($Debug) { "debug" } else { "release" }
    $wbVersion = (Select-String -Path Cargo.lock -Pattern '^name = "wasm-bindgen"$' -Context 0, 1).Context.PostContext[0] -replace '.*"(.*)"', '$1'
    $wb = Get-Command wasm-bindgen -ErrorAction SilentlyContinue
    if (-not $wb) { throw "wasm-bindgen not found: cargo install wasm-bindgen-cli --version $wbVersion" }
    $have = (& wasm-bindgen --version) -replace 'wasm-bindgen ', ''
    if ($have -ne $wbVersion) { Write-Warning "wasm-bindgen-cli $have, but Cargo.lock uses $wbVersion (cargo install wasm-bindgen-cli --version $wbVersion)" }

    $target = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $root "target" }
    $wasmDir = Join-Path $target "wasm32-unknown-unknown\$prof"
    foreach ($crate in @("gm-runner", "gm-ide")) {
        Write-Host "== building $crate (wasm32, $prof)"
        $cargoArgs = @("rustc", "-p", $crate, "--lib", "--target", "wasm32-unknown-unknown", "--crate-type", "cdylib")
        if (-not $Debug) { $cargoArgs += "--release" }
        & cargo $cargoArgs
        if ($LASTEXITCODE -ne 0) { throw "cargo failed for $crate" }
    }
    New-Item -ItemType Directory -Force (Join-Path $Out "pkg") | Out-Null
    foreach ($name in @("gm_runner", "gm_ide")) {
        & wasm-bindgen --target web --no-typescript --out-dir (Join-Path $Out "pkg") (Join-Path $wasmDir "$name.wasm")
        if ($LASTEXITCODE -ne 0) { throw "wasm-bindgen failed for $name" }
    }
    Copy-Item -Force web\*.html, web\*.js $Out
    # optional SoundFont for MIDI: $env:GM81_SOUNDFONT or web\soundfonts\default.sf2 (ignored by version control)
    $sf = if ($env:GM81_SOUNDFONT) { $env:GM81_SOUNDFONT } elseif (Test-Path web\soundfonts\default.sf2) { "web\soundfonts\default.sf2" } else { $null }
    if ($sf) {
        New-Item -ItemType Directory -Force (Join-Path $Out "soundfonts") | Out-Null
        Copy-Item -Force $sf (Join-Path $Out "soundfonts\default.sf2")
        Set-Content -Encoding ascii (Join-Path $Out "config.js") 'window.GM81_SOUNDFONT = "soundfonts/default.sf2";'
        Write-Host "SoundFont: $sf"
    } else {
        Set-Content -Encoding ascii (Join-Path $Out "config.js") 'window.GM81_SOUNDFONT = null;'
        Write-Host "no SoundFont bundled: MIDI music will be silent (set GM81_SOUNDFONT to a .sf2 file)"
    }
    Write-Host "web build ready in $Out  (serve: python -m http.server -d $Out 8000)"
} finally {
    Pop-Location
}
