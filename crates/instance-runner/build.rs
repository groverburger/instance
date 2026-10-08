//! Embeds the Instance app icon (rendered by instance-brand) into the Windows executable.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if os == "windows" && env == "msvc" {
        // link.exe accepts a compiled resource file as an input, so no resource
        // compiler is needed.
        let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("app_icon.res");
        std::fs::write(&out, instance_brand::res_file(&instance_brand::ICO_SIZES)).expect("write app_icon.res");
        println!("cargo:rustc-link-arg-bins={}", out.display());
    }
}
