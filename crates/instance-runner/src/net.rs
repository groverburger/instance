//! Multiplayer functions (`mplay_*`, specs/multiplayer.md Part A) on top of the
//! `instance-net` session engine (Instance's own protocol, Part B).
//!
//! Only TCP/IP is supported; IPX, modem and serial initialisation return false.
//! Environment overrides (diagnostics/tests): `INSTANCE_MPLAY_PORT` (or `GM81_MPLAY_PORT`) sets the base port
//! (default 40810).

use crate::builtins::{bo, r, zero, Reg, A};
use crate::game::Game;
use instance_net::{Mplay, NetValue, Target};
use gml::{Error, Value};

/// Creates the game's multiplayer state.
pub fn new_state(game_id: i32) -> Mplay {
    let mut cfg = instance_net::Config::default();
    if let Some(p) = instance_brand::appdirs::env_var("MPLAY_PORT").and_then(|p| p.parse().ok()) {
        cfg.port = p;
    }
    Mplay::new(game_id, cfg)
}

fn to_net(v: Value) -> NetValue {
    match v {
        Value::Real(x) => NetValue::Real(x),
        Value::Str(s) => NetValue::Str(s.as_bytes().to_vec()),
    }
}

fn from_net(v: NetValue) -> Value {
    match v {
        NetValue::Real(x) => Value::Real(x),
        NetValue::Str(s) => Value::from(s),
    }
}

fn bytes(b: Vec<u8>) -> Result<Value, Error> {
    Ok(Value::from(b))
}

/// A player argument: an id (0 = all/any) or a name string.
fn target(a: &A, i: usize) -> Target {
    match a.val(i) {
        Value::Str(s) => Target::Name(s.as_bytes().to_vec()),
        Value::Real(x) => {
            let id = x.round();
            Target::Id(if id >= 0.0 && id <= u32::MAX as f64 { id as u32 } else { u32::MAX })
        }
    }
}

/// Registers every `mplay_*` function.
pub fn register(l: &mut Reg) {
    // 413_01 connections
    l.push(("mplay_init_ipx", |_, _, _, _| bo(false)));
    l.push(("mplay_init_modem", |_, _, _, _| bo(false)));
    l.push(("mplay_init_serial", |_, _, _, _| bo(false)));
    l.push(("mplay_init_tcpip", |g, _, _, a| {
        let addr = a.s(0)?;
        bo(g.net.init_tcpip(&addr))
    }));
    l.push(("mplay_connect_status", |g, _, _, _| r(g.net.connect_status() as f64)));
    l.push(("mplay_end", |g, _, _, _| {
        g.net.end();
        zero()
    }));
    l.push(("mplay_ipaddress", |g, _, _, _| crate::builtins::st(&g.net.ipaddress())));
    // 413_02 sessions
    l.push(("mplay_session_mode", |g, _, _, a| {
        g.net.session_mode(a.b(0)?);
        zero()
    }));
    l.push(("mplay_session_create", |g, _, _, a| {
        let name = a.sb(0)?;
        let max = a.i(1)?;
        let player = a.sb(2)?;
        bo(g.net.session_create(&name, max, &player))
    }));
    l.push(("mplay_session_find", |g, _, _, _| r(g.net.session_find() as f64)));
    l.push(("mplay_session_name", |g, _, _, a| bytes(g.net.session_name(a.i(0)?))));
    l.push(("mplay_session_join", |g, _, _, a| {
        let n = a.i(0)?;
        let player = a.sb(1)?;
        bo(g.net.session_join(n, &player))
    }));
    l.push(("mplay_session_status", |g, _, _, _| r(g.net.session_status() as f64)));
    l.push(("mplay_session_end", |g, _, _, _| {
        g.net.session_end();
        zero()
    }));
    // 413_03 players
    l.push(("mplay_player_find", |g, _, _, _| r(g.net.player_find() as f64)));
    l.push(("mplay_player_name", |g, _, _, a| bytes(g.net.player_name(a.i(0)?))));
    l.push(("mplay_player_id", |g, _, _, a| r(g.net.player_id(a.i(0)?) as f64)));
    // 413_04 shared data
    l.push(("mplay_data_write", |g, _, _, a| {
        let ind = a.r(0)?;
        g.net.data_write(ind, to_net(a.val(1)));
        zero()
    }));
    l.push(("mplay_data_read", |g, _, _, a| Ok(from_net(g.net.data_read(a.r(0)?)))));
    l.push(("mplay_data_mode", |g, _, _, a| {
        g.net.data_mode(a.b(0)?);
        zero()
    }));
    // 413_05 messages
    l.push(("mplay_message_send", |g, _, _, a| {
        let to = target(a, 0);
        let id = a.r(1)?;
        g.net.message_send(&to, id, to_net(a.val(2)), false);
        zero()
    }));
    l.push(("mplay_message_send_guaranteed", |g, _, _, a| {
        let to = target(a, 0);
        let id = a.r(1)?;
        g.net.message_send(&to, id, to_net(a.val(2)), true);
        zero()
    }));
    l.push(("mplay_message_receive", |g, _, _, a| bo(g.net.message_receive(&target(a, 0)))));
    l.push(("mplay_message_id", |g, _, _, _| r(g.net.last_message().map(|m| m.id).unwrap_or(0.0))));
    l.push(("mplay_message_value", |g, _, _, _| {
        Ok(g.net.last_message().map(|m| from_net(m.value.clone())).unwrap_or(Value::Real(0.0)))
    }));
    l.push(("mplay_message_player", |g, _, _, _| r(g.net.last_message().map(|m| m.from as f64).unwrap_or(0.0))));
    l.push(("mplay_message_name", |g, _, _, _| bytes(g.net.last_message().map(|m| m.from_name.clone()).unwrap_or_default())));
    l.push(("mplay_message_count", |g, _, _, a| r(g.net.message_count(&target(a, 0)) as f64)));
    l.push(("mplay_message_clear", |g, _, _, a| {
        g.net.message_clear(&target(a, 0));
        zero()
    }));
}

/// Applies inbound network events between steps (§A.8).
pub fn pump(g: &mut Game) {
    g.net.pump();
}
