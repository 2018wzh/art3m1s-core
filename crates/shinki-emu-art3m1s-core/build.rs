use shinki_emu_build::{BuildAsset, CoreBuildManifest, write_core_build_manifest};
use std::{env, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=build.rs");
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").ok_or("missing CARGO_MANIFEST_DIR")?)
        .join("../..")
        .canonicalize()?;
    let mut manifest = CoreBuildManifest::new("art3m1s-core", "rust-cdylib")?;
    for path in [
        "LICENSE",
        "crates/shinki-emu-art3m1s-core/THIRD_PARTY_NOTICES.md",
        "crates/shinki-emu-art3m1s-core/MODIFICATIONS.md",
        "crates/pf8/LICENSE",
    ] {
        manifest.licenses.push(BuildAsset {
            source: root.join(path),
            dest: format!("licenses/art3m1s-core/{path}"),
        });
    }
    write_core_build_manifest(&manifest)?;
    Ok(())
}
