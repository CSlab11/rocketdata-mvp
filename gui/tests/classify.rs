mod common;

use std::fs;
use std::path::Path;
use std::process::Command;

use serde_json::Value;

use common::install_with_fake_tag;

/// Cartella di Markdown con il testo delle unità restituite dal rocket-tag finto.
fn markdown_folder() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("sotto")).unwrap();
    fs::write(dir.path().join("a.md"), "Primo testo.\n\nSecondo testo\nsu due righe.\n").unwrap();
    fs::write(dir.path().join("sotto/b.md"), "- Terzo testo.\n").unwrap();
    dir
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

/// I test usano un rocket-tag finto, senza dipendere dal tool reale (REQ-148).
#[test]
fn il_rocket_tag_finto_scrive_il_file_di_uscita() {
    let (dir, _) = install_with_fake_tag();
    let folder = markdown_folder();
    let out = folder.path().join("uscita/rules.json");
    let status = Command::new(dir.path().join("rocket-tag.exe"))
        .args(["classify", "rules"])
        .arg(folder.path())
        .arg("-o")
        .arg(&out)
        .output()
        .unwrap()
        .status;
    assert!(status.success());
    assert_eq!(read_json(&out)["items"].as_array().unwrap().len(), 3);
}
