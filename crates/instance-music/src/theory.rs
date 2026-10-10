//! Pitches, keys/scales, chord symbols, roman numerals and voicing.

/// Pitch class of a note letter (C = 0).
fn letter_pc(c: char) -> Option<i32> {
    Some(match c.to_ascii_uppercase() {
        'C' => 0,
        'D' => 2,
        'E' => 4,
        'F' => 5,
        'G' => 7,
        'A' => 9,
        'B' => 11,
        _ => return None,
    })
}

/// Parses a note letter with accidentals (`C`, `F#`, `Bb`, `Ebb`) at the start
/// of `s`. Returns the pitch-class offset from C (may be <0 or >11 for `Cb`,
/// `B#`) and the remaining text.
pub fn parse_pitch_letter(s: &str) -> Option<(i32, &str)> {
    let mut chars = s.char_indices();
    let (_, c) = chars.next()?;
    let mut pc = letter_pc(c)?;
    let mut end = c.len_utf8();
    for (i, c) in chars {
        match c {
            '#' => pc += 1,
            'b' => pc -= 1,
            _ => {
                end = i;
                return Some((pc, &s[end..]));
            }
        }
        end = i + c.len_utf8();
    }
    Some((pc, &s[end..]))
}

/// Parses a full pitch such as `C4`, `F#3`, `Bb-1` (MIDI 60 = C4).
pub fn parse_pitch(s: &str) -> Option<i32> {
    let (pc, rest) = parse_pitch_letter(s)?;
    let oct: i32 = rest.parse().ok()?;
    let m = (oct + 1) * 12 + pc;
    (0..=127).contains(&m).then_some(m)
}

const NAMES: [&str; 12] = ["C", "C#", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B"];

/// Name of a pitch class (sharps for C#/F#, flats otherwise).
pub fn pc_name(pc: i32) -> &'static str {
    NAMES[pc.rem_euclid(12) as usize]
}

/// Scientific name of a MIDI note (`60` → `C4`).
pub fn note_name(m: i32) -> String {
    format!("{}{}", pc_name(m), m.div_euclid(12) - 1)
}

/// Note name of a frequency in Hz (nearest equal-tempered pitch).
pub fn freq_note_name(hz: f64) -> String {
    if hz <= 0.0 {
        return "-".into();
    }
    let m = (69.0 + 12.0 * (hz / 440.0).log2()).round() as i32;
    note_name(m)
}

/// A key: tonic pitch class and scale.
#[derive(Clone, Debug, PartialEq)]
pub struct Key {
    pub tonic: i32,
    pub mode: String,
    /// Semitone offsets of the scale degrees from the tonic.
    pub scale: Vec<i32>,
}

/// Known scales (name, intervals). Seven-note scales support roman numerals.
pub const SCALES: &[(&str, &[i32])] = &[
    ("major", &[0, 2, 4, 5, 7, 9, 11]),
    ("ionian", &[0, 2, 4, 5, 7, 9, 11]),
    ("minor", &[0, 2, 3, 5, 7, 8, 10]),
    ("aeolian", &[0, 2, 3, 5, 7, 8, 10]),
    ("dorian", &[0, 2, 3, 5, 7, 9, 10]),
    ("phrygian", &[0, 1, 3, 5, 7, 8, 10]),
    ("lydian", &[0, 2, 4, 6, 7, 9, 11]),
    ("mixolydian", &[0, 2, 4, 5, 7, 9, 10]),
    ("locrian", &[0, 1, 3, 5, 6, 8, 10]),
    ("harmonic_minor", &[0, 2, 3, 5, 7, 8, 11]),
    ("melodic_minor", &[0, 2, 3, 5, 7, 9, 11]),
    ("pentatonic", &[0, 2, 4, 7, 9]),
    ("minor_pentatonic", &[0, 3, 5, 7, 10]),
    ("blues", &[0, 3, 5, 6, 7, 10]),
    ("whole_tone", &[0, 2, 4, 6, 8, 10]),
    ("chromatic", &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]),
];

impl Key {
    /// Parses `A minor`, `C`, `F# dorian`, `Eb major`, `Am`.
    pub fn parse(s: &str) -> Result<Key, String> {
        let s = s.trim();
        let (pc, rest) = parse_pitch_letter(s).ok_or_else(|| format!("key '{s}': expected a tonic like C, F#, Bb"))?;
        let mut mode = rest.trim().to_ascii_lowercase().replace([' ', '-'], "_");
        if mode.is_empty() || mode == "maj" {
            mode = "major".into();
        } else if mode == "m" || mode == "min" {
            mode = "minor".into();
        }
        let scale = SCALES
            .iter()
            .find(|(n, _)| *n == mode)
            .map(|(_, v)| v.to_vec())
            .ok_or_else(|| format!("key '{s}': unknown scale '{mode}' (known: {})", SCALES.iter().map(|s| s.0).collect::<Vec<_>>().join(", ")))?;
        Ok(Key { tonic: pc.rem_euclid(12), mode, scale })
    }

    /// Whether a MIDI note belongs to the scale.
    pub fn contains(&self, m: i32) -> bool {
        self.scale.contains(&(m - self.tonic).rem_euclid(12))
    }
}

/// Chord qualities: symbol suffix → intervals above the root.
const QUALITIES: &[(&str, &[i32])] = &[
    ("", &[0, 4, 7]),
    ("maj", &[0, 4, 7]),
    ("M", &[0, 4, 7]),
    ("m", &[0, 3, 7]),
    ("min", &[0, 3, 7]),
    ("-", &[0, 3, 7]),
    ("dim", &[0, 3, 6]),
    ("o", &[0, 3, 6]),
    ("°", &[0, 3, 6]),
    ("aug", &[0, 4, 8]),
    ("+", &[0, 4, 8]),
    ("sus2", &[0, 2, 7]),
    ("sus4", &[0, 5, 7]),
    ("sus", &[0, 5, 7]),
    ("5", &[0, 7]),
    ("6", &[0, 4, 7, 9]),
    ("m6", &[0, 3, 7, 9]),
    ("69", &[0, 4, 7, 9, 14]),
    ("7", &[0, 4, 7, 10]),
    ("maj7", &[0, 4, 7, 11]),
    ("M7", &[0, 4, 7, 11]),
    ("m7", &[0, 3, 7, 10]),
    ("min7", &[0, 3, 7, 10]),
    ("-7", &[0, 3, 7, 10]),
    ("mmaj7", &[0, 3, 7, 11]),
    ("m(maj7)", &[0, 3, 7, 11]),
    ("dim7", &[0, 3, 6, 9]),
    ("o7", &[0, 3, 6, 9]),
    ("°7", &[0, 3, 6, 9]),
    ("m7b5", &[0, 3, 6, 10]),
    ("ø", &[0, 3, 6, 10]),
    ("ø7", &[0, 3, 6, 10]),
    ("7sus4", &[0, 5, 7, 10]),
    ("7sus2", &[0, 2, 7, 10]),
    ("aug7", &[0, 4, 8, 10]),
    ("+7", &[0, 4, 8, 10]),
    ("add9", &[0, 4, 7, 14]),
    ("madd9", &[0, 3, 7, 14]),
    ("9", &[0, 4, 7, 10, 14]),
    ("maj9", &[0, 4, 7, 11, 14]),
    ("m9", &[0, 3, 7, 10, 14]),
    ("7b9", &[0, 4, 7, 10, 13]),
    ("7#9", &[0, 4, 7, 10, 15]),
    ("11", &[0, 7, 10, 14, 17]),
    ("m11", &[0, 3, 7, 10, 14, 17]),
    ("13", &[0, 4, 7, 10, 14, 21]),
];

/// A chord: root pitch class, intervals above it, optional bass pitch class.
#[derive(Clone, Debug, PartialEq)]
pub struct Chord {
    pub root: i32,
    pub intervals: Vec<i32>,
    pub bass: Option<i32>,
    /// The symbol as written.
    pub symbol: String,
}

fn quality(q: &str) -> Option<Vec<i32>> {
    QUALITIES.iter().find(|(n, _)| *n == q).map(|(_, v)| v.to_vec())
}

fn roman_value(s: &str) -> Option<(usize, bool, usize)> {
    // returns (degree 1..7, uppercase, length)
    const R: [&str; 7] = ["VII", "III", "VI", "IV", "II", "V", "I"];
    const D: [usize; 7] = [7, 3, 6, 4, 2, 5, 1];
    for (r, d) in R.iter().zip(D) {
        if s.len() >= r.len() {
            let head = &s[..r.len()];
            if head == *r {
                return Some((d, true, r.len()));
            }
            if head == r.to_ascii_lowercase() {
                return Some((d, false, r.len()));
            }
        }
    }
    None
}

impl Chord {
    /// Parses a chord symbol (`Am7`, `Cmaj7/G`, `F#dim`, `Bbsus4`) or, given a
    /// key, a roman numeral (`i`, `VI`, `bVII`, `V7`, `ii7`, `viio`, `iiø7`).
    pub fn parse(sym: &str, key: Option<&Key>) -> Result<Chord, String> {
        let (body, bass) = match sym.rfind('/') {
            Some(i) if i > 0 => {
                let b = &sym[i + 1..];
                let (pc, rest) = parse_pitch_letter(b).ok_or_else(|| format!("chord '{sym}': bad bass note '{b}'"))?;
                if !rest.is_empty() {
                    return Err(format!("chord '{sym}': bad bass note '{b}'"));
                }
                (&sym[..i], Some(pc.rem_euclid(12)))
            }
            _ => (sym, None),
        };
        let first = body.chars().next().ok_or("empty chord symbol")?;
        if first.is_ascii_uppercase() && letter_pc(first).is_some() {
            let (pc, q) = parse_pitch_letter(body).unwrap();
            let intervals = quality(q).ok_or_else(|| {
                format!("chord '{sym}': unknown quality '{q}' (known: {})", QUALITIES.iter().map(|q| if q.0.is_empty() { "(major)" } else { q.0 }).collect::<Vec<_>>().join(" "))
            })?;
            return Ok(Chord { root: pc.rem_euclid(12), intervals, bass, symbol: sym.into() });
        }
        // roman numeral
        let mut s = body;
        let mut shift = 0;
        while let Some(r) = s.strip_prefix('b') {
            shift -= 1;
            s = r;
        }
        while let Some(r) = s.strip_prefix('#') {
            shift += 1;
            s = r;
        }
        let (deg, upper, len) = roman_value(s).ok_or_else(|| format!("chord '{sym}': expected a chord like Am7 or a roman numeral like VI"))?;
        let key = key.ok_or_else(|| format!("chord '{sym}': roman numerals need a `key` line"))?;
        if key.scale.len() != 7 {
            return Err(format!("chord '{sym}': roman numerals need a 7-note scale, key is {}", key.mode));
        }
        let suffix = &s[len..];
        let q = match (upper, suffix) {
            (true, s) => s.to_string(),
            (false, "" | "7" | "9" | "11" | "6" | "add9" | "maj7") => format!("m{suffix}"),
            (false, s) => s.to_string(), // o, o7, ø7, dim, dim7
        };
        let intervals = quality(&q).ok_or_else(|| format!("chord '{sym}': unknown roman suffix '{suffix}'"))?;
        let root = (key.tonic + key.scale[deg - 1] + shift).rem_euclid(12);
        Ok(Chord { root, intervals, bass, symbol: sym.into() })
    }

    /// Pitch classes (root first).
    pub fn pcs(&self) -> Vec<i32> {
        self.intervals.iter().map(|i| (self.root + i).rem_euclid(12)).collect()
    }

    /// Semitones above the root of chord degree `d` (1 root, 3 third, 5 fifth,
    /// 7 seventh, 9/11/13 extensions; 8, 10, 12, 14… are the octave-up forms).
    /// A degree the chord lacks falls back to the usual interval (3rd per
    /// chord quality, b7 for 7).
    pub fn degree(&self, d: i32) -> i32 {
        let class = (d - 1).rem_euclid(7) + 1; // 1..7
        let oct = (d - 1).div_euclid(7) * 12;
        let has = |opts: &[i32]| opts.iter().copied().find(|o| self.intervals.iter().any(|i| i.rem_euclid(12) == *o));
        let semis = match class {
            1 => 0,
            2 => has(&[2, 1, 3]).unwrap_or(2),
            3 => has(&[4, 3]).or_else(|| has(&[5, 2])).unwrap_or(4),
            4 => has(&[5, 6]).unwrap_or(5),
            5 => has(&[7, 6, 8]).unwrap_or(7),
            6 => has(&[9, 8]).unwrap_or(9),
            _ => has(&[10, 11, 9]).unwrap_or(10),
        };
        semis + oct
    }
}

/// Voices `chord` as a closed chord near `octave` (root in that octave when
/// there is no previous chord), choosing the inversion closest to `prev`
/// for smooth voice leading. A slash bass is added below.
pub fn voice_chord(chord: &Chord, octave: i32, prev: &[i32]) -> Vec<i32> {
    let mut ints: Vec<i32> = chord.intervals.clone();
    // extensions above the octave are folded into the closed voicing
    let pcs: Vec<i32> = {
        let mut v: Vec<i32> = Vec::new();
        for i in ints.drain(..) {
            let pc = (chord.root + i).rem_euclid(12);
            if !v.contains(&pc) {
                v.push(pc);
            }
        }
        v
    };
    let n = pcs.len();
    let base = (octave + 1) * 12;
    let centre = base + chord.root + 5; // middle of a root-position chord
    let prev_upper: Vec<i32> = if prev.len() > n { prev[prev.len() - n..].to_vec() } else { prev.to_vec() };
    let mut best: Option<(f64, Vec<i32>)> = None;
    for inv in 0..n {
        for shift in -1..=1 {
            let mut v = Vec::with_capacity(n);
            let mut p = base + shift * 12 + pcs[inv];
            v.push(p);
            for k in 1..n {
                let pc = pcs[(inv + k) % n];
                p += (pc - p).rem_euclid(12);
                if (pc - p).rem_euclid(12) == 0 && v.last() == Some(&p) {
                    p += 12;
                }
                v.push(p);
            }
            let mean = v.iter().sum::<i32>() as f64 / n as f64;
            let cost = if prev_upper.is_empty() {
                // root position preferred, near the requested octave
                inv as f64 * 3.0 + (mean - centre as f64).abs()
            } else if prev_upper.len() == n {
                v.iter().zip(&prev_upper).map(|(a, b)| (a - b).abs() as f64).sum::<f64>() + 0.3 * (mean - centre as f64).abs()
            } else {
                let pm = prev_upper.iter().sum::<i32>() as f64 / prev_upper.len() as f64;
                (mean - pm).abs() * n as f64 + 0.3 * (mean - centre as f64).abs()
            };
            if best.as_ref().map(|b| cost < b.0).unwrap_or(true) {
                best = Some((cost, v));
            }
        }
    }
    let mut v = best.unwrap().1;
    if let Some(b) = chord.bass {
        let low = v[0];
        let mut bn = low - (low - b).rem_euclid(12);
        if bn == low {
            bn -= 12;
        }
        v.insert(0, bn);
    }
    v.retain(|m| (0..=127).contains(m));
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pitches() {
        assert_eq!(parse_pitch("C4"), Some(60));
        assert_eq!(parse_pitch("A4"), Some(69));
        assert_eq!(parse_pitch("Bb3"), Some(58));
        assert_eq!(parse_pitch("F#2"), Some(42));
        assert_eq!(parse_pitch("C-1"), Some(0));
        assert_eq!(note_name(61), "C#4");
        assert_eq!(freq_note_name(440.0), "A4");
    }

    #[test]
    fn chords() {
        let c = Chord::parse("Am7", None).unwrap();
        assert_eq!(c.pcs(), vec![9, 0, 4, 7]);
        let c = Chord::parse("Cmaj7/G", None).unwrap();
        assert_eq!((c.root, c.bass), (0, Some(7)));
        let k = Key::parse("A minor").unwrap();
        assert_eq!(Chord::parse("VI", Some(&k)).unwrap().pcs(), vec![5, 9, 0]);
        assert_eq!(Chord::parse("iv", Some(&k)).unwrap().pcs(), vec![2, 5, 9]);
        assert_eq!(Chord::parse("V7", Some(&k)).unwrap().pcs(), vec![4, 8, 11, 2]);
        assert_eq!(Chord::parse("bVII", Some(&Key::parse("C").unwrap())).unwrap().root, 10);
        assert!(Chord::parse("Cxyz", None).is_err());
        assert_eq!(c.degree(5), 7);
        assert_eq!(Chord::parse("Am", None).unwrap().degree(3), 3);
        assert_eq!(Chord::parse("Am", None).unwrap().degree(10), 15);
    }

    #[test]
    fn voice_leading_is_smooth() {
        let c = Chord::parse("C", None).unwrap();
        let v1 = voice_chord(&c, 4, &[]);
        assert_eq!(v1, vec![60, 64, 67]);
        let f = Chord::parse("F", None).unwrap();
        let v2 = voice_chord(&f, 4, &v1);
        // C-E-G → C-F-A (second inversion of F), not F-A-C up a fourth
        assert_eq!(v2, vec![60, 65, 69]);
    }
}
