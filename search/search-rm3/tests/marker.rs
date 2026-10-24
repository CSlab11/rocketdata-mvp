use std::fs;
use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_search-rm3")).args(args).output().unwrap()
}

#[test]
fn l_indice_contiene_il_marcatore_del_sotto_tool() {
    let src = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    fs::write(src.path().join("a.md"), "# A\nclock\n").unwrap();
    let out = run(&["index", "--src", src.path().to_str().unwrap(), "--index", dir.path().to_str().unwrap()]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    assert_eq!(fs::read_to_string(dir.path().join("search-tool")).unwrap(), "search-rm3");
    assert!(run(&["query", "--index", dir.path().to_str().unwrap(), "--q", "clock"]).status.success());
}

#[test]
fn errore_se_la_cartella_non_contiene_un_indice() {
    let dir = tempfile::tempdir().unwrap();
    let out = run(&["query", "--index", dir.path().to_str().unwrap(), "--q", "clock"]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("non contiene un indice"));
}

#[test]
fn errore_se_l_indice_e_di_un_altro_sotto_tool() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("search-tool"), "search-semantica").unwrap();
    fs::write(dir.path().join("index.json"), "{}").unwrap();
    let out = run(&["query", "--index", dir.path().to_str().unwrap(), "--q", "clock"]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("search-semantica"));
}
