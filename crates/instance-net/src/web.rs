//! `wasm32` stand-in for [`Mplay`]: browsers cannot open raw TCP/UDP sockets, so
//! `mplay_init_tcpip` fails and every other call behaves as "not connected".

use crate::{Config, Message, NetValue, Target};

/// Highest shared-data index (same as the native build).
pub const MAX_DATA_INDEX: u32 = 1_000_000;
/// Most players in a session (same as the native build).
pub const PLAYER_CAP: usize = 256;

/// Game-side multiplayer state; never connects on the web.
pub struct Mplay {
    pub cfg: Config,
}

impl Mplay {
    pub fn new(_game_id: i32, cfg: Config) -> Mplay {
        Mplay { cfg }
    }
    pub fn init_tcpip(&mut self, _addr: &str) -> bool {
        false
    }
    pub fn connect_status(&self) -> i32 {
        0
    }
    pub fn end(&mut self) {}
    pub fn ipaddress(&mut self) -> String {
        "127.0.0.1".into()
    }
    pub fn session_mode(&mut self, _move_host: bool) {}
    pub fn session_create(&mut self, _name: &[u8], _max_players: i32, _player: &[u8]) -> bool {
        false
    }
    pub fn session_find(&mut self) -> usize {
        0
    }
    pub fn session_name(&self, _n: i32) -> Vec<u8> {
        Vec::new()
    }
    pub fn session_join(&mut self, _n: i32, _player: &[u8]) -> bool {
        false
    }
    pub fn session_status(&mut self) -> i32 {
        0
    }
    pub fn session_end(&mut self) {}
    pub fn simulate_crash(&mut self) {}
    pub fn pump(&mut self) {}
    pub fn player_find(&mut self) -> usize {
        0
    }
    pub fn player_name(&self, _n: i32) -> Vec<u8> {
        Vec::new()
    }
    pub fn player_id(&self, _n: i32) -> u32 {
        0
    }
    pub fn my_id(&self) -> u32 {
        0
    }
    pub fn data_mode(&mut self, _guaranteed: bool) {}
    pub fn data_write(&mut self, _ind: f64, _val: NetValue) {}
    pub fn data_read(&mut self, _ind: f64) -> NetValue {
        NetValue::Real(0.0)
    }
    pub fn message_send(&mut self, _to: &Target, _id: f64, _val: NetValue, _guaranteed: bool) {}
    pub fn message_receive(&mut self, _from: &Target) -> bool {
        false
    }
    pub fn last_message(&self) -> Option<&Message> {
        None
    }
    pub fn message_count(&mut self, _from: &Target) -> usize {
        0
    }
    pub fn message_clear(&mut self, _from: &Target) {}
}

pub fn udp_datagrams_received() -> usize {
    0
}

pub fn local_ipv4() -> String {
    "127.0.0.1".into()
}
