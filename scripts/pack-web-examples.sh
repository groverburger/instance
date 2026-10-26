#!/usr/bin/env sh
# Packs the example games of the web build into web/examples/ (committed; the Pages
# workflow only copies them, because it cannot read instance-gamekit).
#   scripts/pack-web-examples.sh [GAME ...]     (default: emberwood gravity_grade)
# Reads the games from $INSTANCE_GAMEKIT (default: ../instance-gamekit next to this
# repository) and never writes there.
#   emberwood.gm81, gravity_grade.gm81  from games/emberwood and games/gravity_grade, with
#       their sounds (WAVs over $MUSIC_MIN_KB KB, default 256) transcoded to OGG Vorbis:
#       the WAVs make the games too big to download for a demo
# spelunky.gm81 (and spelunky-COPYING.txt, spelunky.png) are made separately and only
# copied in (docs/reference.md, "The web build").
# Needs: target/release/gmk-pack (cargo build --release -p instance-format --bins), or
#        $GMK_PACK; a Vorbis encoder: ffmpeg, oggenc, or Python with the "soundfile"
#        package ($PYTHON, default python3 / python). $OGG_QUALITY: Vorbis quality
#        (-1..10, default 3, about 100 kbit/s).
set -eu
root=$(cd "$(dirname "$0")/.." && pwd)
gamekit=${INSTANCE_GAMEKIT:-$root/../instance-gamekit}
out=$root/web/examples
quality=${OGG_QUALITY:-3}
min_kb=${MUSIC_MIN_KB:-256}

pack=${GMK_PACK:-$root/target/release/gmk-pack}
[ -x "$pack" ] || [ ! -x "$pack.exe" ] || pack=$pack.exe
[ -x "$pack" ] || { echo "gmk-pack not found at $pack: cargo build --release -p instance-format --bins" >&2; exit 1; }
[ -d "$gamekit/games" ] || { echo "no games folder in $gamekit (set INSTANCE_GAMEKIT)" >&2; exit 1; }

python=${PYTHON:-}
if [ -z "$python" ]; then
  for p in python3 python; do
    if command -v "$p" >/dev/null && "$p" -c "import soundfile" 2>/dev/null; then python=$p; break; fi
  done
fi
encoder=
if command -v ffmpeg >/dev/null; then encoder=ffmpeg
elif command -v oggenc >/dev/null; then encoder=oggenc
elif [ -n "$python" ]; then encoder=python
else
  echo "no Vorbis encoder: install ffmpeg or vorbis-tools (oggenc), or 'pip install soundfile' (and set PYTHON)" >&2
  exit 1
fi

# WAV $1 -> OGG Vorbis $2 (Vorbis keeps the exact sample count, so loops stay seamless).
# Instance plays OGG in any sound kind (specs/sound.md 1.2); GameMaker 8.1 itself would
# not, so these copies are for Instance only.
to_ogg() {
  case $encoder in
    ffmpeg) ffmpeg -v error -y -i "$1" -c:a libvorbis -q:a "$quality" "$2" ;;
    oggenc) oggenc -Q -q "$quality" -o "$2" "$1" ;;
    python) "$python" - "$1" "$2" "$quality" <<'EOF'
import sys, soundfile as sf
data, rate = sf.read(sys.argv[1], always_2d=True)
# libsndfile's compression level 0..1 maps to Vorbis quality 10..-1 (roughly)
level = min(1.0, max(0.0, (10.0 - float(sys.argv[3])) / 11.0))
# written in blocks: libsndfile's Vorbis encoder can crash on one large write
with sf.SoundFile(sys.argv[2], "w", rate, data.shape[1], format="OGG", subtype="VORBIS",
                  compression_level=level) as out:
    for i in range(0, len(data), 32768):
        out.write(data[i:i + 32768])
EOF
    ;;
  esac
}

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT INT TERM
mkdir -p "$out"

# Copies games/$1 to the scratch folder (without saved files and art/music sources),
# transcodes its large WAVs (music) to OGG Vorbis and packs it into $out/$1.gm81.
pack_game() {
  echo "== $1 (WAVs over $min_kb KB to OGG Vorbis q$quality, with $encoder)"
  src=$gamekit/games/$1
  dir=$tmp/$1
  mkdir -p "$dir"
  for f in "$src"/* "$src"/.[!.]*; do
    [ -e "$f" ] || continue
    case $(basename "$f") in *.gm81|*.gmk|*.cfg|*.sav|*.ini|*.dat|art-src|music-src) continue ;; esac
    cp -R "$f" "$dir/"
  done
  for wav in "$dir"/sounds/*.wav; do
    [ -f "$wav" ] || continue
    kb=$(( $(wc -c < "$wav") / 1024 ))
    [ "$kb" -ge "$min_kb" ] || continue
    base=$(basename "$wav" .wav)
    toml=$dir/sounds/$base.toml
    grep -q "^data = \"$base.wav\"" "$toml" || { echo "$toml: data is not $base.wav" >&2; exit 1; }
    to_ogg "$wav" "$dir/sounds/$base.ogg"
    rm "$wav"
    sed -e "s/^data = \"$base\.wav\"/data = \"$base.ogg\"/" \
        -e "s/^file_name = \"$base\.wav\"/file_name = \"$base.ogg\"/" \
        -e 's/^file_type = "\.wav"/file_type = ".ogg"/' "$toml" > "$toml.new"
    mv "$toml.new" "$toml"
    echo "  $base: ${kb} KB WAV -> $(( $(wc -c < "$dir/sounds/$base.ogg") / 1024 )) KB OGG"
  done
  "$pack" "$dir" "$out/$1.gm81"
}

[ $# -gt 0 ] || set -- emberwood gravity_grade
for game in "$@"; do pack_game "$game"; done

ls -l "$out"/*.gm81
