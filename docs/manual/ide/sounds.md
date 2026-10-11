# Sounds and music

A **sound** resource holds one sound effect or piece of music. Create one with
**Resources > Create Sound** and double-click it in the tree to open the
**Sound Properties** form.

## The sound form

| Field | What it does |
|---|---|
| Name | The name you use in actions and code. |
| Load Sound… | Loads a sound file into the resource. |
| Save Sound… | Saves the stored sound data back to a file. |
| File | The name, type and size of the loaded file. |
| Kind | How the sound is played (see below). |
| Effects | Chorus, Echo, Flanger, Gargle and Reverb. Tick any combination. |
| Volume | The playback volume, from 0 (silent) to 1 (full). |
| Pan | Left–right balance, from -1 (left) through 0 (centre) to 1 (right). |
| Preload | Load the sound into memory when the game starts, so it plays without delay the first time. |
| OK, Cancel | Apply or discard your changes and close the form. |

The sound file is stored inside the project, so you don't need to ship it separately.

## Supported files

**Load Sound…** accepts WAV, MIDI (`.mid`, `.midi`), MP3 and OGG files. Loading a file
also picks a sensible kind:

| File | Kind chosen |
|---|---|
| `.wav` | Normal Sound |
| `.mid`, `.midi` | Background Music |
| anything else (`.mp3`, `.ogg`, …) | Use multimedia player |

You can change the kind afterwards.

## Kinds

| Kind | Use it for |
|---|---|
| Normal Sound | Short effects. Several copies can play at the same time, and the effects settings apply. |
| Background Music | Music. Only one background music sound plays at a time: starting another stops the previous one. MIDI files are usually played this way. |
| 3D Sound | Sounds positioned in space with the 3D sound functions. |
| Use multimedia player | Compressed formats such as MP3. These play one at a time and the effects don't apply. |

MIDI music is played with the system's synthesiser on Windows and with a SoundFont on
other systems; see [Sound and music](../runtime/sound.md).

## Playing sounds

Use the **Play Sound** action (main1 tab) or the
[`sound_play`](../reference/sound-and-music.md#sound_play) and
[`sound_loop`](../reference/sound-and-music.md#sound_loop) functions:

```gml
sound_play(snd_coin);      // play once
sound_loop(snd_music);     // play over and over
sound_stop(snd_music);
```

> **Tip:** The sound form has no play button. To hear a sound, run the game, or save
> it with **Save Sound…** and open the file in a media player.

See also: [Sound and music](../runtime/sound.md) · [Sound functions](../reference/sound-and-music.md) · [Actions](actions.md#main1)
