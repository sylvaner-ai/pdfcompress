// Lie libgs (Ghostscript en bibliothèque partagée).
// GS_LIB_DIR permet d'indiquer son dossier ; à défaut, on essaie Homebrew sur macOS.
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-env-changed=GS_LIB_DIR");
    if let Ok(dir) = std::env::var("GS_LIB_DIR") {
        println!("cargo:rustc-link-search=native={dir}");
    } else {
        for dir in ["/opt/homebrew/lib", "/usr/local/lib"] {
            if Path::new(dir).join("libgs.dylib").exists() {
                println!("cargo:rustc-link-search=native={dir}");
            }
        }
    }
    println!("cargo:rustc-link-lib=dylib=gs");
}
