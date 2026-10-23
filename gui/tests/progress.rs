mod common;

use common::{Gui, dump_dom, install_with_fake, run, wait_output};

#[test]
fn barra_di_avanzamento_senza_righe_per_pagina() {
    let (_dir, exe) = install_with_fake();
    let gui = Gui::start(&exe);
    let src = tempfile::tempdir().unwrap();
    assert_eq!(run(&gui, src.path().to_str().unwrap(), "C:\\dst"), 202);
    let snapshot = wait_output(&gui);
    assert!(snapshot["lines"].as_array().unwrap().iter().any(|l| l == "[1/2] pagina 2/2"));

    let dom = dump_dom(&gui.url("/#doc-ripper"));
    assert!(dom.contains(r#"<div id="progress" class="progress"><progress max="100" value="100">"#), "{dom}");
    assert!(dom.contains("File 2 di 2 (100%)"), "{dom}");
    assert!(dom.contains("[2/2] vendite.xlsx"), "{dom}");
    assert!(!dom.contains("pagina 1/2"), "{dom}");
}
