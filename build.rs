//! Builds every dossier photo in assets/suspects/ into the program, so they
//! work in a single-file release build and in the browser.

use std::path::Path;
use std::{env, fs};

fn main() {
    println!("cargo:rerun-if-changed=assets/suspects");
    let mut photos: Vec<_> = fs::read_dir("assets/suspects")
        .map(|dir| {
            dir.filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| path.extension().is_some_and(|ext| ext == "png"))
                .collect()
        })
        .unwrap_or_default();
    photos.sort();
    let mut code = String::from("/// Dossier photos by file name (without .png), from assets/suspects/.\n");
    code.push_str("pub static PHOTOS: &[(&str, &[u8])] = &[\n");
    for path in photos {
        let name = path.file_stem().unwrap_or_default().to_string_lossy().into_owned();
        let full = fs::canonicalize(&path).expect("photo path");
        code.push_str(&format!("    ({name:?}, include_bytes!({full:?})),\n"));
    }
    code.push_str("];\n");
    let out = Path::new(&env::var("OUT_DIR").expect("set by cargo")).join("photos.rs");
    fs::write(out, code).expect("write photos.rs");
}
