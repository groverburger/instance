# Sound and music — behavioural specification

Sources: manual chapter 406 (`406_00` … `406_04`), resource pages `108_00`,
`203_00`, `408_02`, `409_02`; sound resource record in `file-formats.md` §5.1;
summary in `runtime.md` §15. Paraphrased. Open items: `OPEN_QUESTIONS.md` section
`[SND]`. This document supersedes the stub behaviour listed as R-21 in
OPEN_QUESTIONS.

---

## 1. Sound resources

A sound resource has (file-formats §5.1): `kind`, original file name/extension,
the raw file bytes, `effects` bitmask, default `volume` (0..1), default `pan`
(−1..1), `preload` flag.

### 1.1 Kinds `[406_00]`, `[203_00]`, `[408_02]`

| kind | Name | Behaviour |
|---|---|---|
| 0 | normal | Sound effects (usually WAV). Any number may play at once, including several copies of the same sound. Effects, volume, pan allowed. |
| 1 | background music | Usually MIDI, sometimes WAV. Same as normal except that **only one background-music sound plays at a time**: starting one stops the one currently playing `[406_00]`. |
| 2 | 3D | Positional sound (WAV or MIDI), **mono** `[406_00]`; controlled by `sound_3d_*` (§6). |
| 3 | multimedia player | Anything else, notably MP3. Only one at a time; no effects; no volume or pan changes; loop timing may be imprecise and start may be delayed `[406_00]`. |

The IDE gives MIDI files kind 1 by default `[203_00]`.

### 1.2 Formats (recommended decoders)
Detect by content, not extension:
* RIFF/WAVE — PCM 8/16/24/32-bit and IEEE float; IMA/MS ADPCM (via `hound`/
  `symphonia`).
* Standard MIDI File (`MThd`, also RIFF `RMID`) — §4.
* MP3 (and anything else `symphonia` decodes) — intended for kind 3. **Recommended:**
  also play it if it was given another kind (GM would fail silently; playing is
  friendlier and harmless).
* DirectMusic segments (`.sgt`) — not supported: the sound exists but plays nothing
  and `sound_isplaying` is false.

---

## 2. Playback functions `[406_01]`

| Function | Semantics |
|---|---|
| `sound_play(index)` | Start one playback (a new *voice*) of the sound, once. Kind 1: stop the current background music first. Kind 3: stop the current multimedia sound first. |
| `sound_loop(index)` | As `sound_play` but the voice loops forever. |
| `sound_stop(index)` | Stop every voice of the sound. |
| `sound_stop_all()` | Stop every voice of every sound (including MIDI and multimedia). |
| `sound_isplaying(index)` | True while any voice of the sound is audible. GM may lag in both directions (true slightly after stop due to echo tails, false right after play) `[406_01]`; **recommended:** true from the `sound_play` call until the voice (including effect tails) finishes or is stopped. |

* A normal/3D sound played while already playing starts an extra overlapping voice.
* **Recommended** voice limit: 128 simultaneous PCM voices; when exceeded, the
  oldest non-looping voice is stopped.
* Non-existent index: **recommended** ignore silently (no error) for all sound
  functions; query functions return 0/false/`""`.
* Sounds are independent of rooms: they keep playing across room changes
  (runtime.md §15). `game_restart` and game end stop all sounds (runtime.md §3.3).
  Playing sounds are not saved by `game_save`.
* If there is no audio device the game runs and all sound calls succeed silently;
  `sound_isplaying` follows the logical state (recommended).

### 2.1 Actions
`action_sound(sound, loop)` = `sound_loop` if loop else `sound_play`;
`action_end_sound(sound)` = `sound_stop`; `action_if_sound(sound)` =
`sound_isplaying` (see `actions.json`).

---

## 3. Volume, pan, fade, tempo `[406_01]`

All of these only **attenuate** (never amplify) `[406_01]` and do not work for kind 3
sounds `[406_01]`.

| Function | Semantics |
|---|---|
| `sound_volume(index,value)` | Set the sound's volume, 0 (silent) .. 1 (full). Applies to its playing voices and later plays. Cancels a running fade of that sound. |
| `sound_global_volume(value)` | Master volume 0..1 for all sounds. |
| `sound_fade(index,value,time)` | Change the sound's volume from its current value to `value` linearly over `time` milliseconds (real time). For fading music in or out `[406_01]`. A new fade replaces a running one; `time ≤ 0` = immediate. The fade proceeds whether or not the sound is playing; the final value persists. |
| `sound_pan(index,value)` | Pan −1 (left) .. 0 (centre) .. 1 (right). Applies to playing voices and later plays. |
| `sound_background_tempo(factor)` | Multiply the tempo of MIDI background music by `factor`; 1 = normal; valid 0.01..100 `[406_01]` (**recommended** clamp). Persists; affects the current and future MIDI playback. No effect on WAV. |
| `sound_set_search_directory(dir)` | Directory (no trailing backslash) where DirectMusic files look for referenced band/style files `[406_01]`. Stored; no effect in Instance. |

Initial per-sound volume and pan = the resource's stored values; the master volume
starts at 1. Values are clamped to their ranges.

Mapping to gain (**UNSPECIFIED – recommended**, SND-1/SND-2):
```
gain        = clamp(volume,0,1) · clamp(global,0,1)        (linear amplitude)
left_gain   = gain · (1 − max(pan, 0))
right_gain  = gain · (1 + min(pan, 0))
```
(GM routes these to DirectSound attenuation in hundredths of a decibel; the exact
curve must be measured. If measurement shows a dB-linear mapping, replace the
linear factors by `10^(−A·(1−v)/20)`-style curves with the measured A.)

Mono sources are duplicated to both channels before panning; stereo sources have
each channel scaled by its side's gain.

---

## 4. MIDI

### 4.1 Rules
* GM plays MIDI (kinds 0–2) through DirectMusic's software synthesiser, so it
  sounds the same on all machines; via the multimedia player (kind 3) the system's
  synthesiser is used and may sound different `[203_00]`.
* By default only one MIDI plays at a time `[108_00]`. **Recommended:** a single MIDI
  player; starting any MIDI sound (kinds 0–2) stops the MIDI currently playing.
  Background-music exclusivity (kind 1) still applies across WAV and MIDI.
* Looping (`sound_loop`) restarts at the beginning seamlessly; at the loop point
  send all-notes-off and reset controllers on all channels, then replay from tick 0.
* `sound_background_tempo` scales the sequencer clock (§3).
* Volume/pan/fade apply (scale channel volume/pan, §4.3). Effects apply only on the
  SoundFont path (§4.2). 3D MIDI (kind 2): apply distance gain and pan only.

### 4.2 Recommended implementation
1. Parse the SMF with an own sequencer (e.g. `midly` for parsing), formats 0 and 1,
   tempo map, running status, SysEx passed through.
2. **Primary output on Windows: the OS synthesiser** through the multimedia MIDI
   output API (`midiOutOpen` on `MIDI_MAPPER` / device 0 → "Microsoft GS Wavetable
   Synth", `midiOutShortMsg`/`midiOutLongMsg`). This is the closest match to GM's
   sound set. Send a GS reset at open; send all-notes-off on stop.
3. **Fallback (and non-Windows): SoundFont synthesis** with `rustysynth`, rendered
   into the PCM mixer (so volume, pan, effects and 3D behave like WAV). SoundFont
   lookup order: runner option/path in `INSTANCE_SOUNDFONT` (or `GM81_SOUNDFONT`), then
   `<runner dir>/soundfonts/default.sf2`, then a platform default if any. If no
   SoundFont is available MIDI plays silently but `sound_isplaying` follows the
   logical state (looping music reports true).
4. Runner option to force the SoundFont path on Windows.

### 4.3 Volume on the OS-synth path
Do not use the device-wide `midiOutSetVolume` (it changes the system mixer).
Instead intercept Control Change 7 (channel volume) and 10 (pan) from the file and
send `CC7 = round(file_cc7 · gain)` (file default 100 when absent) and pan
re-centred by the GM pan; re-send all 16 channels whenever volume/pan/fade
changes (fades: at most every 20 ms).

---

## 5. Effects `[406_02]`

### 5.1 Selection
`sound_effect_set(snd, effect)` sets the combination of effects (sum of `se_*`
flags) for a sound, replacing the previous combination and **resetting all effect
parameters to their defaults** `[406_02]`. The resource's stored `effects` field
(bits 1,2,4,8,16 only) is the initial combination. Effects apply to kinds 0–2
(WAV and SoundFont-rendered MIDI), not to kind 3 `[406_02]`.

* Effects apply to the sound's voices that start after the call (**recommended**:
  also to already playing voices).
* Processing order (**recommended**): chorus → echo → flanger → gargle → reverb →
  compressor → equalizer (ascending flag value).
* Parameter functions apply only to an effect currently selected for the sound
  (**recommended**: ignored otherwise); every parameter is clamped to its range.

### 5.2 Parameters (ranges and defaults as stated in `[406_02]`)

`sound_effect_chorus(snd,wetdry,depth,feedback,frequency,wave,delay,phase)` and
`sound_effect_flanger(…same…)` — modulated-delay effects:

| Param | Meaning | Range | Default |
|---|---|---|---|
| wetdry | processed : unprocessed mix (%) | 0..100 | 50 |
| depth | LFO modulation depth of the delay | 0..100 | 25 |
| feedback | output fed back into input (%) | −99..99 | 0 |
| frequency | LFO frequency (Hz) | 0..10 | 0 |
| wave | LFO shape: 0 triangle, 1 sine | 0/1 | 1 |
| delay | base delay (ms) | 0..20 | 0 |
| phase | left/right LFO phase difference (0..4 → −180°, −90°, 0°, 90°, 180°) [I] | 0..4 | 2 |

`sound_effect_echo(snd,wetdry,feedback,leftdelay,rightdelay,pandelay)`:

| Param | Meaning | Range | Default |
|---|---|---|---|
| wetdry | mix (%) | 0..100 | 50 |
| feedback | feedback (%) | 0..100 | 0 |
| leftdelay / rightdelay | per-channel delay (ms) | 1..2000 | 333 / 333 |
| pandelay | swap left/right delays on each repeat (0 no, 1 yes) | 0/1 | 0 |

`sound_effect_gargle(snd,rate,wave)`: amplitude modulation; `rate` Hz 1..1000
(default 1); `wave` 0 triangle, 1 square (default 0).

`sound_effect_reverb(snd,gain,mix,time,ratio)`:

| Param | Meaning | Range | Default |
|---|---|---|---|
| gain | input gain (dB) | −96..0 | 0 |
| mix | reverb mix (dB) | −96..0 | 0 |
| time | reverb time (ms) | 0.001..3000 | 1000 |
| ratio | high-frequency decay-time ratio | 0.001..0.999 | 0.001 |

`sound_effect_compressor(snd,gain,attack,release,threshold,ratio,delay)`:

| Param | Meaning | Range | Default |
|---|---|---|---|
| gain | make-up gain (dB) | −60..60 | 0 |
| attack | attack time (ms) | 0.01..500 | 0.01 |
| release | release time (ms) | 50..3000 | 50 |
| threshold | threshold (dB) | −60..0 | −10 |
| ratio | compression ratio (n:1) | 1..100 | 10 |
| delay | look-ahead delay (ms) | 0..4 | 0 |

`sound_effect_equalizer(snd,center,bandwidth,gain)`: one peaking band; `center` Hz
80..16000, `bandwidth` semitones 1..36, `gain` dB −15..15. Defaults not stated;
**recommended** 8000 Hz, 12 semitones, 0 dB.

### 5.3 DSP recommendations
These mimic the DirectX Media Object effects GM used; bit-exact output is not
required:
* chorus/flanger: stereo modulated delay line (flanger uses shorter delays), LFO per
  parameters, feedback, wet/dry.
* echo: two delay lines with feedback, optional ping-pong swap.
* gargle: multiply by a unipolar triangle/square LFO at `rate`.
* reverb: a Schroeder/Freeverb-style network with decay time `time` and
  high-frequency damping from `ratio`, input gain and wet level in dB.
* compressor: peak/RMS feed-forward compressor with the given parameters.
* equalizer: RBJ peaking biquad.
Effect tails keep a voice "playing" until they decay below −90 dB (cap 3 s).

---

## 6. 3D sound `[406_03]`

Only for kind-2 sounds `[406_03]` (calls on other kinds are ignored). Settings are per
sound resource and apply to all its voices. Coordinates: the listener is fixed at
the origin; **x** grows to the right, **y** upward, **z** away from the listener
(forward); units are metres `[406_03]`. Listener orientation: facing +z, up +y.
The listener functions present (commented out) in `fnames`
(`sound_3d_set_listener_*`, `_distance_factor`, `_rolloff_factor`,
`_doppler_factor`) are **not** GML functions in 8.1.

| Function | Semantics | Default |
|---|---|---|
| `sound_3d_set_sound_position(snd,x,y,z)` | source position | (0,0,0) |
| `sound_3d_set_sound_velocity(snd,x,y,z)` | velocity (m/s), used only for Doppler; does not move the sound `[406_03]` | (0,0,0) |
| `sound_3d_set_sound_distance(snd,mindist,maxdist)` | full loudness up to `mindist`; attenuates beyond it up to `maxdist` | 1 m, 1 000 000 000 m `[406_03]` |
| `sound_3d_set_sound_cone(snd,x,y,z,anglein,angleout,voloutside)` | directional source: cone axis (x,y,z), inner/outer cone angles (degrees, full angles), `voloutside` = attenuation outside the outer cone in hundredths of dB (negative) | omnidirectional (360, 360, 0) |

Recommended rendering model (DirectSound-like, **[I]**):
```
d          = |p|                    (p = position)
dist_gain  = mindist / clamp(d, mindist, maxdist)     (rolloff factor 1)
θ          = angle between cone axis and (−p) (direction from source to listener)
cone_dB    = 0                         if θ ≤ anglein/2
           = voloutside/100            if θ ≥ angleout/2
           = linear interpolation in θ between them otherwise
cone_gain  = 10^(cone_dB/20)
pan        = clamp(p.x / max(d, ε), −1, 1)  → equal-power panning
doppler    = 343 / (343 + v·p̂)        (pitch factor; v·p̂ > 0 = receding), clamp 0.25..4
gain       = dist_gain · cone_gain · volume · global
```
The source is downmixed to mono first. Sounds at the origin are centred.
Beyond `maxdist` the gain stops decreasing (DirectSound semantics); the manual
phrases `maxdist` as where the sound can no longer be heard — with a 1-billion-metre
default this only matters for user-set values (SND-4).

---

## 7. Resource management `[408_02]`, `[409_02]`, `[203_00]`

| Function | Semantics |
|---|---|
| `sound_exists(ind)` | whether a sound with that index exists |
| `sound_get_name(ind)` | resource name (`""` if none) |
| `sound_get_kind(ind)` | 0 normal, 1 background, 2 3D, 3 multimedia (−1 if none, **recommended**) |
| `sound_get_preload(ind)` | the preload flag |
| `sound_discard(index)` | free the decoded audio memory. **Recommended:** stops its voices; a later play reloads transparently (like a non-preloaded sound). |
| `sound_restore(index)` | load the sound into audio memory now |
| `sound_add(fname,kind,preload)` | add a new sound from a file; returns the new index, −1 on error (e.g. missing file). Name **recommended** `__newsound<index>`. Volume 1, pan 0, effects none. |
| `sound_replace(index,fname,kind,preload)` | replace the sound's data and kind (stopping its voices); returns success |
| `sound_delete(index)` | stop and remove the sound; the index stays unused |

* Preload: preloaded sounds are decoded at game start; others on first use
  (decoding may run on a worker thread, but the first `sound_play` must still be
  heard — block briefly or start as soon as ready).
* `kind` outside 0..3: **recommended** clamp to 0.

---

## 8. CD audio `[406_04]`

| Function | Semantics | Instance default (no CD support) |
|---|---|---|
| `cd_init()` | must be called before the others and after changing CDs | no-op |
| `cd_present()` | CD in the default drive | false |
| `cd_number()` | number of tracks | 0 |
| `cd_playing()` | playing | false |
| `cd_paused()` | paused **or stopped** | true |
| `cd_track()` | current track (1 = first) | 0 |
| `cd_length()` | total length (ms) | 0 |
| `cd_track_length(n)` | length of track n (ms) | 0 |
| `cd_position()` | position on the CD (ms) | 0 |
| `cd_track_position()` | position in the current track (ms) | 0 |
| `cd_play(first,last)` | play tracks first..last (1,1000 = whole CD) | no-op |
| `cd_stop()`, `cd_pause()`, `cd_resume()` | transport | no-op |
| `cd_set_position(pos)`, `cd_set_track_position(pos)` | seek (ms) | no-op |
| `cd_open_door()`, `cd_close_door()` | tray | no-op |

Optional on Windows: implement with MCI `cdaudio` commands.

`MCI_command(str)`: sends a command string to the Windows Media Control Interface and
returns its reply string `[406_04]`. **Recommended:** on Windows pass through to
`mciSendStringA` and return the reply (or `""` on error); elsewhere return `""`.

---

## 9. Constants

| Constant | Value | Source |
|---|---|---|
| `se_none` | 0 | [I] |
| `se_chorus` | 1 | [I] matches resource effect bit 1 |
| `se_echo` | 2 | [I] bit 2 |
| `se_flanger` | 4 | [I] bit 4 |
| `se_gargle` | 8 | [I] bit 8 |
| `se_reverb` | 16 | [I] bit 16 |
| `se_compressor` | 32 | [I] (code only; not in the sound form) |
| `se_equalizer` | 64 | [I] (code only) |

Sound kinds 0–3 are documented numbers, not named constants (`fnames` has none).
