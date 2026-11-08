mod common;

use common::{Gui, dump_dom, install_with_fake, post_json, run, wait_output};

#[test]
fn ricorda_le_cartelle_dopo_il_riavvio() {
    let (_dir, exe) = install_with_fake();
    let appdata = tempfile::tempdir().unwrap();
    let src = tempfile::tempdir().unwrap();
    let src = src.path().to_str().unwrap().to_string();
    let dst = r"C:\Uscita conversioni".to_string();

    {
        let gui = Gui::start_with_appdata(&exe, appdata.path());
        assert_eq!(run(&gui, &src, &dst), 202);
        wait_output(&gui);
    }
    let saved = std::fs::read_to_string(appdata.path().join("rocketdata-gui/settings.json")).unwrap();
    assert!(saved.contains("Uscita conversioni"), "{saved}");

    // Nuovo processo: i campi sono precompilati con le ultime cartelle.
    let gui = Gui::start_with_appdata(&exe, appdata.path());
    let body = ureq::get(&gui.url("/api/settings")).call().unwrap().into_body().read_to_string().unwrap();
    let settings: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(settings["src"], src.as_str());
    assert_eq!(settings["dst"], dst.as_str());

    let dom = dump_dom(&gui.url("/#doc-ripper"));
    assert!(dom.contains(r#"value="C:\Uscita conversioni""#), "{dom}");
}

/// Modalità predefinita ai; l'ultima modalità usata si ricorda anche dopo il riavvio (REQ-156).
#[test]
fn ricorda_la_modalita_dopo_il_riavvio() {
    let (dir, exe) = install_with_fake();
    // Docling finto accanto a doc-ripper.exe, così la modalità ai è disponibile (REQ-155).
    std::fs::create_dir_all(dir.path().join("docling").join("python")).unwrap();
    std::fs::write(dir.path().join("docling").join("python").join("python.exe"), b"").unwrap();
    std::fs::write(dir.path().join("docling").join("convert_pdf.py"), b"").unwrap();
    let appdata = tempfile::tempdir().unwrap();
    let src = tempfile::tempdir().unwrap();
    let src = src.path().to_str().unwrap().to_string();

    {
        let gui = Gui::start_with_appdata(&exe, appdata.path());
        let dom = dump_dom(&gui.url("/#doc-ripper"));
        assert!(dom.contains(r#"id="mode-ai" value="ai" checked="""#), "{dom}");
        let (code, body) = post_json(&gui, "/api/doc-ripper/run", &serde_json::json!({"src": src, "dst": src, "mode": "simple"}));
        assert_eq!(code, 202, "{body}");
        wait_output(&gui);
    }

    let gui = Gui::start_with_appdata(&exe, appdata.path());
    let body = ureq::get(&gui.url("/api/settings")).call().unwrap().into_body().read_to_string().unwrap();
    let settings: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(settings["mode"], "simple");
    let dom = dump_dom(&gui.url("/#doc-ripper"));
    assert!(dom.contains(r#"id="mode-simple" value="simple" checked="""#), "{dom}");
    assert!(!dom.contains(r#"value="ai" checked"#), "{dom}");
}

/// Un file delle impostazioni salvato prima di REQ-156, senza `mode`, si legge ancora.
#[test]
fn impostazioni_senza_modalita() {
    let (_dir, exe) = install_with_fake();
    let appdata = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(appdata.path().join("rocketdata-gui")).unwrap();
    std::fs::write(appdata.path().join("rocketdata-gui/settings.json"), r#"{"src":"C:\\a","dst":"C:\\b"}"#).unwrap();
    let gui = Gui::start_with_appdata(&exe, appdata.path());
    let body = ureq::get(&gui.url("/api/settings")).call().unwrap().into_body().read_to_string().unwrap();
    let settings: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(settings["dst"], r"C:\b");
    assert!(settings["mode"].is_null(), "{settings}");
}