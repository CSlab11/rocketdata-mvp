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

/// Docling finto accanto a doc-ripper.exe: `docling\python\python.exe` e `docling\convert_pdf.py`.
fn install_docling(dir: &std::path::Path) {
    fs::create_dir_all(dir.join("docling").join("python")).unwrap();
    fs::write(dir.join("docling").join("python").join("python.exe"), b"").unwrap();
    fs::write(dir.join("docling").join("convert_pdf.py"), b"").unwrap();
}

/// Senza Docling la modalità ai è disabilitata con un messaggio, resta simple; il server la rifiuta (REQ-155).
#[test]
fn senza_docling_la_modalita_ai_e_disabilitata() {
    let (dir, exe) = common::install_with_fake();
    let gui = Gui::start(&exe);
    let status = status(&gui);
    assert_eq!(status["docling"], false);
    assert!(status["docling_message"].as_str().unwrap().contains("Docling"), "{status}");

    let dom = dump_dom(&gui.url("/#doc-ripper"));
    assert!(dom.contains(r#"id="mode-ai" value="ai" disabled="""#), "{dom}");
    assert!(dom.contains("Modalità ai non disponibile"), "{dom}");

    let src = dir.path().to_str().unwrap();
    let (code, body) = common::post_json(&gui, "/api/doc-ripper/run", &serde_json::json!({"src": src, "dst": src, "mode": "ai"}));
    assert_eq!(code, 409, "{body}");
}

/// Con Docling installato la modalità ai è disponibile (REQ-155).
#[test]
fn con_docling_la_modalita_ai_e_disponibile() {
    let (dir, exe) = common::install_with_fake();
    install_docling(dir.path());
    let gui = Gui::start(&exe);
    let status = status(&gui);
    assert_eq!(status["docling"], true);
    assert!(status["docling_message"].is_null(), "{status}");

    let dom = dump_dom(&gui.url("/#doc-ripper"));
    assert!(!dom.contains("disabled=\"\" type=\"radio\"") && !dom.contains(r#"value="ai" disabled"#), "{dom}");
    assert!(!dom.contains("Modalità ai non disponibile"), "{dom}");
}