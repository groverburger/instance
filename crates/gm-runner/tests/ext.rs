//! Extension packages and DLL functions (specs/extensions.md).
//!
//! The DLL tests build `gm-testdll` (x64 and x86) and `gm81-dllhost32` (x86) into a
//! separate target directory on first use; they are skipped when the x86 target is
//! not installed.

use gm_format as fmt;
use gm_format::extpkg::{self, ExtConstant, ExtFile, ExtFunction, ExtPackage};
use gm_runner::ext::bundle::{self, PackageFiles};
use gm_runner::Game;
use gml::{Value, Vm};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn code(src: &str) -> fmt::Action {
    let mut a = fmt::builtin_action(1, 603).expect("code action").instantiate(1);
    a.args[0] = src.to_string();
    a
}

/// A project with one object whose Create event runs `create` and Game End runs
/// `end`.
fn project(create: &str, end: &str, packages: &[&str]) -> fmt::Project {
    let mut p = fmt::Project::default();
    p.settings.display_errors = true;
    p.objects.push(Some(fmt::Object {
        name: "obj".into(),
        sprite: -1,
        visible: true,
        events: vec![
            fmt::Event { event_type: 0, number: 0, actions: vec![code(create)] },
            fmt::Event { event_type: 7, number: 3, actions: vec![code(end)] },
        ],
        ..Default::default()
    }));
    p.rooms.push(Some(fmt::Room {
        name: "room0".into(),
        instances: vec![fmt::Instance { x: 0, y: 0, object: 0, id: 100001, creation_code: String::new(), locked: false }],
        ..Default::default()
    }));
    p.last_instance_id = 100002;
    p.room_order.push(0);
    p.extension_packages = packages.iter().map(|s| s.to_string()).collect();
    p
}

fn start(p: fmt::Project) -> (Game, Vm) {
    let mut vm = Vm::new();
    let mut g = Game::new(p, &mut vm, true).expect("game");
    g.start(&mut vm).expect("start");
    (g, vm)
}

fn global(vm: &mut Vm, name: &str) -> Value {
    let id = vm.intern(name);
    vm.globals().get_value(id).cloned().unwrap_or(Value::Real(-12345.0))
}

fn greal(vm: &mut Vm, name: &str) -> f64 {
    match global(vm, name) {
        Value::Real(r) => r,
        v => panic!("{} is {:?}", name, v),
    }
}

fn gstr(vm: &mut Vm, name: &str) -> String {
    match global(vm, name) {
        Value::Str(s) => s.to_string_lossy(),
        v => panic!("{} is {:?}", name, v),
    }
}

fn func(name: &str, argc: i32, ret: i32, args: &[i32]) -> ExtFunction {
    let mut t = [extpkg::ARG_REAL; 17];
    t[..args.len()].copy_from_slice(args);
    ExtFunction {
        name: name.into(),
        call_convention: 11,
        argument_count: argc,
        arg_types: t,
        return_type: ret,
        ..Default::default()
    }
}

fn preload(pkg: &ExtPackage, files: &[Vec<u8>]) {
    bundle::preload(vec![PackageFiles {
        name: pkg.name.clone(),
        ged: extpkg::write_ged(pkg),
        dat: extpkg::write_dat(files, 777),
        origin: "test".into(),
    }]);
}

#[test]
fn gml_extension_functions_constants_init_final() {
    let gml = "#define ext_twice\nreturn argument0 * 2 + EXT_K;\n#define ext_greet\nglobal.greeted = argument0 + ' ' + __ext_helper();\nreturn global.greeted;\n#define __ext_helper\nreturn 'world';\n#define __ext_init\nglobal.init_order = 'i';\n#define __ext_final\nglobal.final_ran = 1;\n";
    let pkg = ExtPackage {
        name: "Test GML Ext".into(),
        files: vec![ExtFile {
            file_name: "test.gml".into(),
            kind: extpkg::FILE_GML,
            init_function: "__ext_init".into(),
            final_function: "__ext_final".into(),
            functions: vec![
                func("ext_twice", 1, extpkg::ARG_REAL, &[extpkg::ARG_REAL]),
                func("ext_greet", 1, extpkg::ARG_STRING, &[extpkg::ARG_STRING]),
                ExtFunction { hidden: true, ..func("__ext_init", 0, extpkg::ARG_REAL, &[]) },
                ExtFunction { hidden: true, ..func("__ext_final", 0, extpkg::ARG_REAL, &[]) },
            ],
            constants: vec![
                ExtConstant { name: "EXT_K".into(), value: "10".into(), hidden: false },
                ExtConstant { name: "EXT_S".into(), value: "'abc'".into(), hidden: true },
                ExtConstant { name: "EXT_E".into(), value: "c_red + 1".into(), hidden: false },
            ],
            ..Default::default()
        }],
        ..Default::default()
    };
    preload(&pkg, &[gml.as_bytes().to_vec()]);
    let p = project(
        "global.init_seen = global.init_order; global.a = ext_twice(5); global.b = ext_greet('hello'); global.c = EXT_S; global.d = EXT_E;",
        "",
        &["Test GML Ext"],
    );
    // script_execute through the index of an extension function is not needed by GM
    // code; replace the last statement with a plain call.
    let mut p = p;
    if let Some(o) = p.objects[0].as_mut() {
        o.events[0].actions[0].args[0] = o.events[0].actions[0].args[0].replace("", "");
    }
    let (mut g, mut vm) = start(p);
    assert!(g.ext.unavailable().is_empty(), "{:?}", g.ext.unavailable());
    assert_eq!(gstr(&mut vm, "init_seen"), "i", "init ran before Create");
    assert_eq!(greal(&mut vm, "a"), 20.0);
    assert_eq!(gstr(&mut vm, "b"), "hello world");
    assert_eq!(gstr(&mut vm, "c"), "abc");
    assert_eq!(greal(&mut vm, "d"), 255.0 + 1.0);
    g.game_end(&mut vm).unwrap();
    assert_eq!(greal(&mut vm, "final_ran"), 1.0);
}

#[test]
fn missing_package_is_a_load_error() {
    let p = project("", "", &["No Such Package 123"]);
    let mut vm = Vm::new();
    let e = Game::new(p, &mut vm, true).err().expect("must fail");
    assert!(e.contains("extension package No Such Package 123 not found"), "{e}");
}

// ---------------------------------------------------------------- DLLs

struct Built {
    dll64: PathBuf,
    dll86: Option<PathBuf>,
}

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn cargo_build(args: &[&str], target_dir: &Path) -> bool {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    std::process::Command::new(cargo)
        .current_dir(workspace())
        .arg("build")
        .args(args)
        .arg("--target-dir")
        .arg(target_dir)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn built() -> &'static Built {
    static B: OnceLock<Built> = OnceLock::new();
    B.get_or_init(|| {
        let td = workspace().join("target").join("gm-ext-test");
        assert!(cargo_build(&["-p", "gm-testdll"], &td), "building gm-testdll");
        let dll64 = td.join("debug").join("gm_testdll.dll");
        let dll86 = if cargo_build(&["-p", "gm-testdll", "-p", "gm81-dllhost32", "--target", "i686-pc-windows-msvc"], &td) {
            let d = td.join("i686-pc-windows-msvc").join("debug");
            std::env::set_var("GM81_DLLHOST32", d.join("gm81-dllhost32.exe"));
            Some(d.join("gm_testdll.dll"))
        } else {
            eprintln!("x86 target not available; skipping x86 DLL tests");
            None
        };
        Built { dll64, dll86 }
    })
}

const DLL_CODE: &str = r#"
var d;
d = global.dll;
global.bits = external_call(external_define(d, "gmt_bits", dll_cdecl, ty_real, 0));
global.add = external_call(external_define(d, "gmt_add", dll_cdecl, ty_real, 2, ty_real, ty_real), 2.5, 4);
global.sub = external_call(external_define(d, "gmt_add_std", dll_stdcall, ty_real, 2, ty_real, ty_real), 10, 4);
global.cat = external_call(external_define(d, "gmt_concat", dll_cdecl, ty_string, 2, ty_string, ty_string), "foo", "bar");
global.cats = external_call(external_define(d, "gmt_concat_std", dll_stdcall, ty_string, 2, ty_string, ty_string), "foo", "bar");
global.mix = external_call(external_define(d, "gmt_mixed", dll_cdecl, ty_real, 2, ty_string, ty_real), "abcd", 1.5);
global.rep = external_call(external_define(d, "gmt_repeat", dll_cdecl, ty_string, 2, ty_real, ty_string), 3, "xy");
global.len4 = external_call(external_define(d, "gmt_len4", dll_cdecl, ty_real, 4, ty_string, ty_string, ty_string, ty_string), "a", "bb", "ccc", "");
global.s11 = external_call(external_define(d, "gmt_sum11", dll_cdecl, ty_real, 11, ty_real, ty_real, ty_real, ty_real, ty_real, ty_real, ty_real, ty_real, ty_real, ty_real, ty_real), 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1);
global.cnt_id = external_define(d, "gmt_counter", dll_cdecl, ty_real, 0);
external_call(global.cnt_id);
global.cnt = external_call(global.cnt_id);
global.bad = external_define(d, "no_such_export", dll_cdecl, ty_real, 0);
"#;

fn run_dll_suite(dll: &Path, bits: f64) {
    let src = format!("global.dll = \"{}\";\n{}", dll.display(), DLL_CODE);
    let (mut g, mut vm) = start(project(&src, "", &[]));
    assert_eq!(greal(&mut vm, "bits"), bits);
    assert_eq!(greal(&mut vm, "add"), 6.5);
    assert_eq!(greal(&mut vm, "sub"), 6.0);
    assert_eq!(gstr(&mut vm, "cat"), "foobar");
    assert_eq!(gstr(&mut vm, "cats"), "barfoo");
    assert_eq!(greal(&mut vm, "mix"), 6.0);
    assert_eq!(gstr(&mut vm, "rep"), "xyxyxy");
    assert_eq!(greal(&mut vm, "len4"), 6.0);
    assert_eq!(greal(&mut vm, "s11"), 1.0 + 2.0 + 3.0 + 4.0 + 6.0 + 100.0);
    // 16 real arguments (possible for extension functions): call the bridge directly
    let id16 = g.ext.dlls.define(&dll.to_string_lossy(), "gmt_sum16", 0, 0, &[0; 16]).unwrap();
    let args: Vec<Value> = (0..16).map(|_| Value::Real(1.0)).collect();
    assert_eq!(g.ext.dlls.call(id16, &args).unwrap(), Value::Real(1.0 + 2.0 + 3.0 + 4.0 + 11.0 + 100.0));
    assert_eq!(greal(&mut vm, "cnt"), 2.0);
    assert_eq!(greal(&mut vm, "bad"), -1.0, "failed define returns -1 when errors are ignored");
    assert!(g.error_occurred);
    // wrong argument type and freed DLL are runtime errors
    let id = greal(&mut vm, "cnt_id") as usize;
    assert!(g.ext.dlls.call(id, &[Value::Real(1.0)]).is_err());
    g.ext.dlls.free(&dll.file_name().unwrap().to_string_lossy());
    assert!(g.ext.dlls.call(id, &[]).is_err());
}

#[test]
fn external_functions_x64_in_process() {
    let b = built();
    run_dll_suite(&b.dll64, 64.0);
}

#[test]
fn external_functions_x86_helper() {
    let b = built();
    let Some(d) = &b.dll86 else { return };
    run_dll_suite(d, 32.0);
}

#[test]
fn dll_extension_package_x86() {
    let b = built();
    let Some(d) = &b.dll86 else { return };
    let bytes = std::fs::read(d).unwrap();
    let mut add = func("tdll_add", 2, extpkg::ARG_REAL, &[extpkg::ARG_REAL, extpkg::ARG_REAL]);
    add.external_name = "gmt_add".into();
    let mut cat = func("tdll_concat", 2, extpkg::ARG_STRING, &[extpkg::ARG_STRING, extpkg::ARG_STRING]);
    cat.external_name = "gmt_concat_std".into();
    cat.call_convention = 12;
    let mut init = func("__tdll_init", 0, extpkg::ARG_REAL, &[]);
    init.external_name = "gmt_counter".into();
    init.hidden = true;
    let cnt = func("gmt_counter", 0, extpkg::ARG_REAL, &[]);
    let pkg = ExtPackage {
        name: "Test DLL Ext".into(),
        files: vec![
            ExtFile {
                file_name: "gm_testdll.dll".into(),
                kind: extpkg::FILE_DLL,
                init_function: "__tdll_init".into(),
                functions: vec![add, cat, init, cnt],
                ..Default::default()
            },
            ExtFile {
                file_name: "wrap.gml".into(),
                kind: extpkg::FILE_GML,
                functions: vec![func("tdll_twice_add", 1, extpkg::ARG_REAL, &[extpkg::ARG_REAL])],
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    preload(&pkg, &[bytes, b"#define tdll_twice_add\nreturn tdll_add(argument0, argument0);\n".to_vec()]);
    let (g, mut vm) = start(project(
        "global.a = tdll_add(1, 2); global.c = tdll_concat('x', 'y'); global.t = tdll_twice_add(21); global.n = gmt_counter();",
        "",
        &["Test DLL Ext"],
    ));
    assert!(g.ext.unavailable().is_empty(), "{:?}", g.ext.unavailable());
    assert_eq!(greal(&mut vm, "a"), 3.0);
    assert_eq!(gstr(&mut vm, "c"), "yx");
    assert_eq!(greal(&mut vm, "t"), 42.0);
    assert_eq!(greal(&mut vm, "n"), 2.0, "init function ran once before");
    let tmp = g.ext.temp_dir.clone().unwrap();
    assert!(tmp.join("gm_testdll.dll").is_file());
    drop(g);
    assert!(!tmp.exists(), "temp files removed at exit");
}

/// The three packages shipped with GM 8.1, when installed (test inputs only).
#[test]
fn shipped_packages_load() {
    let names = ["GM Room Transitions", "GM Printing", "GM Windows Dialogs"];
    if names.iter().any(|n| bundle::find_package(n).is_none()) {
        eprintln!("shipped packages not installed; skipping");
        return;
    }
    let b = built();
    let code = if b.dll86.is_some() {
        // harmless calls into the real (x86, Delphi, stdcall) Printing.dll
        "global.ok = 1; pr_set_font_size(17); global.fs = pr_get_font_size(); pr_set_title('gm81 test'); global.title = pr_get_title(); global.np = pr_get_printer_count();"
    } else {
        "global.ok = 1;"
    };
    let p = project(code, "", &names[..if b.dll86.is_some() { 3 } else { 1 }]);
    let (g, mut vm) = start(p);
    assert_eq!(greal(&mut vm, "ok"), 1.0);
    if b.dll86.is_some() {
        assert_eq!(greal(&mut vm, "fs"), 17.0);
        assert_eq!(gstr(&mut vm, "title"), "gm81 test");
        assert!(greal(&mut vm, "np") >= 0.0);
    }
    let bad = g.ext.unavailable();
    assert!(bad.is_empty(), "{:?}", bad);
    assert!(!g.error_occurred, "init functions ran without errors: {}", g.error_last);
    let n = g.ext.function_names().count();
    eprintln!("{} extension functions registered from {:?}", n, g.ext.loaded);
    assert!(n > 100);
}
