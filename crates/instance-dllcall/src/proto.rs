//! `instance-dllhost32` helper protocol (specs/extensions.md §6.2).
//!
//! Transport: the helper's stdin (requests) and stdout (replies) as binary pipes.
//! Every frame is `u32 length` (bytes after this field) + `u8 op` + payload, all
//! little-endian; strings are `u32 len` + bytes. Op codes (gm81-clean's choice):
//!
//! | op | request payload | reply payload (OK) |
//! |---|---|---|
//! | 0x01 LOAD | `str path` | `u32 handle` |
//! | 0x02 DEFINE | `u32 handle; str symbol; u8 conv (0 cdecl, 1 stdcall); u8 restype (0 real, 1 string); u8 argc; argc × u8 type` | `u32 fn_id` |
//! | 0x03 CALL | `u32 fn_id; u8 argc; argc × value` | `value` |
//! | 0x04 FREE | `u32 handle` | — |
//! | 0x05 SET_CWD | `str path` | — |
//! | 0x06 QUIT | — | (no reply; the helper exits) |
//!
//! `value` = `u8 tag` (0 real → `f64`, 1 string → `str`). Replies use op 0x80 (OK) or
//! 0x81 (ERROR, payload `str message`).

use std::io::{self, Read, Write};

pub const LOAD: u8 = 0x01;
pub const DEFINE: u8 = 0x02;
pub const CALL: u8 = 0x03;
pub const FREE: u8 = 0x04;
pub const SET_CWD: u8 = 0x05;
pub const QUIT: u8 = 0x06;
pub const OK: u8 = 0x80;
pub const ERROR: u8 = 0x81;

/// Largest frame accepted.
pub const MAX_FRAME: usize = 64 << 20;

/// A value crossing the pipe.
#[derive(Clone, Debug, PartialEq)]
pub enum PVal {
    Real(f64),
    Str(Vec<u8>),
}

/// Payload builder.
#[derive(Default)]
pub struct Enc(pub Vec<u8>);

impl Enc {
    pub fn u8(mut self, v: u8) -> Self {
        self.0.push(v);
        self
    }
    pub fn u32(mut self, v: u32) -> Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    pub fn f64(mut self, v: f64) -> Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    pub fn str(mut self, s: &[u8]) -> Self {
        self.0.extend_from_slice(&(s.len() as u32).to_le_bytes());
        self.0.extend_from_slice(s);
        self
    }
    pub fn val(self, v: &PVal) -> Self {
        match v {
            PVal::Real(x) => self.u8(0).f64(*x),
            PVal::Str(s) => self.u8(1).str(s),
        }
    }
}

fn bad() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "malformed helper frame")
}

/// Payload reader.
pub struct Dec<'a> {
    b: &'a [u8],
    p: usize,
}

impl<'a> Dec<'a> {
    pub fn new(b: &'a [u8]) -> Self {
        Dec { b, p: 0 }
    }
    fn take(&mut self, n: usize) -> io::Result<&'a [u8]> {
        if self.b.len() - self.p < n {
            return Err(bad());
        }
        let s = &self.b[self.p..self.p + n];
        self.p += n;
        Ok(s)
    }
    pub fn u8(&mut self) -> io::Result<u8> {
        Ok(self.take(1)?[0])
    }
    pub fn u32(&mut self) -> io::Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    pub fn f64(&mut self) -> io::Result<f64> {
        Ok(f64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    pub fn str(&mut self) -> io::Result<Vec<u8>> {
        let n = self.u32()? as usize;
        Ok(self.take(n)?.to_vec())
    }
    pub fn val(&mut self) -> io::Result<PVal> {
        match self.u8()? {
            0 => Ok(PVal::Real(self.f64()?)),
            1 => Ok(PVal::Str(self.str()?)),
            _ => Err(bad()),
        }
    }
}

/// Writes one frame and flushes.
pub fn write_frame(w: &mut impl Write, op: u8, payload: &[u8]) -> io::Result<()> {
    let mut b = Vec::with_capacity(payload.len() + 5);
    b.extend_from_slice(&((payload.len() + 1) as u32).to_le_bytes());
    b.push(op);
    b.extend_from_slice(payload);
    w.write_all(&b)?;
    w.flush()
}

/// Reads one frame `(op, payload)`.
pub fn read_frame(r: &mut impl Read) -> io::Result<(u8, Vec<u8>)> {
    let mut l = [0u8; 4];
    r.read_exact(&mut l)?;
    let n = u32::from_le_bytes(l) as usize;
    if n == 0 || n > MAX_FRAME {
        return Err(bad());
    }
    let mut b = vec![0u8; n];
    r.read_exact(&mut b)?;
    let op = b[0];
    b.remove(0);
    Ok((op, b))
}
