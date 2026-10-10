# instance-music

Write game music as text, get a Standard MIDI File (what GameMaker 8.1 plays
as background music) or a WAV rendered through a SoundFont; then **measure**
sounds you cannot hear.

* `gmk-music render SONG.txt -o song.mid` — tiny MIDI file (kind 1 sound in GM).
* `gmk-music render SONG.txt -o song.wav --loop [--rate 22050] [--mono]` — seamless loop.
* `gmk-music info SONG.txt` — duration, sections, per-track stats, warnings.
* `gmk-sounds GAME.gm81` — analyses every sound in a project (levels,
  spectrum, description, loop seams, which effects sound alike).

```sh
cargo build --release -p instance-music          # target/release/gmk-music(.exe), gmk-sounds(.exe)
export INSTANCE_SOUNDFONT=/path/to/GeneralUser-GS.sf2   # needed for WAV output and MIDI analysis
```

Examples: [`examples/explore.txt`](examples/explore.txt) (calm loop, 87 s, two
variants), [`examples/underground.txt`](examples/underground.txt) (tense loop,
77 s, two variants), [`examples/fanfare.txt`](examples/fanfare.txt) (10 s one-shot).

---

## 1. A complete small song

```text
title "Meadow"
tempo 96                      # BPM (quarter notes)
time 4/4
key A minor                   # enables roman numerals and the out-of-key check

track pad   inst="Pad 2 (warm)" oct=4 vol=70 reverb=80 gate=100
track bass  inst="Fingered Bass" oct=2 human=6
track lead  inst=Flute oct=5 vol=95 human=5
track drums drums

prog verse = | Am | F | C | G |            # one chord per bar
pattern walk = 1q. 5e 8q 5q                 # chord degrees: root, fifth, octave
pattern hook = E5q. D5e C5q A4q | A4h C5h | C5q. B4e A4q E5q | D5w |

grid beat                                   # 16 steps = one 4/4 bar of 16ths
  kick   x.......x.x.....
  snare  ....x.......x...
  hat    x.x.x.x.x.x.x.xo

section A bars=4
  pad:   mp *w over verse                   # whole chords, voice-led
  bass:  mf walk over verse
  drums: beat*4
section B bars=4
  pad:   mf *w over verse
  bass:  mf walk over verse
  lead:  mf hook
  drums: beat*4

arrange |: A B B :|                          # loop region (for --loop)
```

`gmk-music info meadow.txt` prints what was produced and flags mistakes
(bar checks, part lengths, ranges, missing dynamics, silence).

---

## 2. Reference

### File structure

One statement per line. `#` (at line start or after a space) and `//`
start comments; a trailing `\` continues the line. Names: letters, digits, `_`.

| Statement | Meaning |
|---|---|
| `title "Name"` | MIDI title |
| `tempo 120` | song tempo, BPM. In a section: that section's tempo; `tempo 120->80` ramps across the section |
| `time 3/4` | time signature (song, or the current section) |
| `key D major` / `key E phrygian` / `key Am` | scales: major minor dorian phrygian lydian mixolydian locrian harmonic_minor melodic_minor pentatonic minor_pentatonic blues whole_tone chromatic |
| `swing 58` | swing for off-beat eighths, 50 (straight) … 75 |
| `track NAME opt=val …` | a track (MIDI channel), see below |
| `pattern NAME = notes…` | reusable notes |
| `prog NAME = \| Am \| F G \|` | named chord progression |
| `grid NAME [step=s]` + rows | step-sequencer pattern (drums, riffs) |
| `section NAME [bars=8] [tempo=90] [time=3/4]` | starts a section; following `TRACK: notes` lines belong to it |
| `TRACK: notes…` | notes for TRACK in the current section; several lines for the same track **concatenate**. Outside any section they go to an implicit section `main` |
| `arrange intro \|: A B A(+2) :\| outro` | play order; default: every section once, in order |
| `variant NAME` + indented lines | re-orchestration (§2.9) |
| `end` | ends a section/variant block (optional) |

Definitions (`pattern`, `prog`, `grid`, `track`) must come **before** use.

### Track options

| Option | Default | Notes |
|---|---|---|
| `inst=` | 1 (piano) | GM name (`"String Ensemble 1"`, `pad_2_warm`, `warm pad`, `strings`) or **1-based** GM number 1–128. `gmk-music instruments` lists them |
| `drums` / `drums=KIT` | — | channel 10; kits: standard room power electronic tr808 jazz brush orchestra |
| `oct=` | 4 | octave for notes without one, for `{chords}` and chord degrees |
| `vol=` `expr=` | 100, 127 | CC7 / CC11, 0–127 |
| `pan=` | 0 | -100 (left) … 100 (right) |
| `reverb=` `chorus=` | 40, 0 | CC91 / CC93 |
| `vel=` | 80 | starting velocity (= `mf`) |
| `gate=` | 95 (drums 50) | % of each note's length that sounds; use 100 for pads/legato |
| `human=` | 0 | random velocity ± N (deterministic) |
| `transpose=` | 0 | semitones (not drums) |
| `swing=` | song | per-track swing |
| `ch=` | auto | 1–16 (auto skips 10) |
| `bendrange=` | 2 | pitch-bend range in semitones |

### Notes

```text
C4q  D#4e.  Bb3h~ Bb3q  F4>  G4^  A4@110  E5:2w  Cq  r  rh.  r:bar
```

* **Pitch**: uppercase letter, `#`/`b` accidentals, octave digit; `C4` = middle C
  (MIDI 60), `A4` = 440 Hz. Without an octave (`Eb`) the current octave is used.
  Lowercase `c4` is an error (lowercase words are drums, patterns, keywords).
* **Duration** right after the pitch: `w` whole, `h` half, `q` quarter, `e`
  eighth, `s` sixteenth, `t` 32nd; `.` dotted, `..` double-dotted, `3`
  triplet (`e3` = one eighth-triplet), `5` quintuplet; `+` adds (`h+e`).
  After a `:` also multiples and bars: `C4:3q`, `r:2bar`, `kick:e`.
  **Omitted = the previous duration** (each line/pattern starts at `q`).
* **Rest**: `r` + duration (`rq`, `rh.`, `r:2w`). `r h` is an error — attach durations.
* **Modifiers** after the duration: `>` accent (+16 velocity, repeatable),
  `^` staccato (half length), `~` tie to the next note of the same pitch,
  `@100` explicit velocity.
* **Repeat one item**: `rw*6`, `kick:e*4`, `C4s*8`.
* **Octave**: `o3` sets the current octave, `o+1` / `o-1` shift it.
* **Bar check**: `|` warns (in `info`/`render`) if it is not on a bar line —
  write one between bars; it is the cheapest way to catch rhythm mistakes.

### Chords

* Note list: `[C4 E4 G4]h`, `[kick crash]q` — duration/modifiers after `]`.
* Chord symbol: `{Am7}h`, `{Cmaj7/G}w`, `{F#dim}q`, `{Bbsus4}`. Voiced
  automatically near the track octave with **smooth voice leading** from the
  previous chord; a slash bass is added below.
  Qualities: (major) `m` `dim` `aug` `+` `sus2` `sus4` `5` `6` `m6` `69` `7`
  `maj7` `m7` `mmaj7` `dim7` `m7b5` `ø` `7sus4` `aug7` `add9` `madd9` `9`
  `maj9` `m9` `7b9` `7#9` `11` `m11` `13`.
* Roman numerals (need `key`): `{i}` `{VI}` `{bVII}` `{V7}` `{ii7}` `{viio}`
  `{iiø7}`. Uppercase = major, lowercase = minor; degrees come from the key's
  scale (in A minor, `VI` = F).

### Patterns, groups, repetition, transposition

```text
pattern riff = E4e G4 A4 B4 | D5q B4 A4 G4 |
lead: riff riff*2 riff+5 riff*2-12     # *N repeat, +N/-N semitones
lead: (C4e D4 E4)*3 F4q.               # inline group, same suffixes
```

A pattern starts with duration `q` but inherits the caller's octave and
velocity; its `|` bar checks are relative to its own start.

### Chord progressions and `over` (accompaniment in one line)

```text
prog verse = | Am | F G | C . . G | % | x |
bass: 1q. 5e 8q 5q over verse
keys: 1e 5 8 10 12 10 8 5 over | Dmaj7 | Bm7 | Gmaj7 | A7sus4 A7 |
pad:  *w over | i | VI | III | VII |
```

* Bars are split by `|`; chords in a bar share it evenly; `.` extends the
  previous chord by one slot (`| C . . G |` = C for 3 beats, G for 1);
  `%` repeats the previous bar; `x` = silence.
* `PATTERN over PROGRESSION`: the pattern **restarts on every chord** and is
  repeated / cut to the chord's length. Everything before `over` on the line
  is the pattern (leading `mf`, `o3`, `vol=…` stay outside and apply first);
  the rest of the line is the progression, or a `prog` name (`verse*2`).
* Inside such patterns, numbers are **chord degrees** placed from the chord
  root in the current octave: `1` root, `3` third, `5` fifth, `7` seventh,
  `9` `11` `13`; `8` `10` `12` `14` `15` are the octave-up forms. `'` raises
  an octave, `,` lowers (`5,` = fifth below the root). A degree the chord
  lacks uses the usual interval (b7 for `7`). `0` = the chord's bass
  (the slash bass of `D/F#`, else the root), at or below `1`.
  `*` = the whole chord, voice-led (`*w`, `*q. *e rh`).

### Grids (drums and riffs)

```text
grid beat step=s          # each cell is a 16th (default); step=e for eighths
  kick   x...x...x...x...
  snare  ....x.......x..o
  ohat   ..x...x...x...x.
  C2     x==.x==.........  # pitched rows: notes or chord degrees (with `over`)
```

`x` hit, `X` accent, `o` ghost (half velocity), `=` holds the previous hit
one more step, `.` `-` `_` rest; spaces and `|` are ignored. All rows must
have the same number of steps. A grid is used like a pattern: `drums: beat*4`.

Drum names (`gmk-music drums`): kick kick2 snare snare2 rim clap hat phat
(pedal) ohat crash crash2 china splash ride ride2 bell (ride bell) tom1 (high)
… tom6 (low floor) — aliases hitom midtom lowtom floortom — tamb cowbell
shaker cabasa maracas claves wood_hi wood_lo triangle bongo_hi/lo conga_hi/lo
timbale_hi/lo agogo_hi/lo guiro whistle vibraslap castanets jingle sticks …

### Dynamics and controllers (inline, take no time)

* Dynamics set the velocity of following notes: `ppp 24`, `pp 36`, `p 50`,
  `mp 64`, `mf 80`, `f 96`, `ff 112`, `fff 124`; or `vel=90`.
* `vol=` `expr=` `pan=` `mod=` `reverb=` `chorus=` `cutoff=` `reso=`
  `attack=` `release=` `delay=` (synth delay send) `ccN=` (any controller), `bend=` (semitones),
  `inst=NAME` (program change; use `_` for spaces).
* Ramps: `expr=40->120:4bar`, `pan=-60->60:2w` (smooth steps over the length).
  Use `expr` for swells and fades — `vol` is the mix level.
* Sustain pedal: `ped+` … `ped-`.

### Sections and arrangement

* A section's length is `bars=N`, else its longest part rounded up to whole
  bars. Shorter parts are padded with silence (`info` notes it); with
  `bars=N` longer parts are cut.
* A section without `tempo` plays at the song tempo.
* `arrange`: `A*2`, `A(+2)` (transpose pitched tracks), `A(-drums -lead)`
  (mute tracks), `A(x2 +5 -bass)`; `|:` and `:|` mark the loop region.

### Variants (per-area versions of one theme)

```text
variant cave            # lines must be indented
  tempo 80              # new base tempo (all tempi scale)
  transpose -2          # pitched tracks
  inst lead Ocarina
  synth lead supersaw   # another built-in synth preset (§2.10)
  vol drums 60
  shift bells 12        # transpose one track
  mute drums bass
  arrange A B A         # a different section order (may use extra sections)
```

`--variant cave` renders it; `--variant all` renders the base and every
variant (`out.mid`, `out_cave.mid`, …). Keep the leitmotif in patterns and
build area variants by re-orchestrating, transposing and re-arranging.

### Built-in synthesiser (`synth=`) — for synthwave, electronic, chiptune-ish

General MIDI SoundFont patches sound like "MIDI": no filter sweeps, no
detuned supersaws, no sidechain pumping. A track with `synth=PRESET` is played
by a built-in synthesiser instead when rendering **WAV** (MIDI output writes
its `inst=` program, or a GM stand-in, so it still plays in GameMaker). A song
whose tracks are all `synth=` needs no SoundFont.

```text
track lead  synth=lead oct=5 vol=104 delay=38 reverb=45
track arp   synth=pluck oct=4 vol=90 pan=-30 delay=50 duck=35 gate=60
track pad   synth=pad oct=4 vol=76 reverb=70 duck=80 gate=100     # pumps with the kick
track bass  synth=bass oct=2 vol=112 duck=45 gate=80
track drums drums synth=drums vol=108 reverb=25
...
  arp: mf cutoff=30->95:4bar arp16 over | F#m | D | A | E |     # filter sweep
```

`gmk-music synths` lists presets and options. Presets: `supersaw` (7 detuned
saws), `lead` (3 saws, mono, glide, delayed vibrato), `square` (PWM), `pluck`
(snappy filter envelope: arpeggios), `pad` (slow, drifting, chorus), `strings`
(string machine), `brass` (swelling filter), `bass` (saw + sub, punchy), `reese`
(detuned dark bass), `sub` (sine), `bell` (FM), `epiano` (FM tine), `riser`
(noise swept up by the filter), `drums` (drum machine: 808/Linn-style kick,
gated-reverb snare, clap, metallic hats, toms, cymbals — on a `drums` track).

Track options (synth tracks only): `cutoff` 0..127 (20 Hz·1000^(v/127):
64 ≈ 650 Hz, 90 ≈ 2.7 kHz, 110 ≈ 8 kHz), `reso` 0..127, `envamt` (filter
envelope, octaves), `attack` `decay` `release` 0..127 (1 ms·10000^(v/127): 64 ≈
100 ms), `sustain` 0..127, `detune` (cents), `unison` 1..8, `width` 0..100,
`glide` (ms), `drive` 0..100, `vib` (cents), `duck` 0..100 (sidechain: the kick
— any note 35/36 on the drum channel — pushes the track down; 60–80 on pads
gives the classic pump), `delay` 0..127 (send to a ping-pong delay in dotted
eighths). The inline controls `cutoff=` `reso=` `attack=` `release=` `delay=`
`mod=` (vibrato), `vol` `expr` `pan` `reverb` `chorus`, pitch bend and the pedal
drive it during the song; ramps make filter sweeps.

Balance by rendering each track alone (a variant that mutes the others,
`--normalize off`) and comparing RMS: lead ≈ bass ≈ drums, pads and arps
4–6 dB below. Render stereo (omit `--mono`): the unison, chorus and delay are wide.

---

## 3. Output and GameMaker

* **MIDI** (`.mid`): type 1, 480 ticks/quarter, a conductor track (tempo,
  time/key signatures, section markers, `loopStart`/`loopEnd` markers when
  `|: :|` is used) and one track per song track. Every track ends exactly at
  the song end, so `sound_loop` keeps the bar grid. GameMaker loops the
  **whole file**: put an intro in a separate sound, not before `|:`.
  GM 8.1 plays MIDI through the Windows synth ("Microsoft GS Wavetable
  Synth", max ~32 voices); the Instance runner uses the same or a SoundFont.
* **WAV**: rendered with rustysynth (the runner's SoundFont synth),
  peak-normalised to -1 dBFS (`--normalize -3`, `--normalize off`).
  `--loop` renders the loop region as the second of two passes, so the release
  tail of the end already sounds at the start: no gap, no click (the loop is
  stretched by at most 32 samples to a whole number of synth blocks).
  WAV is stored uncompressed in the project: 22050 Hz mono = 2.6 MB/min
  (32000 Hz stereo = 7.7 MB/min). `synth=` tracks only sound like themselves in WAV.
  Prefer MIDI for long music; use WAV when it must sound identical everywhere.

Adding a rendered song to a project with `instance-format`:

```rust
let src = instance_music::song::parse(&std::fs::read_to_string("explore.txt")?)?;
let song = instance_music::compile(&src, None)?;
let midi = instance_music::smf::write_smf(&song);
p.sounds.push(Some(Sound { name: "bgm_explore".into(), kind: 1, file_type: ".mid".into(),
    file_name: "explore.mid".into(), data: Some(midi), volume: 1.0, preload: true, ..Sound::default() }));
// in GML: sound_loop(bgm_explore);
```

### Rust builder

Same features from code (each method takes the same text as the notation line;
`part_items`/`pattern_items` accept `song::Item`s built programmatically):

```rust
let song = SongBuilder::new("Demo").tempo(100.0).key("A minor")
    .track("bass", "inst=\"Fingered Bass\" oct=2").track("drums", "drums")
    .pattern("walk", "1e 5 8 5")
    .grid("beat", "s", &[("kick", "x...x...x...x..."), ("hat", "x.x.x.x.x.x.x.x.")])
    .section("A", "").part("bass", "walk over | i | VI | III | VII |").part("drums", "beat*4")
    .variant("dark", "transpose -3\nmute drums")
    .arrange("|: A A :|")
    .compile(None)?;                         // or .build() for the SongSrc
let wav = render::render_song(&song, &sf, &RenderOpts { looped: true, ..Default::default() })?;
```

---

## 4. Writing music that sounds good (checklist)

* **Roles**: bass (oct 1–2, roots/fifths on strong beats), harmony (pad or
  arpeggio, oct 3–4, `*w over`/arp patterns), melody (oct 4–6, the motif),
  drums. Keep them in different registers; `info` shows each track's range.
* **Motif first**: a 2–4 bar idea in a `pattern`, repeated, varied
  (`hook+5`, a changed ending), answered by a contrasting phrase.
  Put chord tones on strong beats, passing notes on weak ones.
* **Form**: 8-bar sections, e.g. `intro A B A2`; vary the orchestration and
  dynamics per section (`mp` → `f`), thin out before the loop restarts, and end
  the loop on a chord that leads back to the first (V → I).
* **Dynamics**: every line should start with a dynamic; use `>` accents,
  ghost notes `o` in grids and `human=4..10`; `info` warns about flat velocity.
* **Loops**: 60–120 s for background music; check `render --loop` reports
  `loop seam: smooth`.
* **Mix**: pads `vol=60–75`, lead 90–110; pan instruments apart; reverb 60–100
  for calm, 30–60 for driving music.

## 5. gmk-sounds (analysis for authors who cannot listen)

```text
gmk-sounds GAME.gm81|FILE.wav|FILE.mid|FILE.mp3|FILE.ogg ... [--per-second] [--window SECS]
           [--json] [--soundfont PATH] [--threshold 0.85] [--loop] [--no-similarity]
```

Per sound: kind, format, duration, rate/channels; peak and RMS dBFS,
clipping %, DC offset, leading/trailing silence, attack time; spectral
centroid, flatness (0 = pure tone, >0.3 = noise), energy in bands sub <80 Hz,
low 80–300, mid 300–2k, high 2–8k, air >8k; a one-line description
(`short percussive bright noise`, `medium decaying mid tone ~880 Hz (A5),
rising`, `music/ambience, low register, varied loudness (14 dB …)`); and a
`check` line for problems (clipping, very quiet, DC, late start).

Loops (kind 1, `sound_loop(...)` found in the project's code, a name containing
"loop", or `--loop`) get a seam verdict: `smooth`, `CLICK likely` (the wrap
jumps far more than the samples around it), `GAP` (silence at the ends),
`FADES OUT` / `FADES IN`.

MIDI is rendered through the SoundFont first (note statistics are shown
either way). `--per-second` prints RMS/peak/centroid per window for sounds of
3 s or more — read it to see sections, build-ups and silent gaps.

The **similarity table** compares the sound effects of each project (not
music): 70 % correlation of mel-band spectral shape + 30 % envelope shape
(two loops: spectrum only), scaled down for very different lengths. Pairs at
or above `--threshold` (default 0.85) are listed as **THESE SOUND ALIKE**:
make them differ in register (centroid), texture (tone vs noise), envelope or
rhythm. Calibration: deepcore's drill and thruster loops (reported by a player
as too similar) score 0.91; its next most similar pair scores 0.76.
`--json` emits everything, including the matrices.
