//! A Rust builder with the same power as the text notation, for generator
//! programs (e.g. a game generator that also writes its music).
//!
//! ```
//! use instance_music::SongBuilder;
//! let mut b = SongBuilder::new("Demo");
//! b.tempo(100.0)
//!     .key("A minor")
//!     .track("bass", "inst=\"Fingered Bass\" oct=2")
//!     .track("drums", "drums")
//!     .pattern("walk", "1e 5 8 5")
//!     .grid("beat", "s", &[("kick", "x...x...x...x..."), ("hat", "x.x.x.x.x.x.x.x.")])
//!     .section("A", "")
//!     .part("bass", "walk over | i | VI | III | VII |")
//!     .part("drums", "beat*4")
//!     .variant("dark", "tempo 80\ntranspose -3\nmute drums")
//!     .arrange("|: A A :|");
//! let song = b.compile(None).unwrap();
//! assert_eq!(song.tracks[0].notes.len(), 64);
//! assert_eq!(song.tracks[0].notes[0].key, 45); // A2
//! let dark = b.compile(Some("dark")).unwrap();
//! assert_eq!(dark.tracks[0].notes[0].key, 42); // F#2
//! assert!(dark.tracks[1].notes.is_empty());
//! assert!((dark.duration() - song.duration() * 100.0 / 80.0).abs() < 1e-9);
//! let midi: Vec<u8> = instance_music::smf::write_smf(&song);
//! assert_eq!(&midi[..4], b"MThd");
//! ```
//!
//! Every method takes the same text as the corresponding notation line, so
//! the README reference applies. Notes can also be supplied as [`Item`]s
//! built in code ([`SongBuilder::part_items`], [`SongBuilder::pattern_items`]).
//! The first error is kept and returned by [`SongBuilder::build`].

use crate::compile::{compile, Song};
use crate::song::{bar_ticks, parse_arrange, parse_into, parse_stream, validate, Item, Section, SongSrc, StreamCtx};

/// Builds a [`SongSrc`] step by step.
#[derive(Clone, Debug, Default)]
pub struct SongBuilder {
    src: SongSrc,
    current: Option<usize>,
    error: Option<String>,
}

impl SongBuilder {
    pub fn new(title: &str) -> Self {
        let mut b = SongBuilder::default();
        b.src.title = title.into();
        b
    }

    fn text(&mut self, line: &str) -> &mut Self {
        if self.error.is_none() {
            if let Err(e) = parse_into(&mut self.src, line) {
                self.error = Some(format!("{e} (in `{line}`)"));
            }
        }
        self
    }

    fn fail(&mut self, e: String) -> &mut Self {
        if self.error.is_none() {
            self.error = Some(e);
        }
        self
    }

    /// Song tempo in BPM.
    pub fn tempo(&mut self, bpm: f64) -> &mut Self {
        self.text(&format!("tempo {bpm}"))
    }
    /// Time signature, e.g. `time(3, 4)`.
    pub fn time(&mut self, num: u32, den: u32) -> &mut Self {
        self.text(&format!("time {num}/{den}"))
    }
    /// Key, e.g. `"A minor"`, `"Eb major"`, `"D dorian"`.
    pub fn key(&mut self, key: &str) -> &mut Self {
        self.text(&format!("key {key}"))
    }
    /// Swing percentage (50 straight .. 75).
    pub fn swing(&mut self, pct: f64) -> &mut Self {
        self.text(&format!("swing {pct}"))
    }
    /// A track with notation options, e.g. `("lead", "inst=Flute vol=90 oct=5")`.
    pub fn track(&mut self, name: &str, options: &str) -> &mut Self {
        self.text(&format!("track {name} {options}"))
    }
    /// A named pattern in notation.
    pub fn pattern(&mut self, name: &str, notes: &str) -> &mut Self {
        self.text(&format!("pattern {name} = {notes}"))
    }
    /// A named pattern from items built in code.
    pub fn pattern_items(&mut self, name: &str, items: Vec<Item>) -> &mut Self {
        if self.src.pattern(name).is_some() {
            return self.fail(format!("pattern '{name}' defined twice"));
        }
        self.src.patterns.push((name.into(), items));
        self
    }
    /// A named chord progression, e.g. `("verse", "| Am | F | C | G |")`.
    pub fn prog(&mut self, name: &str, chords: &str) -> &mut Self {
        self.text(&format!("prog {name} = {chords}"))
    }
    /// A drum/step grid: `step` is a duration (`s`, `e`), rows are (label, cells).
    pub fn grid(&mut self, name: &str, step: &str, rows: &[(&str, &str)]) -> &mut Self {
        let mut t = format!("grid {name} step={step}\n");
        for (l, c) in rows {
            t += &format!("  {l} {c}\n");
        }
        self.text(&t)
    }
    /// Starts a section; options as in notation (`"bars=8 tempo=90 time=3/4"`).
    pub fn section(&mut self, name: &str, options: &str) -> &mut Self {
        let n = self.src.sections.len();
        self.text(&format!("section {name} {options}"));
        if self.src.sections.len() > n {
            self.current = Some(n);
        }
        self
    }
    fn ensure_section(&mut self) -> usize {
        match self.current {
            Some(s) => s,
            None => {
                self.src.sections.push(Section { name: "main".into(), bars: None, tempo: None, time: None, parts: Vec::new() });
                self.current = Some(self.src.sections.len() - 1);
                self.src.sections.len() - 1
            }
        }
    }
    /// Appends notation to a track's part in the current section.
    pub fn part(&mut self, track: &str, notes: &str) -> &mut Self {
        if self.error.is_some() {
            return self;
        }
        let Some(drums) = self.src.track(track).map(|t| t.drums) else {
            return self.fail(format!("unknown track '{track}'"));
        };
        let s = self.ensure_section();
        let time = self.src.sections[s].time.unwrap_or(self.src.time);
        let ctx = StreamCtx { src: &self.src, bar: bar_ticks(time), key: self.src.key.as_ref(), line: 0, drums };
        match parse_stream(notes, &ctx) {
            Ok(items) => {
                self.src.sections[s].parts.push((track.into(), items, 0));
                self
            }
            Err(e) => self.fail(format!("{track}: {e}")),
        }
    }
    /// Appends items built in code to a track's part in the current section.
    pub fn part_items(&mut self, track: &str, items: Vec<Item>) -> &mut Self {
        if self.src.track(track).is_none() {
            return self.fail(format!("unknown track '{track}'"));
        }
        let s = self.ensure_section();
        self.src.sections[s].parts.push((track.into(), items, 0));
        self
    }
    /// The arrangement, e.g. `"intro |: A A B A(+2 -drums) :|"`.
    pub fn arrange(&mut self, arrangement: &str) -> &mut Self {
        match parse_arrange(arrangement) {
            Ok(a) => {
                self.src.arrange = Some(a);
                self
            }
            Err(e) => self.fail(e),
        }
    }
    /// A variant with notation lines (`"transpose -3\nmute drums\ninst lead Celesta"`).
    pub fn variant(&mut self, name: &str, lines: &str) -> &mut Self {
        self.current = None;
        let body: String = lines.lines().map(|l| format!("  {}\n", l.trim())).collect();
        self.text(&format!("variant {name}\n{body}end"))
    }
    /// The finished song source.
    pub fn build(&self) -> Result<SongSrc, String> {
        if let Some(e) = &self.error {
            return Err(e.clone());
        }
        validate(&self.src)?;
        Ok(self.src.clone())
    }
    /// Builds and compiles (optionally a variant).
    pub fn compile(&self, variant: Option<&str>) -> Result<Song, String> {
        compile(&self.build()?, variant)
    }
}
