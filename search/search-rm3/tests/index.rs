use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_search-rm3")).args(args).output().unwrap()
}

fn index(src: &Path, dir: &Path) -> Output {
    run(&["index", "--src", src.to_str().unwrap(), "--index", dir.to_str().unwrap()])
}

fn indexed_files(dir: &Path) -> Vec<String> {
    let json: serde_json::Value = serde_json::from_str(&fs::read_to_string(dir.join("index.json")).unwrap()).unwrap();
    let mut files: Vec<String> =
        json["sections"].as_array().unwrap().iter().map(|s| s["file"].as_str().unwrap().to_string()).collect();
    files.dedup();
    files
}

#[test]
fn indicizza_ricorsivamente_i_md_escluso_index_md() {
    let src = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(src.path().join("sub")).unwrap();
    fs::write(src.path().join("a_pdf.md"), "# A\nclock\n").unwrap();
    fs::write(src.path().join("sub/b_docx.md"), "# B\nregister\n").unwrap();
    fs::write(src.path().join("sub/index.md"), "# Non è l'indice della radice\n").unwrap();
    fs::write(src.path().join("index.md"), "# Indice\n").unwrap();
    fs::write(src.path().join("dati.csv"), "a,b\n").unwrap();

    let out = index(src.path(), dir.path());
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(indexed_files(dir.path()), ["a_pdf.md", "sub/b_docx.md", "sub/index.md"]);
}

#[test]
fn esclude_la_cartella_index_nella_radice() {
    let src = tempfile::tempdir().unwrap();
    let dir = src.path().join(".index");
    fs::create_dir_all(&dir).unwrap();
    fs::create_dir_all(src.path().join("sub/.index")).unwrap();
    fs::write(dir.join("vecchio.md"), "# Vecchio\n").unwrap();
    fs::write(src.path().join("a.md"), "# A\n").unwrap();
    fs::write(src.path().join("sub/.index/c.md"), "# C\n").unwrap();

    let out = index(src.path(), &dir);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(indexed_files(&dir), ["a.md", "sub/.index/c.md"]);
}

#[test]
fn ricrea_l_indice_da_zero() {
    let src = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    fs::write(src.path().join("vecchio.md"), "# Vecchio\n").unwrap();
    assert!(index(src.path(), dir.path()).status.success());

    fs::remove_file(src.path().join("vecchio.md")).unwrap();
    fs::write(src.path().join("nuovo.md"), "# Nuovo\n").unwrap();
    assert!(index(src.path(), dir.path()).status.success());
    assert_eq!(indexed_files(dir.path()), ["nuovo.md"]);
}
