#!/usr/bin/env bash
# Builds gm-ide and gm-runner in release mode and wraps them as macOS app bundles:
#   dist/macos/GM81 Clean IDE.app     (gm-ide; gm-runner is copied next to it inside
#                                      Contents/MacOS so Run (F5) finds it)
#   dist/macos/GM81 Clean Runner.app  (gm-runner; opens .gm81/.gmk files)
# Usage: scripts/macos-bundle.sh [--target aarch64-apple-darwin|x86_64-apple-darwin] [OUT_DIR]
# Must run on macOS (the bundles are ad-hoc signed with codesign when available).
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"
target_args=()
target_dir=release
if [ "${1:-}" = "--target" ]; then
  target_args=(--target "$2")
  target_dir="$2/release"
  shift 2
fi
out=${1:-dist/macos}
bin="${CARGO_TARGET_DIR:-$root/target}/$target_dir"
version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)

cargo build --release ${target_args[@]+"${target_args[@]}"} -p gm-ide -p gm-runner
mkdir -p "$out"
icons=$(mktemp -d)
cargo run -q -p gm-brand --example write_icons -- "$icons"

# make_app NAME EXECUTABLE BUNDLE_ID DOC_TYPES_XML
make_app() {
  local app="$out/$1.app"
  rm -rf "$app"
  mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
  cp "$bin/$2" "$app/Contents/MacOS/"
  cp "$icons/gm81-clean.icns" "$app/Contents/Resources/gm81-clean.icns"
  cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>$1</string>
  <key>CFBundleDisplayName</key><string>$1</string>
  <key>CFBundleIdentifier</key><string>$3</string>
  <key>CFBundleExecutable</key><string>$2</string>
  <key>CFBundleIconFile</key><string>gm81-clean</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$version</string>
  <key>CFBundleVersion</key><string>$version</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>CFBundleDocumentTypes</key>
  <array>
    <dict>
      <key>CFBundleTypeName</key><string>GameMaker project</string>
      <key>CFBundleTypeExtensions</key><array><string>gm81</string><string>gmk</string></array>
      <key>CFBundleTypeRole</key><string>$4</string>
      <key>CFBundleTypeIconFile</key><string>gm81-clean</string>
    </dict>
  </array>
</dict>
</plist>
PLIST
  if command -v codesign >/dev/null; then codesign --force --deep --sign - "$app" || true; fi
  echo "built $app"
}

make_app "GM81 Clean IDE" gm-ide org.gm81clean.ide Editor
# the IDE looks for gm-runner next to its own executable
cp "$bin/gm-runner" "$out/GM81 Clean IDE.app/Contents/MacOS/"
if command -v codesign >/dev/null; then codesign --force --deep --sign - "$out/GM81 Clean IDE.app" || true; fi
make_app "GM81 Clean Runner" gm-runner org.gm81clean.runner Viewer
rm -rf "$icons"
