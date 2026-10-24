use std::fs;
use std::process::Command;

/// Percorsi Windows con spazi e lettere accentate, sottocartelle con `\` (REQ-061).
#[test]
fn funziona_con_percorsi_windows_con_spazi_e_accenti() {
    let root = tempfile::tempdir().unwrap();
    let src = root.path().join("Manuali tecnici à");
    let dir = root.path().join("indice è");
    fs::create_dir_all(src.join("sotto cartella")).unwrap();
    fs::write(src.join("sotto cartella").join("file con spazi_pdf.md"), "# Clock\nclock tree\n").unwrap();

    let exe = env!("CARGO_BIN_EXE_search-rm3");
    let out = Command::new(exe).args(["index", "--src"]).arg(&src).arg("--index").arg(&dir).output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let out = Command::new(exe).args(["query", "--q", "clock", "--index"]).arg(&dir).output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["results"][0]["file"], "sotto cartella/file con spazi_pdf.md");
}
