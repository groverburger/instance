//! Parses the extension packages shipped with GM 8.1 when they are installed locally
//! (test input only; never committed). Skips when absent.

use std::path::PathBuf;

fn dir() -> Option<PathBuf> {
    let d = std::env::var_os("GM81_EXTENSIONS")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Users\Grob\Documents\gamemaker\GameMaker\extensions"));
    d.is_dir().then_some(d)
}

#[test]
fn shipped_packages_parse() {
    let Some(d) = dir() else {
        eprintln!("shipped extensions not found; skipping");
        return;
    };
    for name in ["GM Printing", "GM Room Transitions", "GM Windows Dialogs"] {
        let ged = match std::fs::read(d.join(format!("{name}.ged"))) {
            Ok(b) => b,
            Err(_) => continue,
        };
        let p = gm_format::extpkg::read_ged(&ged).unwrap();
        assert_eq!(p.name, name);
        assert_eq!(gm_format::extpkg::write_ged(&p), ged);
        let dat = std::fs::read(d.join(format!("{name}.dat"))).unwrap();
        let files = gm_format::extpkg::read_dat(&dat, p.files.len()).unwrap();
        for (f, bytes) in p.files.iter().zip(&files) {
            eprintln!(
                "{}: {} kind {} init {:?} final {:?} {} fns {} consts {} bytes (head {:02x?})",
                name,
                f.file_name,
                f.kind,
                f.init_function,
                f.final_function,
                f.functions.len(),
                f.constants.len(),
                bytes.len(),
                &bytes[..bytes.len().min(4)]
            );
            if f.kind == gm_format::extpkg::FILE_DLL {
                assert_eq!(&bytes[..2], b"MZ");
            }
        }
    }
}
