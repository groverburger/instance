//! The background session thread: host (server) and client roles, host migration
//! (specs/multiplayer.md §B.4–§B.8).
//!
//! The game thread talks to this thread only through channels: [`Cmd`] in, [`Event`]
//! out. All sockets are non-blocking and polled in one loop per role.
//!
//! UDP data-channel framing details not fixed by the spec (documented here):
//! client → host datagrams of type MSG_SEND / DATA_SET carry the prefix
//! `u32 udp_cookie; u32 sender_id`; host → client datagrams of type MSG_DELIVER /
//! DATA_UPDATE carry the prefix `u32 udp_cookie` (the recipient's cookie).

use crate::conn::Conn;
use crate::wire::{reject, t, FrameBuf, NetValue, SessionInfo, CAP_UDP_DATA, HELLO_MAGIC, PROTO_VERSION, R, W};
use crate::Config;
use std::collections::{HashMap, HashSet};
use std::io::{ErrorKind, Read, Write};
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::time::{Duration, Instant};

/// Number of client data/message datagrams accepted by hosts in this process.
pub static UDP_DATA_IN: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Hard cap on players per session (§B.10).
pub const PLAYER_CAP: usize = 256;
/// Highest shared-data index.
pub const MAX_DATA_INDEX: u32 = 1_000_000;

/// Game → network thread.
#[derive(Debug)]
pub(crate) enum Cmd {
    Msg { to: u32, guaranteed: bool, id: f64, val: NetValue },
    Data { index: u32, val: NetValue, guaranteed: bool },
    End,
    /// Test hook: drop every connection without a goodbye (simulated crash).
    Crash,
}

/// Network thread → game.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    PlayerJoined(u32, Vec<u8>),
    PlayerLeft(u32),
    Msg { from: u32, id: f64, val: NetValue },
    Data(u32, NetValue),
    /// The host changed (migration). `me` = this game is now the host.
    HostChanged { host_id: u32, me: bool },
    /// The session is over for this game.
    Ended,
}

#[derive(Clone, Debug)]
pub(crate) struct Player {
    pub id: u32,
    pub name: Vec<u8>,
    /// Address as seen by the host (from PEER_ADDR), used for migration.
    pub ip: Option<IpAddr>,
    pub port_hint: u16,
}

/// Session state shared by both roles (and carried across migration).
pub(crate) struct State {
    pub cfg: Config,
    pub game_id: i32,
    pub token: u64,
    pub name: Vec<u8>,
    pub max_players: u16,
    pub my_id: u32,
    pub host_id: u32,
    /// Join order, host first.
    pub players: Vec<Player>,
    pub data: HashMap<u32, NetValue>,
    pub next_id: u32,
    pub move_mode: bool,
}

impl State {
    fn player_name(&self, id: u32) -> Vec<u8> {
        self.players.iter().find(|p| p.id == id).map(|p| p.name.clone()).unwrap_or_default()
    }
    fn set_data(&mut self, index: u32, val: NetValue) {
        if val.is_zero() {
            self.data.remove(&index);
        } else {
            self.data.insert(index, val);
        }
    }
}

pub(crate) fn random_u64() -> u64 {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    let mut h = RandomState::new().build_hasher();
    h.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0));
    h.write_u32(std::process::id());
    h.finish()
}

/// Opens the host's TCP listener on `base..base+9` (§B.1).
pub(crate) fn bind_listener(base: u16) -> Option<TcpListener> {
    for k in 0..10u16 {
        let port = base.wrapping_add(k);
        if let Ok(l) = TcpListener::bind(("0.0.0.0", port)) {
            l.set_nonblocking(true).ok()?;
            return Some(l);
        }
    }
    None
}

/// Opens the host's UDP socket (discovery + data) with address reuse (§B.3).
pub(crate) fn bind_host_udp(port: u16) -> Option<UdpSocket> {
    use socket2::{Domain, Protocol, Socket, Type};
    let s = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP)).ok()?;
    s.set_reuse_address(true).ok()?;
    let _ = s.set_broadcast(true);
    let addr: SocketAddr = ([0, 0, 0, 0], port).into();
    s.bind(&addr.into()).ok()?;
    s.set_nonblocking(true).ok()?;
    Some(s.into())
}

// ====================================================================== host

struct Client {
    id: u32,
    conn: Conn,
    udp_cookie: u32,
    udp_addr: Option<SocketAddr>,
}

pub(crate) struct Host {
    pub st: State,
    pub listener: TcpListener,
    pub udp: Option<UdpSocket>,
    clients: Vec<Client>,
    pending: Vec<(Conn, Instant)>,
    /// Slots last changed over UDP → time of that broadcast (§B.6 re-broadcast).
    udp_dirty: HashMap<u32, Instant>,
    /// After migration: players expected to RESUME, and the deadline.
    resuming: Option<(HashSet<u32>, Instant)>,
    closing: bool,
}

impl Host {
    pub fn new(st: State, listener: TcpListener, udp: Option<UdpSocket>) -> Host {
        Host {
            st,
            listener,
            udp,
            clients: Vec::new(),
            pending: Vec::new(),
            udp_dirty: HashMap::new(),
            resuming: None,
            closing: false,
        }
    }

    fn tcp_port(&self) -> u16 {
        self.listener.local_addr().map(|a| a.port()).unwrap_or(self.st.cfg.port)
    }

    fn cap(&self) -> usize {
        if self.st.max_players == 0 {
            PLAYER_CAP
        } else {
            (self.st.max_players as usize).min(PLAYER_CAP)
        }
    }

    fn info(&self) -> SessionInfo {
        let host_name = self.st.player_name(self.st.my_id);
        SessionInfo {
            token: self.st.token,
            tcp_port: self.tcp_port(),
            name: self.st.name.clone(),
            host_name,
            player_count: self.st.players.len() as u16,
            max_players: self.st.max_players,
            accepting: !self.closing && self.st.players.len() < self.cap() && self.resuming.is_none(),
        }
    }

    fn welcome(&self, id: u32, caps: u32, cookie: u32) -> Vec<u8> {
        let mut w = W::new();
        w.u32(id).u32(self.st.my_id).u64(self.st.token).str(&self.st.name).u16(self.st.max_players).u32(caps).u32(cookie);
        w.u16(self.st.players.len() as u16);
        for p in &self.st.players {
            w.u32(p.id).str(&p.name);
        }
        w.u32(self.st.data.len() as u32);
        let mut keys: Vec<_> = self.st.data.keys().copied().collect();
        keys.sort_unstable();
        for k in keys {
            w.u32(k).value(&self.st.data[&k]);
        }
        w.0
    }

    fn peer_addr_frame(p: &Player) -> Vec<u8> {
        let mut w = W::new();
        let ip = p.ip.map(|i| i.to_string()).unwrap_or_default();
        w.u32(p.id).str(ip.as_bytes()).u16(p.port_hint);
        w.0
    }

    fn broadcast_tcp(&mut self, ty: u8, payload: &[u8], except: u32) {
        for c in &mut self.clients {
            if c.id != except {
                c.conn.send(ty, payload);
            }
        }
    }

    /// Sends to one client over UDP if bound and `!guaranteed`, else TCP.
    fn send_to(&mut self, idx: usize, ty: u8, payload: &[u8], guaranteed: bool) -> bool {
        let c = &mut self.clients[idx];
        if !guaranteed {
            if let (Some(addr), Some(udp)) = (c.udp_addr, &self.udp) {
                let mut w = W::new();
                w.u32(c.udp_cookie).bytes(payload);
                let d = W::datagram(ty, &w.0);
                if d.len() <= crate::wire::MAX_DATAGRAM && udp.send_to(&d, addr).is_ok() {
                    return true;
                }
            }
        }
        c.conn.send(ty, payload);
        false
    }

    fn deliver(&mut self, from: u32, to: u32, id: f64, val: NetValue, guaranteed: bool, ev: &Sender<Event>) {
        let mut w = W::new();
        w.u32(from).f64(id).value(&val);
        if to == 0 || to == self.st.my_id {
            if from != self.st.my_id {
                let _ = ev.send(Event::Msg { from, id, val: val.clone() });
            }
        }
        for i in 0..self.clients.len() {
            let cid = self.clients[i].id;
            if (to == 0 && cid != from) || to == cid {
                self.send_to(i, t::MSG_DELIVER, &w.0, guaranteed);
            }
        }
    }

    fn apply_data(&mut self, index: u32, val: NetValue, guaranteed: bool, ev: &Sender<Event>) {
        if index > MAX_DATA_INDEX {
            return;
        }
        self.st.set_data(index, val.clone());
        let _ = ev.send(Event::Data(index, val.clone()));
        let mut w = W::new();
        w.u32(index).value(&val);
        let mut any_udp = false;
        for i in 0..self.clients.len() {
            any_udp |= self.send_to(i, t::DATA_UPDATE, &w.0, guaranteed);
        }
        if any_udp {
            self.udp_dirty.insert(index, Instant::now());
        }
    }

    fn remove_client(&mut self, id: u32, ev: &Sender<Event>) {
        self.clients.retain(|c| c.id != id);
        if let Some(pos) = self.st.players.iter().position(|p| p.id == id) {
            self.st.players.remove(pos);
            let _ = ev.send(Event::PlayerLeft(id));
            let mut w = W::new();
            w.u32(id);
            self.broadcast_tcp(t::PLAYER_LEFT, &w.0, id);
        }
    }

    /// Handles the first frame of a not-yet-identified connection. Returns the
    /// connection back when it should stay pending.
    fn handle_pending(&mut self, mut conn: Conn, ty: u8, p: &[u8], ev: &Sender<Event>) {
        let mut r = R::new(p);
        match ty {
            t::LIST_SESSIONS => {
                let ok = (|| -> std::io::Result<bool> { Ok(r.i32()? == self.st.game_id && r.u16()? == PROTO_VERSION) })().unwrap_or(false);
                let mut w = W::new();
                if ok {
                    w.u16(1);
                    self.info().write(&mut w);
                } else {
                    w.u16(0);
                }
                conn.send(t::SESSION_LIST, &w.0);
                conn.close(Duration::from_millis(200));
            }
            t::HELLO => {
                let parsed = (|| -> std::io::Result<(u16, i32, u64, u32, Vec<u8>)> {
                    if r.take(6)? != HELLO_MAGIC {
                        return Err(crate::wire::bad("magic"));
                    }
                    Ok((r.u16()?, r.i32()?, r.u64()?, r.u32()?, r.name()?))
                })();
                let reason = match &parsed {
                    Err(_) => Some(reject::MALFORMED),
                    Ok((v, _, _, _, _)) if *v != PROTO_VERSION => Some(reject::VERSION),
                    Ok((_, g, _, _, _)) if *g != self.st.game_id => Some(reject::WRONG_GAME),
                    Ok((_, _, tok, _, _)) if *tok != 0 && *tok != self.st.token => Some(reject::CLOSING),
                    _ if self.closing || self.resuming.is_some() => Some(reject::CLOSING),
                    _ if self.st.players.len() >= self.cap() => Some(reject::FULL),
                    _ => None,
                };
                if let Some(reason) = reason {
                    conn.send(t::REJECT, &[reason]);
                    conn.close(Duration::from_millis(200));
                    return;
                }
                let (_, _, _, caps, name) = parsed.unwrap();
                let id = self.st.next_id;
                self.st.next_id += 1;
                let agreed = if self.udp.is_some() && self.st.cfg.use_udp { caps & CAP_UDP_DATA } else { 0 };
                let cookie = if agreed & CAP_UDP_DATA != 0 { (random_u64() as u32) | 1 } else { 0 };
                let player = Player {
                    id,
                    name: name.clone(),
                    ip: Some(conn.peer.ip()),
                    port_hint: self.st.cfg.port,
                };
                self.st.players.push(player.clone());
                conn.send(t::WELCOME, &self.welcome(id, agreed, cookie));
                let mut w = W::new();
                w.u32(id).str(&name);
                self.broadcast_tcp(t::PLAYER_JOINED, &w.0, id);
                if self.st.move_mode {
                    let f = Self::peer_addr_frame(&player);
                    self.broadcast_tcp(t::PEER_ADDR, &f, id);
                    for q in self.st.players.iter().filter(|q| q.id != self.st.my_id) {
                        conn.send(t::PEER_ADDR, &Self::peer_addr_frame(q));
                    }
                }
                let _ = ev.send(Event::PlayerJoined(id, name));
                self.clients.push(Client {
                    id,
                    conn,
                    udp_cookie: cookie,
                    udp_addr: None,
                });
            }
            t::RESUME => {
                let parsed = (|| -> std::io::Result<(u64, u32)> { Ok((r.u64()?, r.u32()?)) })();
                let ok = match (&parsed, &self.resuming) {
                    (Ok((tok, id)), Some((set, _))) => *tok == self.st.token && set.contains(id),
                    _ => false,
                };
                if !ok {
                    conn.send(t::REJECT, &[reject::CLOSING]);
                    conn.close(Duration::from_millis(200));
                    return;
                }
                let (_, id) = parsed.unwrap();
                if let Some((set, _)) = &mut self.resuming {
                    set.remove(&id);
                    if set.is_empty() {
                        self.resuming = None;
                    }
                }
                if let Some(p) = self.st.players.iter_mut().find(|p| p.id == id) {
                    p.ip = Some(conn.peer.ip());
                }
                let cookie = 0; // the UDP channel is not re-established after migration
                conn.send(t::WELCOME, &self.welcome(id, 0, cookie));
                if self.st.move_mode {
                    let me = self.st.players.iter().find(|p| p.id == id).cloned();
                    for q in self.st.players.iter().filter(|q| q.id != self.st.my_id) {
                        conn.send(t::PEER_ADDR, &Self::peer_addr_frame(q));
                    }
                    if let Some(me) = me {
                        let f = Self::peer_addr_frame(&me);
                        self.broadcast_tcp(t::PEER_ADDR, &f, id);
                    }
                }
                self.clients.push(Client {
                    id,
                    conn,
                    udp_cookie: 0,
                    udp_addr: None,
                });
            }
            _ => conn.close(Duration::from_millis(0)),
        }
    }

    fn handle_udp(&mut self, d: &[u8], from: SocketAddr, ev: &Sender<Event>) {
        let Some((ty, p)) = crate::wire::parse_datagram(d) else { return };
        let mut r = R::new(p);
        match ty {
            t::DISCOVER => {
                let Ok((nonce, gid, ver)) = (|| -> std::io::Result<_> { Ok((r.u32()?, r.i32()?, r.u16()?)) })() else { return };
                if gid != self.st.game_id || ver != PROTO_VERSION {
                    return;
                }
                let mut w = W::new();
                w.u32(nonce);
                self.info().write(&mut w);
                if let Some(u) = &self.udp {
                    let _ = u.send_to(&W::datagram(t::SESSION_INFO, &w.0), from);
                }
            }
            t::UDP_BIND => {
                let Ok((cookie, id)) = (|| -> std::io::Result<_> { Ok((r.u32()?, r.u32()?)) })() else { return };
                if let Some(c) = self.clients.iter_mut().find(|c| c.id == id && c.udp_cookie == cookie && cookie != 0) {
                    c.udp_addr = Some(from);
                    if let Some(u) = &self.udp {
                        let _ = u.send_to(&W::datagram(t::UDP_BIND_ACK, p), from);
                    }
                }
            }
            t::MSG_SEND | t::DATA_SET => {
                let Ok((cookie, id)) = (|| -> std::io::Result<_> { Ok((r.u32()?, r.u32()?)) })() else { return };
                if !self.clients.iter().any(|c| c.id == id && c.udp_cookie == cookie && cookie != 0) {
                    return;
                }
                UDP_DATA_IN.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                self.handle_client_frame(id, ty, r.rest(), false, ev);
            }
            _ => {}
        }
    }

    fn handle_client_frame(&mut self, from: u32, ty: u8, p: &[u8], via_tcp: bool, ev: &Sender<Event>) {
        let mut r = R::new(p);
        match ty {
            t::MSG_SEND => {
                let Ok((to, g, id, val)) = (|| -> std::io::Result<_> { Ok((r.u32()?, r.u8()?, r.f64()?, r.value()?)) })() else { return };
                self.deliver(from, to, id, val, g != 0 && via_tcp, ev);
            }
            t::DATA_SET => {
                let Ok((index, val)) = (|| -> std::io::Result<_> { Ok((r.u32()?, r.value()?)) })() else { return };
                self.apply_data(index, val, via_tcp, ev);
            }
            t::BYE => {
                if let Some(c) = self.clients.iter_mut().find(|c| c.id == from) {
                    c.conn.dead = true;
                }
            }
            _ => {}
        }
    }

    /// Runs the host until the session ends. Returns normally in every case.
    pub fn run(mut self, cmds: &Receiver<Cmd>, ev: &Sender<Event>) {
        let mut buf = vec![0u8; 2048];
        loop {
            let mut busy = false;
            // new connections
            loop {
                match self.listener.accept() {
                    Ok((s, _)) => {
                        if let Ok(c) = Conn::new(s) {
                            self.pending.push((c, Instant::now()));
                        }
                        busy = true;
                    }
                    Err(_) => break,
                }
            }
            // pending handshakes
            let pend = std::mem::take(&mut self.pending);
            for (mut c, since) in pend {
                let frames = c.poll();
                if let Some((ty, p)) = frames.into_iter().next() {
                    busy = true;
                    self.handle_pending(c, ty, &p, ev);
                } else if !c.dead && since.elapsed() < Duration::from_secs(5) {
                    self.pending.push((c, since));
                }
            }
            // UDP
            if self.udp.is_some() {
                loop {
                    let r = self.udp.as_ref().unwrap().recv_from(&mut buf);
                    match r {
                        Ok((n, from)) => {
                            busy = true;
                            let d = buf[..n].to_vec();
                            self.handle_udp(&d, from, ev);
                        }
                        Err(_) => break,
                    }
                }
            }
            // client traffic
            let mut i = 0;
            while i < self.clients.len() {
                let frames = self.clients[i].conn.poll();
                let id = self.clients[i].id;
                for (ty, p) in frames {
                    busy = true;
                    self.handle_client_frame(id, ty, &p, true, ev);
                }
                i += 1;
            }
            let dead: Vec<u32> = self.clients.iter().filter(|c| c.conn.dead).map(|c| c.id).collect();
            for id in dead {
                self.remove_client(id, ev);
            }
            // commands from the game
            loop {
                match cmds.try_recv() {
                    Ok(Cmd::Msg { to, guaranteed, id, val }) => {
                        busy = true;
                        let me = self.st.my_id;
                        self.deliver(me, to, id, val, guaranteed, ev);
                    }
                    Ok(Cmd::Data { index, val, guaranteed }) => {
                        busy = true;
                        self.apply_data(index, val, guaranteed, ev);
                    }
                    Ok(Cmd::End) => {
                        self.shutdown(0);
                        return;
                    }
                    Ok(Cmd::Crash) => {
                        for c in &self.clients {
                            let _ = c.conn.s.shutdown(std::net::Shutdown::Both);
                        }
                        return;
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        self.shutdown(1);
                        return;
                    }
                }
            }
            // re-broadcast slots changed over UDP (§B.6)
            if !self.udp_dirty.is_empty() {
                let due: Vec<u32> = self.udp_dirty.iter().filter(|(_, t0)| t0.elapsed() >= Duration::from_secs(1)).map(|(k, _)| *k).collect();
                for k in due {
                    self.udp_dirty.remove(&k);
                    let v = self.st.data.get(&k).cloned().unwrap_or_default();
                    let mut w = W::new();
                    w.u32(k).value(&v);
                    self.broadcast_tcp(t::DATA_UPDATE, &w.0, 0);
                }
            }
            // migration: drop players that did not come back (§B.8)
            if let Some((set, deadline)) = &self.resuming {
                if Instant::now() >= *deadline {
                    let gone: Vec<u32> = set.iter().copied().collect();
                    self.resuming = None;
                    for id in gone {
                        self.remove_client(id, ev);
                    }
                }
            }
            for c in &mut self.clients {
                c.conn.keepalive(self.st.cfg.ping_interval, self.st.cfg.peer_timeout);
                c.conn.flush();
            }
            if !busy {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    }

    /// Ends hosting: hands the session over (migration mode) or closes it.
    fn shutdown(&mut self, reason: u8) {
        self.closing = true;
        let next = self.clients.iter().map(|c| c.id).min();
        match (self.st.move_mode, next) {
            (true, Some(new_host)) => {
                let mut w = W::new();
                w.u32(new_host);
                self.broadcast_tcp(t::MIGRATE, &w.0, 0);
            }
            _ => self.broadcast_tcp(t::SESSION_CLOSED, &[reason], 0),
        }
        for c in &mut self.clients {
            c.conn.close(Duration::from_millis(500));
        }
    }

    /// Makes this host expect the other players to RESUME (after migration).
    pub fn expect_resume(&mut self, ids: HashSet<u32>, within: Duration) {
        if !ids.is_empty() {
            self.resuming = Some((ids, Instant::now() + within));
        }
    }
}

// ====================================================================== client

pub(crate) struct UdpChan {
    pub sock: UdpSocket,
    pub host: SocketAddr,
    pub cookie: u32,
    pub bound: bool,
    pub last_bind: Option<Instant>,
}

pub(crate) struct Client2 {
    pub st: State,
    pub conn: Conn,
    pub udp: Option<UdpChan>,
    /// Own non-guaranteed writes awaiting their echo: index → (time, value).
    pub own_udp: HashMap<u32, (Instant, NetValue)>,
    pub saw_peer_addr: bool,
}

/// What a client loop ended with.
pub(crate) enum ClientExit {
    /// Left or session closed: report Ended (already sent when `ended_sent`).
    Done,
    /// Host handed over (MIGRATE) or vanished with migration enabled.
    Migrate(u32),
}

impl Client2 {
    fn handle(&mut self, ty: u8, p: &[u8], ev: &Sender<Event>) -> Option<ClientExit> {
        let mut r = R::new(p);
        match ty {
            t::PLAYER_JOINED => {
                if let Ok((id, name)) = (|| -> std::io::Result<_> { Ok((r.u32()?, r.name()?)) })() {
                    if !self.st.players.iter().any(|q| q.id == id) && self.st.players.len() < PLAYER_CAP {
                        self.st.players.push(Player { id, name: name.clone(), ip: None, port_hint: self.st.cfg.port });
                        self.st.next_id = self.st.next_id.max(id + 1);
                        let _ = ev.send(Event::PlayerJoined(id, name));
                    }
                }
            }
            t::PLAYER_LEFT => {
                if let Ok(id) = r.u32() {
                    if let Some(pos) = self.st.players.iter().position(|q| q.id == id) {
                        self.st.players.remove(pos);
                        let _ = ev.send(Event::PlayerLeft(id));
                    }
                }
            }
            t::PEER_ADDR => {
                if let Ok((id, ip, port)) = (|| -> std::io::Result<_> { Ok((r.u32()?, r.name()?, r.u16()?)) })() {
                    self.saw_peer_addr = true;
                    if let Some(q) = self.st.players.iter_mut().find(|q| q.id == id) {
                        q.ip = String::from_utf8_lossy(&ip).parse().ok();
                        q.port_hint = port;
                    }
                }
            }
            t::MSG_DELIVER => {
                if let Ok((from, id, val)) = (|| -> std::io::Result<_> { Ok((r.u32()?, r.f64()?, r.value()?)) })() {
                    let _ = ev.send(Event::Msg { from, id, val });
                }
            }
            t::DATA_UPDATE => {
                if let Ok((index, val)) = (|| -> std::io::Result<_> { Ok((r.u32()?, r.value()?)) })() {
                    if index <= MAX_DATA_INDEX {
                        self.own_udp.remove(&index);
                        self.st.set_data(index, val.clone());
                        let _ = ev.send(Event::Data(index, val));
                    }
                }
            }
            t::SESSION_CLOSED => {
                self.conn.close(Duration::from_millis(0));
                return Some(ClientExit::Done);
            }
            t::MIGRATE => {
                if let Ok(id) = r.u32() {
                    self.conn.close(Duration::from_millis(0));
                    self.st.move_mode = true;
                    return Some(ClientExit::Migrate(id));
                }
            }
            _ => {}
        }
        None
    }

    fn send_nonguaranteed(&mut self, ty: u8, payload: &[u8]) {
        if let Some(u) = &self.udp {
            if u.bound {
                let mut w = W::new();
                w.u32(u.cookie).u32(self.st.my_id).bytes(payload);
                let d = W::datagram(ty, &w.0);
                if d.len() <= crate::wire::MAX_DATAGRAM && u.sock.send_to(&d, u.host).is_ok() {
                    return;
                }
            }
        }
        self.conn.send(ty, payload);
    }

    pub fn run(mut self, cmds: &Receiver<Cmd>, ev: &Sender<Event>) -> (ClientExit, State) {
        let mut buf = vec![0u8; 2048];
        loop {
            let mut busy = false;
            for (ty, p) in self.conn.poll() {
                busy = true;
                if let Some(x) = self.handle(ty, &p, ev) {
                    return (x, self.st);
                }
            }
            if self.conn.dead {
                // host vanished (§A.7, §B.8)
                if self.st.move_mode || self.saw_peer_addr {
                    let host = self.st.host_id;
                    let next = self.st.players.iter().filter(|q| q.id != host).map(|q| q.id).min();
                    if let Some(n) = next {
                        self.st.move_mode = true;
                        return (ClientExit::Migrate(n), self.st);
                    }
                }
                return (ClientExit::Done, self.st);
            }
            // UDP channel
            let mut udp_frames = Vec::new();
            if let Some(u) = &mut self.udp {
                if !u.bound && u.last_bind.map(|t0| t0.elapsed() >= Duration::from_millis(500)).unwrap_or(true) {
                    let mut w = W::new();
                    w.u32(u.cookie).u32(self.st.my_id);
                    let _ = u.sock.send_to(&W::datagram(t::UDP_BIND, &w.0), u.host);
                    u.last_bind = Some(Instant::now());
                }
                while let Ok((n, _)) = u.sock.recv_from(&mut buf) {
                    busy = true;
                    let Some((ty, p)) = crate::wire::parse_datagram(&buf[..n]) else { continue };
                    let mut r = R::new(p);
                    if r.u32().ok() != Some(u.cookie) {
                        continue;
                    }
                    match ty {
                        t::UDP_BIND_ACK => u.bound = true,
                        t::MSG_DELIVER | t::DATA_UPDATE => udp_frames.push((ty, r.rest().to_vec())),
                        _ => {}
                    }
                }
            }
            for (ty, p) in udp_frames {
                self.handle(ty, &p, ev);
            }
            // own UDP writes that were never echoed: resend reliably
            if !self.own_udp.is_empty() {
                let due: Vec<u32> = self.own_udp.iter().filter(|(_, (t0, _))| t0.elapsed() >= Duration::from_secs(2)).map(|(k, _)| *k).collect();
                for k in due {
                    if let Some((_, v)) = self.own_udp.remove(&k) {
                        let mut w = W::new();
                        w.u32(k).value(&v);
                        self.conn.send(t::DATA_SET, &w.0);
                    }
                }
            }
            loop {
                match cmds.try_recv() {
                    Ok(Cmd::Msg { to, guaranteed, id, val }) => {
                        busy = true;
                        let mut w = W::new();
                        w.u32(to).u8(guaranteed as u8).f64(id).value(&val);
                        if guaranteed {
                            self.conn.send(t::MSG_SEND, &w.0);
                        } else {
                            self.send_nonguaranteed(t::MSG_SEND, &w.0);
                        }
                    }
                    Ok(Cmd::Data { index, val, guaranteed }) => {
                        busy = true;
                        self.st.set_data(index, val.clone());
                        let mut w = W::new();
                        w.u32(index).value(&val);
                        if guaranteed {
                            self.own_udp.remove(&index);
                            self.conn.send(t::DATA_SET, &w.0);
                        } else {
                            if self.udp.as_ref().map(|u| u.bound).unwrap_or(false) {
                                self.own_udp.insert(index, (Instant::now(), val));
                            }
                            self.send_nonguaranteed(t::DATA_SET, &w.0);
                        }
                    }
                    Ok(Cmd::Crash) => {
                        let _ = self.conn.s.shutdown(std::net::Shutdown::Both);
                        return (ClientExit::Done, self.st);
                    }
                    Ok(Cmd::End) | Err(TryRecvError::Disconnected) => {
                        self.conn.send(t::BYE, &[]);
                        self.conn.close(Duration::from_millis(500));
                        return (ClientExit::Done, self.st);
                    }
                    Err(TryRecvError::Empty) => break,
                }
            }
            self.conn.keepalive(self.st.cfg.ping_interval, self.st.cfg.peer_timeout);
            self.conn.flush();
            if !busy {
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    }
}

// ====================================================================== joining

/// Contents of a WELCOME frame.
pub(crate) struct Welcome {
    pub my_id: u32,
    pub host_id: u32,
    pub token: u64,
    pub name: Vec<u8>,
    pub max_players: u16,
    pub caps: u32,
    pub cookie: u32,
    pub players: Vec<(u32, Vec<u8>)>,
    pub data: Vec<(u32, NetValue)>,
}

/// Result of a successful blocking handshake.
pub(crate) struct Joined {
    pub conn: Conn,
    pub w: Welcome,
}

/// Connects, sends `hello` (a complete frame) and waits for WELCOME/REJECT.
pub(crate) fn handshake(addr: SocketAddr, hello: &[u8], deadline: Instant) -> Option<Joined> {
    let left = deadline.checked_duration_since(Instant::now())?;
    let mut s = TcpStream::connect_timeout(&addr, left.max(Duration::from_millis(1))).ok()?;
    let _ = s.set_nodelay(true);
    s.write_all(hello).ok()?;
    let mut fb = FrameBuf::default();
    let mut buf = [0u8; 16384];
    loop {
        if let Some((ty, p)) = fb.next().ok()? {
            match ty {
                t::WELCOME => {
                    let w = parse_welcome(&p).ok()?;
                    let conn = Conn::new(s).ok()?.with_leftover(fb);
                    return Some(Joined { conn, w });
                }
                _ => return None, // REJECT or garbage
            }
        }
        let left = deadline.checked_duration_since(Instant::now())?;
        s.set_read_timeout(Some(left.max(Duration::from_millis(1)))).ok()?;
        match s.read(&mut buf) {
            Ok(0) => return None,
            Ok(n) => fb.push(&buf[..n]),
            Err(e) if e.kind() == ErrorKind::Interrupted => {}
            Err(_) => return None,
        }
    }
}

fn parse_welcome(p: &[u8]) -> std::io::Result<Welcome> {
    let mut r = R::new(p);
    let my_id = r.u32()?;
    let host_id = r.u32()?;
    let token = r.u64()?;
    let name = r.name()?;
    let max_players = r.u16()?;
    let caps = r.u32()?;
    let cookie = r.u32()?;
    let n = r.u16()? as usize;
    if n > PLAYER_CAP {
        return Err(crate::wire::bad("too many players"));
    }
    let mut players = Vec::with_capacity(n);
    for _ in 0..n {
        players.push((r.u32()?, r.name()?));
    }
    let nd = r.u32()? as usize;
    if nd > MAX_DATA_INDEX as usize + 1 {
        return Err(crate::wire::bad("too much data"));
    }
    let mut data = Vec::new();
    for _ in 0..nd {
        data.push((r.u32()?, r.value()?));
    }
    Ok(Welcome {
        my_id,
        host_id,
        token,
        name,
        max_players,
        caps,
        cookie,
        players,
        data,
    })
}

/// Builds a HELLO frame.
pub(crate) fn hello_frame(game_id: i32, token: u64, caps: u32, name: &[u8]) -> Vec<u8> {
    let mut w = W::new();
    w.bytes(HELLO_MAGIC).u16(PROTO_VERSION).i32(game_id).u64(token).u32(caps).str(name);
    W::frame(t::HELLO, &w.0)
}

// ====================================================================== thread

/// The session thread body: runs the current role and handles migration.
pub(crate) enum Role {
    Host(Host),
    Client(Client2),
}

pub(crate) fn run(mut role: Role, cmds: Receiver<Cmd>, ev: Sender<Event>) {
    loop {
        match role {
            Role::Host(h) => {
                h.run(&cmds, &ev);
                let _ = ev.send(Event::Ended);
                return;
            }
            Role::Client(c) => {
                let (exit, st) = c.run(&cmds, &ev);
                match exit {
                    ClientExit::Done => {
                        let _ = ev.send(Event::Ended);
                        return;
                    }
                    ClientExit::Migrate(new_host) => match migrate(st, new_host, &ev) {
                        Some(r) => role = r,
                        None => {
                            let _ = ev.send(Event::Ended);
                            return;
                        }
                    },
                }
            }
        }
    }
}

/// Host migration (§B.8): become the new host or reconnect to it.
fn migrate(mut st: State, new_host: u32, ev: &Sender<Event>) -> Option<Role> {
    let old = st.host_id;
    if let Some(pos) = st.players.iter().position(|q| q.id == old) {
        st.players.remove(pos);
        let _ = ev.send(Event::PlayerLeft(old));
    }
    st.next_id = st.next_id.max(st.players.iter().map(|q| q.id).max().unwrap_or(0) + 1);
    st.host_id = new_host;
    let window = st.cfg.migrate_timeout;
    if new_host == st.my_id {
        let listener = bind_listener(st.cfg.port)?;
        let udp = bind_host_udp(st.cfg.port);
        let others: HashSet<u32> = st.players.iter().map(|q| q.id).filter(|&id| id != st.my_id).collect();
        let _ = ev.send(Event::HostChanged { host_id: new_host, me: true });
        let mut h = Host::new(st, listener, udp);
        h.expect_resume(others, window);
        return Some(Role::Host(h));
    }
    let target = st.players.iter().find(|q| q.id == new_host)?.clone();
    let ip = target.ip?;
    let mut w = W::new();
    w.u64(st.token).u32(st.my_id);
    let frame = W::frame(t::RESUME, &w.0);
    let deadline = Instant::now() + window;
    while Instant::now() < deadline {
        for k in 0..10u16 {
            let addr = SocketAddr::new(ip, target.port_hint.wrapping_add(k));
            let d = (Instant::now() + Duration::from_millis(500)).min(deadline);
            if let Some(Joined { conn, w: j }) = handshake(addr, &frame, d) {
                if j.token != st.token {
                    continue;
                }
                // resync players and data with the new host
                let new_ids: HashSet<u32> = j.players.iter().map(|p| p.0).collect();
                for q in st.players.iter().filter(|q| !new_ids.contains(&q.id)) {
                    let _ = ev.send(Event::PlayerLeft(q.id));
                }
                let mut players = Vec::new();
                for (id, name) in &j.players {
                    let prev = st.players.iter().find(|q| q.id == *id).cloned();
                    if prev.is_none() {
                        let _ = ev.send(Event::PlayerJoined(*id, name.clone()));
                    }
                    players.push(prev.unwrap_or(Player { id: *id, name: name.clone(), ip: None, port_hint: st.cfg.port }));
                }
                st.players = players;
                let new_data: HashMap<u32, NetValue> = j.data.into_iter().collect();
                for (k, v) in &new_data {
                    if st.data.get(k) != Some(v) {
                        let _ = ev.send(Event::Data(*k, v.clone()));
                    }
                }
                for k in st.data.keys() {
                    if !new_data.contains_key(k) {
                        let _ = ev.send(Event::Data(*k, NetValue::Real(0.0)));
                    }
                }
                st.data = new_data;
                st.host_id = j.host_id;
                let _ = ev.send(Event::HostChanged { host_id: j.host_id, me: false });
                return Some(Role::Client(Client2 {
                    st,
                    conn,
                    udp: None,
                    own_udp: HashMap::new(),
                    saw_peer_addr: true,
                }));
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    None
}
