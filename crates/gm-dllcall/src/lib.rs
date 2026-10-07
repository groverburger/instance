//! Native DLL calls for GM-style external functions (specs/extensions.md §5, §6):
//! typed call shims, PE architecture detection and the `gm81-dllhost32` helper
//! protocol (§6.2). Shared by the runner (x64 in-process calls) and the 32-bit helper.

use std::ffi::{c_char, c_void};

mod shim;
pub mod proto;

/// One argument of a native call.
#[derive(Clone, Copy, Debug)]
pub enum Arg {
    Real(f64),
    /// NUL-terminated ANSI string, valid for the duration of the call.
    Ptr(*const c_char),
}

impl Arg {
    #[inline]
    pub fn real(&self) -> f64 {
        match self {
            Arg::Real(x) => *x,
            Arg::Ptr(_) => 0.0,
        }
    }
    #[inline]
    pub fn ptr(&self) -> *const c_char {
        match self {
            Arg::Ptr(p) => *p,
            Arg::Real(_) => std::ptr::null(),
        }
    }
}

/// Raw result of a native call.
#[derive(Clone, Copy, Debug)]
pub enum Ret {
    Real(f64),
    Ptr(*const c_char),
}

pub(crate) fn string_mask(a: &[Arg]) -> u32 {
    a.iter()
        .enumerate()
        .filter(|(_, x)| matches!(x, Arg::Ptr(_)))
        .map(|(i, _)| 1u32 << i)
        .sum()
}

/// Whether GM 8.1 allows this signature: ≤ 4 arguments of any type, or up to 16
/// arguments all real.
pub fn signature_supported(arg_is_string: &[bool]) -> bool {
    arg_is_string.len() <= 4 || (arg_is_string.len() <= 16 && arg_is_string.iter().all(|s| !s))
}

/// Calls a native function.
///
/// # Safety
/// `fp` must point to a function taking exactly `args` (reals as `double`, strings as
/// `char*`) and returning `double` (`ret_str == false`) or `char*`, with the given
/// convention (`stdcall` only matters on 32-bit x86).
pub unsafe fn invoke(fp: *const c_void, stdcall: bool, args: &[Arg], ret_str: bool) -> Option<Ret> {
    #[cfg(target_arch = "x86")]
    {
        let _ = stdcall;
        if !signature_supported(&args.iter().map(|a| matches!(a, Arg::Ptr(_))).collect::<Vec<_>>()) {
            return None;
        }
        Some(x86::call_any(fp, args, ret_str))
    }
    #[cfg(not(target_arch = "x86"))]
    {
        let _ = stdcall;
        shim::call_c(fp, args, ret_str)
    }
}

/// Typed calls with an explicit convention (the x86 path of [`invoke`] uses a
/// convention-agnostic thunk instead).
///
/// # Safety
/// As [`invoke`].
pub unsafe fn invoke_typed(fp: *const c_void, stdcall: bool, args: &[Arg], ret_str: bool) -> Option<Ret> {
    #[cfg(target_arch = "x86")]
    if stdcall {
        return shim::call_stdcall(fp, args, ret_str);
    }
    let _ = stdcall;
    shim::call_c(fp, args, ret_str)
}

#[cfg(target_arch = "x86")]
mod x86 {
    //! Convention-agnostic 32-bit call thunk: arguments are pushed exactly as both
    //! cdecl and stdcall expect (right to left; doubles as 8 bytes, pointers as 4),
    //! and ESP is restored from a callee-saved register afterwards, so it does not
    //! matter whether the callee pops its arguments. The x87 control word is
    //! restored after the call (DLL runtimes such as Delphi's change it).
    use super::{Arg, Ret};
    use std::arch::asm;
    use std::ffi::{c_char, c_void};

    pub unsafe fn call_any(fp: *const c_void, args: &[Arg], ret_str: bool) -> Ret {
        let mut words: Vec<u32> = Vec::with_capacity(args.len() * 2);
        for a in args {
            match a {
                Arg::Real(x) => {
                    let b = x.to_bits();
                    words.push(b as u32);
                    words.push((b >> 32) as u32);
                }
                Arg::Ptr(p) => words.push(*p as usize as u32),
            }
        }
        let mut cw: u16 = 0;
        asm!("fnstcw word ptr [{0}]", in(reg) &mut cw as *mut u16, options(nostack));
        let n = words.len();
        let ptr = words.as_ptr();
        let r = if ret_str {
            let eax: u32;
            asm!(
                "mov edi, esp",
                "2:",
                "test ecx, ecx",
                "jz 3f",
                "dec ecx",
                "push dword ptr [edx + ecx*4]",
                "jmp 2b",
                "3:",
                "call eax",
                "mov esp, edi",
                inout("eax") fp => eax,
                inout("ecx") n => _,
                inout("edx") ptr => _,
                out("edi") _,
                clobber_abi("C"),
            );
            Ret::Ptr(eax as usize as *const c_char)
        } else {
            let lo: u32;
            let hi: u32;
            asm!(
                "mov edi, esp",
                "2:",
                "test ecx, ecx",
                "jz 3f",
                "dec ecx",
                "push dword ptr [edx + ecx*4]",
                "jmp 2b",
                "3:",
                "call eax",
                "mov esp, edi",
                "sub esp, 8",
                "fstp qword ptr [esp]",
                "pop eax",
                "pop edx",
                inout("eax") fp => lo,
                inout("ecx") n => _,
                inout("edx") ptr => hi,
                out("edi") _,
                clobber_abi("C"),
            );
            Ret::Real(f64::from_bits(((hi as u64) << 32) | lo as u64))
        };
        asm!("fnclex", "fldcw word ptr [{0}]", in(reg) &cw as *const u16, options(nostack));
        drop(words);
        r
    }
}

/// Copies a returned C string (NULL → empty).
///
/// # Safety
/// `p` must be NULL or point to a NUL-terminated string.
pub unsafe fn copy_cstr(p: *const c_char) -> Vec<u8> {
    if p.is_null() {
        Vec::new()
    } else {
        std::ffi::CStr::from_ptr(p).to_bytes().to_vec()
    }
}

/// CPU architecture of a PE image.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Machine {
    X86,
    X64,
    Other(u16),
}

/// Reads the PE header machine field (§6: 0x014C = x86, 0x8664 = x64).
pub fn pe_machine(bytes: &[u8]) -> Option<Machine> {
    if bytes.len() < 0x40 || &bytes[..2] != b"MZ" {
        return None;
    }
    let off = u32::from_le_bytes(bytes[0x3C..0x40].try_into().ok()?) as usize;
    let h = bytes.get(off..off + 6)?;
    if &h[..4] != b"PE\0\0" {
        return None;
    }
    Some(match u16::from_le_bytes([h[4], h[5]]) {
        0x014C => Machine::X86,
        0x8664 => Machine::X64,
        m => Machine::Other(m),
    })
}

/// Reads the machine of a PE file on disk (only the headers are read).
pub fn pe_machine_of(path: &std::path::Path) -> Option<Machine> {
    use std::io::Read;
    let mut f = std::fs::File::open(path).ok()?;
    let mut b = vec![0u8; 4096];
    let n = f.read(&mut b).ok()?;
    b.truncate(n);
    pe_machine(&b)
}

/// Export names to try for `name`: the name itself, then (for stdcall on x86) the
/// decorated forms `name@N` and `_name@N`, where N is the argument stack size.
pub fn symbol_candidates(name: &str, arg_is_string: &[bool], ptr_size: usize) -> Vec<String> {
    let n: usize = arg_is_string.iter().map(|s| if *s { ptr_size } else { 8 }).sum();
    vec![name.to_string(), format!("{name}@{n}"), format!("_{name}@{n}"), format!("_{name}")]
}

/// Machine of the current process.
pub const fn host_machine() -> Machine {
    if cfg!(target_arch = "x86") {
        Machine::X86
    } else if cfg!(target_arch = "x86_64") {
        Machine::X64
    } else {
        Machine::Other(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    extern "C" fn add3(a: f64, s: *const c_char, b: f64) -> f64 {
        let l = unsafe { std::ffi::CStr::from_ptr(s) }.to_bytes().len();
        a + b + l as f64
    }
    extern "C" fn sum16(
        a: f64, b: f64, c: f64, d: f64, e: f64, f: f64, g: f64, h: f64,
        i: f64, j: f64, k: f64, l: f64, m: f64, n: f64, o: f64, p: f64,
    ) -> f64 {
        a + b + c + d + e + f + g + h + i + j + k + l + m + n + o + p
    }
    extern "C" fn hello(s: *const c_char) -> *const c_char {
        let _ = s;
        c"hello".as_ptr()
    }

    #[test]
    fn shim_calls() {
        let s = std::ffi::CString::new("abcd").unwrap();
        let r = unsafe { invoke(add3 as *const c_void, false, &[Arg::Real(1.0), Arg::Ptr(s.as_ptr()), Arg::Real(2.0)], false) };
        assert!(matches!(r, Some(Ret::Real(x)) if x == 7.0));
        let args: Vec<Arg> = (1..=16).map(|i| Arg::Real(i as f64)).collect();
        let r = unsafe { invoke(sum16 as *const c_void, true, &args, false) };
        assert!(matches!(r, Some(Ret::Real(x)) if x == 136.0));
        let r = unsafe { invoke(hello as *const c_void, false, &[Arg::Ptr(s.as_ptr())], true) };
        match r {
            Some(Ret::Ptr(p)) => assert_eq!(unsafe { copy_cstr(p) }, b"hello"),
            _ => panic!(),
        }
        // 5 arguments with a string: not a GM signature
        let args = [Arg::Real(0.0), Arg::Real(0.0), Arg::Real(0.0), Arg::Real(0.0), Arg::Ptr(s.as_ptr())];
        assert!(unsafe { invoke(add3 as *const c_void, false, &args, false) }.is_none());
        assert!(!signature_supported(&[false, false, false, false, true]));
        assert!(signature_supported(&[true, true, true, true]));
    }

    #[test]
    fn own_exe_machine() {
        let exe = std::env::current_exe().unwrap();
        assert_eq!(pe_machine_of(&exe), Some(host_machine()));
    }
}
