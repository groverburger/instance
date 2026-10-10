//! General MIDI tables: the 128 programs, the channel-10 drum map, drum kits
//! and typical playing ranges (for `info` warnings).

/// GM program names, index = program number - 1 (GM numbers are 1-based).
pub const PROGRAMS: [&str; 128] = [
    "Acoustic Grand Piano", "Bright Acoustic Piano", "Electric Grand Piano", "Honky-tonk Piano",
    "Electric Piano 1", "Electric Piano 2", "Harpsichord", "Clavinet",
    "Celesta", "Glockenspiel", "Music Box", "Vibraphone", "Marimba", "Xylophone", "Tubular Bells", "Dulcimer",
    "Drawbar Organ", "Percussive Organ", "Rock Organ", "Church Organ", "Reed Organ", "Accordion", "Harmonica", "Tango Accordion",
    "Acoustic Guitar (nylon)", "Acoustic Guitar (steel)", "Electric Guitar (jazz)", "Electric Guitar (clean)",
    "Electric Guitar (muted)", "Overdriven Guitar", "Distortion Guitar", "Guitar Harmonics",
    "Acoustic Bass", "Electric Bass (finger)", "Electric Bass (pick)", "Fretless Bass",
    "Slap Bass 1", "Slap Bass 2", "Synth Bass 1", "Synth Bass 2",
    "Violin", "Viola", "Cello", "Contrabass", "Tremolo Strings", "Pizzicato Strings", "Orchestral Harp", "Timpani",
    "String Ensemble 1", "String Ensemble 2", "Synth Strings 1", "Synth Strings 2", "Choir Aahs", "Voice Oohs", "Synth Voice", "Orchestra Hit",
    "Trumpet", "Trombone", "Tuba", "Muted Trumpet", "French Horn", "Brass Section", "Synth Brass 1", "Synth Brass 2",
    "Soprano Sax", "Alto Sax", "Tenor Sax", "Baritone Sax", "Oboe", "English Horn", "Bassoon", "Clarinet",
    "Piccolo", "Flute", "Recorder", "Pan Flute", "Blown Bottle", "Shakuhachi", "Whistle", "Ocarina",
    "Lead 1 (square)", "Lead 2 (sawtooth)", "Lead 3 (calliope)", "Lead 4 (chiff)", "Lead 5 (charang)", "Lead 6 (voice)", "Lead 7 (fifths)", "Lead 8 (bass + lead)",
    "Pad 1 (new age)", "Pad 2 (warm)", "Pad 3 (polysynth)", "Pad 4 (choir)", "Pad 5 (bowed)", "Pad 6 (metallic)", "Pad 7 (halo)", "Pad 8 (sweep)",
    "FX 1 (rain)", "FX 2 (soundtrack)", "FX 3 (crystal)", "FX 4 (atmosphere)", "FX 5 (brightness)", "FX 6 (goblins)", "FX 7 (echoes)", "FX 8 (sci-fi)",
    "Sitar", "Banjo", "Shamisen", "Koto", "Kalimba", "Bagpipe", "Fiddle", "Shanai",
    "Tinkle Bell", "Agogo", "Steel Drums", "Woodblock", "Taiko Drum", "Melodic Tom", "Synth Drum", "Reverse Cymbal",
    "Guitar Fret Noise", "Breath Noise", "Seashore", "Bird Tweet", "Telephone Ring", "Helicopter", "Applause", "Gunshot",
];

/// Extra short aliases (normalised form → 1-based program).
const ALIASES: &[(&str, u8)] = &[
    ("piano", 1), ("grandpiano", 1), ("brightpiano", 2), ("epiano", 5), ("rhodes", 5), ("epiano2", 6),
    ("organ", 17), ("churchorgan", 20), ("nylonguitar", 25), ("guitar", 25), ("steelguitar", 26),
    ("jazzguitar", 27), ("cleanguitar", 28), ("mutedguitar", 29), ("overdrive", 30), ("distortion", 31),
    ("bass", 34), ("fingerbass", 34), ("fingeredbass", 34), ("pickbass", 35), ("fretless", 36),
    ("slapbass", 37), ("synthbass", 39), ("synthbass2", 40), ("strings", 49), ("harp", 47), ("pizzicato", 46),
    ("tremolo", 45), ("slowstrings", 50), ("synthstrings", 51), ("choir", 53), ("aahs", 53), ("oohs", 54),
    ("horn", 61), ("frenchhorn", 61), ("brass", 62), ("synthbrass", 63), ("sax", 66), ("altosax", 66),
    ("tenorsax", 67), ("square", 81), ("squarelead", 81), ("saw", 82), ("sawlead", 82), ("calliope", 83),
    ("chiff", 84), ("charang", 85), ("newage", 89), ("warmpad", 90), ("pad", 90), ("polysynth", 91),
    ("choirpad", 92), ("bowedpad", 93), ("metallicpad", 94), ("halo", 95), ("halopad", 95), ("sweep", 96),
    ("sweeppad", 96), ("rain", 97), ("soundtrack", 98), ("crystal", 99), ("atmosphere", 100),
    ("brightness", 101), ("goblins", 102), ("echoes", 103), ("scifi", 104), ("steeldrum", 115),
    ("taiko", 117), ("synthdrum", 119),
];

fn norm(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_alphanumeric()).map(|c| c.to_ascii_lowercase()).collect()
}

/// Resolves an instrument: a 1-based GM number (`1`..`128`) or a name
/// (case/punctuation-insensitive: `Pad 2 (warm)`, `pad_2_warm`, `warm pad`,
/// `strings`). Returns the 0-based program.
pub fn program(s: &str) -> Result<u8, String> {
    if let Ok(n) = s.trim().parse::<u32>() {
        return if (1..=128).contains(&n) { Ok(n as u8 - 1) } else { Err(format!("instrument number {n} out of range 1..128 (GM numbers are 1-based)")) };
    }
    let n = norm(s);
    if n.is_empty() {
        return Err("empty instrument name".into());
    }
    if let Some(i) = PROGRAMS.iter().position(|p| norm(p) == n) {
        return Ok(i as u8);
    }
    if let Some((_, p)) = ALIASES.iter().find(|(a, _)| *a == n) {
        return Ok(p - 1);
    }
    // all words present (in any order), e.g. "warm pad" → "Pad 2 (warm)"
    let words: Vec<String> = s.split(|c: char| !c.is_ascii_alphanumeric()).filter(|w| !w.is_empty()).map(norm).collect();
    let hits: Vec<usize> = (0..128).filter(|&i| {
        let p = norm(PROGRAMS[i]);
        words.iter().all(|w| p.contains(w.as_str()))
    }).collect();
    match hits.len() {
        1 => Ok(hits[0] as u8),
        0 => Err(format!("unknown instrument '{s}' (use a GM name like \"String Ensemble 1\" or a number 1..128; see README)")),
        _ => Err(format!(
            "ambiguous instrument '{s}': {}",
            hits.iter().take(8).map(|&i| format!("{} ({})", PROGRAMS[i], i + 1)).collect::<Vec<_>>().join(", ")
        )),
    }
}

/// Channel-10 drum sounds: name → GM key. First name of each key is canonical.
pub const DRUMS: &[(&str, u8)] = &[
    ("kick", 36), ("bd", 36), ("kick2", 35), ("rim", 37), ("sidestick", 37), ("snare", 38), ("sd", 38),
    ("clap", 39), ("snare2", 40), ("tom6", 41), ("floortom_lo", 41), ("hat", 42), ("hh", 42),
    ("tom5", 43), ("floortom", 43), ("phat", 44), ("pedalhat", 44), ("tom4", 45), ("lowtom", 45),
    ("ohat", 46), ("openhat", 46), ("oh", 46), ("tom3", 47), ("midtom", 47), ("tom2", 48),
    ("crash", 49), ("tom1", 50), ("hitom", 50), ("ride", 51), ("china", 52), ("bell", 53),
    ("ridebell", 53), ("tamb", 54), ("tambourine", 54), ("splash", 55), ("cowbell", 56), ("crash2", 57),
    ("vibraslap", 58), ("ride2", 59), ("bongo_hi", 60), ("bongo_lo", 61), ("conga_mute", 62),
    ("conga_hi", 63), ("conga_lo", 64), ("timbale_hi", 65), ("timbale_lo", 66), ("agogo_hi", 67),
    ("agogo_lo", 68), ("cabasa", 69), ("maracas", 70), ("whistle", 71), ("whistle_long", 72),
    ("guiro", 73), ("guiro_long", 74), ("claves", 75), ("wood_hi", 76), ("wood_lo", 77),
    ("cuica_mute", 78), ("cuica", 79), ("triangle_mute", 80), ("triangle", 81), ("shaker", 82),
    ("jingle", 83), ("belltree", 84), ("castanets", 85), ("surdo_mute", 86), ("surdo", 87),
    ("sticks", 31), ("click", 33), ("metronome", 34),
];

/// GM key of a drum name (case-insensitive).
pub fn drum(name: &str) -> Option<u8> {
    let l = name.to_ascii_lowercase();
    DRUMS.iter().find(|(n, _)| *n == l).map(|(_, k)| *k)
}

/// Canonical drum name of a key, if any.
pub fn drum_name(key: u8) -> Option<&'static str> {
    DRUMS.iter().find(|(_, k)| *k == key).map(|(n, _)| *n)
}

/// Drum kits (program on channel 10, GS numbering), 0-based program.
pub const KITS: &[(&str, u8)] = &[
    ("standard", 0), ("room", 8), ("power", 16), ("electronic", 24), ("tr808", 25), ("808", 25),
    ("jazz", 32), ("brush", 40), ("orchestra", 48),
];

/// Typical range (lowest, highest MIDI note) of a 0-based program.
pub fn range(p: u8) -> (i32, i32) {
    match p {
        0..=7 => (21, 108),
        8 => (60, 108),
        9 => (77, 108),
        10 => (60, 96),
        11 => (53, 89),
        12 => (45, 96),
        13 => (65, 108),
        14 => (60, 77),
        15 => (48, 96),
        16..=23 => (36, 96),
        24..=31 => (40, 88),
        32..=39 => (28, 67),
        40 => (55, 103),
        41 => (48, 91),
        42 => (36, 76),
        43 => (28, 60),
        44 | 45 | 48..=51 => (28, 103),
        46 => (23, 103),
        47 => (40, 57),
        52..=54 => (40, 84),
        55 => (36, 96),
        56 | 59 => (54, 86),
        57 => (40, 72),
        58 => (28, 58),
        60 => (41, 77),
        61..=63 => (36, 84),
        64 => (56, 88),
        65 => (49, 81),
        66 => (44, 76),
        67 => (36, 69),
        68 => (58, 91),
        69 => (52, 81),
        70 => (34, 72),
        71 => (50, 94),
        72 => (74, 108),
        73..=79 => (60, 96),
        80..=103 => (36, 96),
        104..=111 => (48, 84),
        112..=119 => (36, 84),
        _ => (0, 127),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn programs_resolve() {
        assert_eq!(program("1"), Ok(0));
        assert_eq!(program("Pad 2 (warm)"), Ok(89));
        assert_eq!(program("pad_2_warm"), Ok(89));
        assert_eq!(program("warm pad"), Ok(89));
        assert_eq!(program("String Ensemble 1"), Ok(48));
        assert_eq!(program("strings"), Ok(48));
        assert_eq!(program("Fretless Bass"), Ok(35));
        assert!(program("guitar electric").is_err()); // ambiguous
        assert!(program("kazoo").is_err());
        assert_eq!(drum("Kick"), Some(36));
        assert_eq!(drum_name(46), Some("ohat"));
    }
}
