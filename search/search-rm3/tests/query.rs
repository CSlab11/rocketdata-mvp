use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use serde_json::Value;

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_search-rm3")).args(args).output().unwrap()
}

/// Indicizza i file dati e restituisce la cartella dell'indice.
fn indexed(files: &[(&str, &str)]) -> (tempfile::TempDir, tempfile::TempDir) {
    let src = tempfile::tempdir().unwrap();
    let dir = tempfile::tempdir().unwrap();
    for (name, content) in files {
        let path = src.path().join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }
    let out = run(&["index", "--src", src.path().to_str().unwrap(), "--index", dir.path().to_str().unwrap()]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    (src, dir)
}

fn query(dir: &Path, extra: &[&str]) -> Value {
    let mut args = vec!["query", "--index", dir.to_str().unwrap()];
    args.extend_from_slice(extra);
    let out = run(&args);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    serde_json::from_slice(&out.stdout).unwrap()
}

const MANUAL: &str = "Origine: [m.pdf](C:/m.pdf)\n\n# Clocks\nThe clock tree.\n\n## APB2 clock\nEnable the APB2 clock in RCC_APB2ENR.\n\n## Timers\nTimers count.\n";

#[test]
fn risponde_con_query_terms_e_results() {
    let (_src, dir) = indexed(&[("sub/manual_pdf.md", MANUAL)]);
    let json = query(dir.path(), &["--q", "How is the APB2 clock enabled?"]);
    assert_eq!(json["query"], "How is the APB2 clock enabled?");
    let terms: Vec<&str> = json["terms"].as_array().unwrap().iter().map(|t| t["term"].as_str().unwrap()).collect();
    assert_eq!(&terms[..3], ["apb2", "clock", "enabl"]);
    assert_eq!(json["terms"][0]["source"], "query");
    assert!(json["terms"][0]["weight"].is_number());

    let first = &json["results"][0];
    assert_eq!(first["file"], "sub/manual_pdf.md");
    assert_eq!(first["heading"], "APB2 clock");
    assert_eq!(first["line"], 6);
    assert_eq!(first["text"], "## APB2 clock\nEnable the APB2 clock in RCC_APB2ENR.\n\n");
    let scores: Vec<f64> = json["results"].as_array().unwrap().iter().map(|r| r["score"].as_f64().unwrap()).collect();
    assert!(scores.windows(2).all(|w| w[0] >= w[1]), "{scores:?}");
}

#[test]
fn al_piu_top_risultati() {
    let files: Vec<(String, String)> = (0..15).map(|i| (format!("f{i:02}.md"), format!("# S{i}\nclock {i}\n"))).collect();
    let refs: Vec<(&str, &str)> = files.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    let (_src, dir) = indexed(&refs);
    assert_eq!(query(dir.path(), &["--q", "clock"])["results"].as_array().unwrap().len(), 10);
    assert_eq!(query(dir.path(), &["--q", "clock", "--top", "3"])["results"].as_array().unwrap().len(), 3);
}
