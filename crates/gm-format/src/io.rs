//! Low-level primitive reader/writer (spec §0.1).

use std::io::{Read, Write};

use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;

use crate::error::{Error, Result};
use crate::text::{decode_cp1252, encode_cp1252};

/// Cursor over a byte slice with GameMaker primitive decoders.
pub(crate) struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    /// Added to `pos` when reporting offsets (absolute offset of `data[0]`).
    base: usize,
    context: String,
    /// When set, every consumed byte is appended here and every zlib block is replaced
    /// by `u32 inflated_len` + inflated bytes (see [`crate::gmk::canonical_bytes`]).
    pub(crate) record: Option<Vec<u8>>,
}

impl<'a> Reader<'a> {
    pub fn new(data: &'a [u8], context: impl Into<String>) -> Self {
        Reader {
            data,
            pos: 0,
            base: 0,
            context: context.into(),
            record: None,
        }
    }

    pub fn with_base(mut self, base: usize) -> Self {
        self.base = base;
        self
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    /// Absolute offset of the cursor, for error reporting.
    pub fn offset(&self) -> usize {
        self.base + self.pos
    }

    pub fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    pub fn context(&self) -> &str {
        &self.context
    }

    fn take_raw(&mut self, n: usize) -> Result<&'a [u8]> {
        if n > self.remaining() {
            return Err(Error::UnexpectedEof {
                context: self.context.clone(),
                offset: self.offset(),
                needed: n,
                available: self.remaining(),
            });
        }
        let s = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }

    pub fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let s = self.take_raw(n)?;
        if let Some(r) = &mut self.record {
            r.extend_from_slice(s);
        }
        Ok(s)
    }

    pub fn i32(&mut self) -> Result<i32> {
        let b = self.take(4)?;
        Ok(i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub fn u32(&mut self) -> Result<u32> {
        Ok(self.i32()? as u32)
    }

    pub fn bool(&mut self) -> Result<bool> {
        Ok(self.i32()? != 0)
    }

    pub fn f64(&mut self) -> Result<f64> {
        let b = self.take(8)?;
        let mut a = [0u8; 8];
        a.copy_from_slice(b);
        Ok(f64::from_le_bytes(a))
    }

    /// A count (`i32`) that must be non-negative.
    pub fn count(&mut self, what: &str) -> Result<usize> {
        let at = self.offset();
        let n = self.i32()?;
        if n < 0 {
            return Err(self.invalid_at(at, format!("negative {what} count {n}")));
        }
        Ok(n as usize)
    }

    pub fn blob(&mut self) -> Result<Vec<u8>> {
        let n = self.u32()? as usize;
        Ok(self.take(n)?.to_vec())
    }

    pub fn string(&mut self) -> Result<String> {
        let n = self.u32()? as usize;
        Ok(decode_cp1252(self.take(n)?))
    }

    /// Read an `i32` that must equal `expected`.
    pub fn expect(&mut self, expected: i32, what: &'static str) -> Result<()> {
        let at = self.offset();
        let v = self.i32()?;
        if v != expected {
            return Err(Error::UnexpectedValue {
                context: self.context.clone(),
                offset: at,
                what,
                expected: expected as i64,
                found: v as i64,
            });
        }
        Ok(())
    }

    /// Read an `i32` version that must be one of `allowed`.
    pub fn version(&mut self, allowed: &[i32], what: &'static str) -> Result<i32> {
        let at = self.offset();
        let v = self.i32()?;
        if !allowed.contains(&v) {
            return Err(Error::UnsupportedVersion {
                context: self.context.clone(),
                offset: at,
                what,
                found: v,
            });
        }
        Ok(v)
    }

    pub fn invalid_at(&self, offset: usize, message: impl Into<String>) -> Error {
        Error::InvalidValue {
            context: self.context.clone(),
            offset,
            message: message.into(),
        }
    }

    /// Fail unless the whole buffer has been consumed.
    pub fn end(&self) -> Result<()> {
        if self.remaining() != 0 {
            return Err(Error::TrailingBytes {
                context: self.context.clone(),
                offset: self.offset(),
                remaining: self.remaining(),
            });
        }
        Ok(())
    }

    /// Read a `zblock`, inflate it and parse it with `f`, which must consume it exactly.
    pub fn zblock<T>(
        &mut self,
        name: &str,
        f: impl FnOnce(&mut Reader<'_>) -> Result<T>,
    ) -> Result<T> {
        let at = self.offset();
        let len_bytes = self.take_raw(4)?;
        let n =
            u32::from_le_bytes([len_bytes[0], len_bytes[1], len_bytes[2], len_bytes[3]]) as usize;
        let payload = self.take_raw(n)?;
        let ctx = format!("{name} (zlib block at {at:#x})");
        let inflated = inflate(payload).map_err(|message| Error::Zlib {
            context: ctx.clone(),
            offset: at,
            message,
        })?;
        if let Some(r) = &mut self.record {
            r.extend_from_slice(&(inflated.len() as u32).to_le_bytes());
            r.extend_from_slice(&inflated);
        }
        let mut sub = Reader::new(&inflated, ctx);
        let v = f(&mut sub)?;
        sub.end()?;
        Ok(v)
    }
}

pub(crate) fn inflate(data: &[u8]) -> std::result::Result<Vec<u8>, String> {
    let mut out = Vec::new();
    ZlibDecoder::new(data)
        .read_to_end(&mut out)
        .map_err(|e| e.to_string())?;
    Ok(out)
}

pub(crate) fn deflate(data: &[u8]) -> Vec<u8> {
    let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
    enc.write_all(data).expect("writing to Vec cannot fail");
    enc.finish().expect("writing to Vec cannot fail")
}

/// Growable output buffer with GameMaker primitive encoders.
#[derive(Default)]
pub(crate) struct Writer {
    pub buf: Vec<u8>,
}

impl Writer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn i32(&mut self, v: i32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub fn u32(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub fn bool(&mut self, v: bool) {
        self.i32(v as i32);
    }

    pub fn f64(&mut self, v: f64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub fn count(&mut self, n: usize) -> Result<()> {
        let v =
            i32::try_from(n).map_err(|_| Error::InvalidModel(format!("count {n} too large")))?;
        self.i32(v);
        Ok(())
    }

    pub fn blob(&mut self, b: &[u8]) {
        self.u32(b.len() as u32);
        self.buf.extend_from_slice(b);
    }

    pub fn string(&mut self, s: &str) {
        self.blob(&encode_cp1252(s));
    }

    pub fn raw(&mut self, b: &[u8]) {
        self.buf.extend_from_slice(b);
    }

    /// Write a `zblock` whose plain contents are produced by `f`.
    pub fn zblock(&mut self, f: impl FnOnce(&mut Writer) -> Result<()>) -> Result<()> {
        let mut sub = Writer::new();
        f(&mut sub)?;
        let z = deflate(&sub.buf);
        self.blob(&z);
        Ok(())
    }
}
