//! `external_define` / `external_call` / `external_free` (extensions.md §5, §6).
//!
//! DLLs built for this process's architecture are loaded in-process with
//! `libloading` and called through `gm_dllcall`'s typed shims. 32-bit x86 DLLs
//! (all GM 8.1-era DLLs) run in the `gm81-dllhost32` helper process when the runner
//! is 64-bit. The helper is located via `GM81_DLLHOST32`, next to the runner, or in
//! a cargo `target/i686-pc-windows-msvc/{debug,release}` directory.

use gm_dllcall::proto::{self, Dec, Enc, PVal};
use gm_dllcall::{Arg, Machine, Ret};
use gml::{Error, Value};
use std::ffi::{c_void, CString};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

/// `dll_cdecl` / `dll_stdcall`, `ty_real` / `ty_string` (extensions.md §7).
pub const DLL_STDCALL: i32 = 1;
pub const TY_STRING: i32 = 1;

enum Backend {
    InProc(libloading::Library),
    Helper(u32),
}

struct Lib {
    /// Lower-case file name (for `external_free`).
    key: String,
    path: PathBuf,
    backend: Option<Backend>,
}

enum FnBackend {
    InProc(*const c_void),
    Helper(u32),
}

struct Def {
    lib: usize,
    name: String,
    backend: FnBackend,
    stdcall: bool,
    ret_str: bool,
    types: Vec<bool>,
}

struct Helper {
    child: Child,
    tx: BufWriter<ChildStdin>,
    rx: BufReader<ChildStdout>,
}

impl Helper {
    fn request(&mut self, op: u8, payload: &[u8]) -> Result<Vec<u8>, String> {
        proto::write_frame(&mut self.tx, op, payload).map_err(|_| "external DLL crashed".to_string())?;
        match proto::read_frame(&mut self.rx) {
            Ok((proto::OK, p)) => Ok(p),
            Ok((_, p)) => Err(Dec::new(&p).str().map(|s| String::from_utf8_lossy(&s).to_string()).unwrap_or_default()),
            Err(_) => Err("external DLL crashed".to_string()),
        }
    }
}

impl Drop for Helper {
    fn drop(&mut self) {
        let _ = proto::write_frame(&mut self.tx, proto::QUIT, &[]);
        let t0 = std::time::Instant::now();
        while t0.elapsed() < std::time::Duration::from_millis(500) {
            if let Ok(Some(_)) = self.child.try_wait() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// All DLL state of a game.
#[derive(Default)]
pub struct Dlls {
    libs: Vec<Lib>,
    defs: Vec<Option<Def>>,
    helper: Option<Helper>,
    helper_dead: bool,
    /// Directories searched for a DLL name without path (§5.1).
    pub search: Vec<PathBuf>,
    /// Current directory given to the helper.
    pub cwd: String,
    /// Print DLL loads (debug mode, §6.4).
    pub verbose: bool,
}

/// Locates the 32-bit helper executable.
pub fn find_helper() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("GM81_DLLHOST32") {
        let p = PathBuf::from(p);
        if p.is_file() {
            return Some(p);
        }
    }
    let exe = std::env::current_exe().ok()?;
    let name = "gm81-dllhost32.exe";
    let mut dir = exe.parent();
    for _ in 0..4 {
        let d = dir?;
        for c in [
            d.join(name),
            d.join("i686-pc-windows-msvc").join("release").join(name),
            d.join("i686-pc-windows-msvc").join("debug").join(name),
            d.join("target").join("i686-pc-windows-msvc").join("release").join(name),
            d.join("target").join("i686-pc-windows-msvc").join("debug").join(name),
        ] {
            if c.is_file() && gm_dllcall::pe_machine_of(&c) == Some(Machine::X86) {
                return Some(c);
            }
        }
        dir = d.parent();
    }
    None
}

fn cstring(b: &[u8]) -> CString {
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    CString::new(&b[..end]).unwrap_or_default()
}

impl Dlls {
    fn helper(&mut self) -> Result<&mut Helper, String> {
        if self.helper_dead {
            return Err("external DLL crashed".into());
        }
        if self.helper.is_none() {
            let exe = find_helper().ok_or("32-bit DLL helper gm81-dllhost32.exe not found")?;
            let mut cmd = Command::new(&exe);
            cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::inherit());
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
            }
            let mut child = cmd.spawn().map_err(|e| format!("cannot start {}: {}", exe.display(), e))?;
            let tx = BufWriter::new(child.stdin.take().ok_or("helper stdin")?);
            let rx = BufReader::new(child.stdout.take().ok_or("helper stdout")?);
            let mut h = Helper { child, tx, rx };
            if !self.cwd.is_empty() {
                let _ = h.request(proto::SET_CWD, &Enc::default().str(self.cwd.as_bytes()).0);
            }
            self.helper = Some(h);
        }
        Ok(self.helper.as_mut().unwrap())
    }

    fn helper_request(&mut self, op: u8, payload: &[u8]) -> Result<Vec<u8>, String> {
        let r = self.helper()?.request(op, payload);
        if let Err(m) = &r {
            if m == "external DLL crashed" {
                self.helper_dead = true;
                self.helper = None;
            }
        }
        r
    }

    /// Resolves a DLL name to a file (working dir, program dir, temp dirs), or
    /// `None` to leave it to the system search.
    fn resolve(&self, dll: &str) -> Option<PathBuf> {
        let p = Path::new(dll);
        if p.is_absolute() {
            return p.is_file().then(|| p.to_path_buf());
        }
        self.search.iter().map(|d| d.join(p)).find(|c| c.is_file())
    }

    fn load(&mut self, dll: &str) -> Result<usize, String> {
        let path = self.resolve(dll);
        let key_path = path.clone().unwrap_or_else(|| PathBuf::from(dll));
        let canon = std::fs::canonicalize(&key_path).unwrap_or(key_path.clone());
        if let Some(i) = self.libs.iter().position(|l| l.backend.is_some() && l.path == canon) {
            return Ok(i);
        }
        let key = Path::new(dll).file_name().map(|f| f.to_string_lossy().to_lowercase()).unwrap_or_default();
        let machine = path.as_deref().and_then(gm_dllcall::pe_machine_of);
        let in_proc = match machine {
            Some(m) => m == gm_dllcall::host_machine(),
            None => true,
        };
        if self.verbose {
            eprintln!("loading DLL {} ({:?}, {})", canon.display(), machine, if in_proc { "in-process" } else { "helper" });
        }
        let backend = if in_proc {
            let lib = unsafe { libloading::Library::new(&key_path) }.map_err(|e| format!("cannot load {}: {}", dll, e))?;
            Backend::InProc(lib)
        } else if machine == Some(Machine::X86) {
            let p = self.helper_request(proto::LOAD, &Enc::default().str(canon.to_string_lossy().as_bytes()).0)?;
            Backend::Helper(Dec::new(&p).u32().map_err(|e| e.to_string())?)
        } else {
            return Err(format!("{} is built for an unsupported architecture", dll));
        };
        self.libs.push(Lib {
            key,
            path: canon,
            backend: Some(backend),
        });
        Ok(self.libs.len() - 1)
    }

    /// `external_define`: returns the new id.
    pub fn define(&mut self, dll: &str, name: &str, calltype: i32, restype: i32, types: &[i32]) -> Result<usize, String> {
        let types: Vec<bool> = types.iter().map(|&t| t == TY_STRING).collect();
        if !gm_dllcall::signature_supported(&types) {
            return Err("too many arguments or unsupported argument types".into());
        }
        let li = self.load(dll)?;
        let stdcall = calltype == DLL_STDCALL;
        let ret_str = restype == TY_STRING;
        let backend = match self.libs[li].backend.as_ref().unwrap() {
            Backend::InProc(lib) => {
                let mut fp = None;
                for cand in gm_dllcall::symbol_candidates(name, &types, std::mem::size_of::<usize>()) {
                    let mut n = cand.into_bytes();
                    n.push(0);
                    if let Ok(s) = unsafe { lib.get::<*const c_void>(&n) } {
                        fp = Some(*s);
                        break;
                    }
                }
                FnBackend::InProc(fp.ok_or_else(|| format!("function {} not found in {}", name, dll))?)
            }
            Backend::Helper(h) => {
                let mut e = Enc::default().u32(*h).str(name.as_bytes()).u8(stdcall as u8).u8(ret_str as u8).u8(types.len() as u8);
                for t in &types {
                    e = e.u8(*t as u8);
                }
                let p = self.helper_request(proto::DEFINE, &e.0)?;
                FnBackend::Helper(Dec::new(&p).u32().map_err(|e| e.to_string())?)
            }
        };
        self.defs.push(Some(Def {
            lib: li,
            name: name.to_string(),
            backend,
            stdcall,
            ret_str,
            types,
        }));
        Ok(self.defs.len() - 1)
    }

    /// Number of arguments of a defined function.
    pub fn arity(&self, id: usize) -> Option<usize> {
        self.defs.get(id).and_then(|d| d.as_ref()).map(|d| d.types.len())
    }

    /// `external_call`.
    pub fn call(&mut self, id: usize, args: &[Value]) -> Result<Value, Error> {
        let d = self
            .defs
            .get(id)
            .and_then(|d| d.as_ref())
            .ok_or_else(|| Error::new(format!("Unknown external function id {}.", id)))?;
        if args.len() != d.types.len() {
            return Err(Error::new(format!("Wrong number of arguments to external function {}.", d.name)));
        }
        for (a, s) in args.iter().zip(&d.types) {
            if a.is_str() != *s {
                return Err(Error::new(format!("Wrong type of arguments to external function {}.", d.name)));
            }
        }
        match d.backend {
            FnBackend::InProc(fp) => {
                let strings: Vec<CString> = args.iter().filter_map(|a| a.as_bytes().map(cstring)).collect();
                let mut si = 0;
                let cargs: Vec<Arg> = args
                    .iter()
                    .map(|a| match a {
                        Value::Real(x) => Arg::Real(*x),
                        Value::Str(_) => {
                            si += 1;
                            Arg::Ptr(strings[si - 1].as_ptr())
                        }
                    })
                    .collect();
                let r = unsafe { gm_dllcall::invoke(fp, d.stdcall, &cargs, d.ret_str) }
                    .ok_or_else(|| Error::new("Unsupported external function signature."))?;
                Ok(match r {
                    Ret::Real(x) => Value::Real(x),
                    Ret::Ptr(p) => Value::from(unsafe { gm_dllcall::copy_cstr(p) }),
                })
            }
            FnBackend::Helper(fid) => {
                let mut e = Enc::default().u32(fid).u8(args.len() as u8);
                for a in args {
                    e = e.val(&match a {
                        Value::Real(x) => PVal::Real(*x),
                        Value::Str(s) => PVal::Str(s.as_bytes().to_vec()),
                    });
                }
                let name = d.name.clone();
                let p = self.helper_request(proto::CALL, &e.0).map_err(|m| Error::new(format!("{} ({})", m, name)))?;
                match Dec::new(&p).val() {
                    Ok(PVal::Real(x)) => Ok(Value::Real(x)),
                    Ok(PVal::Str(s)) => Ok(Value::from(s)),
                    Err(_) => Err(Error::new("external DLL crashed")),
                }
            }
        }
    }

    /// `external_free`: unloads every library with that file name.
    pub fn free(&mut self, dll: &str) {
        let key = Path::new(dll).file_name().map(|f| f.to_string_lossy().to_lowercase()).unwrap_or_default();
        for li in 0..self.libs.len() {
            if self.libs[li].key != key || self.libs[li].backend.is_none() {
                continue;
            }
            for d in self.defs.iter_mut() {
                if d.as_ref().map(|d| d.lib == li).unwrap_or(false) {
                    *d = None;
                }
            }
            match self.libs[li].backend.take() {
                Some(Backend::Helper(h)) => {
                    let _ = self.helper_request(proto::FREE, &Enc::default().u32(h).0);
                }
                Some(Backend::InProc(lib)) => drop(lib),
                None => {}
            }
        }
    }

    /// Unloads everything and stops the helper.
    pub fn shutdown(&mut self) {
        self.defs.clear();
        for l in self.libs.iter_mut() {
            l.backend = None;
        }
        self.libs.clear();
        self.helper = None;
    }
}

impl Drop for Dlls {
    fn drop(&mut self) {
        self.shutdown();
    }
}
