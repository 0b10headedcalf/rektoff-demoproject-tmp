//! Registers every level with the grader.
//!
//! Finds each `levels/<id>/tests/checks.rs` and writes `levels.rs`, which
//! declares one module per level and a `checks_for(id)` lookup. Adding a level
//! is adding a folder: no grader source changes.

use std::path::Path;
use std::{env, fs};

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("set by cargo");
    let levels_dir = Path::new(&manifest_dir).join("../levels");
    // Re-run when a level is added, removed or edited.
    println!("cargo:rerun-if-changed={}", levels_dir.display());

    let mut levels = Vec::new();
    for entry in fs::read_dir(&levels_dir).expect("levels directory is readable") {
        let entry = entry.expect("read levels directory");
        let checks = entry.path().join("tests/checks.rs");
        if let Ok(checks) = checks.canonicalize() {
            levels.push((entry.file_name().to_string_lossy().to_string(), checks));
        }
    }
    levels.sort();
    assert!(!levels.is_empty(), "no levels found in {}", levels_dir.display());

    // A level id like "level-01-missing-signer" becomes module `level_01_missing_signer`.
    let module = |id: &str| -> String {
        id.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect()
    };

    let mut code = String::new();
    for (id, checks) in &levels {
        code += &format!("#[path = {:?}]\nmod {};\n", checks, module(id));
    }
    code += "\nfn checks_for(level: &str) -> Option<&'static [Check]> {\n    match level {\n";
    for (id, _) in &levels {
        code += &format!("        {:?} => Some({}::CHECKS),\n", id, module(id));
    }
    code += "        _ => None,\n    }\n}\n";

    let out_dir = env::var("OUT_DIR").expect("set by cargo");
    fs::write(Path::new(&out_dir).join("levels.rs"), code).expect("write levels.rs");
}
