#!/usr/bin/env sh
# Builds the browser version (IDE + player) into dist/web.
#   scripts/build-web.sh [--debug] [OUT_DIR]
# Needs: rustup target add wasm32-unknown-unknown
#        cargo install wasm-bindgen-cli --version <the wasm-bindgen version in Cargo.lock>
# Serve with e.g.: python3 -m http.server -d dist/web 8000   → http://localhost:8000/
set -eu
root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"
profile=release
flag=--release
if [ "${1:-}" = "--debug" ]; then profile=debug; flag=; shift; fi
out=${1:-dist/web}

want=$(grep -A1 '^name = "wasm-bindgen"$' Cargo.lock | sed -n 's/^version = "\(.*\)"/\1/p')
command -v wasm-bindgen >/dev/null || { echo "wasm-bindgen not found: cargo install wasm-bindgen-cli --version $want" >&2; exit 1; }
have=$(wasm-bindgen --version | sed 's/wasm-bindgen //')
[ "$have" = "$want" ] || echo "warning: wasm-bindgen-cli $have, but Cargo.lock uses $want" >&2

target=${CARGO_TARGET_DIR:-$root/target}
for crate in instance-runner instance-ide; do
  echo "== building $crate (wasm32, $profile)"
  cargo rustc -p "$crate" --lib --target wasm32-unknown-unknown $flag --crate-type cdylib
done
mkdir -p "$out/pkg"
for name in instance_runner instance_ide; do
  wasm-bindgen --target web --no-typescript --out-dir "$out/pkg" "$target/wasm32-unknown-unknown/$profile/$name.wasm"
done
cp web/*.html web/*.js "$out/"
# optional SoundFont for MIDI: $INSTANCE_SOUNDFONT ($GM81_SOUNDFONT) or web/soundfonts/default.sf2
# (ignored by version control; scripts/fetch-soundfont.sh downloads GeneralUser GS there)
sf=${INSTANCE_SOUNDFONT:-${GM81_SOUNDFONT:-}}
[ -z "$sf" ] && [ -f web/soundfonts/default.sf2 ] && sf=web/soundfonts/default.sf2
if [ -n "$sf" ]; then
  mkdir -p "$out/soundfonts"
  cp "$sf" "$out/soundfonts/default.sf2"
  for lic in "$(dirname "$sf")"/LICENSE*.txt; do [ -f "$lic" ] && cp "$lic" "$out/soundfonts/"; done
  echo 'window.INSTANCE_SOUNDFONT = "soundfonts/default.sf2";' > "$out/config.js"
  echo "SoundFont: $sf"
else
  echo 'window.INSTANCE_SOUNDFONT = null;' > "$out/config.js"
  echo "no SoundFont bundled: MIDI music will be silent (run scripts/fetch-soundfont.sh or set INSTANCE_SOUNDFONT to a .sf2 file)"
fi
echo "web build ready in $out  (serve: python3 -m http.server -d $out 8000)"
