mod common;

use std::fs;
use std::path::PathBuf;

use serde_json::json;

use common::{Gui, dump_dom, gui_exe, post_json};

/// Copia gui.exe in una cartella temporanea, con o senza un search-rm3.exe accanto.
fn install(with_search_rm3: bool) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let exe = dir.path().join("gui.exe");
    fs::copy(gui_exe(), &exe).unwrap();
    if with_search_rm3 {
        fs::write(dir.path().join("search-rm3.exe"), b"").unwrap();
    }
    (dir, exe)
}

fn status(gui: &Gui) -> serde_json::Value {
    let body = ureq::get(&gui.url("/api/search/status")).call().unwrap().into_body().read_to_string().unwrap();
    serde_json::from_str(&body).unwrap()
}

#[test]
fn search_rm3_assente_disabilita_indicizzazione_e_ricerca() {
    let (_dir, exe) = install(false);
    let gui = Gui::start(&exe);
    let status = status(&gui);
    assert_eq!(status["available"], false);
    assert!(status["message"].as_str().unwrap().contains("search-rm3.exe non trovato"));
    assert_eq!(post_json(&gui, "/api/search/index", &json!({ "src": "C:\\dati" })).0, 503);
    assert_eq!(post_json(&gui, "/api/search/query", &json!({ "src": "C:\\dati", "q": "clock" })).0, 503);

    let dom = dump_dom(&gui.url("/#search"));
    assert!(dom.contains("search-rm3.exe non trovato"), "{dom}");
    assert!(dom.contains(r#"<button id="search-index" type="button" disabled="">"#), "{dom}");
    assert!(dom.contains(r#"<button id="search-run" class="primary" type="submit" disabled="">"#), "{dom}");
}

#[test]
fn search_rm3_presente_abilita_indicizzazione_e_ricerca() {
    let (_dir, exe) = install(true);
    let gui = Gui::start(&exe);
    assert_eq!(status(&gui)["available"], true);

    let dom = dump_dom(&gui.url("/#search"));
    assert!(dom.contains(r#"<button id="search-index" type="button">"#), "{dom}");
    assert!(dom.contains(r#"<button id="search-run" class="primary" type="submit">"#), "{dom}");
    assert!(!dom.contains("non trovato"), "{dom}");
}
