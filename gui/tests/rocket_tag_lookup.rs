mod common;

use std::fs;
use std::path::PathBuf;

use serde_json::json;

use common::{Gui, dump_dom, gui_exe, post_json};

/// Copia gui.exe in una cartella temporanea, con o senza un rocket-tag.exe accanto.
fn install(with_rocket_tag: bool) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let exe = dir.path().join("gui.exe");
    fs::copy(gui_exe(), &exe).unwrap();
    if with_rocket_tag {
        fs::write(dir.path().join("rocket-tag.exe"), b"").unwrap();
    }
    (dir, exe)
}

fn status(gui: &Gui) -> serde_json::Value {
    let body = ureq::get(&gui.url("/api/classify/status")).call().unwrap().into_body().read_to_string().unwrap();
    serde_json::from_str(&body).unwrap()
}

#[test]
fn rocket_tag_assente_disabilita_la_classificazione() {
    let (_dir, exe) = install(false);
    let gui = Gui::start(&exe);
    let status = status(&gui);
    assert_eq!(status["available"], false);
    assert!(status["message"].as_str().unwrap().contains("rocket-tag.exe non trovato"));
    assert_eq!(post_json(&gui, "/api/classify/run", &json!({"src": "C:\\dati", "classifier": "rules"})).0, 503);

    let dom = dump_dom(&gui.url("/#classify"));
    assert!(dom.contains("rocket-tag.exe non trovato"), "{dom}");
    assert!(dom.contains(r#"<button id="classify-start" class="primary" type="button" disabled="">"#), "{dom}");
}

#[test]
fn rocket_tag_presente_abilita_la_classificazione() {
    let (_dir, exe) = install(true);
    let gui = Gui::start(&exe);
    assert_eq!(status(&gui)["available"], true);

    let dom = dump_dom(&gui.url("/#classify"));
    assert!(dom.contains(r#"<button id="classify-start" class="primary" type="button">"#), "{dom}");
    assert!(!dom.contains("non trovato"), "{dom}");
}
