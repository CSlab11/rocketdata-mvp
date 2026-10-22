//! Copia vendor/pdfium/pdfium.dll accanto agli eseguibili generati (doc-ripper.exe e test),
//! dove doc-ripper la cerca a runtime. La DLL si scarica con scripts/fetch-pdfium.ps1.

use std::path::{Path, PathBuf};
use std::{env, fs};

fn main() {
    let dll = Path::new("vendor/pdfium/pdfium.dll");
    println!("cargo:rerun-if-changed={}", dll.display());
    if !dll.is_file() {
        println!("cargo:warning=pdfium.dll assente: eseguire scripts/fetch-pdfium.ps1 per convertire i PDF");
        return;
    }
    // OUT_DIR = target/<profilo>/build/<pacchetto>/out
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let profile_dir = out_dir.ancestors().nth(3).unwrap();
    for dir in [profile_dir.to_path_buf(), profile_dir.join("deps")] {
        fs::copy(dll, dir.join("pdfium.dll")).unwrap();
    }
}
