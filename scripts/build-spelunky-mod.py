"""Builds the Instance web mod of Spelunky 1.1 (Derek Yu, 2009) from the original source.

The original plays all sound through supersound.dll, wrapped by the SS_* scripts. The
mod embeds the game's sound files as sound resources and rewrites the SS_* scripts on
top of GameMaker's built-in sound functions, so the game needs no DLL and no files
next to it (and so runs in the browser).

Inputs are Derek Yu's December 2009 release: spelunky_1_1_src.zip (spelunky.gmk,
COPYING.txt) and the Spelunky 1.1 game download (its `sound` folder).

usage: python -I scripts/build-spelunky-mod.py UNPACKED_ORIGINAL_DIR SOUND_DIR OUT_DIR
  UNPACKED_ORIGINAL_DIR: `gmk-unpack spelunky.gmk DIR` of the original 1.1 source
  SOUND_DIR: the `sound` folder of the Spelunky 1.1 game download
then: gmk-pack OUT_DIR web/examples/spelunky.gm81
      and copy COPYING.txt to web/examples/spelunky-COPYING.txt
"""
import os, re, shutil, sys

src, sound_dir, out = sys.argv[1:4]
if os.path.exists(out):
    shutil.rmtree(out)
shutil.copytree(src, out)

# --- sounds: every file SS_LoadSound loads ---------------------------------------
music_create = open(os.path.join(out, "objects/oMusic/create.1.gml"), encoding="latin-1").read()
files = re.findall(r'SS_LoadSound\("sound/([^"]+)"', music_create)
assert files and all(os.path.exists(os.path.join(sound_dir, f)) for f in files), files

os.makedirs(os.path.join(out, "sounds"), exist_ok=True)
names = {}
for i, f in enumerate(files):
    base, ext = os.path.splitext(f)
    name = "ss_" + re.sub(r"[^A-Za-z0-9_]", "_", base)
    names[f] = name
    shutil.copyfile(os.path.join(sound_dir, f), os.path.join(out, "sounds", name + ext))
    with open(os.path.join(out, "sounds", name + ".toml"), "w", newline="\n") as t:
        t.write(f'index = {i}\nname = "{name}"\ndata = "{name}{ext}"\neffects = 0\n'
                f'file_name = "{f}"\nfile_type = "{ext}"\nkind = 0\nlast_changed = 0.0\n'
                f'pan = 0.0\npreload = true\nvolume = 1.0\n')

tree_path = os.path.join(out, "resource_tree.txt")
tree = open(tree_path, encoding="utf-8").read()
music = [names[f] for f in files if f.endswith(".ogg")]
effects = [names[f] for f in files if not f.endswith(".ogg")]
block = "Sounds\n  Music/\n" + "".join(f"    {n}\n" for n in music) + \
        "  Effects/\n" + "".join(f"    {n}\n" for n in effects)
assert "\nSounds\nBackgrounds\n" in tree
tree = tree.replace("\nSounds\nBackgrounds\n", "\n" + block + "Backgrounds\n")
open(tree_path, "w", encoding="utf-8", newline="\n").write(tree)

# --- the SS_* scripts --------------------------------------------------------------
# Handles are now sound resource indices (reals; -1 = none) instead of the DLL's
# strings. Volume (0..10000) and pan (-10000..10000) keep supersound's ranges and
# DirectSound meaning (volume is attenuation in hundredths of a dB). GameMaker 8.1
# cannot change pitch, so frequency is remembered, and lowering it (the music
# slowing down when the player dies) fades the music instead. There is no pause
# either: pausing stops the sound and resuming starts it again.
header = "// Instance mod: supersound.dll replaced by the built-in sound functions.\n"
load_cases = "".join(f'    case "sound/{f}": return {names[f]};\n' for f in files)
scripts = {
    "SS_Init": """
for (i = 0; i < 256; i += 1)
{
    global.ssVol[i] = 10000;
    global.ssFreq[i] = 44100;
    global.ssPaused[i] = false;
    global.ssLooping[i] = false;
}
return "1";
""",
    "SS_Unload": """
sound_stop_all();
""",
    "SS_LoadSound": """
// argument0: file name, argument1: 1 to stream (ignored)
// returns the sound resource, or -1
switch (argument0)
{
%s}
return -1;
""" % load_cases,
    "SS_Apply": """
// sets the built-in volume of sound argument0 from its supersound volume and frequency
var v, f;
v = global.ssVol[argument0];
f = min(1, global.ssFreq[argument0] / 44100);
if (v <= 0) sound_volume(argument0, 0);
else sound_volume(argument0, power(10, (v - 10000) / 2000) * f * f * f * f);
""",
    "SS_PlaySound": """
if (argument0 < 0) return "0";
global.ssPaused[argument0] = false;
global.ssLooping[argument0] = false;
sound_play(argument0);
return "1";
""",
    "SS_LoopSound": """
if (argument0 < 0) return "0";
global.ssPaused[argument0] = false;
global.ssLooping[argument0] = true;
sound_loop(argument0);
return "1";
""",
    "SS_StopSound": """
if (argument0 < 0) return "0";
global.ssPaused[argument0] = false;
sound_stop(argument0);
return "1";
""",
    "SS_FreeSound": """
if (argument0 < 0) return "0";
sound_stop(argument0);
return "1";
""",
    "SS_PauseSound": """
if (argument0 < 0) return -1;
if (!sound_isplaying(argument0)) return 0;
global.ssPaused[argument0] = true;
sound_stop(argument0);
return 1;
""",
    "SS_ResumeSound": """
if (argument0 < 0) return -1;
if (!global.ssPaused[argument0]) return 0;
global.ssPaused[argument0] = false;
if (global.ssLooping[argument0]) sound_loop(argument0);
else sound_play(argument0);
return 1;
""",
    "SS_IsSoundPlaying": """
if (argument0 < 0) return -1;
return sound_isplaying(argument0);
""",
    "SS_IsSoundPaused": """
if (argument0 < 0) return -1;
return global.ssPaused[argument0];
""",
    "SS_IsSoundLooping": """
if (argument0 < 0) return -1;
return global.ssLooping[argument0];
""",
    "SS_IsHandleValid": """
return sound_exists(argument0);
""",
    "SS_SetSoundVol": """
// argument1: 0 (muted) .. 10000 (full volume)
if (argument0 < 0) return "0";
global.ssVol[argument0] = median(0, real(argument1), 10000);
SS_Apply(argument0);
return "1";
""",
    "SS_GetSoundVol": """
if (argument0 < 0) return -1;
return global.ssVol[argument0];
""",
    "SS_SetSoundFreq": """
// argument1: 100 .. 100000 (44100 = normal speed; lower values fade the sound)
if (argument0 < 0) return "0";
global.ssFreq[argument0] = median(100, real(argument1), 100000);
SS_Apply(argument0);
return "1";
""",
    "SS_GetSoundFreq": """
if (argument0 < 0) return -1;
return global.ssFreq[argument0];
""",
    "SS_SetSoundPan": """
// argument1: -10000 (left) .. 10000 (right)
if (argument0 < 0) return "0";
sound_pan(argument0, median(-10000, real(argument1), 10000) / 10000);
return "1";
""",
    "SS_GetSoundPan": """
return 0;
""",
    "SS_GetSoundPosition": """
return 0;
""",
    "SS_SetSoundPosition": """
return 0;
""",
    "SS_GetSoundLength": """
return 0;
""",
    "SS_GetSoundBytesPerSecond": """
return 0;
""",
}

script_dir = os.path.join(out, "scripts")
for name, body in scripts.items():
    path = os.path.join(script_dir, name + ".gml")
    if name != "SS_Apply":
        assert os.path.exists(path), name
    open(path, "w", encoding="latin-1", newline="\n").write(header + body.lstrip("\n"))

# SS_Apply is new: no index, so the packer gives it a free slot and a tree leaf
# (docs/project-folder.md); put it next to the other SS_* scripts.
tree = open(tree_path, encoding="utf-8").read()
tree = re.sub(r"(\n(\s+)SS_Unload\n)", r"\1\2SS_Apply\n", tree, count=1)
open(tree_path, "w", encoding="utf-8", newline="\n").write(tree)

# --- game information: say it is a mod (Spelunky User License 3.(4)) ---------------
info_path = os.path.join(out, "game_information.rtf")
info = open(info_path, encoding="latin-1").read()
note = (r"\par This is a mod of the Spelunky 1.1 source, made to run in Instance: sound "
        r"goes through the built-in sound functions instead of supersound.dll. "
        r"Spelunky \'a9 2008, 2009 Derek Yu and Mossmouth, LLC; shared under the "
        r"Spelunky User License v1.1b.\par ")
i = info.rfind("}")
open(info_path, "w", encoding="latin-1", newline="").write(info[:i] + note + info[i:])
print("mod written to", out, "-", len(files), "sounds")
