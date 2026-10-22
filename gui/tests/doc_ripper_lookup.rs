mod common;

use std::fs;
use std::path::PathBuf;

use common::{Gui, dump_dom, gui_exe};

/// Copia gui.exe in una cartella temporanea, con o senza un doc-ripper.exe accanto.
fn install(with_doc_ripper: bool) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let exe = dir.path().join("gui.exe");
    fs::copy(gui_exe(), &exe).unwrap();
    if with_doc_ripper {
        fs::write(dir.path().join("doc-ripper.exe"), b"").unwrap();
    }
    (dir, exe)
}

fn status(gui: &Gui) -> serde_json::Value {
    let body = ureq::get(&gui.url("/api/doc-ripper/status")).call().unwrap().into_body().read_to_string().unwrap();
    serde_json::from_str(&body).unwrap()
}

#[test]
fn doc_ripper_assente_disabilita_l_avvio() {
    let (_dir, exe) = install(false);
    let gui = Gui::start(&exe);
    let status = status(&gui);
    assert_eq!(status["available"], false);
    assert!(status["message"].as_str().unwrap().contains("doc-ripper.exe non trovato"));

    let dom = dump_dom(&gui.url("/#doc-ripper"));
    assert!(dom.contains("doc-ripper.exe non trovato"), "{dom}");
    assert!(dom.contains(r#"<button id="start" class="primary" type="button" disabled="">"#), "{dom}");
}

#[test]
fn doc_ripper_presente_abilita_l_avvio() {
    let (_dir, exe) = install(true);
    let gui = Gui::start(&exe);
    assert_eq!(status(&gui)["available"], true);

    let dom = dump_dom(&gui.url("/#doc-ripper"));
    assert!(dom.contains(r#"<button id="start" class="primary" type="button">"#), "{dom}");
    assert!(!dom.contains("non trovato"), "{dom}");
}
