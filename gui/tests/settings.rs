mod common;

use common::{Gui, dump_dom, install_with_fake, run, wait_output};

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
