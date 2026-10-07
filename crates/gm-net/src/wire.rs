//! Byte-level encoding of the gm81-clean multiplayer protocol (specs/multiplayer.md
//! Part B §B.2, §B.9).

use std::io;

/// Protocol version carried in DISCOVER / LIST_SESSIONS / HELLO.
pub const PROTO_VERSION: u16 = 1;
/// UDP datagram magic.
pub const UDP_MAGIC: &[u8; 4] = b"GMMP";
/// HELLO magic.
pub const HELLO_MAGIC: &[u8; 6] = b"GM81MP";
/// Largest allowed TCP frame body (type + payload).
pub const MAX_FRAME: usize = 1_048_576;
/// Largest UDP datagram we send.
pub const MAX_DATAGRAM: usize = 1200;
/// Largest name string.
pub const MAX_NAME: usize = 65_536;
/// Largest message/data string value.
pub const MAX_VALUE_STR: usize = 30_000;
/// Capability bit: UDP data channel.
pub const CAP_UDP_DATA: u32 = 1;

/// Message type codes (§B.9).
pub mod t {
    pub const DISCOVER: u8 = 0x01;
    pub const SESSION_INFO: u8 = 0x02;
    pub const UDP_BIND: u8 = 0x03;
    pub const UDP_BIND_ACK: u8 = 0x04;
    pub const HELLO: u8 = 0x10;
    pub const WELCOME: u8 = 0x11;
    pub const REJECT: u8 = 0x12;
    pub const PLAYER_JOINED: u8 = 0x13;
    pub const PLAYER_LEFT: u8 = 0x14;
    pub const PEER_ADDR: u8 = 0x15;
    pub const RESUME: u8 = 0x16;
    pub const LIST_SESSIONS: u8 = 0x20;
    pub const SESSION_LIST: u8 = 0x21;
    pub const MSG_SEND: u8 = 0x30;
    pub const MSG_DELIVER: u8 = 0x31;
    pub const DATA_SET: u8 = 0x40;
    pub const DATA_UPDATE: u8 = 0x41;
    pub const PING: u8 = 0x50;
    pub const PONG: u8 = 0x51;
    pub const BYE: u8 = 0x52;
    pub const SESSION_CLOSED: u8 = 0x53;
    pub const MIGRATE: u8 = 0x54;
}

/// REJECT reasons (§B.4).
pub mod reject {
    pub const FULL: u8 = 1;
    pub const WRONG_GAME: u8 = 2;
    pub const VERSION: u8 = 3;
    pub const CLOSING: u8 = 4;
    pub const MALFORMED: u8 = 5;
}

/// A shared-data slot value or message value: a real or a byte string.
#[derive(Clone, Debug, PartialEq)]
pub enum NetValue {
    Real(f64),
    Str(Vec<u8>),
}

impl Default for NetValue {
    fn default() -> Self {
        NetValue::Real(0.0)
    }
}

impl NetValue {
    /// Whether this is the initial value of every slot (real 0).
    pub fn is_zero(&self) -> bool {
        matches!(self, NetValue::Real(x) if *x == 0.0)
    }
}

/// Little-endian byte writer.
#[derive(Default, Clone, Debug)]
pub struct W(pub Vec<u8>);

impl W {
    pub fn new() -> W {
        W(Vec::new())
    }
    pub fn u8(&mut self, v: u8) -> &mut Self {
        self.0.push(v);
        self
    }
    pub fn u16(&mut self, v: u16) -> &mut Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    pub fn u32(&mut self, v: u32) -> &mut Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    pub fn i32(&mut self, v: i32) -> &mut Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    pub fn u64(&mut self, v: u64) -> &mut Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    pub fn f64(&mut self, v: f64) -> &mut Self {
        self.0.extend_from_slice(&v.to_le_bytes());
        self
    }
    pub fn bytes(&mut self, b: &[u8]) -> &mut Self {
        self.0.extend_from_slice(b);
        self
    }
    pub fn str(&mut self, s: &[u8]) -> &mut Self {
        self.u32(s.len() as u32);
        self.0.extend_from_slice(s);
        self
    }
    pub fn value(&mut self, v: &NetValue) -> &mut Self {
        match v {
            NetValue::Real(x) => {
                self.u8(0).f64(*x);
            }
            NetValue::Str(s) => {
                self.u8(1).str(s);
            }
        }
        self
    }
    /// Wraps the body as a TCP frame: `u32 length; u8 type; payload`.
    pub fn frame(ty: u8, payload: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(payload.len() + 5);
        out.extend_from_slice(&((payload.len() + 1) as u32).to_le_bytes());
        out.push(ty);
        out.extend_from_slice(payload);
        out
    }
    /// Wraps the body as a UDP datagram: `"GMMP"; u8 version; u8 type; payload`.
    pub fn datagram(ty: u8, payload: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(payload.len() + 6);
        out.extend_from_slice(UDP_MAGIC);
        out.push(PROTO_VERSION as u8);
        out.push(ty);
        out.extend_from_slice(payload);
        out
    }
}

/// Error for malformed input.
pub fn bad(what: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, what.to_string())
}

/// Little-endian byte reader with bounds checking.
pub struct R<'a> {
    pub b: &'a [u8],
    pub p: usize,
}

impl<'a> R<'a> {
    pub fn new(b: &'a [u8]) -> R<'a> {
        R { b, p: 0 }
    }
    pub fn take(&mut self, n: usize) -> io::Result<&'a [u8]> {
        if self.b.len() - self.p < n {
            return Err(bad("truncated"));
        }
        let s = &self.b[self.p..self.p + n];
        self.p += n;
        Ok(s)
    }
    pub fn u8(&mut self) -> io::Result<u8> {
        Ok(self.take(1)?[0])
    }
    pub fn u16(&mut self) -> io::Result<u16> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }
    pub fn u32(&mut self) -> io::Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    pub fn i32(&mut self) -> io::Result<i32> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    pub fn u64(&mut self) -> io::Result<u64> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    pub fn f64(&mut self) -> io::Result<f64> {
        Ok(f64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    /// A `str` of at most `max` bytes.
    pub fn str_max(&mut self, max: usize) -> io::Result<Vec<u8>> {
        let n = self.u32()? as usize;
        if n > max {
            return Err(bad("string too long"));
        }
        Ok(self.take(n)?.to_vec())
    }
    pub fn name(&mut self) -> io::Result<Vec<u8>> {
        self.str_max(MAX_NAME)
    }
    pub fn value(&mut self) -> io::Result<NetValue> {
        match self.u8()? {
            0 => Ok(NetValue::Real(self.f64()?)),
            1 => Ok(NetValue::Str(self.str_max(MAX_VALUE_STR * 4)?)),
            _ => Err(bad("bad value tag")),
        }
    }
    pub fn rest(&mut self) -> &'a [u8] {
        let s = &self.b[self.p..];
        self.p = self.b.len();
        s
    }
}

/// Parses a UDP datagram header; returns (type, payload) when magic and version match.
pub fn parse_datagram(d: &[u8]) -> Option<(u8, &[u8])> {
    if d.len() < 6 || &d[..4] != UDP_MAGIC || d[4] != PROTO_VERSION as u8 {
        return None;
    }
    Some((d[5], &d[6..]))
}

/// Body of a SESSION_INFO / SESSION_LIST entry.
#[derive(Clone, Debug, PartialEq)]
pub struct SessionInfo {
    pub token: u64,
    pub tcp_port: u16,
    pub name: Vec<u8>,
    pub host_name: Vec<u8>,
    pub player_count: u16,
    pub max_players: u16,
    pub accepting: bool,
}

impl SessionInfo {
    pub fn write(&self, w: &mut W) {
        w.u64(self.token)
            .u16(self.tcp_port)
            .str(&self.name)
            .str(&self.host_name)
            .u16(self.player_count)
            .u16(self.max_players)
            .u8(self.accepting as u8);
    }
    pub fn read(r: &mut R) -> io::Result<SessionInfo> {
        Ok(SessionInfo {
            token: r.u64()?,
            tcp_port: r.u16()?,
            name: r.name()?,
            host_name: r.name()?,
            player_count: r.u16()?,
            max_players: r.u16()?,
            accepting: r.u8()? != 0,
        })
    }
}

/// Incremental TCP frame splitter.
#[derive(Default)]
pub struct FrameBuf {
    buf: Vec<u8>,
}

impl FrameBuf {
    pub fn push(&mut self, data: &[u8]) {
        self.buf.extend_from_slice(data);
    }
    /// Next complete frame `(type, payload)`; `Err` on a protocol violation.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> io::Result<Option<(u8, Vec<u8>)>> {
        if self.buf.len() < 4 {
            return Ok(None);
        }
        let n = u32::from_le_bytes(self.buf[..4].try_into().unwrap()) as usize;
        if n == 0 || n > MAX_FRAME {
            return Err(bad("bad frame length"));
        }
        if self.buf.len() < 4 + n {
            return Ok(None);
        }
        let ty = self.buf[4];
        let payload = self.buf[5..4 + n].to_vec();
        self.buf.drain(..4 + n);
        Ok(Some((ty, payload)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_split() {
        let mut w = W::new();
        w.u32(7).value(&NetValue::Str(b"hi".to_vec()));
        let f = W::frame(t::MSG_SEND, &w.0);
        let mut fb = FrameBuf::default();
        fb.push(&f[..3]);
        assert!(fb.next().unwrap().is_none());
        fb.push(&f[3..]);
        fb.push(&f);
        let (ty, p) = fb.next().unwrap().unwrap();
        assert_eq!(ty, t::MSG_SEND);
        let mut r = R::new(&p);
        assert_eq!(r.u32().unwrap(), 7);
        assert_eq!(r.value().unwrap(), NetValue::Str(b"hi".to_vec()));
        assert!(fb.next().unwrap().is_some());
        assert!(fb.next().unwrap().is_none());
    }

    #[test]
    fn rejects_huge_frame() {
        let mut fb = FrameBuf::default();
        fb.push(&(MAX_FRAME as u32 + 1).to_le_bytes());
        assert!(fb.next().is_err());
    }
}
