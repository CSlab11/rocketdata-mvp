mod common;

use std::fs;

use common::{Gui, install_with_fake, run, wait_output};

fn lines(snapshot: &serde_json::Value) -> Vec<String> {
    snapshot["lines"].as_array().unwrap().iter().map(|l| l.as_str().unwrap().to_string()).collect()
}

#[test]
fn avvia_doc_ripper_con_src_e_dst() {
    let (_dir, exe) = install_with_fake();
    let gui = Gui::start(&exe);
    let src = tempfile::tempdir().unwrap();
    let dst = tempfile::tempdir().unwrap();
    let (src, dst) = (src.path().to_str().unwrap(), dst.path().to_str().unwrap());

    assert_eq!(run(&gui, src, dst), 202);
    let snapshot = wait_output(&gui);
    let lines = lines(&snapshot);
    assert!(lines.contains(&format!("argomenti: --src {src} --dst {dst}")), "{lines:?}");
    assert!(lines.contains(&"Riepilogo".to_string()), "{lines:?}");
    assert_eq!(snapshot["exit_code"], 0);
}

#[test]
fn riporta_il_codice_di_uscita() {
    let (_dir, exe) = install_with_fake();
    let gui = Gui::start(&exe);
    let src = tempfile::tempdir().unwrap();
    fs::write(src.path().join("fallisci"), "").unwrap();

    assert_eq!(run(&gui, src.path().to_str().unwrap(), "C:\\fuori"), 202);
    assert_eq!(wait_output(&gui)["exit_code"], 1);
}

#[test]
fn una_esecuzione_alla_volta_e_campi_obbligatori() {
    let (_dir, exe) = install_with_fake();
    let gui = Gui::start(&exe);
    let src = tempfile::tempdir().unwrap();
    fs::write(src.path().join("lento"), "").unwrap();
    let src = src.path().to_str().unwrap();

    assert_eq!(run(&gui, "", "C:\\dst"), 400);
    assert_eq!(run(&gui, src, "C:\\dst"), 202);
    assert_eq!(run(&gui, src, "C:\\dst"), 409);
    wait_output(&gui);
    assert_eq!(run(&gui, src, "C:\\dst"), 202);
    wait_output(&gui);
}
