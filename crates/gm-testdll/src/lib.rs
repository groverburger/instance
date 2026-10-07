//! Test DLL for the runner's external-function bridge. Build for x64 normally and for
//! x86 with `--target i686-pc-windows-msvc`. Exports GM-style functions (doubles and
//! C strings) with both calling conventions (on x64 both are the C convention).

#![allow(clippy::missing_safety_doc)]

use std::ffi::{c_char, CStr};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

static COUNTER: AtomicU32 = AtomicU32::new(0);
static BUF: Mutex<Vec<u8>> = Mutex::new(Vec::new());

unsafe fn s(p: *const c_char) -> &'static [u8] {
    if p.is_null() {
        b""
    } else {
        CStr::from_ptr(p).to_bytes()
    }
}

/// Stores `b` in a static buffer and returns a pointer to it (like the manual's
/// example DLL returning a static string).
fn ret(b: &[u8]) -> *const c_char {
    let mut g = BUF.lock().unwrap();
    g.clear();
    g.extend_from_slice(b);
    g.push(0);
    g.as_ptr() as *const c_char
}

#[no_mangle]
pub extern "C" fn gmt_add(a: f64, b: f64) -> f64 {
    a + b
}

#[no_mangle]
pub unsafe extern "C" fn gmt_concat(a: *const c_char, b: *const c_char) -> *const c_char {
    let mut v = s(a).to_vec();
    v.extend_from_slice(s(b));
    ret(&v)
}

#[no_mangle]
pub unsafe extern "C" fn gmt_mixed(a: *const c_char, x: f64) -> f64 {
    s(a).len() as f64 * x
}

#[no_mangle]
pub unsafe extern "C" fn gmt_repeat(x: f64, a: *const c_char) -> *const c_char {
    ret(&s(a).repeat(x.max(0.0) as usize))
}

#[no_mangle]
pub unsafe extern "C" fn gmt_len4(a: *const c_char, b: *const c_char, c: *const c_char, d: *const c_char) -> f64 {
    (s(a).len() + s(b).len() + s(c).len() + s(d).len()) as f64
}

#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "C" fn gmt_sum16(
    a: f64, b: f64, c: f64, d: f64, e: f64, f: f64, g: f64, h: f64,
    i: f64, j: f64, k: f64, l: f64, m: f64, n: f64, o: f64, p: f64,
) -> f64 {
    a + 2.0 * b + 3.0 * c + 4.0 * d + e + f + g + h + i + j + k + l + m + n + o + 100.0 * p
}

#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "C" fn gmt_sum11(a: f64, b: f64, c: f64, d: f64, e: f64, f: f64, g: f64, h: f64, i: f64, j: f64, k: f64) -> f64 {
    a + 2.0 * b + 3.0 * c + 4.0 * d + e + f + g + h + i + j + 100.0 * k
}

#[no_mangle]
pub extern "C" fn gmt_counter() -> f64 {
    COUNTER.fetch_add(1, Ordering::SeqCst) as f64 + 1.0
}

#[no_mangle]
pub extern "C" fn gmt_bits() -> f64 {
    (std::mem::size_of::<usize>() * 8) as f64
}

#[cfg(target_arch = "x86")]
#[no_mangle]
pub extern "stdcall" fn gmt_add_std(a: f64, b: f64) -> f64 {
    a - b
}

#[cfg(target_arch = "x86")]
#[no_mangle]
pub unsafe extern "stdcall" fn gmt_concat_std(a: *const c_char, b: *const c_char) -> *const c_char {
    let mut v = s(b).to_vec();
    v.extend_from_slice(s(a));
    ret(&v)
}

#[cfg(not(target_arch = "x86"))]
#[no_mangle]
pub extern "C" fn gmt_add_std(a: f64, b: f64) -> f64 {
    a - b
}

#[cfg(not(target_arch = "x86"))]
#[no_mangle]
pub unsafe extern "C" fn gmt_concat_std(a: *const c_char, b: *const c_char) -> *const c_char {
    let mut v = s(b).to_vec();
    v.extend_from_slice(s(a));
    ret(&v)
}
