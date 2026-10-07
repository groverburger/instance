//! Extension packages: `.ged` package descriptions and `.dat` payloads
//! (spec file-formats.md §11).

use crate::error::{Error, Result};
use crate::gmk::make_table;
use crate::io::{inflate, Reader};

/// Kind of a package file (`file_kind`).
pub const FILE_DLL: i32 = 1;
/// GML file (`#define` sections).
pub const FILE_GML: i32 = 2;
/// Action library [I].
pub const FILE_ACTION_LIB: i32 = 3;
/// Other data file [I].
pub const FILE_OTHER: i32 = 4;

/// Argument/return type code: string.
pub const ARG_STRING: i32 = 1;
/// Argument/return type code: real.
pub const ARG_REAL: i32 = 2;

/// A whole `.ged` package description.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExtPackage {
    pub editable: bool,
    pub name: String,
    pub folder: String,
    pub version_text: String,
    pub author: String,
    pub date: String,
    pub license: String,
    pub description: String,
    pub help_file: String,
    pub hidden: bool,
    pub uses: Vec<String>,
    pub files: Vec<ExtFile>,
}

/// One file of a package.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExtFile {
    pub file_name: String,
    pub original_path: String,
    pub kind: i32,
    pub init_function: String,
    pub final_function: String,
    pub functions: Vec<ExtFunction>,
    pub constants: Vec<ExtConstant>,
}

/// One function record.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExtFunction {
    pub name: String,
    pub external_name: String,
    pub call_convention: i32,
    pub help_line: String,
    pub hidden: bool,
    pub argument_count: i32,
    pub arg_types: [i32; 17],
    pub return_type: i32,
}

/// One constant record.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExtConstant {
    pub name: String,
    pub value: String,
    pub hidden: bool,
}

/// Parses a `.ged` file (must be consumed exactly).
pub fn read_ged(data: &[u8]) -> Result<ExtPackage> {
    let mut r = Reader::new(data, "extension package (.ged)");
    r.expect(700, "ged version")?;
    let mut p = ExtPackage {
        editable: r.bool()?,
        name: r.string()?,
        folder: r.string()?,
        version_text: r.string()?,
        author: r.string()?,
        date: r.string()?,
        license: r.string()?,
        description: r.string()?,
        help_file: r.string()?,
        hidden: r.bool()?,
        ..Default::default()
    };
    let n = r.count("uses")?;
    for _ in 0..n {
        p.uses.push(r.string()?);
    }
    let nf = r.count("file")?;
    for _ in 0..nf {
        r.expect(700, "ged file version")?;
        let mut f = ExtFile {
            file_name: r.string()?,
            original_path: r.string()?,
            kind: r.i32()?,
            init_function: r.string()?,
            final_function: r.string()?,
            ..Default::default()
        };
        let nfn = r.count("function")?;
        for _ in 0..nfn {
            r.expect(700, "ged function version")?;
            let mut func = ExtFunction {
                name: r.string()?,
                external_name: r.string()?,
                call_convention: r.i32()?,
                help_line: r.string()?,
                hidden: r.bool()?,
                argument_count: r.i32()?,
                ..Default::default()
            };
            for t in func.arg_types.iter_mut() {
                *t = r.i32()?;
            }
            func.return_type = r.i32()?;
            f.functions.push(func);
        }
        let nc = r.count("constant")?;
        for _ in 0..nc {
            r.expect(700, "ged constant version")?;
            f.constants.push(ExtConstant {
                name: r.string()?,
                value: r.string()?,
                hidden: r.bool()?,
            });
        }
        p.files.push(f);
    }
    r.end()?;
    Ok(p)
}

/// Decodes a `.dat` payload into the bytes of each file (one chunk per `.ged` file
/// entry, in order). `expected` is the number of chunks to read.
pub fn read_dat(data: &[u8], expected: usize) -> Result<Vec<Vec<u8>>> {
    if data.len() < 4 {
        return Err(Error::InvalidValue {
            context: "extension payload (.dat)".into(),
            offset: 0,
            message: "file too short".into(),
        });
    }
    let seed = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    let (_, inv) = make_table(seed);
    let enc = &data[4..];
    let mut plain: Vec<u8> = enc.iter().map(|&b| inv[b as usize]).collect();
    if let (Some(p), Some(e)) = (plain.first_mut(), enc.first()) {
        *p = *e;
    }
    let mut r = Reader::new(&plain, "extension payload (.dat)").with_base(4);
    let mut out = Vec::with_capacity(expected);
    for i in 0..expected {
        let at = r.offset();
        let z = r.blob()?;
        let bytes = inflate(&z).map_err(|message| Error::Zlib {
            context: format!("extension payload chunk {i}"),
            offset: at,
            message,
        })?;
        out.push(bytes);
    }
    Ok(out)
}

/// Encodes file contents as a `.dat` payload with the given seed (inverse of
/// [`read_dat`]).
pub fn write_dat(files: &[Vec<u8>], seed: u32) -> Vec<u8> {
    let (t, _) = make_table(seed);
    let mut plain = Vec::new();
    for f in files {
        let z = crate::io::deflate(f);
        plain.extend_from_slice(&(z.len() as u32).to_le_bytes());
        plain.extend_from_slice(&z);
    }
    let mut out = seed.to_le_bytes().to_vec();
    for (k, &b) in plain.iter().enumerate() {
        out.push(if k == 0 { b } else { t[b as usize] });
    }
    out
}

/// Writes a `.ged` file (inverse of [`read_ged`]).
pub fn write_ged(p: &ExtPackage) -> Vec<u8> {
    let mut w = crate::io::Writer::new();
    w.i32(700);
    w.bool(p.editable);
    for s in [
        &p.name,
        &p.folder,
        &p.version_text,
        &p.author,
        &p.date,
        &p.license,
        &p.description,
        &p.help_file,
    ] {
        w.string(s);
    }
    w.bool(p.hidden);
    w.i32(p.uses.len() as i32);
    for u in &p.uses {
        w.string(u);
    }
    w.i32(p.files.len() as i32);
    for f in &p.files {
        w.i32(700);
        w.string(&f.file_name);
        w.string(&f.original_path);
        w.i32(f.kind);
        w.string(&f.init_function);
        w.string(&f.final_function);
        w.i32(f.functions.len() as i32);
        for func in &f.functions {
            w.i32(700);
            w.string(&func.name);
            w.string(&func.external_name);
            w.i32(func.call_convention);
            w.string(&func.help_line);
            w.bool(func.hidden);
            w.i32(func.argument_count);
            for t in func.arg_types {
                w.i32(t);
            }
            w.i32(func.return_type);
        }
        w.i32(f.constants.len() as i32);
        for c in &f.constants {
            w.i32(700);
            w.string(&c.name);
            w.string(&c.value);
            w.bool(c.hidden);
        }
    }
    w.buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let mut p = ExtPackage {
            name: "Test".into(),
            ..Default::default()
        };
        p.files.push(ExtFile {
            file_name: "a.gml".into(),
            kind: FILE_GML,
            init_function: "a_init".into(),
            functions: vec![ExtFunction {
                name: "a".into(),
                argument_count: 2,
                arg_types: [ARG_REAL; 17],
                return_type: ARG_REAL,
                ..Default::default()
            }],
            constants: vec![ExtConstant {
                name: "A_C".into(),
                value: "3".into(),
                hidden: false,
            }],
            ..Default::default()
        });
        let g = write_ged(&p);
        assert_eq!(read_ged(&g).unwrap(), p);
        let files = vec![b"#define a\nreturn 1".to_vec(), vec![1, 2, 3]];
        let d = write_dat(&files, 12345);
        assert_eq!(read_dat(&d, 2).unwrap(), files);
    }
}
