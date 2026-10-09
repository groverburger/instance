#!/usr/bin/env sh
# Downloads the General MIDI SoundFont used for MIDI music (GeneralUser GS v2.0.3
# by S. Christian Collins, ~32 MB; licence: free to use, modify and include in
# software, see web/soundfonts/LICENSE-GeneralUser-GS.txt) into web/soundfonts/
# (ignored by version control). scripts/build-web.* bundles it into the web build
# as soundfonts/default.sf2; for the desktop runner copy it next to the executable
# as soundfonts/default.sf2 or point INSTANCE_SOUNDFONT at it.
#   scripts/fetch-soundfont.sh [OUT_DIR]
set -eu
root=$(cd "$(dirname "$0")/.." && pwd)
out=${1:-$root/web/soundfonts}
rev=97049183643d5fc5a9322a69c5b09efb667c6c3a   # GeneralUser-GS repository, v2.0.3
base=https://raw.githubusercontent.com/mrbumpy409/GeneralUser-GS/$rev
sha=9575028c7a1f589f5770fccc8cff2734566af40cd26ed836944e9a5152688cfe
lic_sha=7b32efefdf95ce38a043799f0659853ddc00fbaa14d8c50f0aca16b9b8b405be

sha256() {
  if command -v sha256sum >/dev/null; then sha256sum "$1" | cut -d' ' -f1
  else shasum -a 256 "$1" | cut -d' ' -f1; fi
}
fetch() { # url dest sha
  if [ -f "$2" ] && [ "$(sha256 "$2")" = "$3" ]; then echo "up to date: $2"; return; fi
  curl -fL --retry 3 -o "$2.part" "$1"
  got=$(sha256 "$2.part")
  if [ "$got" != "$3" ]; then rm -f "$2.part"; echo "checksum mismatch for $1: $got" >&2; exit 1; fi
  mv "$2.part" "$2"
  echo "fetched: $2"
}
mkdir -p "$out"
fetch "$base/documentation/LICENSE.txt" "$out/LICENSE-GeneralUser-GS.txt" "$lic_sha"
fetch "$base/GeneralUser-GS.sf2" "$out/default.sf2" "$sha"
