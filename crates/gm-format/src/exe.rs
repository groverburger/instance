//! gm81-clean's own standalone-executable layout (specs/exe-format.md):
//! `[runner][payload (.gm81 v810)][bundle?][footer]`.
//!
//! * v1 footer, 24 bytes: `"GM81CLN\x01"`, u64 LE payload offset, u64 LE payload length.
//! * v2 footer, 40 bytes: `"GM81CLN\x02"`, payload offset/length, bundle offset/length.
//!
//! The bundle carries the extension packages used by the project:
//! u32 LE count, then per package u32-length-prefixed UTF-8 name, `.ged` bytes and
//! `.dat` bytes.

pub const MAGIC_V1: [u8; 8] = *b"GM81CLN\x01";
pub const MAGIC_V2: [u8; 8] = *b"GM81CLN\x02";
pub const FOOTER_V1_LEN: usize = 24;
pub const FOOTER_V2_LEN: usize = 40;

/// One extension package as installed (`<name>.ged` + `<name>.dat`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundledPackage {
    pub name: String,
    pub ged: Vec<u8>,
    pub dat: Vec<u8>,
}

/// Serialise an extension bundle.
pub fn write_bundle(packages: &[BundledPackage]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&(packages.len() as u32).to_le_bytes());
    for p in packages {
        for part in [p.name.as_bytes(), &p.ged, &p.dat] {
            out.extend_from_slice(&(part.len() as u32).to_le_bytes());
            out.extend_from_slice(part);
        }
    }
    out
}

/// Parse an extension bundle.
pub fn read_bundle(data: &[u8]) -> Option<Vec<BundledPackage>> {
    fn u32_at(data: &[u8], pos: usize) -> Option<usize> {
        let b = data.get(pos..pos.checked_add(4)?)?;
        Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
    }
    let rd = |pos: &mut usize| -> Option<Vec<u8>> {
        let n = u32_at(data, *pos)?;
        let s = data.get(*pos + 4..(*pos + 4).checked_add(n)?)?;
        *pos += 4 + n;
        Some(s.to_vec())
    };
    let count = u32_at(data, 0)?;
    let mut out = Vec::new();
    let mut p = 4usize;
    for _ in 0..count {
        let name = String::from_utf8(rd(&mut p)?).ok()?;
        let ged = rd(&mut p)?;
        let dat = rd(&mut p)?;
        out.push(BundledPackage { name, ged, dat });
    }
    Some(out)
}

/// Build a standalone executable: runner bytes, payload, optional bundle, footer.
/// A v2 footer is written when `bundle` is `Some`.
pub fn build(runner: &[u8], payload: &[u8], bundle: Option<&[u8]>) -> Vec<u8> {
    let mut out = Vec::with_capacity(runner.len() + payload.len() + FOOTER_V2_LEN);
    out.extend_from_slice(runner);
    let payload_off = out.len() as u64;
    out.extend_from_slice(payload);
    match bundle {
        None => {
            out.extend_from_slice(&MAGIC_V1);
            out.extend_from_slice(&payload_off.to_le_bytes());
            out.extend_from_slice(&(payload.len() as u64).to_le_bytes());
        }
        Some(b) => {
            let bundle_off = out.len() as u64;
            out.extend_from_slice(b);
            out.extend_from_slice(&MAGIC_V2);
            out.extend_from_slice(&payload_off.to_le_bytes());
            out.extend_from_slice(&(payload.len() as u64).to_le_bytes());
            out.extend_from_slice(&bundle_off.to_le_bytes());
            out.extend_from_slice(&(b.len() as u64).to_le_bytes());
        }
    }
    out
}

/// Data embedded in a standalone executable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Embedded {
    pub payload: Vec<u8>,
    /// Raw extension bundle (v2 footers only).
    pub bundle: Option<Vec<u8>>,
}

fn u64_at(b: &[u8], at: usize) -> u64 {
    let mut a = [0u8; 8];
    a.copy_from_slice(&b[at..at + 8]);
    u64::from_le_bytes(a)
}

fn slice(data: &[u8], off: u64, len: u64, limit: usize) -> Option<Vec<u8>> {
    let off = usize::try_from(off).ok()?;
    let end = off.checked_add(usize::try_from(len).ok()?)?;
    (end <= limit).then(|| data[off..end].to_vec())
}

/// Find the embedded game in the bytes of an executable (v1 or v2 footer).
pub fn read_embedded(data: &[u8]) -> Option<Embedded> {
    if data.len() >= FOOTER_V2_LEN {
        let f = &data[data.len() - FOOTER_V2_LEN..];
        if f[..8] == MAGIC_V2 {
            let limit = data.len() - FOOTER_V2_LEN;
            return Some(Embedded {
                payload: slice(data, u64_at(f, 8), u64_at(f, 16), limit)?,
                bundle: Some(slice(data, u64_at(f, 24), u64_at(f, 32), limit)?),
            });
        }
    }
    if data.len() >= FOOTER_V1_LEN {
        let f = &data[data.len() - FOOTER_V1_LEN..];
        if f[..8] == MAGIC_V1 {
            let limit = data.len() - FOOTER_V1_LEN;
            return Some(Embedded {
                payload: slice(data, u64_at(f, 8), u64_at(f, 16), limit)?,
                bundle: None,
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v1_round_trip() {
        let exe = build(b"MZrunner", b"payload!", None);
        let e = read_embedded(&exe).unwrap();
        assert_eq!(e.payload, b"payload!");
        assert_eq!(e.bundle, None);
        assert_eq!(exe.len(), 8 + 8 + FOOTER_V1_LEN);
    }

    #[test]
    fn v2_round_trip() {
        let pk = vec![BundledPackage {
            name: "Pkg One".into(),
            ged: vec![1, 2, 3],
            dat: vec![9; 10],
        }];
        let b = write_bundle(&pk);
        let exe = build(b"MZrunner", b"payload", Some(&b));
        let e = read_embedded(&exe).unwrap();
        assert_eq!(e.payload, b"payload");
        assert_eq!(read_bundle(e.bundle.as_deref().unwrap()).unwrap(), pk);
    }

    #[test]
    fn plain_files_have_no_payload() {
        assert_eq!(
            read_embedded(b"MZ just a runner with no footer at all......"),
            None
        );
        let mut bad = build(b"MZ", b"p", None);
        let n = bad.len();
        bad[n - 1] = 0xff; // absurd length
        assert_eq!(read_embedded(&bad), None);
    }
}
