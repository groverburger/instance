//! A non-blocking framed TCP connection with keep-alive (§B.2, §B.7).

use crate::wire::{t, FrameBuf, R, W};
use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpStream};
use std::time::{Duration, Instant};

pub(crate) struct Conn {
    pub s: TcpStream,
    pub peer: SocketAddr,
    rx: FrameBuf,
    tx: Vec<u8>,
    pub last_recv: Instant,
    last_send: Instant,
    seq: u32,
    pub dead: bool,
}

impl Conn {
    pub fn new(s: TcpStream) -> std::io::Result<Conn> {
        s.set_nonblocking(true)?;
        let _ = s.set_nodelay(true);
        let peer = s.peer_addr()?;
        let now = Instant::now();
        Ok(Conn {
            s,
            peer,
            rx: FrameBuf::default(),
            tx: Vec::new(),
            last_recv: now,
            last_send: now,
            seq: 0,
            dead: false,
        })
    }

    /// Takes over bytes already read past the last frame (from a blocking handshake).
    pub fn with_leftover(mut self, fb: FrameBuf) -> Conn {
        self.rx = fb;
        self
    }

    pub fn send(&mut self, ty: u8, payload: &[u8]) {
        if self.dead {
            return;
        }
        self.tx.extend_from_slice(&W::frame(ty, payload));
        self.last_send = Instant::now();
        self.flush();
    }

    /// Writes as much of the outgoing buffer as the socket accepts.
    pub fn flush(&mut self) {
        while !self.tx.is_empty() && !self.dead {
            match self.s.write(&self.tx) {
                Ok(0) => self.dead = true,
                Ok(n) => {
                    self.tx.drain(..n);
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                Err(e) if e.kind() == ErrorKind::Interrupted => {}
                Err(_) => self.dead = true,
            }
        }
    }

    /// Reads everything available and returns complete frames. PING is answered and
    /// PONG swallowed here.
    pub fn poll(&mut self) -> Vec<(u8, Vec<u8>)> {
        let mut out = Vec::new();
        if self.dead {
            return out;
        }
        let mut buf = [0u8; 16384];
        loop {
            match self.s.read(&mut buf) {
                Ok(0) => {
                    self.dead = true;
                    break;
                }
                Ok(n) => {
                    self.rx.push(&buf[..n]);
                    self.last_recv = Instant::now();
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                Err(e) if e.kind() == ErrorKind::Interrupted => {}
                Err(_) => {
                    self.dead = true;
                    break;
                }
            }
        }
        loop {
            match self.rx.next() {
                Ok(Some((ty, p))) => match ty {
                    t::PING => {
                        let seq = R::new(&p).u32().unwrap_or(0);
                        let mut w = W::new();
                        w.u32(seq);
                        self.send(t::PONG, &w.0);
                    }
                    t::PONG => {}
                    _ => out.push((ty, p)),
                },
                Ok(None) => break,
                Err(_) => {
                    self.dead = true;
                    break;
                }
            }
        }
        out
    }

    /// Sends PING when idle and declares the peer gone after `timeout` of silence.
    pub fn keepalive(&mut self, interval: Duration, timeout: Duration) {
        if self.dead {
            return;
        }
        if self.last_send.elapsed() >= interval {
            self.seq = self.seq.wrapping_add(1);
            let mut w = W::new();
            w.u32(self.seq);
            self.send(t::PING, &w.0);
        }
        if self.last_recv.elapsed() >= timeout {
            self.dead = true;
        }
    }

    /// Flushes pending output (waiting up to `limit`) and closes the connection.
    pub fn close(&mut self, limit: Duration) {
        let end = Instant::now() + limit;
        while !self.tx.is_empty() && !self.dead && Instant::now() < end {
            self.flush();
            if !self.tx.is_empty() {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
        let _ = self.s.shutdown(Shutdown::Write);
        // drain briefly so the peer sees an orderly close rather than a reset
        let mut buf = [0u8; 4096];
        let end = Instant::now() + Duration::from_millis(50).min(limit);
        while Instant::now() < end {
            match self.s.read(&mut buf) {
                Ok(0) => break,
                Ok(_) => {}
                Err(e) if e.kind() == ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(1)),
                Err(_) => break,
            }
        }
        self.dead = true;
    }
}
