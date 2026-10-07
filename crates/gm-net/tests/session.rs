//! In-process host + clients on localhost (distinct base ports per test).

use gm_net::{Config, Mplay, NetValue, Target};
use std::time::{Duration, Instant};

const GAME: i32 = 4242;

fn cfg(port: u16) -> Config {
    Config {
        port,
        find_lan: Duration::from_millis(400),
        find_addressed: Duration::from_millis(1500),
        ..Config::default()
    }
}

fn wait(what: &str, mut f: impl FnMut() -> bool) {
    let t0 = Instant::now();
    while t0.elapsed() < Duration::from_secs(10) {
        if f() {
            return;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("timed out waiting for {what}");
}

fn s(x: &str) -> NetValue {
    NetValue::Str(x.as_bytes().to_vec())
}

fn names(m: &mut Mplay) -> Vec<String> {
    let n = m.player_find();
    (0..n as i32).map(|i| String::from_utf8(m.player_name(i)).unwrap()).collect()
}

fn host_and_two(port: u16, lan: bool, move_mode: bool) -> (Mplay, Mplay, Mplay) {
    let mut h = Mplay::new(GAME, cfg(port));
    assert!(h.init_tcpip(""));
    assert_eq!(h.connect_status(), 2);
    h.session_mode(move_mode);
    assert!(h.session_create(b"Arena", 0, b"hoster"));
    assert_eq!(h.session_status(), 1);
    let addr = if lan { String::new() } else { format!("127.0.0.1:{port}") };
    let mut cs = Vec::new();
    for name in ["alice", "bob"] {
        let mut c = Mplay::new(GAME, cfg(port));
        assert!(c.init_tcpip(&addr));
        assert_eq!(c.session_find(), 1, "{name} finds the session");
        assert_eq!(c.session_name(0), b"Arena");
        assert_eq!(c.session_name(1), b"");
        assert!(c.session_join(0, name.as_bytes()));
        assert_eq!(c.session_status(), 2);
        cs.push(c);
    }
    let b = cs.pop().unwrap();
    let a = cs.pop().unwrap();
    (h, a, b)
}

#[test]
fn lan_session_players_data_messages_leave_end() {
    let (mut h, mut a, mut b) = host_and_two(41010, true, false);
    wait("host sees 3 players", || h.player_find() == 3);
    wait("alice sees 3 players", || a.player_find() == 3);
    wait("bob sees 3 players", || b.player_find() == 3);
    assert_eq!(names(&mut h), ["hoster", "alice", "bob"]);
    assert_eq!(names(&mut a), ["alice", "hoster", "bob"]);
    assert_eq!(names(&mut b), ["bob", "hoster", "alice"]);
    h.player_find();
    assert_eq!(h.player_id(0), 1);
    let alice_id = h.player_id(1);
    let bob_id = h.player_id(2);
    assert!(alice_id > 1 && bob_id > alice_id);
    assert_eq!(h.player_id(7), 0);
    assert_eq!(h.player_name(-1), b"");

    // shared data: written anywhere, converges everywhere
    h.data_write(5.0, NetValue::Real(1.5));
    a.data_write(6.0, s("from alice"));
    b.data_write(1_000_001.0, NetValue::Real(9.0)); // out of range: ignored
    assert_eq!(b.data_read(1_000_001.0), NetValue::Real(0.0));
    for m in [&mut h, &mut a, &mut b] {
        wait("data converges", || m.data_read(5.0) == NetValue::Real(1.5) && m.data_read(6.0) == s("from alice"));
    }
    // concurrent writes to one slot converge to the host's order
    for k in 0..20 {
        a.data_write(7.0, NetValue::Real(k as f64));
        b.data_write(7.0, NetValue::Real(100.0 + k as f64));
    }
    wait("slot 7 converges", || {
        let x = h.data_read(7.0);
        a.data_read(7.0) == x && b.data_read(7.0) == x && x != NetValue::Real(0.0)
    });

    // messages: to one player by id, by name, and to all
    a.message_send(&Target::Id(1), 10.0, s("hi host"), true);
    b.message_send(&Target::Name(b"alice".to_vec()), 11.0, NetValue::Real(3.0), true);
    h.message_send(&Target::Id(0), 12.0, s("everyone"), true);
    wait("host got alice's message", || h.message_count(&Target::Id(0)) == 1);
    assert!(h.message_receive(&Target::Name(b"alice".to_vec())));
    let m = h.last_message().unwrap().clone();
    assert_eq!((m.from, m.id, m.value, m.from_name), (alice_id, 10.0, s("hi host"), b"alice".to_vec()));
    assert!(!h.message_receive(&Target::Id(0)));
    wait("alice got 2 messages", || a.message_count(&Target::Id(0)) == 2);
    assert_eq!(a.message_count(&Target::Id(1)), 1);
    assert_eq!(a.message_count(&Target::Id(bob_id)), 1);
    assert!(a.message_receive(&Target::Id(bob_id)));
    assert_eq!(a.last_message().unwrap().value, NetValue::Real(3.0));
    assert!(a.message_receive(&Target::Id(0)));
    assert_eq!(a.last_message().unwrap().value, s("everyone"));
    wait("bob got broadcast", || b.message_count(&Target::Id(0)) == 1);
    b.message_clear(&Target::Id(0));
    assert_eq!(b.message_count(&Target::Id(0)), 0);
    // ordering of guaranteed messages from one sender
    for k in 0..50 {
        a.message_send(&Target::Id(bob_id), k as f64, NetValue::Real(k as f64), true);
    }
    wait("bob got 50", || b.message_count(&Target::Id(alice_id)) == 50);
    for k in 0..50 {
        assert!(b.message_receive(&Target::Id(alice_id)));
        assert_eq!(b.last_message().unwrap().id, k as f64);
    }
    // to self
    b.message_send(&Target::Id(bob_id), 1.0, NetValue::Real(1.0), true);
    assert_eq!(b.message_count(&Target::Id(bob_id)), 1);
    // unknown target: dropped
    b.message_send(&Target::Name(b"nobody".to_vec()), 1.0, NetValue::Real(1.0), true);

    // bob leaves
    b.session_end();
    assert_eq!(b.session_status(), 0);
    wait("host sees bob leave", || h.player_find() == 2);
    wait("alice sees bob leave", || a.player_find() == 2);

    // a late joiner receives the host's table
    let mut c = Mplay::new(GAME, cfg(41010));
    c.init_tcpip("");
    assert_eq!(c.session_find(), 1);
    assert!(c.session_join(0, b"carol"));
    assert_eq!(c.data_read(6.0), s("from alice"));
    wait("carol listed", || h.player_find() == 3);
    h.player_find();
    assert!(h.player_id(2) > bob_id, "ids are never reused");

    // host ends: everyone's session is over
    h.session_end();
    assert_eq!(h.session_status(), 0);
    wait("alice session over", || a.session_status() == 0);
    wait("carol session over", || c.session_status() == 0);
    assert_eq!(a.data_read(6.0), NetValue::Real(0.0), "data cleared at session end");
    assert_eq!(a.player_find(), 0);
    h.end();
    assert_eq!(h.connect_status(), 0);
}

#[test]
fn addressed_find_and_join() {
    let (mut h, mut a, mut b) = host_and_two(41030, false, false);
    wait("3 players", || h.player_find() == 3 && a.player_find() == 3 && b.player_find() == 3);
    // other game ids never see the session
    let mut other = Mplay::new(GAME + 1, cfg(41030));
    other.init_tcpip("127.0.0.1:41030");
    assert_eq!(other.session_find(), 0);
    let mut lan_other = Mplay::new(GAME + 1, cfg(41030));
    lan_other.init_tcpip("");
    assert_eq!(lan_other.session_find(), 0);
}

#[test]
fn unreliable_channel_converges() {
    let (mut h, mut a, mut b) = host_and_two(41050, true, false);
    wait("3 players", || a.player_find() == 3 && b.player_find() == 3);
    a.data_mode(false);
    b.data_mode(false);
    h.data_mode(false);
    // give the UDP binding a moment to complete
    std::thread::sleep(Duration::from_millis(700));
    for k in 0..100 {
        a.data_write((k % 10) as f64, NetValue::Real(k as f64));
    }
    h.data_write(50.0, s("host-udp"));
    for m in [&mut h, &mut a, &mut b] {
        wait("udp data converges", || (0..10).all(|i| m.data_read(i as f64) == NetValue::Real((90 + i) as f64)) && m.data_read(50.0) == s("host-udp"));
    }
    b.message_send(&Target::Id(0), 7.0, s("fast"), false);
    wait("host got fast message", || h.message_count(&Target::Id(0)) == 1);
    wait("alice got fast message", || a.message_count(&Target::Id(0)) == 1);
    assert!(gm_net::udp_datagrams_received() > 0, "the UDP data channel was used");
}

#[test]
fn session_full_rejects() {
    let mut h = Mplay::new(GAME, cfg(41070));
    h.init_tcpip("");
    assert!(h.session_create(b"Duel", 2, b"one"));
    let mut a = Mplay::new(GAME, cfg(41070));
    a.init_tcpip("");
    assert_eq!(a.session_find(), 1);
    assert!(a.session_join(0, b"two"));
    let mut b = Mplay::new(GAME, cfg(41070));
    b.init_tcpip("127.0.0.1:41070");
    assert_eq!(b.session_find(), 0, "full sessions are not listed");
    // joining a stale entry fails cleanly
    let mut c = Mplay::new(GAME, cfg(41070));
    c.init_tcpip("");
    assert!(!c.session_join(0, b"x"));
    assert!(!c.session_create(b"x", 0, b"x") || c.session_status() == 1);
}

#[test]
fn host_migration() {
    let (mut h, mut a, mut b) = host_and_two(41090, true, true);
    wait("3 players", || a.player_find() == 3 && b.player_find() == 3 && h.player_find() == 3);
    a.player_find();
    let alice_id = a.player_id(0);
    h.data_write(1.0, s("kept"));
    wait("data", || a.data_read(1.0) == s("kept") && b.data_read(1.0) == s("kept"));
    std::thread::sleep(Duration::from_millis(100));
    h.session_end();
    wait("alice becomes host", || a.session_status() == 1);
    wait("bob stays joined with 2 players", || b.session_status() == 2 && b.player_find() == 2);
    assert_eq!(names(&mut b), ["bob", "alice"]);
    assert_eq!(names(&mut a), ["alice", "bob"]);
    assert_eq!(b.data_read(1.0), s("kept"));
    b.data_write(2.0, NetValue::Real(5.0));
    wait("new host gets data", || a.data_read(2.0) == NetValue::Real(5.0));
    b.message_send(&Target::Id(alice_id), 1.0, s("to new host"), true);
    wait("new host gets message", || a.message_receive(&Target::Id(0)));
    // a new player can still join the migrated session
    let mut c = Mplay::new(GAME, cfg(41090));
    c.init_tcpip("");
    assert_eq!(c.session_find(), 1);
    assert!(c.session_join(0, b"carol"));
    wait("carol visible", || b.player_find() == 3);
    a.session_end();
    wait("bob becomes host", || b.session_status() == 1);
}

#[test]
fn unsupported_and_idle_functions() {
    let mut m = Mplay::new(GAME, cfg(41110));
    assert_eq!(m.connect_status(), 0);
    assert!(!m.session_create(b"x", 0, b"y"), "not connected");
    assert_eq!(m.session_find(), 0);
    assert_eq!(m.player_find(), 0);
    m.data_write(3.0, NetValue::Real(2.0));
    assert_eq!(m.data_read(3.0), NetValue::Real(2.0), "local-only data outside a session");
    assert!(!m.message_receive(&Target::Id(0)));
    assert!(m.last_message().is_none());
    assert!(!m.ipaddress().is_empty());
}

#[test]
fn host_crash_elects_lowest_id() {
    let (mut h, mut a, mut b) = host_and_two(41130, true, true);
    wait("3 players", || a.player_find() == 3 && b.player_find() == 3);
    h.simulate_crash();
    wait("alice elected", || a.session_status() == 1);
    wait("bob reconnected", || b.session_status() == 2 && b.player_find() == 2);
}

#[test]
fn host_crash_without_migration_ends_session() {
    let (mut h, mut a, _b) = host_and_two(41150, true, false);
    wait("3 players", || a.player_find() == 3);
    h.simulate_crash();
    wait("alice session over", || a.session_status() == 0);
}
