use std::fs;
use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_search-rm3")).args(args).output().unwrap()
}

fn assert_error(out: &Output) {
    assert!(!out.status.success());
    assert_ne!(out.status.code(), Some(0));
    assert!(!out.stderr.is_empty());
    assert!(out.stdout.is_empty(), "{}", String::from_utf8_lossy(&out.stdout));
}

#[test]
fn errore_se_la_cartella_di_origine_non_esiste() {
    let root = tempfile::tempdir().unwrap();
    let missing = root.path().join("manca");
    let out = run(&["index", "--src", missing.to_str().unwrap(), "--index", root.path().join("i").to_str().unwrap()]);
    assert_error(&out);
    assert!(String::from_utf8_lossy(&out.stderr).contains("Errore"));
}

#[test]
fn errore_se_un_md_non_si_legge() {
    let src = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    fs::write(src.path().join("rotto.md"), [0xff, 0xfe, 0x00]).unwrap();
    let out = run(&["index", "--src", src.path().to_str().unwrap(), "--index", dir.path().to_str().unwrap()]);
    assert_error(&out);
    assert!(String::from_utf8_lossy(&out.stderr).contains("rotto.md"));
}

#[test]
fn errore_se_mancano_le_opzioni_comuni() {
    assert_error(&run(&["index", "--src", "."]));
    assert_error(&run(&["query", "--q", "clock"]));
}

#[test]
fn domanda_di_sole_stopword_riesce_senza_risultati() {
    let src = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    fs::write(src.path().join("a.md"), "# A\nclock\n").unwrap();
    assert!(run(&["index", "--src", src.path().to_str().unwrap(), "--index", dir.path().to_str().unwrap()]).status.success());
    let out = run(&["query", "--index", dir.path().to_str().unwrap(), "--q", "what is the"]);
    assert_eq!(out.status.code(), Some(0));
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(json["terms"].as_array().unwrap().is_empty());
    assert!(json["results"].as_array().unwrap().is_empty());
}
