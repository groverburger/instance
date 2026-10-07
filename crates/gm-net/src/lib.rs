//! gm81-clean multiplayer: the `mplay_*` behaviour of specs/multiplayer.md Part A over
//! gm81-clean's own TCP/UDP protocol (Part B). Not compatible with GameMaker's
//! DirectPlay sessions.
//!
//! [`Mplay`] is the game-side object. It never blocks except where the spec allows
//! (`session_find`, `session_join`, and briefly while ending a session). All socket
//! work for an active session runs on a background thread; the game side exchanges
//! commands and events with it over channels and applies inbound events whenever an
//! `mplay_*` function is called (or [`Mplay::pump`] is called by the runner).

mod conn;
pub mod discovery;
mod session;
pub mod wire;

pub use discovery::Found;
pub use session::{Event, MAX_DATA_INDEX, PLAYER_CAP};
pub use wire::NetValue;

use session::{Client2, Cmd, Host, Player, Role, State, UdpChan};
use std::collections::{HashMap, VecDeque};
use std::net::{SocketAddr, UdpSocket};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Default port for TCP sessions and UDP discovery/data (§B.1).
pub const DEFAULT_PORT: u16 = 40810;

/// Tunables (defaults follow the spec; tests shorten them).
#[derive(Clone, Debug)]
pub struct Config {
    /// Base port: UDP discovery/data and the first TCP port tried by hosts.
    pub port: u16,
    /// LAN discovery wait (spec: 1000 ms).
    pub find_lan: Duration,
    /// Addressed discovery limit (spec: 3000 ms).
    pub find_addressed: Duration,
    /// Join handshake limit (spec: 5000 ms).
    pub join_timeout: Duration,
    /// Keep-alive interval (spec: 2 s).
    pub ping_interval: Duration,
    /// Silence after which a peer is gone (spec: 10 s).
    pub peer_timeout: Duration,
    /// Time allowed to reach the new host after migration (spec: 5 s).
    pub migrate_timeout: Duration,
    /// Offer/accept the optional UDP data channel.
    pub use_udp: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            port: DEFAULT_PORT,
            find_lan: Duration::from_millis(1000),
            find_addressed: Duration::from_millis(3000),
            join_timeout: Duration::from_millis(5000),
            ping_interval: Duration::from_secs(2),
            peer_timeout: Duration::from_secs(10),
            migrate_timeout: Duration::from_secs(5),
            use_udp: true,
        }
    }
}

/// A player designator in message functions: an id (0 = all/any) or a name.
#[derive(Clone, Debug, PartialEq)]
pub enum Target {
    Id(u32),
    Name(Vec<u8>),
}

/// A received message.
#[derive(Clone, Debug, PartialEq)]
pub struct Message {
    pub from: u32,
    pub from_name: Vec<u8>,
    pub id: f64,
    pub value: NetValue,
}

struct Session {
    cmd: Sender<Cmd>,
    ev: Receiver<Event>,
    thread: Option<JoinHandle<()>>,
    my_id: u32,
    host_id: u32,
    is_host: bool,
    /// Live player list, join order, host first.
    players: Vec<(u32, Vec<u8>)>,
}

/// Game-side multiplayer state (one per running game).
pub struct Mplay {
    pub cfg: Config,
    game_id: i32,
    connected: bool,
    address: String,
    move_mode: bool,
    guaranteed: bool,
    found: Vec<Found>,
    sess: Option<Session>,
    data: HashMap<u32, NetValue>,
    queue: VecDeque<Message>,
    last: Option<Message>,
    snapshot: Vec<(u32, Vec<u8>)>,
    ip_cache: Option<(Instant, String)>,
}

impl Mplay {
    /// New, unconnected state for the game with the given game id.
    pub fn new(game_id: i32, cfg: Config) -> Mplay {
        Mplay {
            cfg,
            game_id,
            connected: false,
            address: String::new(),
            move_mode: false,
            guaranteed: true,
            found: Vec::new(),
            sess: None,
            data: HashMap::new(),
            queue: VecDeque::new(),
            last: None,
            snapshot: Vec::new(),
            ip_cache: None,
        }
    }

    // ------------------------------------------------------------ connection

    /// `mplay_init_tcpip(addr)`.
    pub fn init_tcpip(&mut self, addr: &str) -> bool {
        if self.connected {
            self.end();
        }
        self.connected = true;
        self.address = addr.trim().to_string();
        true
    }

    /// `mplay_connect_status()`: 0 or 2.
    pub fn connect_status(&self) -> i32 {
        if self.connected {
            2
        } else {
            0
        }
    }

    /// `mplay_end()`.
    pub fn end(&mut self) {
        self.session_end();
        self.connected = false;
        self.found.clear();
    }

    /// `mplay_ipaddress()`: first non-loopback IPv4 (default-route one preferred).
    pub fn ipaddress(&mut self) -> String {
        if let Some((t0, s)) = &self.ip_cache {
            if t0.elapsed() < Duration::from_secs(5) {
                return s.clone();
            }
        }
        let s = local_ipv4();
        self.ip_cache = Some((Instant::now(), s.clone()));
        s
    }

    // ------------------------------------------------------------ sessions

    /// `mplay_session_mode(move)`.
    pub fn session_mode(&mut self, move_host: bool) {
        self.move_mode = move_host;
    }

    /// `mplay_session_create(name, max, player)`.
    pub fn session_create(&mut self, name: &[u8], max_players: i32, player: &[u8]) -> bool {
        if !self.connected || self.sess.is_some() {
            return false;
        }
        let Some(listener) = session::bind_listener(self.cfg.port) else { return false };
        let udp = session::bind_host_udp(self.cfg.port);
        let max = max_players.clamp(0, PLAYER_CAP as i32) as u16;
        let st = State {
            cfg: self.cfg.clone(),
            game_id: self.game_id,
            token: session::random_u64() | 1,
            name: name.to_vec(),
            max_players: max,
            my_id: 1,
            host_id: 1,
            players: vec![Player { id: 1, name: player.to_vec(), ip: None, port_hint: self.cfg.port }],
            data: HashMap::new(),
            next_id: 2,
            move_mode: self.move_mode,
        };
        let host = Host::new(st, listener, udp);
        self.reset_session_state();
        self.start(Role::Host(host), 1, 1, true, vec![(1, player.to_vec())]);
        true
    }

    fn start(&mut self, role: Role, my_id: u32, host_id: u32, is_host: bool, players: Vec<(u32, Vec<u8>)>) {
        let (ctx, crx) = channel();
        let (etx, erx) = channel();
        let thread = std::thread::Builder::new()
            .name("mplay".into())
            .spawn(move || session::run(role, crx, etx))
            .ok();
        self.sess = Some(Session {
            cmd: ctx,
            ev: erx,
            thread,
            my_id,
            host_id,
            is_host,
            players,
        });
    }

    fn reset_session_state(&mut self) {
        self.data.clear();
        self.queue.clear();
        self.last = None;
    }

    /// `mplay_session_find()`: blocking discovery; returns the number found.
    pub fn session_find(&mut self) -> usize {
        self.found.clear();
        if !self.connected {
            return 0;
        }
        self.found = if self.address.is_empty() {
            discovery::find_lan(self.game_id, self.cfg.port, self.cfg.find_lan)
        } else {
            match discovery::parse_address(&self.address, self.cfg.port) {
                Some(a) => discovery::find_addressed(self.game_id, a, self.cfg.port, self.cfg.find_addressed),
                None => Vec::new(),
            }
        };
        self.found.len()
    }

    /// Sessions found by the last `session_find`.
    pub fn found(&self) -> &[Found] {
        &self.found
    }

    /// `mplay_session_name(n)`.
    pub fn session_name(&self, n: i32) -> Vec<u8> {
        usize::try_from(n).ok().and_then(|n| self.found.get(n)).map(|f| f.info.name.clone()).unwrap_or_default()
    }

    /// `mplay_session_join(n, player)`: blocking handshake (≤ join timeout).
    pub fn session_join(&mut self, n: i32, player: &[u8]) -> bool {
        if !self.connected || self.sess.is_some() {
            return false;
        }
        let Some(f) = usize::try_from(n).ok().and_then(|n| self.found.get(n)).cloned() else { return false };
        let udp_sock = if self.cfg.use_udp {
            let unspec: std::net::IpAddr = if f.addr.is_ipv4() { std::net::Ipv4Addr::UNSPECIFIED.into() } else { std::net::Ipv6Addr::UNSPECIFIED.into() };
            UdpSocket::bind(SocketAddr::new(unspec, 0)).ok().filter(|s| s.set_nonblocking(true).is_ok())
        } else {
            None
        };
        let caps = if udp_sock.is_some() { wire::CAP_UDP_DATA } else { 0 };
        let hello = session::hello_frame(self.game_id, f.info.token, caps, player);
        let deadline = Instant::now() + self.cfg.join_timeout;
        let Some(session::Joined { conn, w }) = session::handshake(f.addr, &hello, deadline) else { return false };
        let players: Vec<Player> = w
            .players
            .iter()
            .map(|(id, name)| Player { id: *id, name: name.clone(), ip: None, port_hint: self.cfg.port })
            .collect();
        let udp = match (udp_sock, w.caps & wire::CAP_UDP_DATA != 0) {
            (Some(sock), true) => Some(UdpChan {
                sock,
                host: SocketAddr::new(f.addr.ip(), self.cfg.port),
                cookie: w.cookie,
                bound: false,
                last_bind: None,
            }),
            _ => None,
        };
        let data: HashMap<u32, NetValue> = w.data.iter().cloned().collect();
        let st = State {
            cfg: self.cfg.clone(),
            game_id: self.game_id,
            token: w.token,
            name: w.name.clone(),
            max_players: w.max_players,
            my_id: w.my_id,
            host_id: w.host_id,
            players,
            data: data.clone(),
            next_id: w.players.iter().map(|p| p.0).max().unwrap_or(0) + 1,
            move_mode: false,
        };
        let c = Client2 {
            st,
            conn,
            udp,
            own_udp: HashMap::new(),
            saw_peer_addr: false,
        };
        self.reset_session_state();
        self.data = data;
        self.start(Role::Client(c), w.my_id, w.host_id, false, w.players.clone());
        true
    }

    /// `mplay_session_status()`: 0 none, 1 host, 2 joined.
    pub fn session_status(&mut self) -> i32 {
        self.pump();
        match &self.sess {
            None => 0,
            Some(s) if s.is_host => 1,
            Some(_) => 2,
        }
    }

    /// `mplay_session_end()`.
    pub fn session_end(&mut self) {
        if let Some(mut s) = self.sess.take() {
            let _ = s.cmd.send(Cmd::End);
            if let Some(t) = s.thread.take() {
                let _ = t.join();
            }
        }
        self.reset_session_state();
    }

    /// Test hook: abandon the session without notifying anyone, as if the game
    /// had crashed.
    #[doc(hidden)]
    pub fn simulate_crash(&mut self) {
        if let Some(mut s) = self.sess.take() {
            let _ = s.cmd.send(Cmd::Crash);
            if let Some(t) = s.thread.take() {
                let _ = t.join();
            }
        }
        self.reset_session_state();
    }

    // ------------------------------------------------------------ events

    /// Applies inbound events from the network thread.
    pub fn pump(&mut self) {
        let mut ended = false;
        if let Some(s) = &mut self.sess {
            while let Ok(e) = s.ev.try_recv() {
                match e {
                    Event::PlayerJoined(id, name) => {
                        if !s.players.iter().any(|p| p.0 == id) {
                            s.players.push((id, name));
                        }
                    }
                    Event::PlayerLeft(id) => s.players.retain(|p| p.0 != id),
                    Event::Msg { from, id, val } => {
                        let from_name = s.players.iter().find(|p| p.0 == from).map(|p| p.1.clone()).unwrap_or_default();
                        self.queue.push_back(Message { from, from_name, id, value: val });
                    }
                    Event::Data(i, v) => {
                        if v.is_zero() {
                            self.data.remove(&i);
                        } else {
                            self.data.insert(i, v);
                        }
                    }
                    Event::HostChanged { host_id, me } => {
                        s.host_id = host_id;
                        s.is_host = me;
                        // keep "host first" order
                        if let Some(pos) = s.players.iter().position(|p| p.0 == host_id) {
                            let h = s.players.remove(pos);
                            s.players.insert(0, h);
                        }
                    }
                    Event::Ended => {
                        ended = true;
                        break;
                    }
                }
            }
        }
        if ended {
            if let Some(mut s) = self.sess.take() {
                if let Some(t) = s.thread.take() {
                    let _ = t.join();
                }
            }
            self.reset_session_state();
        }
    }

    // ------------------------------------------------------------ players

    /// `mplay_player_find()`: snapshot the player list (self first).
    pub fn player_find(&mut self) -> usize {
        self.pump();
        self.snapshot.clear();
        if let Some(s) = &self.sess {
            if let Some(me) = s.players.iter().find(|p| p.0 == s.my_id) {
                self.snapshot.push(me.clone());
            }
            for p in &s.players {
                if p.0 != s.my_id {
                    self.snapshot.push(p.clone());
                }
            }
        }
        self.snapshot.len()
    }

    /// `mplay_player_name(n)`.
    pub fn player_name(&self, n: i32) -> Vec<u8> {
        usize::try_from(n).ok().and_then(|n| self.snapshot.get(n)).map(|p| p.1.clone()).unwrap_or_default()
    }

    /// `mplay_player_id(n)`.
    pub fn player_id(&self, n: i32) -> u32 {
        usize::try_from(n).ok().and_then(|n| self.snapshot.get(n)).map(|p| p.0).unwrap_or(0)
    }

    /// This game's player id in the session (0 when not in a session).
    pub fn my_id(&self) -> u32 {
        self.sess.as_ref().map(|s| s.my_id).unwrap_or(0)
    }

    // ------------------------------------------------------------ shared data

    /// `mplay_data_mode(guaranteed)`.
    pub fn data_mode(&mut self, guaranteed: bool) {
        self.guaranteed = guaranteed;
    }

    fn data_index(ind: f64) -> Option<u32> {
        let i = ind.round();
        (0.0..=MAX_DATA_INDEX as f64).contains(&i).then_some(i as u32)
    }

    /// `mplay_data_write(ind, val)`.
    pub fn data_write(&mut self, ind: f64, val: NetValue) {
        self.pump();
        let Some(i) = Self::data_index(ind) else { return };
        let val = truncate(val);
        if val.is_zero() {
            self.data.remove(&i);
        } else {
            self.data.insert(i, val.clone());
        }
        if let Some(s) = &self.sess {
            let _ = s.cmd.send(Cmd::Data { index: i, val, guaranteed: self.guaranteed });
        }
    }

    /// `mplay_data_read(ind)`.
    pub fn data_read(&mut self, ind: f64) -> NetValue {
        self.pump();
        Self::data_index(ind).and_then(|i| self.data.get(&i).cloned()).unwrap_or_default()
    }

    // ------------------------------------------------------------ messages

    fn resolve(&self, t: &Target) -> Option<u32> {
        let s = self.sess.as_ref()?;
        match t {
            Target::Id(id) => Some(*id),
            Target::Name(n) => s.players.iter().find(|p| &p.1 == n).map(|p| p.0),
        }
    }

    /// `mplay_message_send(_guaranteed)(player, id, val)`.
    pub fn message_send(&mut self, to: &Target, id: f64, val: NetValue, guaranteed: bool) {
        self.pump();
        let Some(to) = self.resolve(to) else { return };
        let val = truncate(val);
        let Some(s) = &self.sess else { return };
        if to != 0 && !s.players.iter().any(|p| p.0 == to) {
            return;
        }
        if to == s.my_id {
            let from_name = s.players.iter().find(|p| p.0 == to).map(|p| p.1.clone()).unwrap_or_default();
            self.queue.push_back(Message { from: to, from_name, id, value: val });
            return;
        }
        let _ = s.cmd.send(Cmd::Msg { to, guaranteed, id, val });
    }

    fn matches(m: &Message, from: Option<u32>) -> bool {
        match from {
            Some(0) => true,
            Some(id) => m.from == id,
            None => false,
        }
    }

    /// `mplay_message_receive(player)`.
    pub fn message_receive(&mut self, from: &Target) -> bool {
        self.pump();
        let f = self.resolve(from);
        if let Some(pos) = self.queue.iter().position(|m| Self::matches(m, f)) {
            self.last = self.queue.remove(pos);
            true
        } else {
            false
        }
    }

    /// The last received message, if any.
    pub fn last_message(&self) -> Option<&Message> {
        self.last.as_ref()
    }

    /// `mplay_message_count(player)`.
    pub fn message_count(&mut self, from: &Target) -> usize {
        self.pump();
        let f = self.resolve(from);
        self.queue.iter().filter(|m| Self::matches(m, f)).count()
    }

    /// `mplay_message_clear(player)`.
    pub fn message_clear(&mut self, from: &Target) {
        self.pump();
        let f = self.resolve(from);
        self.queue.retain(|m| !Self::matches(m, f));
    }
}

impl Drop for Mplay {
    fn drop(&mut self) {
        self.session_end();
    }
}

fn truncate(v: NetValue) -> NetValue {
    match v {
        NetValue::Str(mut s) if s.len() > wire::MAX_VALUE_STR => {
            s.truncate(wire::MAX_VALUE_STR);
            NetValue::Str(s)
        }
        v => v,
    }
}

/// Number of non-guaranteed client datagrams (messages/data) hosts in this process
/// have accepted over the UDP channel (diagnostics).
pub fn udp_datagrams_received() -> usize {
    session::UDP_DATA_IN.load(std::sync::atomic::Ordering::Relaxed)
}

/// The machine's IPv4 address (default-route interface preferred), or 127.0.0.1.
pub fn local_ipv4() -> String {
    // connecting a UDP socket sends nothing but selects the default-route source
    if let Ok(s) = UdpSocket::bind(("0.0.0.0", 0)) {
        if s.connect(("192.0.2.1", 9)).is_ok() {
            if let Ok(a) = s.local_addr() {
                if !a.ip().is_unspecified() && !a.ip().is_loopback() {
                    return a.ip().to_string();
                }
            }
        }
    }
    if let Ok(ifs) = if_addrs::get_if_addrs() {
        for i in ifs {
            if let if_addrs::IfAddr::V4(a) = &i.addr {
                if !a.ip.is_loopback() {
                    return a.ip.to_string();
                }
            }
        }
    }
    "127.0.0.1".into()
}
