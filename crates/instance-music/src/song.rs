//! The song source model (AST) and the text-notation parser. See README.md
//! for the notation reference. [`crate::compile`] turns a [`SongSrc`] into
//! timed MIDI events.

use crate::gm;
use crate::theory::{parse_pitch_letter, Chord, Key};

/// Ticks per quarter note used throughout.
pub const PPQ: u32 = 480;

/// How a note's pitch is given.
#[derive(Clone, Debug, PartialEq)]
pub enum Pitch {
    /// MIDI note number (`C4` = 60).
    Abs(i32),
    /// Note letter without octave: pitch class offset, uses the current octave.
    Rel(i32),
    /// Chord degree (1 root, 3 third, 5 fifth, 7, 8 octave …) with octave shift;
    /// only inside a pattern played `over` a progression.
    Degree(i32, i32),
    /// `0`: the chord's bass (slash bass or root), at or below degree 1.
    Bass(i32),
    /// A channel-10 drum key.
    Drum(u8),
}

/// Per-note modifiers.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Mods {
    /// Explicit velocity (`@100`).
    pub vel: Option<u8>,
    /// Number of `>` accents (+16 velocity each); negative = ghost (`o` in grids).
    pub accent: i32,
    /// `^`: staccato (half length).
    pub staccato: bool,
    /// `~`: tie into the next note of the same pitch.
    pub tie: bool,
}

/// MIDI controls settable inline (`vol=80`, `pan=-30->30:2w`).
#[derive(Clone, Debug, PartialEq)]
pub enum Ctl {
    /// A controller number; value 0..127.
    Cc(u8),
    /// Pan: value -100 (left) .. 100 (right).
    Pan,
    /// Pitch bend in semitones (within the track's bend range).
    Bend,
}

/// One element of a note stream.
#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    /// A note; `dur` None = the previous duration.
    Note { pitch: Pitch, dur: Option<u32>, m: Mods },
    /// Simultaneous notes `[C4 E4 G4]q`.
    Chord { pitches: Vec<Pitch>, dur: Option<u32>, m: Mods },
    /// A chord symbol `{Am7}h`, voiced with voice leading near the current
    /// octave; `None` = the current chord of a progression (`*`).
    Symbol { chord: Option<Chord>, dur: Option<u32>, m: Mods },
    Rest(Option<u32>),
    /// `( … )*N+T`
    Group { items: Vec<Item>, repeat: u32, transpose: i32 },
    /// A pattern or grid by name, `name*N+T`.
    Ref { name: String, repeat: u32, transpose: i32 },
    /// Lanes played at the same time (a grid); length = the longest lane.
    Par(Vec<Vec<Item>>),
    /// `PATTERN over PROGRESSION`: the pattern restarts on every chord and is
    /// repeated/cut to the chord's length. `None` chords are silent (`x`).
    Over { pattern: Box<Item>, prog: Vec<(Option<Chord>, u32)> },
    /// `o4`, `o+1`, `o-1`
    Octave { value: i32, relative: bool },
    /// Sticky velocity (`mf`, `vel=90`).
    Vel(u8),
    /// A control change; `to` = ramp target and its length.
    Control { ctl: Ctl, from: f64, to: Option<(f64, u32)> },
    /// Program change (0-based).
    Program(u8),
    /// Sustain pedal down/up (`ped+` / `ped-`).
    Pedal(bool),
    /// `|`: warns when not on a bar line. Holds the source line.
    BarCheck(usize),
}

/// A track (`track NAME key=value …`).
#[derive(Clone, Debug, PartialEq)]
pub struct TrackDef {
    pub name: String,
    /// 0-based GM program (for drums: the kit program).
    pub program: u8,
    pub drums: bool,
    /// 1-based channel if fixed.
    pub channel: Option<u8>,
    pub vol: u8,
    /// -100..100
    pub pan: i32,
    pub expr: u8,
    pub reverb: u8,
    pub chorus: u8,
    /// Default octave for notes without one, chords and degrees.
    pub octave: i32,
    pub vel: u8,
    /// Sounding fraction of each note's length (0.05..1).
    pub gate: f64,
    /// Random velocity variation ±N.
    pub human: u8,
    pub transpose: i32,
    /// Swing ratio for this track (0.5 straight .. 0.75); None = song swing.
    pub swing: Option<f64>,
    pub bend_range: u8,
}

impl TrackDef {
    pub fn new(name: &str) -> Self {
        TrackDef {
            name: name.into(),
            program: 0,
            drums: false,
            channel: None,
            vol: 100,
            pan: 0,
            expr: 127,
            reverb: 40,
            chorus: 0,
            octave: 4,
            vel: 80,
            gate: 0.95,
            human: 0,
            transpose: 0,
            swing: None,
            bend_range: 2,
        }
    }
}

/// A section: what each track plays for a stretch of bars.
#[derive(Clone, Debug, PartialEq)]
pub struct Section {
    pub name: String,
    /// Declared length in bars (parts are padded/cut to it).
    pub bars: Option<u32>,
    /// Tempo at the section start, optionally ramping to a second value.
    pub tempo: Option<(f64, Option<f64>)>,
    pub time: Option<(u32, u32)>,
    /// (track name, items, source line)
    pub parts: Vec<(String, Vec<Item>, usize)>,
}

/// An entry of an arrangement.
#[derive(Clone, Debug, PartialEq)]
pub enum ArrEntry {
    Play { section: String, repeat: u32, transpose: i32, mute: Vec<String> },
    LoopStart,
    LoopEnd,
}

/// A variant: a re-orchestration of the same song (`--variant NAME`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Variant {
    pub name: String,
    pub transpose: i32,
    /// New base tempo (all tempi are scaled by new/old).
    pub tempo: Option<f64>,
    pub inst: Vec<(String, u8)>,
    pub mute: Vec<String>,
    pub vol: Vec<(String, u8)>,
    /// Per-track transposition in semitones.
    pub shift: Vec<(String, i32)>,
    pub arrange: Option<Vec<ArrEntry>>,
}

/// A chord progression: (chord or silence, length in ticks).
pub type Progression = Vec<(Option<Chord>, u32)>;

/// A whole song source.
#[derive(Clone, Debug, PartialEq)]
pub struct SongSrc {
    pub title: String,
    pub tempo: f64,
    pub time: (u32, u32),
    pub key: Option<Key>,
    /// Swing ratio (0.5 = straight).
    pub swing: f64,
    pub tracks: Vec<TrackDef>,
    /// Patterns and grids by name.
    pub patterns: Vec<(String, Vec<Item>)>,
    /// Named chord progressions (`prog`), as (chord, length in ticks).
    pub progs: Vec<(String, Progression)>,
    pub sections: Vec<Section>,
    /// None = every section once in definition order.
    pub arrange: Option<Vec<ArrEntry>>,
    pub variants: Vec<Variant>,
}

impl Default for SongSrc {
    fn default() -> Self {
        SongSrc { title: String::new(), tempo: 120.0, time: (4, 4), key: None, swing: 0.5, tracks: Vec::new(), patterns: Vec::new(), progs: Vec::new(), sections: Vec::new(), arrange: None, variants: Vec::new() }
    }
}

impl SongSrc {
    pub fn track(&self, name: &str) -> Option<&TrackDef> {
        self.tracks.iter().find(|t| t.name == name)
    }
    pub fn pattern(&self, name: &str) -> Option<&Vec<Item>> {
        self.patterns.iter().find(|p| p.0 == name).map(|p| &p.1)
    }
    pub fn prog(&self, name: &str) -> Option<&Vec<(Option<Chord>, u32)>> {
        self.progs.iter().find(|p| p.0 == name).map(|p| &p.1)
    }
    pub fn section(&self, name: &str) -> Option<&Section> {
        self.sections.iter().find(|s| s.name == name)
    }
}

/// Ticks in a bar of `time`.
pub fn bar_ticks(time: (u32, u32)) -> u32 {
    time.0 * PPQ * 4 / time.1.max(1)
}

const KEYWORDS: &[&str] = &[
    "title", "tempo", "time", "key", "swing", "track", "pattern", "grid", "prog", "section", "variant", "arrange", "end",
    "over", "transpose", "inst", "mute", "vol", "shift", "r", "x",
];
const DYNAMICS: &[(&str, u8)] = &[("ppp", 24), ("pp", 36), ("p", 50), ("mp", 64), ("mf", 80), ("f", 96), ("ff", 112), ("fff", 124)];

// ------------------------------------------------------------------ durations

fn unit_ticks(c: char) -> Option<u32> {
    Some(match c {
        'w' => PPQ * 4,
        'h' => PPQ * 2,
        'q' => PPQ,
        'e' => PPQ / 2,
        's' => PPQ / 4,
        't' => PPQ / 8,
        _ => return None,
    })
}

/// Parses a duration: `q`, `h.`, `e3`, `q..`, `h+e`, and after `:` also
/// multiples and bars (`2w`, `bar`, `3bar`, `2q+e`).
pub fn parse_dur(s: &str, bar: u32) -> Result<u32, String> {
    let mut total = 0u32;
    for part in s.split('+') {
        let mut p = part;
        let digits: String = p.chars().take_while(|c| c.is_ascii_digit()).collect();
        let mult: u32 = if digits.is_empty() { 1 } else { digits.parse().map_err(|_| format!("bad duration '{s}'"))? };
        p = &p[digits.len()..];
        let (mut t, rest) = if let Some(r) = p.strip_prefix("bar") {
            (bar, r)
        } else {
            let c = p.chars().next().ok_or_else(|| format!("bad duration '{s}': missing unit (w h q e s t)"))?;
            (unit_ticks(c).ok_or_else(|| format!("bad duration '{s}': unknown unit '{c}' (w h q e s t)"))?, &p[1..])
        };
        match rest {
            "" => {}
            "." => t = t * 3 / 2,
            ".." => t = t * 7 / 4,
            "3" => t = t * 2 / 3,
            "5" => t = t * 4 / 5,
            _ => return Err(format!("bad duration '{s}': unexpected '{rest}' (use . for dotted, 3 for triplet, + to add)")),
        }
        total += t * mult;
    }
    if total == 0 {
        return Err(format!("zero duration '{s}'"));
    }
    Ok(total)
}

/// Splits `s` into (duration text, modifier text): the duration is the
/// leading run of duration characters.
fn split_dur_mods(s: &str) -> (&str, &str) {
    let s0 = s.strip_prefix(':').unwrap_or(s);
    let colon = s0.len() != s.len();
    let end = s0
        .char_indices()
        .find(|(_, c)| !(c.is_ascii_digit() || matches!(c, 'w' | 'h' | 'q' | 'e' | 's' | 't' | '.' | '+' | 'b' | 'a' | 'r')) || (!colon && matches!(c, 'b' | 'a' | 'r')))
        .map(|(i, _)| i)
        .unwrap_or(s0.len());
    (&s0[..end], &s0[end..])
}

fn parse_dur_mods(s: &str, bar: u32) -> Result<(Option<u32>, Mods), String> {
    let (d, m) = split_dur_mods(s);
    let dur = if d.is_empty() { None } else { Some(parse_dur(d, bar)?) };
    let mut mods = Mods::default();
    let mut it = m.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '>' => mods.accent += 1,
            '^' => mods.staccato = true,
            '~' => mods.tie = true,
            '@' => {
                let mut n = String::new();
                while let Some(d) = it.peek().filter(|c| c.is_ascii_digit()) {
                    n.push(*d);
                    it.next();
                }
                let v: u32 = n.parse().map_err(|_| format!("'@' needs a velocity 1..127 in '{s}'"))?;
                mods.vel = Some(v.clamp(1, 127) as u8);
            }
            _ => return Err(format!("unexpected '{c}' in '{s}' (modifiers: > accent, ^ staccato, ~ tie, @N velocity)")),
        }
    }
    Ok((dur, mods))
}

/// `*N` and `+T`/`-T` suffixes of references and groups.
fn parse_rep_suffix(s: &str) -> Result<(u32, i32), String> {
    let mut repeat = 1;
    let mut transpose = 0;
    let mut rest = s;
    while !rest.is_empty() {
        let c = rest.chars().next().unwrap();
        let num: String = rest[1..].chars().take_while(|c| c.is_ascii_digit()).collect();
        if num.is_empty() {
            return Err(format!("bad suffix '{s}' (use *N to repeat, +N/-N to transpose)"));
        }
        let n: i32 = num.parse().unwrap();
        match c {
            '*' | 'x' => repeat = n.max(0) as u32,
            '+' => transpose += n,
            '-' => transpose -= n,
            _ => return Err(format!("bad suffix '{s}' (use *N to repeat, +N/-N to transpose)")),
        }
        rest = &rest[1 + num.len()..];
    }
    Ok((repeat, transpose))
}

// ------------------------------------------------------------------ tokens

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Word(String),
    Open(char),
    /// Closing bracket and its suffix.
    Close(char, String),
    Bar,
}

fn tokenize(s: &str) -> Result<Vec<Tok>, String> {
    let mut out = Vec::new();
    let cs: Vec<char> = s.chars().collect();
    let mut i = 0;
    let word_end = |i: usize| {
        let mut j = i;
        while j < cs.len() && !cs[j].is_whitespace() && !matches!(cs[j], '[' | ']' | '(' | ')' | '|') {
            j += 1;
        }
        j
    };
    while i < cs.len() {
        let c = cs[i];
        if c.is_whitespace() {
            i += 1;
        } else if c == '[' || c == '(' {
            out.push(Tok::Open(c));
            i += 1;
        } else if c == ']' || c == ')' {
            let j = word_end(i + 1);
            out.push(Tok::Close(c, cs[i + 1..j].iter().collect()));
            i = j;
        } else if c == '|' {
            out.push(Tok::Bar);
            i += 1;
        } else if c == '{' {
            let close = cs[i..].iter().position(|c| *c == '}').ok_or("missing '}'")? + i;
            let j = word_end(close + 1);
            out.push(Tok::Word(cs[i..j].iter().collect()));
            i = j;
        } else {
            let j = word_end(i);
            out.push(Tok::Word(cs[i..j].iter().collect()));
            i = j;
        }
    }
    Ok(out)
}

/// Whether `w` would be read as a note/rest/degree (and so cannot name a pattern).
fn looks_like_note(w: &str) -> bool {
    let c = w.chars().next().unwrap_or(' ');
    matches!(c, 'A'..='G') || c.is_ascii_digit()
}

// ------------------------------------------------------------------ parser

/// Parser context for streams.
#[derive(Clone)]
pub struct StreamCtx<'a> {
    pub src: &'a SongSrc,
    pub bar: u32,
    pub key: Option<&'a Key>,
    pub line: usize,
    /// Whether drum names are expected (drum track).
    pub drums: bool,
}

fn parse_control(k: &str, v: &str, ctx: &StreamCtx) -> Result<Item, String> {
    let ctl = match k {
        "vol" | "volume" => Ctl::Cc(7),
        "expr" | "expression" => Ctl::Cc(11),
        "pan" => Ctl::Pan,
        "mod" | "modulation" => Ctl::Cc(1),
        "reverb" => Ctl::Cc(91),
        "chorus" => Ctl::Cc(93),
        "cutoff" | "bright" => Ctl::Cc(74),
        "reso" | "resonance" => Ctl::Cc(71),
        "attack" => Ctl::Cc(73),
        "release" => Ctl::Cc(72),
        "bend" => Ctl::Bend,
        "vel" => {
            let n: u32 = v.parse().map_err(|_| format!("vel={v}: expected 1..127"))?;
            return Ok(Item::Vel(n.clamp(1, 127) as u8));
        }
        "inst" | "instrument" => return Ok(Item::Program(gm::program(&v.replace('_', " "))?)),
        _ => {
            if let Some(n) = k.strip_prefix("cc").and_then(|n| n.parse::<u8>().ok()).filter(|n| *n < 120) {
                Ctl::Cc(n)
            } else {
                return Err(format!("unknown control '{k}' (vol expr pan mod reverb chorus cutoff reso attack release bend ccN vel inst)"));
            }
        }
    };
    let num = |s: &str| s.parse::<f64>().map_err(|_| format!("{k}={v}: '{s}' is not a number"));
    let (from, to) = match v.split_once("->") {
        Some((a, b)) => {
            let (b, d) = b.split_once(':').ok_or_else(|| format!("{k}={v}: a ramp needs a length, e.g. {k}={a}->{b}:2w"))?;
            (num(a)?, Some((num(b)?, parse_dur(d, ctx.bar)?)))
        }
        None => (num(v)?, None),
    };
    Ok(Item::Control { ctl, from, to })
}

/// Parses one word token into an item.
fn parse_word(w: &str, ctx: &StreamCtx) -> Result<Item, String> {
    // `X*N` repeats a single note, rest or chord N times
    if let Some((body, n)) = w.rsplit_once('*') {
        if !body.is_empty() && !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()) && ctx.src.pattern(body).is_none() {
            let item = parse_word(body, ctx)?;
            return Ok(Item::Group { items: vec![item], repeat: n.parse().unwrap(), transpose: 0 });
        }
    }
    if w.starts_with(|c: char| unit_ticks(c).is_some()) && parse_dur(w, ctx.bar).is_ok() && ctx.src.pattern(w).is_none() && gm::drum(w).is_none() {
        return Err(format!("'{w}' is a duration on its own: attach it to the note or rest (C4{w}, r{w}, kick:{w})"));
    }
    // dynamics
    if let Some((_, v)) = DYNAMICS.iter().find(|(n, _)| *n == w) {
        return Ok(Item::Vel(*v));
    }
    if w == "ped+" {
        return Ok(Item::Pedal(true));
    }
    if w == "ped-" {
        return Ok(Item::Pedal(false));
    }
    if let Some((k, v)) = w.split_once('=') {
        return parse_control(k, v, ctx);
    }
    // octave
    if let Some(o) = w.strip_prefix('o') {
        if let Some(n) = o.strip_prefix('+').and_then(|n| n.parse::<i32>().ok()) {
            return Ok(Item::Octave { value: n, relative: true });
        }
        if let Some(n) = o.strip_prefix('-').and_then(|n| n.parse::<i32>().ok()) {
            return Ok(Item::Octave { value: -n, relative: true });
        }
        if let Ok(n) = o.parse::<i32>() {
            return Ok(Item::Octave { value: n, relative: false });
        }
    }
    // chord symbol
    if let Some(body) = w.strip_prefix('{') {
        let close = body.find('}').ok_or("missing '}'")?;
        let chord = Chord::parse(body[..close].trim(), ctx.key)?;
        let (dur, m) = parse_dur_mods(&body[close + 1..], ctx.bar)?;
        return Ok(Item::Symbol { chord: Some(chord), dur, m });
    }
    if let Some(rest) = w.strip_prefix('*') {
        let (dur, m) = parse_dur_mods(rest, ctx.bar)?;
        return Ok(Item::Symbol { chord: None, dur, m });
    }
    // pattern reference / drum name (words starting lowercase)
    let name_end = w.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).unwrap_or(w.len());
    let (name, suffix) = w.split_at(name_end);
    if ctx.src.pattern(name).is_some() {
        let (repeat, transpose) = parse_rep_suffix(suffix)?;
        return Ok(Item::Ref { name: name.into(), repeat, transpose });
    }
    if let Some(k) = gm::drum(name) {
        let (dur, m) = parse_dur_mods(suffix, ctx.bar)?;
        if !ctx.drums {
            return Err(format!("'{name}' is a drum sound but this track is not a drum track (add `drums` to its track line)"));
        }
        return Ok(Item::Note { pitch: Pitch::Drum(k), dur, m });
    }
    // rest
    if let Some(r) = w.strip_prefix('r') {
        if r.is_empty() || r.starts_with(':') || r.starts_with(|c: char| unit_ticks(c).is_some() || c.is_ascii_digit()) {
            let (dur, m) = parse_dur_mods(r, ctx.bar)?;
            if m != Mods::default() {
                return Err(format!("rest '{w}' cannot have modifiers"));
            }
            return Ok(Item::Rest(dur));
        }
    }
    // note
    let first = w.chars().next().unwrap();
    if matches!(first, 'A'..='G') {
        let (pc, rest) = parse_pitch_letter(w).unwrap();
        let (oct, rest) = match rest.chars().next() {
            Some('-') if rest[1..].starts_with('1') => (Some(-1), &rest[2..]),
            Some(d) if d.is_ascii_digit() => (Some(d as i32 - '0' as i32), &rest[1..]),
            _ => (None, rest),
        };
        let (dur, m) = parse_dur_mods(rest, ctx.bar).map_err(|e| format!("note '{w}': {e}"))?;
        let pitch = match oct {
            Some(o) => {
                let n = (o + 1) * 12 + pc;
                if !(0..=127).contains(&n) {
                    return Err(format!("note '{w}' is outside MIDI range"));
                }
                Pitch::Abs(n)
            }
            None => Pitch::Rel(pc),
        };
        return Ok(Item::Note { pitch, dur, m });
    }
    if matches!(first, 'a'..='g') && w.len() >= 2 && w[1..].starts_with(|c: char| c.is_ascii_digit() || c == '#') {
        return Err(format!("'{w}': note names are uppercase (write {}{})", first.to_ascii_uppercase(), &w[1..]));
    }
    // chord degree
    if first.is_ascii_digit() {
        let digits: String = w.chars().take_while(|c| c.is_ascii_digit()).collect();
        let d: i32 = digits.parse().unwrap();
        let mut rest = &w[digits.len()..];
        let mut oct = 0;
        loop {
            if let Some(r) = rest.strip_prefix('\'') {
                oct += 1;
                rest = r;
            } else if let Some(r) = rest.strip_prefix(',') {
                oct -= 1;
                rest = r;
            } else {
                break;
            }
        }
        let (dur, m) = parse_dur_mods(rest, ctx.bar).map_err(|e| format!("degree '{w}': {e}"))?;
        let pitch = if d == 0 { Pitch::Bass(oct) } else { Pitch::Degree(d, oct) };
        if d > 15 {
            return Err(format!("chord degree '{w}' too large (1..15)"));
        }
        return Ok(Item::Note { pitch, dur, m });
    }
    if gm::drum(name).is_none() && !ctx.drums && name.chars().all(|c| c.is_ascii_lowercase() || c == '_' || c.is_ascii_digit()) && !name.is_empty() {
        return Err(format!("unknown word '{w}' (not a note, rest, pattern, dynamic or control; patterns must be defined before use)"));
    }
    Err(format!("cannot read '{w}'"))
}

/// Parses the label of a grid row.
fn grid_pitch(label: &str, ctx: &StreamCtx) -> Result<Pitch, String> {
    if let Some(k) = gm::drum(label) {
        return Ok(Pitch::Drum(k));
    }
    match parse_word(label, &StreamCtx { drums: false, ..ctx.clone() })? {
        Item::Note { pitch, dur: None, m } if m == Mods::default() => Ok(pitch),
        _ => Err(format!("grid row label '{label}' must be a drum name, a note (C2) or a chord degree (1, 5, 8)")),
    }
}

/// Parses grid rows (`label cells`) into a parallel item.
pub fn parse_grid(rows: &[(String, String)], step: u32, ctx: &StreamCtx) -> Result<Item, String> {
    let mut lanes = Vec::new();
    let mut len0 = None;
    for (label, cells) in rows {
        let pitch = grid_pitch(label, ctx)?;
        let cells: Vec<char> = cells.chars().filter(|c| !c.is_whitespace() && *c != '|').collect();
        if let Some(l) = len0 {
            if l != cells.len() {
                return Err(format!("grid row '{label}' has {} steps, the first row has {l}", cells.len()));
            }
        }
        len0 = Some(cells.len());
        let mut lane: Vec<Item> = Vec::new();
        for c in cells {
            match c {
                'x' | 'X' | 'o' => {
                    let accent = match c {
                        'X' => 1,
                        'o' => -2,
                        _ => 0,
                    };
                    lane.push(Item::Note { pitch: pitch.clone(), dur: Some(step), m: Mods { accent, ..Mods::default() } })
                }
                '=' => match lane.last_mut() {
                    Some(Item::Note { dur: Some(d), .. }) => *d += step,
                    Some(Item::Rest(Some(d))) => *d += step,
                    _ => lane.push(Item::Rest(Some(step))),
                },
                '.' | '-' | '_' => lane.push(Item::Rest(Some(step))),
                _ => return Err(format!("grid row '{label}': unexpected '{c}' (x hit, X accent, o ghost, = hold, . rest)")),
            }
        }
        lanes.push(lane);
    }
    Ok(Item::Par(lanes))
}

/// Parses a chord progression `| Am | F G | C . . G | % | x |`.
pub fn parse_prog(toks: &[String], ctx: &StreamCtx) -> Result<Vec<(Option<Chord>, u32)>, String> {
    let mut bars: Vec<Vec<String>> = vec![Vec::new()];
    for t in toks {
        if t == "|" {
            bars.push(Vec::new());
        } else {
            bars.last_mut().unwrap().push(t.clone());
        }
    }
    bars.retain(|b| !b.is_empty());
    let mut out: Vec<(Option<Chord>, u32)> = Vec::new();
    let mut prev_bar: Vec<(Option<Chord>, u32)> = Vec::new();
    for b in bars {
        if b.len() == 1 && b[0] == "%" {
            out.extend(prev_bar.iter().cloned());
            continue;
        }
        let n = b.len() as u32;
        let mut this: Vec<(Option<Chord>, u32)> = Vec::new();
        for (i, c) in b.iter().enumerate() {
            let len = ctx.bar * (i as u32 + 1) / n - ctx.bar * i as u32 / n;
            if c == "." {
                match this.last_mut().or(out.last_mut()) {
                    Some(last) => last.1 += len,
                    None => return Err("'.' at the start of a progression".into()),
                }
                continue;
            }
            let chord = if c == "x" { None } else { Some(Chord::parse(c, ctx.key)?) };
            this.push((chord, len));
        }
        prev_bar = this.clone();
        out.extend(this);
    }
    if out.is_empty() {
        return Err("empty progression".into());
    }
    Ok(out)
}

/// Parses a note stream (one line of tokens).
pub fn parse_stream(text: &str, ctx: &StreamCtx) -> Result<Vec<Item>, String> {
    let toks = tokenize(text)?;
    let mut stack: Vec<Vec<Item>> = vec![Vec::new()];
    let mut opens: Vec<char> = Vec::new();
    let mut i = 0;
    while i < toks.len() {
        match &toks[i] {
            Tok::Open(c) => {
                opens.push(*c);
                stack.push(Vec::new());
            }
            Tok::Close(c, suffix) => {
                let want = if *c == ']' { '[' } else { '(' };
                if opens.pop() != Some(want) {
                    return Err(format!("unmatched '{c}'"));
                }
                let items = stack.pop().unwrap();
                let item = if *c == ']' {
                    let mut pitches = Vec::new();
                    for it in items {
                        match it {
                            Item::Note { pitch, dur: None, m } if m == Mods::default() => pitches.push(pitch),
                            Item::Note { .. } => return Err("notes inside [ ] take no duration or modifiers; put them after ]".into()),
                            _ => return Err("only notes (or drum names) can go inside [ ]".into()),
                        }
                    }
                    let (dur, m) = parse_dur_mods(suffix, ctx.bar)?;
                    Item::Chord { pitches, dur, m }
                } else {
                    let (repeat, transpose) = parse_rep_suffix(suffix)?;
                    Item::Group { items, repeat, transpose }
                };
                stack.last_mut().unwrap().push(item);
            }
            Tok::Bar => {
                if stack.len() == 1 {
                    stack[0].push(Item::BarCheck(ctx.line));
                }
            }
            Tok::Word(w) if w == "over" => {
                // the pattern: everything before `over` on this line / in this group
                // (leading settings such as `mf`, `o3`, `vol=…` stay outside and apply first)
                let level = stack.last_mut().unwrap();
                let lead = level.iter().take_while(|i| matches!(i, Item::Vel(_) | Item::Octave { .. } | Item::Control { .. } | Item::Program(_) | Item::Pedal(_) | Item::BarCheck(_))).count();
                let before: Vec<Item> = level.drain(lead..).filter(|i| !matches!(i, Item::BarCheck(_))).collect();
                if before.is_empty() {
                    return Err("'over' needs a pattern before it, e.g. `1e 5 8 5 over | Am | F |`".into());
                }
                let pattern = if before.len() == 1 { before.into_iter().next().unwrap() } else { Item::Group { items: before, repeat: 1, transpose: 0 } };
                // the progression: a named one (`prog`), or the rest of the line
                let rest: Vec<String> = toks[i + 1..]
                    .iter()
                    .map(|t| match t {
                        Tok::Word(w) => w.clone(),
                        Tok::Bar => "|".into(),
                        Tok::Open(c) => c.to_string(),
                        Tok::Close(c, s) => format!("{c}{s}"),
                    })
                    .collect();
                if rest.is_empty() {
                    return Err("'over' needs a progression after it, e.g. `arp over | Am | F | C | G |`".into());
                }
                let (prog, used) = match ctx.src.prog(rest[0].split('*').next().unwrap_or("")) {
                    Some(chords) => {
                        let rep: u32 = rest[0].split_once('*').map(|(_, r)| r.parse().map_err(|_| format!("bad repeat in '{}'", rest[0]))).transpose()?.unwrap_or(1);
                        let mut v = Vec::new();
                        for _ in 0..rep {
                            v.extend(chords.iter().cloned());
                        }
                        (v, 1)
                    }
                    None => (parse_prog(&rest, ctx)?, rest.len()),
                };
                stack.last_mut().unwrap().push(Item::Over { pattern: Box::new(pattern), prog });
                i += 1 + used;
                continue;
            }
            Tok::Word(w) => {
                let it = parse_word(w, ctx)?;
                stack.last_mut().unwrap().push(it);
            }
        }
        i += 1;
    }
    if !opens.is_empty() {
        return Err(format!("unclosed '{}'", opens.last().unwrap()));
    }
    Ok(stack.pop().unwrap())
}

// ------------------------------------------------------------------ file parser

fn strip_comment(line: &str) -> &str {
    let b = line.as_bytes();
    for i in 0..b.len() {
        let at_word_start = i == 0 || b[i - 1].is_ascii_whitespace();
        if at_word_start && (b[i] == b'#' || (b[i] == b'/' && b.get(i + 1) == Some(&b'/'))) {
            return &line[..i];
        }
    }
    line
}

/// Splits `key=value` options (values may be "quoted").
fn options(s: &str) -> Result<Vec<(String, String)>, String> {
    let mut out = Vec::new();
    let cs: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < cs.len() {
        while i < cs.len() && (cs[i].is_whitespace() || cs[i] == ',') {
            i += 1;
        }
        if i >= cs.len() {
            break;
        }
        let ks = i;
        while i < cs.len() && !cs[i].is_whitespace() && cs[i] != '=' && cs[i] != ',' {
            i += 1;
        }
        let k: String = cs[ks..i].iter().collect();
        let mut v = String::new();
        if i < cs.len() && cs[i] == '=' {
            i += 1;
            if i < cs.len() && cs[i] == '"' {
                i += 1;
                let vs = i;
                while i < cs.len() && cs[i] != '"' {
                    i += 1;
                }
                if i >= cs.len() {
                    return Err("unterminated quote".into());
                }
                v = cs[vs..i].iter().collect();
                i += 1;
            } else {
                let vs = i;
                while i < cs.len() && !cs[i].is_whitespace() && cs[i] != ',' {
                    i += 1;
                }
                v = cs[vs..i].iter().collect();
            }
        }
        out.push((k, v));
    }
    Ok(out)
}

fn parse_time(s: &str) -> Result<(u32, u32), String> {
    let (a, b) = s.trim().split_once('/').ok_or_else(|| format!("time '{s}': expected e.g. 4/4"))?;
    let a: u32 = a.trim().parse().map_err(|_| format!("time '{s}'"))?;
    let b: u32 = b.trim().parse().map_err(|_| format!("time '{s}'"))?;
    if a == 0 || !matches!(b, 1 | 2 | 4 | 8 | 16) {
        return Err(format!("time '{s}': bad signature"));
    }
    Ok((a, b))
}

fn parse_tempo(s: &str) -> Result<(f64, Option<f64>), String> {
    let num = |x: &str| x.trim().parse::<f64>().ok().filter(|v| *v >= 10.0 && *v <= 400.0).ok_or_else(|| format!("tempo '{s}': expected BPM 10..400"));
    match s.split_once("->") {
        Some((a, b)) => Ok((num(a)?, Some(num(b)?))),
        None => Ok((num(s)?, None)),
    }
}

fn parse_swing(s: &str) -> Result<f64, String> {
    let v: f64 = s.trim().trim_end_matches('%').parse().map_err(|_| format!("swing '{s}': expected 50..75"))?;
    if !(50.0..=80.0).contains(&v) {
        return Err(format!("swing '{s}': expected 50 (straight) .. 75"));
    }
    Ok(v / 100.0)
}

/// Parses an arrangement: `intro |: A A B A(+2 -drums) :| outro`, `A*2`.
pub fn parse_arrange(s: &str) -> Result<Vec<ArrEntry>, String> {
    let mut out = Vec::new();
    let cs: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < cs.len() {
        if cs[i].is_whitespace() {
            i += 1;
            continue;
        }
        let st = i;
        let mut depth = 0;
        while i < cs.len() && (depth > 0 || !cs[i].is_whitespace()) {
            if cs[i] == '(' {
                depth += 1;
            } else if cs[i] == ')' {
                depth -= 1;
            }
            i += 1;
        }
        let w: String = cs[st..i].iter().collect();
        match w.as_str() {
            "|:" => out.push(ArrEntry::LoopStart),
            ":|" => out.push(ArrEntry::LoopEnd),
            _ => {
                let (head, opts) = match w.find('(') {
                    Some(p) => (&w[..p], w[p + 1..].strip_suffix(')').ok_or_else(|| format!("arrange '{w}': missing ')'"))?),
                    None => (w.as_str(), ""),
                };
                let (name, mut repeat) = match head.split_once('*') {
                    Some((n, r)) => (n, r.parse::<u32>().map_err(|_| format!("arrange '{w}': bad repeat"))?),
                    None => (head, 1),
                };
                let mut transpose = 0;
                let mut mute = Vec::new();
                for o in opts.split(|c: char| c.is_whitespace() || c == ',').filter(|o| !o.is_empty()) {
                    if let Ok(n) = o.parse::<i32>() {
                        transpose += n;
                    } else if let Some(n) = o.strip_prefix('x').or(o.strip_prefix('*')).and_then(|n| n.parse::<u32>().ok()) {
                        repeat = n;
                    } else if let Some(t) = o.strip_prefix('-') {
                        mute.push(t.to_string());
                    } else {
                        return Err(format!("arrange '{w}': bad option '{o}' (+N/-N transpose, xN repeat, -track mute)"));
                    }
                }
                out.push(ArrEntry::Play { section: name.to_string(), repeat, transpose, mute });
            }
        }
    }
    Ok(out)
}

enum Ctx {
    Top,
    Section(usize),
    Variant(usize),
}

/// Parses a song file.
pub fn parse(text: &str) -> Result<SongSrc, String> {
    let mut src = SongSrc::default();
    parse_into(&mut src, text)?;
    validate(&src)?;
    Ok(src)
}

/// Parses notation text into an existing song (definitions accumulate;
/// statements start at top level). Does not run the final [`validate`].
pub fn parse_into(src: &mut SongSrc, text: &str) -> Result<(), String> {
    // join continuation lines, keep the first line number
    let mut lines: Vec<(usize, String)> = Vec::new();
    let mut pending: Option<(usize, String)> = None;
    for (n, raw) in text.lines().enumerate() {
        let l = strip_comment(raw).trim_end();
        let (body, cont) = match l.strip_suffix('\\') {
            Some(b) => (b, true),
            None => (l, false),
        };
        let cur = match pending.take() {
            Some((ln, mut s)) => {
                s.push(' ');
                s.push_str(body.trim());
                (ln, s)
            }
            None => (n + 1, body.to_string()),
        };
        if cont {
            pending = Some(cur);
        } else {
            lines.push(cur);
        }
    }
    if let Some(p) = pending {
        lines.push(p);
    }

    let mut ctx = Ctx::Top;
    let mut i = 0;
    while i < lines.len() {
        let (ln, line) = &lines[i];
        let ln = *ln;
        i += 1;
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        // a variant's lines are indented; the first unindented line ends it
        if matches!(ctx, Ctx::Variant(_)) && !line.starts_with(char::is_whitespace) {
            ctx = Ctx::Top;
        }
        let err = |e: String| format!("line {ln}: {e}");
        let (word, rest) = match t.find(|c: char| c.is_whitespace()) {
            Some(p) => (&t[..p], t[p..].trim()),
            None => (t, ""),
        };
        let cur_time = match ctx {
            Ctx::Section(s) => src.sections[s].time.unwrap_or(src.time),
            _ => src.time,
        };
        let bar = bar_ticks(cur_time);
        match word {
            "title" => src.title = rest.trim_matches('"').to_string(),
            "tempo" => {
                let tv = parse_tempo(rest).map_err(err)?;
                match ctx {
                    Ctx::Top => {
                        if tv.1.is_some() {
                            return Err(err("tempo ramps (a->b) belong in a section".into()));
                        }
                        src.tempo = tv.0
                    }
                    Ctx::Section(s) => src.sections[s].tempo = Some(tv),
                    Ctx::Variant(v) => src.variants[v].tempo = Some(tv.0),
                }
            }
            "time" => {
                let tv = parse_time(rest).map_err(err)?;
                match ctx {
                    Ctx::Section(s) => src.sections[s].time = Some(tv),
                    _ => src.time = tv,
                }
            }
            "key" => src.key = Some(Key::parse(rest).map_err(err)?),
            "swing" => src.swing = parse_swing(rest).map_err(err)?,
            "end" => ctx = Ctx::Top,
            "track" => {
                ctx = Ctx::Top;
                let (name, opts) = match rest.find(|c: char| c.is_whitespace()) {
                    Some(p) => (&rest[..p], &rest[p..]),
                    None => (rest, ""),
                };
                let name = name.trim_end_matches(':');
                if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                    return Err(err(format!("bad track name '{name}' (letters, digits, _)")));
                }
                if src.track(name).is_some() {
                    return Err(err(format!("track '{name}' defined twice")));
                }
                let mut td = TrackDef::new(name);
                let mut kit = None;
                for (k, v) in options(opts).map_err(err)? {
                    let int = |v: &str, lo: i32, hi: i32| -> Result<i32, String> {
                        v.parse::<i32>().ok().filter(|n| (lo..=hi).contains(n)).ok_or_else(|| err(format!("{k}={v}: expected {lo}..{hi}")))
                    };
                    match k.as_str() {
                        "drums" => {
                            td.drums = true;
                            if !v.is_empty() {
                                kit = Some(v.clone());
                            }
                        }
                        "inst" | "instrument" | "program" => td.program = gm::program(&v.replace('_', " ")).map_err(err)?,
                        "ch" | "channel" => td.channel = Some(int(&v, 1, 16)? as u8),
                        "vol" | "volume" => td.vol = int(&v, 0, 127)? as u8,
                        "pan" => td.pan = int(&v, -100, 100)?,
                        "expr" => td.expr = int(&v, 0, 127)? as u8,
                        "reverb" => td.reverb = int(&v, 0, 127)? as u8,
                        "chorus" => td.chorus = int(&v, 0, 127)? as u8,
                        "oct" | "octave" => td.octave = int(&v, 0, 8)?,
                        "vel" => td.vel = int(&v, 1, 127)? as u8,
                        "gate" => td.gate = int(&v, 5, 100)? as f64 / 100.0,
                        "human" => td.human = int(&v, 0, 40)? as u8,
                        "transpose" => td.transpose = int(&v, -48, 48)?,
                        "swing" => td.swing = Some(parse_swing(&v).map_err(err)?),
                        "bendrange" => td.bend_range = int(&v, 1, 24)? as u8,
                        _ => return Err(err(format!("unknown track option '{k}' (inst ch vol pan expr reverb chorus oct vel gate human transpose swing bendrange drums)"))),
                    }
                }
                if td.drums {
                    td.program = match kit {
                        Some(k) => gm::KITS.iter().find(|(n, _)| *n == k).map(|(_, p)| *p).ok_or_else(|| {
                            err(format!("unknown drum kit '{k}' (known: {})", gm::KITS.iter().map(|k| k.0).collect::<Vec<_>>().join(" ")))
                        })?,
                        None => 0,
                    };
                    if td.channel.is_none() {
                        td.channel = Some(10);
                    }
                    td.gate = td.gate.min(0.5);
                }
                src.tracks.push(td);
            }
            "pattern" | "prog" | "grid" => {
                ctx = Ctx::Top;
                let p = rest.find(|c: char| c.is_whitespace() || c == '=' || c == ':').unwrap_or(rest.len());
                let name = &rest[..p];
                let body = rest[p..].trim_start();
                let body = body.strip_prefix('=').or_else(|| body.strip_prefix(':')).unwrap_or(body).trim();
                if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                    return Err(err(format!("bad {word} name '{name}' (letters, digits, _)")));
                }
                if looks_like_note(name) || name == "r" || name.starts_with('o') && name[1..].parse::<i32>().is_ok() || KEYWORDS.contains(&name) || DYNAMICS.iter().any(|d| d.0 == name) || gm::drum(name).is_some() {
                    return Err(err(format!("{word} name '{name}' clashes with a note, drum, dynamic or keyword; pick another")));
                }
                if src.pattern(name).is_some() || src.prog(name).is_some() {
                    return Err(err(format!("'{name}' defined twice")));
                }
                let sctx = StreamCtx { src, bar, key: src.key.as_ref(), line: ln, drums: true };
                match word {
                    "pattern" => {
                        let items = parse_stream(body, &sctx).map_err(err)?;
                        src.patterns.push((name.to_string(), items));
                    }
                    "prog" => {
                        let toks: Vec<String> = tokenize(body).map_err(err)?.into_iter().map(|t| match t {
                            Tok::Word(w) => w,
                            _ => "|".into(),
                        }).collect();
                        let p = parse_prog(&toks, &sctx).map_err(err)?;
                        src.progs.push((name.to_string(), p));
                    }
                    _ => {
                        // grid NAME [step=s]; rows follow
                        let mut step = PPQ / 4;
                        for (k, v) in options(body).map_err(err)? {
                            match k.as_str() {
                                "step" => step = parse_dur(&v, bar).map_err(err)?,
                                _ => return Err(err(format!("unknown grid option '{k}' (step=s)"))),
                            }
                        }
                        let mut rows = Vec::new();
                        while i < lines.len() {
                            let l = lines[i].1.trim();
                            let Some((label, cells)) = l.split_once(char::is_whitespace) else { break };
                            let cells = cells.trim();
                            if l.is_empty() || cells.is_empty() || !cells.chars().all(|c| "xXo.-=_| ".contains(c)) || KEYWORDS.contains(&label) {
                                break;
                            }
                            rows.push((label.to_string(), cells.to_string()));
                            i += 1;
                        }
                        if rows.is_empty() {
                            return Err(err(format!("grid '{name}' has no rows (e.g. `kick x...x...x...x...` on the next lines)")));
                        }
                        let item = parse_grid(&rows, step, &sctx).map_err(err)?;
                        src.patterns.push((name.to_string(), vec![item]));
                    }
                }
            }
            "section" => {
                let mut it = rest.split_whitespace();
                let name = it.next().ok_or_else(|| err("section needs a name".into()))?.trim_end_matches(':');
                if src.section(name).is_some() {
                    return Err(err(format!("section '{name}' defined twice")));
                }
                let mut sec = Section { name: name.into(), bars: None, tempo: None, time: None, parts: Vec::new() };
                for o in it {
                    let (k, v) = o.split_once('=').unwrap_or((o, ""));
                    match k {
                        "bars" => sec.bars = Some(v.parse().map_err(|_| err(format!("bars={v}")))?),
                        "tempo" => sec.tempo = Some(parse_tempo(v).map_err(err)?),
                        "time" => sec.time = Some(parse_time(v).map_err(err)?),
                        _ => match o.parse::<u32>() {
                            Ok(n) => sec.bars = Some(n),
                            Err(_) => return Err(err(format!("unknown section option '{o}' (bars=N tempo=N time=N/M)"))),
                        },
                    }
                }
                src.sections.push(sec);
                ctx = Ctx::Section(src.sections.len() - 1);
            }
            "variant" => {
                let name = rest.trim_end_matches(':').trim();
                if name.is_empty() || src.variants.iter().any(|v| v.name == name) {
                    return Err(err(format!("variant needs a unique name ('{name}')")));
                }
                src.variants.push(Variant { name: name.into(), ..Variant::default() });
                ctx = Ctx::Variant(src.variants.len() - 1);
            }
            "arrange" => {
                let a = parse_arrange(rest).map_err(err)?;
                match ctx {
                    Ctx::Variant(v) => src.variants[v].arrange = Some(a),
                    _ => src.arrange = Some(a),
                }
            }
            "transpose" | "inst" | "mute" | "vol" | "shift" if !matches!(ctx, Ctx::Variant(_)) => {
                return Err(err(format!("`{word}` is only valid inside a variant (indent the variant's lines)")));
            }
            "transpose" | "inst" | "mute" | "vol" | "shift" => {
                let Ctx::Variant(vi) = ctx else { unreachable!() };
                let v = &mut src.variants[vi];
                let mut words = rest.split_whitespace();
                match word {
                    "transpose" => v.transpose = rest.parse().map_err(|_| err(format!("transpose '{rest}'")))?,
                    "mute" => v.mute.extend(words.map(|w| w.trim_end_matches(',').to_string())),
                    _ => {
                        let track = words.next().ok_or_else(|| err(format!("{word} TRACK VALUE")))?.to_string();
                        let val: String = words.collect::<Vec<_>>().join(" ");
                        match word {
                            "inst" => v.inst.push((track, gm::program(val.trim_matches('"')).map_err(err)?)),
                            "vol" => v.vol.push((track, val.parse::<u8>().ok().filter(|v| *v <= 127).ok_or_else(|| err(format!("vol '{val}'")))?)),
                            _ => v.shift.push((track, val.parse().map_err(|_| err(format!("shift '{val}'")))?)),
                        }
                    }
                }
            }
            _ => {
                // TRACK: stream
                let Some(p) = t.find(':') else {
                    return Err(err(format!("cannot read '{t}' (expected a statement like `track`, `section`, or `TRACKNAME: notes…`)")));
                };
                let tname = t[..p].trim();
                let body = &t[p + 1..];
                let Some(drums) = src.track(tname).map(|t| t.drums) else {
                    return Err(err(format!("unknown track '{tname}' (define it first with `track {tname} inst=…`)")));
                };
                if matches!(ctx, Ctx::Variant(_)) {
                    return Err(err("variants cannot contain notes; define a section and use `arrange` in the variant".into()));
                }
                let sidx = match ctx {
                    Ctx::Section(s) => s,
                    _ => match src.sections.iter().position(|s| s.name == "main") {
                        Some(s) => s,
                        None => {
                            src.sections.push(Section { name: "main".into(), bars: None, tempo: None, time: None, parts: Vec::new() });
                            src.sections.len() - 1
                        }
                    },
                };
                let sctx = StreamCtx { src, bar, key: src.key.as_ref(), line: ln, drums };
                let items = parse_stream(body, &sctx).map_err(err)?;
                let tname = tname.to_string();
                src.sections[sidx].parts.push((tname, items, ln));
            }
        }
    }
    Ok(())
}

/// Checks a complete song: tracks and notes exist, arrangements and
/// variants name known sections and tracks.
pub fn validate(src: &SongSrc) -> Result<(), String> {
    if src.tracks.is_empty() {
        return Err("no tracks (add e.g. `track lead inst=\"Flute\"`)".into());
    }
    if src.sections.is_empty() {
        return Err("no notes (add a section with lines like `lead: C4q D4 E4 F4`)".into());
    }
    let check = |a: &[ArrEntry], who: &str| -> Result<(), String> {
        for e in a {
            if let ArrEntry::Play { section, mute, .. } = e {
                if src.section(section).is_none() {
                    return Err(format!("{who}: unknown section '{section}'"));
                }
                for m in mute {
                    if src.track(m).is_none() {
                        return Err(format!("{who}: unknown track '{m}' in mute"));
                    }
                }
            }
        }
        Ok(())
    };
    if let Some(a) = &src.arrange {
        check(a, "arrange")?;
    }
    for v in &src.variants {
        if let Some(a) = &v.arrange {
            check(a, &format!("variant {}", v.name))?;
        }
        for t in v.mute.iter().chain(v.inst.iter().map(|x| &x.0)).chain(v.vol.iter().map(|x| &x.0)).chain(v.shift.iter().map(|x| &x.0)) {
            if src.track(t).is_none() {
                return Err(format!("variant {}: unknown track '{t}'", v.name));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(src: &SongSrc) -> StreamCtx<'_> {
        StreamCtx { src, bar: 1920, key: None, line: 1, drums: false }
    }

    #[test]
    fn durations() {
        assert_eq!(parse_dur("q", 1920), Ok(480));
        assert_eq!(parse_dur("h.", 1920), Ok(1440));
        assert_eq!(parse_dur("e3", 1920), Ok(160));
        assert_eq!(parse_dur("h+e", 1920), Ok(1200));
        assert_eq!(parse_dur("2w", 1920), Ok(3840));
        assert_eq!(parse_dur("bar", 1440), Ok(1440));
        assert!(parse_dur("y", 1920).is_err());
    }

    #[test]
    fn notes_and_mods() {
        let s = SongSrc::default();
        let items = parse_stream("C4q D#4e. Eb Bb3h~ r rq F4>^ G4@100 [C4 E4 G4]h {Am7}w | 1e 5' 3, 0 *q", &ctx(&s)).unwrap();
        assert_eq!(items[0], Item::Note { pitch: Pitch::Abs(60), dur: Some(480), m: Mods::default() });
        assert_eq!(items[1], Item::Note { pitch: Pitch::Abs(63), dur: Some(360), m: Mods::default() });
        assert_eq!(items[2], Item::Note { pitch: Pitch::Rel(3), dur: None, m: Mods::default() });
        assert_eq!(items[3], Item::Note { pitch: Pitch::Abs(58), dur: Some(960), m: Mods { tie: true, ..Mods::default() } });
        assert_eq!(items[4], Item::Rest(None));
        assert_eq!(items[5], Item::Rest(Some(480)));
        assert_eq!(items[6], Item::Note { pitch: Pitch::Abs(65), dur: None, m: Mods { accent: 1, staccato: true, ..Mods::default() } });
        assert_eq!(items[7], Item::Note { pitch: Pitch::Abs(67), dur: None, m: Mods { vel: Some(100), ..Mods::default() } });
        assert!(matches!(&items[8], Item::Chord { pitches, dur: Some(960), .. } if pitches.len() == 3));
        assert!(matches!(&items[9], Item::Symbol { chord: Some(c), dur: Some(1920), .. } if c.root == 9));
        assert_eq!(items[10], Item::BarCheck(1));
        assert_eq!(items[11], Item::Note { pitch: Pitch::Degree(1, 0), dur: Some(240), m: Mods::default() });
        assert_eq!(items[12], Item::Note { pitch: Pitch::Degree(5, 1), dur: None, m: Mods::default() });
        assert_eq!(items[13], Item::Note { pitch: Pitch::Degree(3, -1), dur: None, m: Mods::default() });
        assert_eq!(items[14], Item::Note { pitch: Pitch::Bass(0), dur: None, m: Mods::default() });
        assert!(matches!(&items[15], Item::Symbol { chord: None, dur: Some(480), .. }));
        assert!(parse_stream("c4", &ctx(&s)).unwrap_err().contains("uppercase"));
        assert!(parse_stream("kick", &ctx(&s)).unwrap_err().contains("drum track"));
    }

    #[test]
    fn groups_controls_prog() {
        let s = SongSrc::default();
        let items = parse_stream("(C4e D4)*3+2 vol=90 pan=-30->30:2w ped+ mf o3 o+1", &ctx(&s)).unwrap();
        assert!(matches!(&items[0], Item::Group { repeat: 3, transpose: 2, items } if items.len() == 2));
        assert_eq!(items[1], Item::Control { ctl: Ctl::Cc(7), from: 90.0, to: None });
        assert_eq!(items[2], Item::Control { ctl: Ctl::Pan, from: -30.0, to: Some((30.0, 3840)) });
        assert_eq!(items[3], Item::Pedal(true));
        assert_eq!(items[4], Item::Vel(80));
        assert_eq!(items[5], Item::Octave { value: 3, relative: false });
        assert_eq!(items[6], Item::Octave { value: 1, relative: true });
        let items = parse_stream("1e 5 over | Am | F G | C . . G | % |", &ctx(&s)).unwrap();
        assert_eq!(items.len(), 1);
        let Item::Over { prog, pattern } = &items[0] else { panic!("{:?}", items[0]) };
        assert!(matches!(&**pattern, Item::Group { items, .. } if items.len() == 2));
        let lens: Vec<u32> = prog.iter().map(|p| p.1).collect();
        assert_eq!(lens, vec![1920, 960, 960, 1440, 480, 1440, 480]);
        assert_eq!(prog[3].0.as_ref().unwrap().symbol, "C");
    }

    #[test]
    fn whole_file() {
        let src = parse(
            r#"
title "Test"   # comment
tempo 100
time 3/4
key D minor
track lead inst="Flute" vol=90 oct=5
track kit drums
pattern motif = D5e F A D6q   // another comment
prog verse = | i | VI | VII | i |
grid beat
  kick  x.. x.. x..
  hat   xxx xxx xxx
section A bars=2 tempo=90
  lead: motif C6q. \
        r
  kit: beat
section B
  lead: 1e 3 5 over verse
variant low
  transpose -12
  mute kit
arrange |: A B :| A(+2 -kit)
"#,
        )
        .unwrap();
        assert_eq!(src.title, "Test");
        assert_eq!(src.time, (3, 4));
        assert_eq!(src.tracks[1].channel, Some(10));
        assert_eq!(src.tracks[0].program, 73);
        assert_eq!(src.sections.len(), 2);
        assert_eq!(src.sections[0].tempo, Some((90.0, None)));
        assert_eq!(src.sections[0].parts[0].1.len(), 3);
        let Item::Par(lanes) = &src.pattern("beat").unwrap()[0] else { panic!() };
        assert_eq!(lanes.len(), 2);
        assert_eq!(src.variants[0].transpose, -12);
        assert_eq!(src.arrange.as_ref().unwrap().len(), 5);
        let Item::Over { prog, .. } = &src.sections[1].parts[0].1[0] else { panic!() };
        assert_eq!(prog[1].0.as_ref().unwrap().root, 10); // VI of D minor = Bb
        assert!(parse("track a\nb: C4").unwrap_err().contains("unknown track"));
        assert!(parse("track a\npattern A = C4").unwrap_err().contains("clashes"));
    }
}
