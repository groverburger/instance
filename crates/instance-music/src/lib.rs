//! Music composition and audio analysis for agents that write GameMaker 8.1
//! games in code and cannot listen to the result.
//!
//! * [`song`] — a compact text notation (see README.md) and its parser;
//!   [`builder::SongBuilder`] offers the same features from Rust.
//! * [`compile`] — turns a song into timed MIDI events (sections,
//!   arrangement, variants, loop region).
//! * [`smf`] — writes a Standard MIDI File (what GameMaker plays as music).
//! * [`render`] — renders to PCM through a SoundFont, optionally as a
//!   seamless loop; [`wav`] encodes/decodes audio files.
//! * [`synth`] — the built-in synthesiser that plays `synth=` tracks in WAV
//!   renders (subtractive/FM voices, drum machine, chorus, delay, reverb,
//!   sidechain).
//! * [`analysis`] — levels, spectrum, envelope, description, loop seams,
//!   loudness over time and similarity between sounds.
//! * [`info`] — a musical report of a compiled song with warnings.
//!
//! Binaries: `gmk-music` (render/info) and `gmk-sounds` (analysis of the
//! sounds in a .gm81/.gmk project or of audio files).

pub mod analysis;
pub mod builder;
pub mod compile;
pub mod gm;
pub mod info;
pub mod render;
pub mod smf;
pub mod song;
pub mod synth;
pub mod theory;
pub mod wav;

pub use builder::SongBuilder;
pub use compile::{compile, Song};
pub use song::{parse, SongSrc};

/// Parses and compiles notation text in one step.
pub fn compile_text(text: &str, variant: Option<&str>) -> Result<Song, String> {
    compile(&parse(text)?, variant)
}
