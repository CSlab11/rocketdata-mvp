//! Parametro `--mode` (REQ-072).

use std::process::Command;

fn run(args: &[&str]) -> std::process::Output {
    let src = tempfile::tempdir().unwrap();
    let dst = tempfile::tempdir().unwrap();
    Command::new(env!("CARGO_BIN_EXE_doc-ripper"))
        .args(["--src", src.path().to_str().unwrap(), "--dst", dst.path().to_str().unwrap()])
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn accetta_ai_e_simple() {
    for mode in ["ai", "simple"] {
        let output = run(&["--mode", mode]);
        assert!(output.status.success(), "{mode}: {}", String::from_utf8_lossy(&output.stderr));
    }
}

#[test]
fn senza_mode_e_ammesso() {
    let output = run(&[]);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
}

#[test]
fn valore_non_ammesso_elenca_i_valori() {
    let output = run(&["--mode", "boh"]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ai") && stderr.contains("simple"), "{stderr}");
}

#[test]
fn l_aiuto_descrive_la_modalita() {
    let output = run(&["--help"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--mode") && stdout.contains("[default: ai]"), "{stdout}");
}
