use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_doc-ripper")).args(args).output().unwrap()
}

#[test]
fn accetta_src_e_dst() {
    let src = tempfile::tempdir().unwrap();
    let dst = tempfile::tempdir().unwrap();
    let out = run(&["--src", src.path().to_str().unwrap(), "--dst", dst.path().to_str().unwrap()]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
}

#[test]
fn stampa_il_riepilogo() {
    let src = tempfile::tempdir().unwrap();
    let dst = tempfile::tempdir().unwrap();
    let out = run(&["--src", src.path().to_str().unwrap(), "--dst", dst.path().to_str().unwrap()]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Riepilogo"), "{stdout}");
    assert!(stdout.contains("Convertiti: 0"));
    assert!(stdout.contains("Saltati: 0"));
    assert!(stdout.contains("Falliti: 0"));
}

#[test]
fn replica_le_sottocartelle_nella_destinazione() {
    let src = tempfile::tempdir().unwrap();
    let dst = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(src.path().join("clienti/2026")).unwrap();
    std::fs::create_dir_all(src.path().join("fornitori")).unwrap();
    let target = dst.path().join("nuova");
    let out = run(&["--src", src.path().to_str().unwrap(), "--dst", target.to_str().unwrap()]);
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stdout));
    assert!(target.join("clienti/2026").is_dir());
    assert!(target.join("fornitori").is_dir());
}

#[test]
fn segnala_i_file_non_supportati_senza_copiarli() {
    let src = tempfile::tempdir().unwrap();
    let dst = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(src.path().join("a")).unwrap();
    std::fs::write(src.path().join("a/note.txt"), "ciao").unwrap();
    std::fs::write(src.path().join("vecchio.doc"), "").unwrap();
    let out = run(&["--src", src.path().to_str().unwrap(), "--dst", dst.path().to_str().unwrap()]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("Saltati: 2"), "{stdout}");
    assert!(stdout.contains("note.txt - non supportato"), "{stdout}");
    assert!(stdout.contains("vecchio.doc - non supportato"), "{stdout}");
    assert!(!dst.path().join("a/note.txt").exists());
    assert!(!dst.path().join("vecchio.doc").exists());
}

#[test]
fn src_e_dst_sono_obbligatori() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().to_str().unwrap();
    assert_eq!(run(&["--src", p]).status.code(), Some(2));
    assert_eq!(run(&["--dst", p]).status.code(), Some(2));
    assert_eq!(run(&[]).status.code(), Some(2));
}

#[test]
fn src_inesistente_e_un_errore() {
    let dst = tempfile::tempdir().unwrap();
    let missing = dst.path().join("non-esiste");
    let out = run(&["--src", missing.to_str().unwrap(), "--dst", dst.path().to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("cartella di origine"));
}
