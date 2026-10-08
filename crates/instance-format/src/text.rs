//! Windows-1252 text conversion.
//!
//! All strings in GameMaker files are ANSI bytes (spec §0.1); this crate decodes them as
//! Windows-1252. The five bytes that are undefined in Windows-1252 (0x81, 0x8D, 0x8F,
//! 0x90, 0x9D) decode to the C1 control code points of the same value, so decoding and
//! re-encoding any byte string is lossless. Characters that cannot be represented are
//! encoded as `?`.

const HIGH: [u16; 32] = [
    0x20AC, 0x0081, 0x201A, 0x0192, 0x201E, 0x2026, 0x2020, 0x2021, 0x02C6, 0x2030, 0x0160, 0x2039,
    0x0152, 0x008D, 0x017D, 0x008F, 0x0090, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022, 0x2013, 0x2014,
    0x02DC, 0x2122, 0x0161, 0x203A, 0x0153, 0x009D, 0x017E, 0x0178,
];

/// Decode Windows-1252 bytes into a `String` (lossless, see module docs).
pub fn decode_cp1252(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|&b| match b {
            0x80..=0x9F => char::from_u32(HIGH[(b - 0x80) as usize] as u32).unwrap_or('?'),
            _ => b as char,
        })
        .collect()
}

/// Encode a string as Windows-1252; unrepresentable characters become `?`.
pub fn encode_cp1252(s: &str) -> Vec<u8> {
    s.chars()
        .map(|c| {
            let u = c as u32;
            if u < 0x80 || (0xA0..=0xFF).contains(&u) {
                u as u8
            } else if let Some(i) = HIGH.iter().position(|&h| h as u32 == u) {
                0x80 + i as u8
            } else {
                b'?'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_bytes_round_trip() {
        let all: Vec<u8> = (0..=255u8).collect();
        assert_eq!(encode_cp1252(&decode_cp1252(&all)), all);
    }

    #[test]
    fn known_chars() {
        assert_eq!(decode_cp1252(&[0x80, b'a', 0xE9]), "\u{20AC}a\u{E9}");
        assert_eq!(encode_cp1252("\u{2014}\u{4E00}"), vec![0x97, b'?']);
    }
}
