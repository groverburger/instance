//! Exported single-resource files (spec §9). Only `.gmspr` (sprite) is verified.

use crate::error::{Error, Result};
use crate::gmk::{read_sprite_body, write_sprite_body, MAGIC};
use crate::io::{Reader, Writer};
use crate::model::Sprite;

/// Read a `.gmspr` exported sprite. The result has an empty name and `last_changed` 0.
pub fn read_sprite_export(data: &[u8]) -> Result<Sprite> {
    let mut r = Reader::new(data, "sprite export");
    let m = r.i32()?;
    if m != MAGIC {
        return Err(Error::BadMagic { found: m });
    }
    let s = r.zblock("sprite", |b| {
        b.version(&[800], "sprite export")?;
        read_sprite_body(b)
    })?;
    r.end()?;
    Ok(s)
}

/// Write a `.gmspr` exported sprite (name and timestamp are not stored).
pub fn write_sprite_export(s: &Sprite) -> Result<Vec<u8>> {
    let mut w = Writer::new();
    w.i32(MAGIC);
    w.zblock(|b| {
        b.i32(800);
        write_sprite_body(b, s)
    })?;
    Ok(w.buf)
}
