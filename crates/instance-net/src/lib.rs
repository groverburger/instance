//! Instance multiplayer: the `mplay_*` behaviour of specs/multiplayer.md Part A over
//! Instance's own TCP/UDP protocol (Part B). Not compatible with GameMaker's
//! DirectPlay sessions.
//!
//! [`Mplay`] is the game-side object. It never blocks except where the spec allows
//! (`session_find`, `session_join`, and briefly while ending a session). All socket
//! work for an active session runs on a background thread; the game side exchanges
//! commands and events with it over channels and applies inbound events whenever an
//! `mplay_*` function is called (or [`Mplay::pump`] is called by the runner).

//!
//! On `wasm32` there are no sockets: [`Mplay`] is a stub whose
//! `mplay_init_tcpip` fails, so games see "no connection".

#[cfg(not(target_arch = "wasm32"))]
mod conn;
#[cfg(not(target_arch = "wasm32"))]
pub mod discovery;
#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
mod session;
#[cfg(target_arch = "wasm32")]
mod web;
pub mod wire;

#[cfg(not(target_arch = "wasm32"))]
pub use discovery::Found;
#[cfg(not(target_arch = "wasm32"))]
pub use native::*;
#[cfg(not(target_arch = "wasm32"))]
pub use session::{Event, MAX_DATA_INDEX, PLAYER_CAP};
#[cfg(target_arch = "wasm32")]
pub use web::*;
pub use wire::NetValue;

use std::time::Duration;

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
