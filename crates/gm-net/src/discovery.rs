//! Session discovery (`mplay_session_find`, §B.3).

use crate::wire::{t, SessionInfo, PROTO_VERSION, R, W};
use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream, ToSocketAddrs, UdpSocket};
use std::time::{Duration, Instant};

/// A session found by discovery.
#[derive(Clone, Debug)]
pub struct Found {
    pub info: SessionInfo,
    /// Host address to join (reply source IP + advertised TCP port).
    pub addr: SocketAddr,
}

/// Parses `host`, `host:port`, `[v6]:port` or a bare IPv6 address.
pub fn parse_address(addr: &str, default_port: u16) -> Option<SocketAddr> {
    let a = addr.trim();
    if a.is_empty() {
        return None;
    }
    let (host, port) = if let Some(rest) = a.strip_prefix('[') {
        let end = rest.find(']')?;
        let port = rest[end + 1..].strip_prefix(':').and_then(|p| p.parse().ok()).unwrap_or(default_port);
        (rest[..end].to_string(), port)
    } else if a.matches(':').count() == 1 {
        let (h, p) = a.split_once(':').unwrap();
        (h.to_string(), p.parse().ok()?)
    } else {
        (a.to_string(), default_port)
    };
    if let Ok(ip) = host.parse::<IpAddr>() {
        return Some(SocketAddr::new(ip, port));
    }
    let list: Vec<SocketAddr> = (host.as_str(), port).to_socket_addrs().ok()?.collect();
    list.iter().find(|a| a.is_ipv4()).or(list.first()).copied()
}

fn discover_datagram(nonce: u32, game_id: i32) -> Vec<u8> {
    let mut w = W::new();
    w.u32(nonce).i32(game_id).u16(PROTO_VERSION);
    W::datagram(t::DISCOVER, &w.0)
}

fn add(out: &mut Vec<Found>, f: Found) {
    if f.info.accepting && !out.iter().any(|o| o.info.token == f.info.token) {
        out.push(f);
    }
}

fn read_reply(d: &[u8], nonce: u32, from: SocketAddr) -> Option<Found> {
    let (ty, p) = crate::wire::parse_datagram(d)?;
    if ty != t::SESSION_INFO {
        return None;
    }
    let mut r = R::new(p);
    if r.u32().ok()? != nonce {
        return None;
    }
    let info = SessionInfo::read(&mut r).ok()?;
    let addr = SocketAddr::new(from.ip(), info.tcp_port);
    Some(Found { info, addr })
}

/// Broadcast targets: limited broadcast, every interface's directed broadcast and
/// loopback (so hosts on this machine are always found).
fn broadcast_targets(port: u16) -> Vec<SocketAddr> {
    let mut v: Vec<SocketAddr> = vec![SocketAddr::new(Ipv4Addr::BROADCAST.into(), port)];
    if let Ok(ifs) = if_addrs::get_if_addrs() {
        for i in ifs {
            if let if_addrs::IfAddr::V4(a) = i.addr {
                if let Some(b) = a.broadcast {
                    let s = SocketAddr::new(b.into(), port);
                    if !v.contains(&s) {
                        v.push(s);
                    }
                }
            }
        }
    }
    v.push(SocketAddr::new(Ipv4Addr::LOCALHOST.into(), port));
    v
}

/// LAN mode: broadcast DISCOVER, collect replies for `wait`, re-sending at 300 ms.
pub fn find_lan(game_id: i32, port: u16, wait: Duration) -> Vec<Found> {
    let mut out = Vec::new();
    let Ok(sock) = UdpSocket::bind(("0.0.0.0", 0)) else { return out };
    let _ = sock.set_broadcast(true);
    let nonce = crate::session::random_u64() as u32;
    let dg = discover_datagram(nonce, game_id);
    let targets = broadcast_targets(port);
    let send = |s: &UdpSocket| {
        for t in &targets {
            let _ = s.send_to(&dg, t);
        }
    };
    send(&sock);
    let start = Instant::now();
    let mut resent = false;
    let mut buf = [0u8; 2048];
    while start.elapsed() < wait {
        if !resent && start.elapsed() >= Duration::from_millis(300).min(wait / 3) {
            send(&sock);
            resent = true;
        }
        let _ = sock.set_read_timeout(Some(Duration::from_millis(20)));
        if let Ok((n, from)) = sock.recv_from(&mut buf) {
            if let Some(f) = read_reply(&buf[..n], nonce, from) {
                add(&mut out, f);
            }
        }
    }
    out
}

/// Addressed mode: unicast DISCOVER plus TCP LIST_SESSIONS; the first answer wins.
pub fn find_addressed(game_id: i32, addr: SocketAddr, udp_port: u16, wait: Duration) -> Vec<Found> {
    let mut out = Vec::new();
    let deadline = Instant::now() + wait;
    let nonce = crate::session::random_u64() as u32;
    let unspec: IpAddr = if addr.is_ipv4() { Ipv4Addr::UNSPECIFIED.into() } else { std::net::Ipv6Addr::UNSPECIFIED.into() };
    let udp = UdpSocket::bind(SocketAddr::new(unspec, 0)).ok();
    if let Some(u) = &udp {
        let dg = discover_datagram(nonce, game_id);
        let _ = u.send_to(&dg, SocketAddr::new(addr.ip(), udp_port));
        if addr.port() != udp_port {
            let _ = u.send_to(&dg, addr);
        }
        let _ = u.set_nonblocking(true);
    }
    // TCP query on a helper thread so UDP replies can be taken meanwhile.
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let r = (|| -> Option<Vec<Found>> {
            let left = deadline.checked_duration_since(Instant::now())?;
            let mut s = TcpStream::connect_timeout(&addr, left).ok()?;
            let mut w = W::new();
            w.i32(game_id).u16(PROTO_VERSION);
            s.write_all(&W::frame(t::LIST_SESSIONS, &w.0)).ok()?;
            let mut fb = crate::wire::FrameBuf::default();
            let mut buf = [0u8; 8192];
            loop {
                if let Some((ty, p)) = fb.next().ok()? {
                    if ty != t::SESSION_LIST {
                        return None;
                    }
                    let mut r = R::new(&p);
                    let n = r.u16().ok()?;
                    let mut v = Vec::new();
                    for _ in 0..n {
                        let info = SessionInfo::read(&mut r).ok()?;
                        let a = SocketAddr::new(addr.ip(), info.tcp_port);
                        v.push(Found { info, addr: a });
                    }
                    return Some(v);
                }
                let left = deadline.checked_duration_since(Instant::now())?;
                s.set_read_timeout(Some(left.max(Duration::from_millis(1)))).ok()?;
                let n = s.read(&mut buf).ok()?;
                if n == 0 {
                    return None;
                }
                fb.push(&buf[..n]);
            }
        })();
        let _ = tx.send(r.unwrap_or_default());
    });
    let mut buf = [0u8; 2048];
    let mut tcp_done = false;
    while Instant::now() < deadline {
        if let Some(u) = &udp {
            while let Ok((n, from)) = u.recv_from(&mut buf) {
                if let Some(f) = read_reply(&buf[..n], nonce, from) {
                    add(&mut out, f);
                }
            }
        }
        if !tcp_done {
            match rx.try_recv() {
                Ok(v) => {
                    tcp_done = true;
                    for f in v {
                        add(&mut out, f);
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => tcp_done = true,
                Err(_) => {}
            }
        }
        if !out.is_empty() {
            // give the other channel a moment to contribute, then stop
            if tcp_done {
                break;
            }
        }
        if tcp_done && out.is_empty() && udp.is_none() {
            break;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    out
}
