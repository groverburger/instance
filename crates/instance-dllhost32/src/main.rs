//! `instance-dllhost32`: loads 32-bit GameMaker DLLs on behalf of the 64-bit runner
//! (specs/extensions.md §6.1 tier 4, protocol in `instance_dllcall::proto`).
//!
//! Requests arrive on stdin and replies go to stdout. All DLL calls run on the main
//! thread, which pumps Win32 messages while idle so DLL-created windows keep working.
//! Build: `cargo build -p instance-dllhost32 --target i686-pc-windows-msvc`.

use instance_dllcall::proto::{self, Dec, Enc, PVal};
use instance_dllcall::{Arg, Ret};
use std::collections::HashMap;
use std::ffi::{c_void, CString};
use std::io::{self, Write};
use std::sync::mpsc;
use std::time::Duration;

struct FnDef {
    fp: *const c_void,
    handle: u32,
    stdcall: bool,
    ret_str: bool,
    types: Vec<bool>,
}

#[derive(Default)]
struct State {
    libs: HashMap<u32, libloading::Library>,
    next_handle: u32,
    fns: Vec<Option<FnDef>>,
}

type Reply = Result<Vec<u8>, String>;

impl State {
    fn handle(&mut self, op: u8, p: &[u8]) -> Reply {
        let mut d = Dec::new(p);
        let e = |x: io::Error| x.to_string();
        match op {
            proto::LOAD => {
                let path = String::from_utf8_lossy(&d.str().map_err(e)?).to_string();
                let lib = unsafe { libloading::Library::new(&path) }.map_err(|x| format!("cannot load {path}: {x}"))?;
                self.next_handle += 1;
                self.libs.insert(self.next_handle, lib);
                Ok(Enc::default().u32(self.next_handle).0)
            }
            proto::DEFINE => {
                let handle = d.u32().map_err(e)?;
                let sym = String::from_utf8_lossy(&d.str().map_err(e)?).to_string();
                let stdcall = d.u8().map_err(e)? == 1;
                let ret_str = d.u8().map_err(e)? == 1;
                let n = d.u8().map_err(e)? as usize;
                let mut types = Vec::with_capacity(n);
                for _ in 0..n {
                    types.push(d.u8().map_err(e)? == 1);
                }
                if !instance_dllcall::signature_supported(&types) {
                    return Err("unsupported argument types".into());
                }
                let lib = self.libs.get(&handle).ok_or("unknown library handle")?;
                let mut fp = None;
                for cand in instance_dllcall::symbol_candidates(&sym, &types, std::mem::size_of::<usize>()) {
                    let mut name = cand.into_bytes();
                    name.push(0);
                    if let Ok(s) = unsafe { lib.get::<*const c_void>(&name) } {
                        fp = Some(*s);
                        break;
                    }
                }
                let fp = fp.ok_or_else(|| format!("function {sym} not found"))?;
                self.fns.push(Some(FnDef { fp, handle, stdcall, ret_str, types }));
                Ok(Enc::default().u32((self.fns.len() - 1) as u32).0)
            }
            proto::CALL => {
                let id = d.u32().map_err(e)? as usize;
                let n = d.u8().map_err(e)? as usize;
                let mut vals = Vec::with_capacity(n);
                for _ in 0..n {
                    vals.push(d.val().map_err(e)?);
                }
                let f = self.fns.get(id).and_then(|f| f.as_ref()).ok_or("unknown function id")?;
                if vals.len() != f.types.len() || vals.iter().zip(&f.types).any(|(v, s)| matches!(v, PVal::Str(_)) != *s) {
                    return Err("wrong number or type of arguments".into());
                }
                let strings: Vec<CString> = vals
                    .iter()
                    .filter_map(|v| match v {
                        PVal::Str(b) => Some(cstring(b)),
                        _ => None,
                    })
                    .collect();
                let mut si = 0;
                let args: Vec<Arg> = vals
                    .iter()
                    .map(|v| match v {
                        PVal::Real(x) => Arg::Real(*x),
                        PVal::Str(_) => {
                            si += 1;
                            Arg::Ptr(strings[si - 1].as_ptr())
                        }
                    })
                    .collect();
                let r = unsafe { instance_dllcall::invoke(f.fp, f.stdcall, &args, f.ret_str) }.ok_or("unsupported signature")?;
                let v = match r {
                    Ret::Real(x) => PVal::Real(x),
                    Ret::Ptr(p) => PVal::Str(unsafe { instance_dllcall::copy_cstr(p) }),
                };
                drop(strings);
                Ok(Enc::default().val(&v).0)
            }
            proto::FREE => {
                let h = d.u32().map_err(e)?;
                for f in self.fns.iter_mut() {
                    if f.as_ref().map(|f| f.handle == h).unwrap_or(false) {
                        *f = None;
                    }
                }
                self.libs.remove(&h);
                Ok(Vec::new())
            }
            proto::SET_CWD => {
                let p = String::from_utf8_lossy(&d.str().map_err(e)?).to_string();
                std::env::set_current_dir(&p).map_err(|x| x.to_string())?;
                Ok(Vec::new())
            }
            _ => Err(format!("unknown op {op}")),
        }
    }
}

/// GML strings may contain NUL bytes; C sees them cut at the first one.
fn cstring(b: &[u8]) -> CString {
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    CString::new(&b[..end]).unwrap_or_default()
}

#[cfg(windows)]
fn pump_messages() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{DispatchMessageW, PeekMessageW, TranslateMessage, MSG, PM_REMOVE};
    unsafe {
        let mut msg: MSG = std::mem::zeroed();
        while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

#[cfg(not(windows))]
fn pump_messages() {}

fn main() {
    let (tx, rx) = mpsc::channel::<(u8, Vec<u8>)>();
    std::thread::spawn(move || {
        let mut stdin = io::stdin().lock();
        while let Ok(f) = proto::read_frame(&mut stdin) {
            if tx.send(f).is_err() {
                break;
            }
        }
    });
    let mut st = State::default();
    let mut out = io::stdout().lock();
    loop {
        pump_messages();
        match rx.recv_timeout(Duration::from_millis(10)) {
            Ok((proto::QUIT, _)) => break,
            Ok((op, p)) => {
                let r = st.handle(op, &p);
                let w = match r {
                    Ok(b) => proto::write_frame(&mut out, proto::OK, &b),
                    Err(m) => proto::write_frame(&mut out, proto::ERROR, &Enc::default().str(m.as_bytes()).0),
                };
                if w.is_err() {
                    break;
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    let _ = out.flush();
    // unload libraries before exit (final functions have already run)
    drop(st);
}
