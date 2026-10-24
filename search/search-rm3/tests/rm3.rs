use std::fs;
use std::process::{Command, Output};

use serde_json::Value;

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_search-rm3")).args(args).output().unwrap()
}

const MANUAL: &str = "# APB2 clock\nEnable the APB2 clock with RCC_APB2ENR.\n\n\
# RCC_APB2ENR\nThe RCC_APB2ENR register: bit IOPAEN.\n\n\
# Timers\nTimers count pulses.\n";

#[test]
fn l_espansione_trova_sezioni_senza_keyword() {
    let src = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    fs::write(src.path().join("m.md"), MANUAL).unwrap();
    assert!(run(&["index", "--src", src.path().to_str().unwrap(), "--index", dir.path().to_str().unwrap()]).status.success());

    let out = run(&["query", "--index", dir.path().to_str().unwrap(), "--q", "apb2 clock"]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let json: Value = serde_json::from_slice(&out.stdout).unwrap();
    let terms = json["terms"].as_array().unwrap();
    assert!(terms.iter().any(|t| t["term"] == "rcc_apb2enr" && t["source"] == "rm3"), "{terms:?}");
    assert!(terms.iter().filter(|t| t["source"] == "query").all(|t| (t["weight"].as_f64().unwrap() - 0.25).abs() < 1e-9));
    let headings: Vec<&str> = json["results"].as_array().unwrap().iter().map(|r| r["heading"].as_str().unwrap()).collect();
    assert_eq!(headings, ["APB2 clock", "RCC_APB2ENR"]);
}

fn indexed() -> (tempfile::TempDir, tempfile::TempDir) {
    let src = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    fs::write(src.path().join("m.md"), MANUAL).unwrap();
    assert!(run(&["index", "--src", src.path().to_str().unwrap(), "--index", dir.path().to_str().unwrap()]).status.success());
    (src, dir)
}

fn query(dir: &std::path::Path, extra: &[&str]) -> Value {
    let mut args = vec!["query", "--index", dir.to_str().unwrap(), "--q", "apb2 clock"];
    args.extend_from_slice(extra);
    let out = run(&args);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    serde_json::from_slice(&out.stdout).unwrap()
}

fn sources(json: &Value, source: &str) -> usize {
    json["terms"].as_array().unwrap().iter().filter(|t| t["source"] == source).count()
}

#[test]
fn opzioni_fb_docs_fb_terms_e_lambda() {
    let (_src, dir) = indexed();
    assert_eq!(query(dir.path(), &[]), query(dir.path(), &["--fb-docs", "10", "--fb-terms", "10", "--lambda", "0.5"]));
    assert_eq!(sources(&query(dir.path(), &["--fb-terms", "2"]), "rm3"), 2);
    assert_eq!(sources(&query(dir.path(), &["--fb-docs", "0"]), "rm3"), 0);

    // Con λ = 1 resta la sola query originale: niente sezioni trovate per espansione.
    let only_query = query(dir.path(), &["--lambda", "1"]);
    assert_eq!(sources(&only_query, "rm3"), 0);
    assert_eq!(only_query["results"].as_array().unwrap().len(), 1);
}

#[test]
fn lambda_fuori_da_zero_uno_e_un_errore() {
    let (_src, dir) = indexed();
    for value in ["2", "-0.1", "x"] {
        let out = run(&["query", "--index", dir.path().to_str().unwrap(), "--q", "clock", "--lambda", value]);
        assert!(!out.status.success(), "--lambda {value}");
    }
}
