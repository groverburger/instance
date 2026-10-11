# Sound and music

Playing sounds and music, sound effects, 3D sound and CD audio.

This page is part of the [function reference](README.md); the [index](index.md) lists every name from A to Z.

- [Basic sound functions](#basic-sound-functions)
- [Sound effects](#sound-effects)
- [3D sound](#3d-sound)
- [CD music](#cd-music)

## Basic sound functions

Playing, looping, stopping and mixing sounds.

### sound_play

`sound_play(index)`

Starts a sound once; a background music sound being played replaces the current background music.

### sound_loop

`sound_loop(index)`

Starts a sound playing repeatedly; background music replaces the current background music.

### sound_stop

`sound_stop(index)`

Stops the given sound, including every simultaneously playing copy of it.

### sound_stop_all

`sound_stop_all()`

Stops every sound that is currently playing.

### sound_isplaying

`sound_isplaying(index)`

Tells whether a copy of the sound is audibly playing; may lag behind play/stop calls due to buffering and echo.

### sound_volume

`sound_volume(index, value)`

Sets the volume of one sound, from 0 (quiet) to 1 (loud).

### sound_global_volume

`sound_global_volume(value)`

Sets the master volume for all sounds, from 0 (quiet) to 1 (loud).

### sound_fade

`sound_fade(index, value, time)`

Gradually changes a sound volume to a target value (0 to 1) over a time in milliseconds.

### sound_pan

`sound_pan(index, value)`

Sets stereo pan of a sound: -1 left, 0 center, 1 right.

### sound_background_tempo

`sound_background_tempo(factor)`

Scales the tempo of midi background music by a factor (1 is normal, allowed 0.01 to 100).

### sound_set_search_directory

`sound_set_search_directory(direction)`

Sets the folder where direct music files are looked for, given without a trailing backslash.

## Sound effects

Chorus, echo, flanger, gargle, reverb, compressor and equalizer effects.

### sound_effect_set

`sound_effect_set(sound, effect)`

Selects the effect or combination (added se_ constants) applied to a sound.

### sound_effect_chorus

`sound_effect_chorus(sound, wetdry, depth, feedback, frequency, wave, delay, phase)`

Configures chorus parameters (wetdry, depth, feedback, frequency, wave, delay, phase) for a sound.

### sound_effect_echo

`sound_effect_echo(sound, wetdry, feedback, leftdelay, rightdelay, pandelay)`

Configures echo parameters (wetdry, feedback, left/right delay, pandelay) for a sound.

### sound_effect_flanger

`sound_effect_flanger(sound, wetdry, depth, feedback, frequency, wave, delay, phase)`

Configures flanger parameters (wetdry, depth, feedback, frequency, wave, delay, phase) for a sound.

### sound_effect_gargle

`sound_effect_gargle(sound, rate, wave)`

Configures gargle effect with modulation rate in Hz (1 to 1000) and waveform.

### sound_effect_reverb

`sound_effect_reverb(sound, gain, mix, time, ratio)`

Configures reverb parameters (gain in dB, mix, time, ratio) for a sound.

### sound_effect_compressor

`sound_effect_compressor(sound, gain, attack, release, threshold, ratio, delay)`

Configures compressor parameters (gain, attack, release, threshold, ratio, delay) for a sound.

### sound_effect_equalizer

`sound_effect_equalizer(sound, center, bandwidth, gain)`

Configures equalizer with center frequency (80 to 16000 Hz), bandwidth and gain.

### Constants: sound effects

| Constant | Value | Meaning |
| --- | --- | --- |
| `se_none` | 0 | Sound effect flag meaning no effect applied. |
| `se_chorus` | 1 | Sound effect flag selecting the chorus effect. |
| `se_echo` | 2 | Sound effect flag selecting the echo effect. |
| `se_flanger` | 4 | Sound effect flag selecting the flanger effect. |
| `se_gargle` | 8 | Sound effect flag selecting the gargle effect. |
| `se_reverb` | 16 | Sound effect flag selecting the reverb effect. |
| `se_compressor` | 32 | Sound effect flag selecting the compressor effect. |
| `se_equalizer` | 64 | Sound effect flag selecting the equalizer effect. |

## 3D sound

Positioning sounds in space.

### sound_3d_set_sound_position

`sound_3d_set_sound_position(sound, x, y, z)`

Places a 3D sound at (x,y,z) meters relative to the listener, which affects loudness.

### sound_3d_set_sound_velocity

`sound_3d_set_sound_velocity(sound, x, y, z)`

Sets a 3D sound velocity vector, used only for doppler effects; it does not move the sound.

### sound_3d_set_sound_distance

`sound_3d_set_sound_distance(sound, mindist, maxdist)`

Sets distance (meters) below which volume is max and beyond which the sound is inaudible; defaults 1 and 1 billion.

### sound_3d_set_sound_cone

`sound_3d_set_sound_cone(sound, x, y, z, anglein, angleout, voloutside)`

Makes a 3D sound directional with direction x,y,z, inner and outer angles, and outside volume reduction.

## CD music

Playing audio CDs, and sending commands to the media control interface.

### cd_init

`cd_init()`

Initializes CD access; call before other CD functions and again when the disc changes.

### cd_present

`cd_present()`

Returns whether a CD is in the default CD drive.

### cd_number

`cd_number()`

Returns the number of tracks on the CD.

### cd_playing

`cd_playing()`

Returns whether the CD is currently playing.

### cd_paused

`cd_paused()`

Returns whether the CD is paused or stopped.

### cd_track

`cd_track()`

Returns the current track number, where 1 is the first.

### cd_length

`cd_length()`

Returns the total CD length in milliseconds.

### cd_track_length

`cd_track_length(n)`

Returns the length of track n in milliseconds.

### cd_position

`cd_position()`

Returns the current position on the whole CD in milliseconds.

### cd_track_position

`cd_track_position()`

Returns the current position within the playing track in milliseconds.

### cd_play

`cd_play(first, last)`

Plays CD tracks from first through last; use 1 and 1000 for the whole disc.

### cd_stop

`cd_stop()`

Stops CD playback.

### cd_pause

`cd_pause()`

Pauses CD playback.

### cd_resume

`cd_resume()`

Resumes paused CD playback.

### cd_set_position

`cd_set_position(pos)`

Seeks to a position on the CD in milliseconds.

### cd_set_track_position

`cd_set_track_position(pos)`

Seeks within the current track to a position in milliseconds.

### cd_open_door

`cd_open_door()`

Opens the CD drive tray.

### cd_close_door

`cd_close_door()`

Closes the CD drive tray.

### MCI_command

`MCI_command(str)`

Sends a command string to the Windows Media Control Interface and returns its reply string; advanced use.
