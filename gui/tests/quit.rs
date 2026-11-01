mod common;

use std::fs;
use std::time::{Duration, Instant};

use common::{Gui, dump_dom, gui_exe, install_with_fake, install_with_fake_tag, plain_canonical, post_json, run, wait_running};

fn wait_exit(gui: &mut Gui) -> bool {
    let deadline = Instant::now() + Duration::from_secs(8);
    while Instant::now() < deadline {
        if gui.child.try_wait().unwrap().is_some() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}

#[test]
fn esci_arresta_server_e_doc_ripper() {
    let (dir, exe) = install_with_fake();
    let mut gui = Gui::start(&exe);
    let src = tempfile::tempdir().unwrap();
    fs::write(src.path().join("lento"), "").unwrap();
    assert_eq!(run(&gui, src.path().to_str().unwrap(), "C:\\dst"), 202);
    let doc_ripper = plain_canonical(dir.path()).join("doc-ripper.exe");
    assert!(wait_running(&doc_ripper, 1), "doc-ripper finto non in esecuzione");

    let response = ureq::post(&gui.url("/api/shutdown")).send_empty().unwrap();
    assert_eq!(response.status(), 202);
    assert!(wait_exit(&mut gui), "il server non si è arrestato");
    assert!(wait_running(&doc_ripper, 0), "doc-ripper ancora in esecuzione");
}

#[test]
fn esci_arresta_anche_rocket_tag() {
    let (dir, exe) = install_with_fake_tag();
    let mut gui = Gui::start(&exe);
    let src = tempfile::tempdir().unwrap();
    fs::write(src.path().join("lento"), "").unwrap();
    let request = serde_json::json!({"src": src.path().to_str().unwrap(), "classifier": "rules"});
    assert_eq!(post_json(&gui, "/api/classify/run", &request).0, 202);
    let rocket_tag = plain_canonical(dir.path()).join("rocket-tag.exe");
    assert!(wait_running(&rocket_tag, 1), "rocket-tag finto non in esecuzione");

    let response = ureq::post(&gui.url("/api/shutdown")).send_empty().unwrap();
    assert_eq!(response.status(), 202);
    assert!(wait_exit(&mut gui), "il server non si è arrestato");
    assert!(wait_running(&rocket_tag, 0), "rocket-tag ancora in esecuzione");
}

#[test]
fn pulsante_esci_nella_pagina() {
    let gui = Gui::start(gui_exe());
    let dom = dump_dom(&gui.url("/"));
    assert!(dom.contains(r#"<button id="quit" class="quit" type="button">Esci</button>"#), "{dom}");
}
