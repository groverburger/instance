# Sound and music

Sounds are resources, just like sprites. A sound can be a short effect (usually a WAV
file) or a piece of music (MIDI, MP3 or WAV). Sound plays independently of rooms:
music keeps going when the room changes.

## Kinds of sound

Each sound resource has a kind, chosen in the sound editor:

| Kind | Meant for | Behaviour |
|---|---|---|
| 0 Normal | sound effects | Any number can play at once, including several copies of the same sound. Volume, pan and effects work. |
| 1 Background music | music | Like normal, except that only one background-music sound plays at a time: starting one stops the one that was playing. |
| 2 3D | positional sound | A mono sound placed in 3D space with the `sound_3d_*` functions. |
| 3 Multimedia player | MP3 and other formats | One at a time; no volume, pan or effects. |

MIDI files get kind 1 when you load them in the IDE. Instance recognises the file
format from its contents, so an MP3 given another kind still plays.

## Playing and stopping

```gml
sound_play(snd_jump);          // play once
sound_loop(mus_level1);        // play over and over
sound_stop(mus_level1);        // stop every copy of this sound
sound_stop_all();              // silence everything
```

- [`sound_play(snd)`](../reference/sound-and-music.md#sound_play) starts a new copy of the
  sound each time. Calling it every step while a key is held starts a new copy every
  step; play sounds on events that happen once (a key *press*, a collision, a Create
  event).
- [`sound_loop(snd)`](../reference/sound-and-music.md#sound_loop) repeats the sound
  seamlessly until it is stopped.
- [`sound_stop(snd)`](../reference/sound-and-music.md#sound_stop) stops all copies of a
  sound; [`sound_stop_all()`](../reference/sound-and-music.md#sound_stop_all) stops
  every sound.
- Playing a sound that doesn't exist reports the error *"Sound does not exist."* but
  lets your code continue.
- `game_restart()` and ending the game stop all sounds. Playing sounds are not part of
  a saved game.

If the computer has no sound device, the game still runs; sound calls simply do
nothing audible.

## sound_isplaying lags behind

[`sound_isplaying(snd)`](../reference/sound-and-music.md#sound_isplaying) tells whether a
sound is currently audible. It reports the state of about a tenth of a second
earlier (105 ms in Instance): right after `sound_play` it is still false for a few
steps, and after `sound_stop` or the natural end it stays true for a few steps.

That matters for this tempting pattern:

```gml
// Step event — DON'T: starts the music several times over
if (!sound_isplaying(mus_title)) sound_loop(mus_title);
```

Because the answer stays false for several steps, `sound_loop` runs several times and
several copies of the music play on top of each other. Start music once instead, for
example in the Room Start event, or remember it yourself:

```gml
// Game Start event of a controller object
global.music = -1;
```

```gml
// Room Start event
if (global.music != mus_title) {
    if (global.music >= 0) sound_stop(global.music);
    sound_loop(mus_title);
    global.music = mus_title;
}
```

(The delay can be changed with the runner option `--sound-status-delay MS`; `0` reports
the current state. Games should not depend on that.)

## Volume, panning and fading

| Function | Effect |
|---|---|
| [`sound_volume(snd, v)`](../reference/sound-and-music.md#sound_volume) | volume of one sound, 0 (silent) to 1 (full) |
| [`sound_global_volume(v)`](../reference/sound-and-music.md#sound_global_volume) | master volume for all sounds, 0 to 1 |
| [`sound_pan(snd, p)`](../reference/sound-and-music.md#sound_pan) | -1 left, 0 centre, 1 right |
| [`sound_fade(snd, v, ms)`](../reference/sound-and-music.md#sound_fade) | change the volume to `v` gradually over `ms` milliseconds |
| [`sound_background_tempo(f)`](../reference/sound-and-music.md#sound_background_tempo) | speed up or slow down MIDI music (1 is normal) |

Volume can only make sounds quieter, never louder than the original. The settings apply
to copies already playing and to later ones. A sound starts with the volume and pan
stored in its resource. None of this works for kind 3 (multimedia) sounds.

```gml
// fade the music out over two seconds before leaving the room
sound_fade(mus_level1, 0, 2000);
alarm[0] = room_speed * 2;   // Alarm 0: room_goto_next()
```

## MIDI music

On Windows, MIDI music is played through the system's built-in synthesiser. On macOS,
Linux and in the browser — or on Windows with the environment variable
`INSTANCE_MIDI=soundfont` — Instance renders MIDI itself with a SoundFont. It looks for
the file named by `INSTANCE_SOUNDFONT`, then for `soundfonts/default.sf2` next to the
runner, then in common system locations; the Windows release and the browser version
include one. Without a SoundFont, MIDI is silent (but `sound_isplaying` still behaves
as if it played).

Only one MIDI piece plays at a time: starting a MIDI sound stops the one playing.

## Effects

[`sound_effect_set(snd, effects)`](../reference/sound-and-music.md#sound_effect_set)
switches effects on for a sound. Add up the ones you want: `se_chorus`, `se_echo`,
`se_flanger`, `se_gargle`, `se_reverb`, `se_compressor`, `se_equalizer`, or `se_none`
for none. Setting the effects resets their parameters to the defaults; the
`sound_effect_*` functions (for example
[`sound_effect_echo()`](../reference/sound-and-music.md#sound_effect_echo) or
[`sound_effect_reverb()`](../reference/sound-and-music.md#sound_effect_reverb)) then tune
them.

```gml
sound_effect_set(snd_shout, se_echo + se_reverb);
sound_effect_echo(snd_shout, 40, 30, 250, 250, 0);
```

## 3D sound

Kind-2 sounds can be placed around the listener, who sits at the origin facing along
the z axis. Positions are in metres: x to the right, y up, z away from the listener.

- [`sound_3d_set_sound_position(snd, x, y, z)`](../reference/sound-and-music.md#sound_3d_set_sound_position)
  places the sound;
- [`sound_3d_set_sound_distance(snd, mindist, maxdist)`](../reference/sound-and-music.md#sound_3d_set_sound_distance)
  sets where it starts to get quieter and where it stops getting quieter;
- [`sound_3d_set_sound_velocity()`](../reference/sound-and-music.md#sound_3d_set_sound_velocity)
  adds a Doppler effect (it doesn't move the sound), and
  [`sound_3d_set_sound_cone()`](../reference/sound-and-music.md#sound_3d_set_sound_cone)
  makes the sound directional.

## Adding sounds at run time

[`sound_add(fname, kind, preload)`](../reference/resources.md#sound_add) loads a sound file
while the game runs and returns its index (-1 if it fails).
[`sound_replace()`](../reference/resources.md#sound_replace) swaps the data of an existing
sound and [`sound_delete()`](../reference/resources.md#sound_delete) removes one. Sounds not
marked *preload* are loaded the first time they play.

## CD audio

The `cd_*` functions exist for compatibility but do nothing in Instance: `cd_present()`
and `cd_playing()` return false, `cd_paused()` returns true, and the other queries
return 0.

See also: [Sounds and music in the IDE](../ide/sounds.md) ·
[Sound and music functions](../reference/sound-and-music.md) · [The game loop](game-loop.md)
