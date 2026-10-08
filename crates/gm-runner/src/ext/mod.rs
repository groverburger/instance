//! Extension packages (specs/extensions.md): loading the packages a project uses,
//! registering their functions and constants, init/final functions, and the DLL
//! bridge behind `external_define` / `external_call` / `external_free`.
//!
//! Extension functions are callable like scripts: [`gml::Host::resolve_script`]
//! maps their names to indices `EXT_SCRIPT_BASE + n`, and
//! [`gml::Host::call_script`] dispatches those here.

pub mod bundle;
#[cfg_attr(target_arch = "wasm32", path = "dll_web.rs")]
pub mod dll;

use crate::builtins::{r, zero, Reg};
use crate::game::Game;
use gm_format::extpkg::{self, ExtPackage};
use gml::{Ctx, Error, Program, Value, Vm};
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

/// Script-index base of extension functions.
pub const EXT_SCRIPT_BASE: i32 = 0x2000_0000;

enum Kind {
    /// A `#define` section of a GML file.
    Gml {
        code: String,
        prog: Option<Rc<Program>>,
        error: Option<String>,
    },
    /// A DLL export defined through the bridge (`external_define` id), or the
    /// error that prevented defining it.
    Dll(Result<usize, String>),
}

struct Func {
    name: String,
    /// Package and file it belongs to (for init/final lookup and messages).
    package: String,
    file: usize,
    kind: Kind,
}

/// Loaded extensions of a game.
#[derive(Default)]
pub struct Extensions {
    funcs: Vec<Func>,
    names: HashMap<String, usize>,
    /// (package, file index, function name) in package/file order.
    inits: Vec<(String, usize, String)>,
    finals: Vec<(String, usize, String)>,
    /// Packages that were loaded (name, origin).
    pub loaded: Vec<(String, String)>,
    pub dlls: dll::Dlls,
    /// `<temp>/gm81_<game_id>_<pid>/` holding extracted DLL/other files.
    pub temp_dir: Option<PathBuf>,
    inits_done: bool,
    finals_done: bool,
}

impl Drop for Extensions {
    fn drop(&mut self) {
        self.dlls.shutdown();
        if let Some(d) = self.temp_dir.take() {
            // a just-exited helper process may hold the DLL image for a moment
            for _ in 0..50 {
                if crate::vfs::remove_dir_all(&d).is_ok() || !crate::vfs::exists(&d) || cfg!(target_arch = "wasm32") {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
        }
    }
}

impl Extensions {
    /// Script index of an extension function name.
    pub fn resolve(&self, name: &str) -> Option<i32> {
        self.names.get(name).map(|&i| EXT_SCRIPT_BASE + i as i32)
    }

    /// Functions that cannot be called (DLL export not defined, GML not compiled),
    /// with the reason.
    pub fn unavailable(&self) -> Vec<(String, String)> {
        self.funcs
            .iter()
            .filter_map(|f| match &f.kind {
                Kind::Dll(Err(e)) => Some((f.name.clone(), e.clone())),
                Kind::Gml { error: Some(e), .. } => Some((f.name.clone(), e.clone())),
                _ => None,
            })
            .collect()
    }

    /// Names of all registered extension functions.
    pub fn function_names(&self) -> impl Iterator<Item = &str> {
        self.funcs.iter().map(|f| f.name.as_str())
    }
}

/// Splits GML text into `#define` sections: (name, body).
pub fn split_defines(text: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for line in text.split_inclusive('\n') {
        let t = line.trim_start();
        if let Some(rest) = t.strip_prefix("#define") {
            if rest.is_empty() || rest.starts_with(|c: char| c.is_whitespace()) {
                let name = rest.split_whitespace().next().unwrap_or("").to_string();
                out.push((name, String::new()));
                continue;
            }
        }
        if let Some(last) = out.last_mut() {
            last.1.push_str(line);
        }
    }
    out
}

fn eval_constant(g: &Game, vm: &mut Vm, name: &str, value: &str) -> Value {
    match vm.compile_expression(&format!("extension constant {}", name), value.as_bytes(), g) {
        Ok(prog) => {
            let mut tmp = crate::host::ConstHost;
            vm.run(&mut tmp, &prog, Ctx::new(gml::special::NOONE, gml::special::NOONE), &[])
                .unwrap_or_else(|_| Value::from(value))
        }
        Err(_) => Value::from(value),
    }
}

/// Loads every package the project uses (§2.3 steps 1–4). Called while building the
/// game, before user constants and code are compiled. Missing packages are an error.
pub fn load(g: &mut Game, vm: &mut Vm, packages: &[String]) -> Result<(), String> {
    let mut ext = std::mem::take(&mut g.ext);
    ext.dlls.verbose = std::env::var_os("GM81_DEBUG_DLL").is_some();
    let r = load_into(g, &mut ext, vm, packages);
    g.ext = ext;
    r
}

fn load_into(g: &mut Game, ext: &mut Extensions, vm: &mut Vm, packages: &[String]) -> Result<(), String> {
    for pname in packages {
        let files = bundle::find_package(pname).ok_or_else(|| format!("extension package {} not found", pname))?;
        let pkg: ExtPackage = extpkg::read_ged(&files.ged).map_err(|e| format!("extension package {}: {}", pname, e))?;
        let contents = extpkg::read_dat(&files.dat, pkg.files.len()).map_err(|e| format!("extension package {}: {}", pname, e))?;
        ext.loaded.push((pkg.name.clone(), files.origin.clone()));
        for (fi, (f, bytes)) in pkg.files.iter().zip(contents).enumerate() {
            match f.kind {
                extpkg::FILE_GML => {
                    let text = gm_format::text::decode_cp1252(&bytes);
                    let sections: HashMap<String, String> = split_defines(&text).into_iter().collect();
                    let mut used = std::collections::HashSet::new();
                    for func in &f.functions {
                        let sec = if func.external_name.is_empty() { &func.name } else { &func.external_name };
                        used.insert(sec.clone());
                        let code = sections.get(sec).cloned();
                        let kind = match code {
                            Some(code) => Kind::Gml { code, prog: None, error: None },
                            None => Kind::Gml {
                                code: String::new(),
                                prog: None,
                                error: Some(format!("extension function {} has no #define section", func.name)),
                            },
                        };
                        register(g, ext, &func.name, pname, fi, kind);
                    }
                    // sections without a record (e.g. init helpers) stay callable
                    for (name, code) in sections {
                        if !used.contains(&name) && !name.is_empty() && !ext.names.contains_key(&name) && !g.script_names.contains_key(&name) {
                            register(g, ext, &name, pname, fi, Kind::Gml { code, prog: None, error: None });
                        }
                    }
                }
                extpkg::FILE_DLL | extpkg::FILE_OTHER => {
                    let dir = temp_dir(g, ext).map_err(|e| format!("extension package {}: {}", pname, e))?;
                    let path = dir.join(&f.file_name);
                    crate::vfs::write(&path, &bytes).map_err(|e| format!("cannot write {}: {}", path.display(), e))?;
                    if f.kind == extpkg::FILE_DLL {
                        let p = path.to_string_lossy().to_string();
                        for func in &f.functions {
                            let export = if func.external_name.is_empty() { &func.name } else { &func.external_name };
                            let n = func.argument_count.clamp(0, 16) as usize;
                            // record codes: 1 string, 2 real → ty_string (1) / ty_real (0)
                            let types: Vec<i32> = func.arg_types[..n].iter().map(|&t| (t == extpkg::ARG_STRING) as i32).collect();
                            // observed: the shipped DLLs' code-12 exports are cdecl (EXT-3
                            // note); the x86 bridge is convention-agnostic anyway.
                            let conv = if func.call_convention == 11 { dll::DLL_STDCALL } else { 0 };
                            let res = (func.return_type == extpkg::ARG_STRING) as i32;
                            let id = ext.dlls.define(&p, export, conv, res, &types);
                            if let Err(e) = &id {
                                eprintln!("extension {}: cannot define {}: {}", pname, func.name, e);
                            }
                            register(g, ext, &func.name, pname, fi, Kind::Dll(id));
                        }
                    }
                }
                _ => {
                    // action libraries only affect the IDE palette
                }
            }
            if !f.init_function.is_empty() {
                ext.inits.push((pname.clone(), fi, f.init_function.clone()));
            }
            if !f.final_function.is_empty() {
                ext.finals.push((pname.clone(), fi, f.final_function.clone()));
            }
            for c in &f.constants {
                let v = eval_constant(g, vm, &c.name, &c.value);
                if g.constants.contains_key(&c.name) {
                    eprintln!("extension {}: constant {} duplicates an existing name", pname, c.name);
                }
                g.constants.insert(c.name.clone(), v);
            }
        }
    }
    Ok(())
}

fn register(g: &Game, ext: &mut Extensions, name: &str, package: &str, file: usize, kind: Kind) {
    if gml::builtins::function_id(name).is_some() || g.script_names.contains_key(name) || ext.names.contains_key(name) {
        eprintln!("extension {}: function {} duplicates an existing name", package, name);
    }
    ext.funcs.push(Func {
        name: name.to_string(),
        package: package.to_string(),
        file,
        kind,
    });
    ext.names.entry(name.to_string()).or_insert(ext.funcs.len() - 1);
}

fn temp_dir(g: &Game, ext: &mut Extensions) -> std::io::Result<PathBuf> {
    if let Some(d) = &ext.temp_dir {
        return Ok(d.clone());
    }
    static SEQ: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let pid = if cfg!(target_arch = "wasm32") { 0 } else { std::process::id() };
    let base = format!("gm81_{}_{}", g.game_id, pid);
    let d = crate::vfs::temp_dir().join(if n == 0 { base } else { format!("{base}_{n}") });
    crate::vfs::create_dir_all(&d)?;
    ext.temp_dir = Some(d.clone());
    Ok(d)
}

/// Compiles the GML extension functions (after every name is registered).
pub fn compile(g: &mut Game, vm: &mut Vm) {
    let mut results = Vec::new();
    for (i, f) in g.ext.funcs.iter().enumerate() {
        if let Kind::Gml { code, error: None, .. } = &f.kind {
            match vm.compile(&f.name, code.as_bytes(), &*g) {
                Ok(p) => results.push((i, Ok(p))),
                Err(e) => results.push((i, Err(format!("COMPILATION ERROR in extension function {}\n{}", f.name, e)))),
            }
        }
    }
    let mut errors = Vec::new();
    for (i, r) in results {
        if let Kind::Gml { prog, error, .. } = &mut g.ext.funcs[i].kind {
            match r {
                Ok(p) => *prog = Some(p),
                Err(e) => {
                    errors.push(e.clone());
                    *error = Some(e);
                }
            }
        }
    }
    for e in errors {
        g.ui_error(&e, false);
    }
    // the bridge searches the game's directories for DLL names without a path
    let mut search = vec![PathBuf::from(&g.working_directory), PathBuf::from(&g.program_directory)];
    if let Some(t) = &g.ext.temp_dir {
        search.push(t.clone());
    }
    search.push(PathBuf::from(&g.temp_directory));
    g.ext.dlls.search = search;
    g.ext.dlls.cwd = g.working_directory.clone();
}

/// Calls extension function `index` (script index minus [`EXT_SCRIPT_BASE`]).
pub fn call(g: &mut Game, vm: &mut Vm, ctx: &Ctx, index: usize, args: &[Value]) -> Result<Value, Error> {
    let f = g.ext.funcs.get(index).ok_or_else(|| Error::new(format!("Unknown extension function {}", index)))?;
    match &f.kind {
        Kind::Gml { prog: Some(p), .. } => {
            let p = p.clone();
            vm.run(g, &p, *ctx, args)
        }
        Kind::Gml { error, .. } => Err(Error::new(error.clone().unwrap_or_else(|| format!("Extension function {} failed to compile", f.name)))),
        Kind::Dll(Ok(id)) => {
            let id = *id;
            g.ext.dlls.call(id, args)
        }
        Kind::Dll(Err(e)) => Err(Error::new(format!("Extension function {} is not available: {}", f.name, e))),
    }
}

fn run_named(g: &mut Game, vm: &mut Vm, package: &str, file: usize, name: &str) -> Result<(), Error> {
    let idx = g.ext.funcs.iter().position(|f| f.name == name && f.package == package && f.file == file);
    match idx {
        Some(i) => {
            let ctx = Ctx::new(gml::special::NOONE, gml::special::NOONE);
            call(g, vm, &ctx, i, &[]).map(|_| ())
        }
        None => {
            eprintln!("extension {}: init/final function {} not found", package, name);
            Ok(())
        }
    }
}

/// Calls every init function (package order, then file order) — once, before the
/// first room (§3.4).
pub fn run_inits(g: &mut Game, vm: &mut Vm) -> Result<(), Error> {
    if g.ext.inits_done {
        return Ok(());
    }
    g.ext.inits_done = true;
    let list = g.ext.inits.clone();
    for (p, f, n) in list {
        if let Err(e) = run_named(g, vm, &p, f, &n) {
            g.report_error(e, -1)?;
        }
    }
    Ok(())
}

/// Calls every final function in reverse order (§3.4) — once.
pub fn run_finals(g: &mut Game, vm: &mut Vm) {
    if g.ext.finals_done || !g.ext.inits_done {
        return;
    }
    g.ext.finals_done = true;
    let list = g.ext.finals.clone();
    for (p, f, n) in list.into_iter().rev() {
        if let Err(e) = run_named(g, vm, &p, f, &n) {
            eprintln!("extension {}: final function {}: {}", p, n, e);
        }
    }
}

/// Registers `external_define`, `external_call`, `external_free`.
pub fn register_builtins(l: &mut Reg) {
    l.push(("external_define", |g, _, _, a| {
        let dll = a.s(0)?;
        let name = a.s(1)?;
        let calltype = a.i(2)?;
        let restype = a.i(3)?;
        let n = a.i(4)?;
        let ok_count = (0..=16).contains(&n) && a.len() == 5 + n as usize;
        let mut types = Vec::new();
        if ok_count {
            for i in 0..n as usize {
                types.push(a.i(5 + i)?);
            }
        }
        let res = if !ok_count || ![0, 1].contains(&calltype) || ![0, 1].contains(&restype) || types.iter().any(|t| ![0, 1].contains(t)) {
            Err("bad type or argument count".to_string())
        } else {
            g.ext.dlls.define(&dll, &name, calltype, restype, &types)
        };
        match res {
            Ok(id) => r(id as f64),
            Err(e) => {
                if g.ext.dlls.verbose {
                    eprintln!("external_define({}, {}): {}", dll, name, e);
                }
                g.report_error(Error::new("Error defining an external function."), -1)?;
                r(-1.0)
            }
        }
    }));
    l.push(("external_call", |g, _, _, a| {
        let id = a.i(0)?;
        if id < 0 {
            return Err(Error::new(format!("Unknown external function id {}.", id)));
        }
        g.ext.dlls.call(id as usize, &a.v[1..])
    }));
    l.push(("external_free", |g, _, _, a| {
        let dll = a.s(0)?;
        g.ext.dlls.free(&dll);
        zero()
    }));
    l.push(("get_function_address", |_, _, _, _| zero()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defines_split() {
        let s = split_defines("#define a\nreturn 1\n#define  b  \r\nx = 2;\n#definex\n");
        assert_eq!(s.len(), 2);
        assert_eq!(s[0], ("a".into(), "return 1\n".into()));
        assert_eq!(s[1].0, "b");
        assert!(s[1].1.contains("#definex"));
    }
}
