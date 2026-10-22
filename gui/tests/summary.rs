mod common;

use std::fs;

use common::{Gui, dump_dom, install_with_fake, run, wait_output};

#[test]
fn riepilogo_strutturato_ed_esito() {
    let (_dir, exe) = install_with_fake();
    let gui = Gui::start(&exe);
    let src = tempfile::tempdir().unwrap();
    assert_eq!(run(&gui, src.path().to_str().unwrap(), "C:\\dst"), 202);

    let snapshot = wait_output(&gui);
    let summary = &snapshot["summary"];
    assert_eq!(summary["converted"].as_array().unwrap().len(), 2);
    assert_eq!(summary["converted"][1]["outputs"][1], "vendite_xlsx_Febbraio.csv");
    assert_eq!(summary["skipped"][0]["file"], "note.txt");
    assert_eq!(summary["skipped"][0]["reason"], "non supportato");
    assert_eq!(summary["failed"].as_array().unwrap().len(), 0);

    let dom = dump_dom(&gui.url("/#doc-ripper"));
    assert!(dom.contains("Conversione completata."), "{dom}");
    assert!(dom.contains("<h2>Convertiti: 2</h2>"), "{dom}");
    assert!(dom.contains("<li>note.txt — non supportato</li>"), "{dom}");
    assert!(dom.contains("<h2>Falliti: 0</h2>"), "{dom}");
}

#[test]
fn esito_con_errori() {
    let (_dir, exe) = install_with_fake();
    let gui = Gui::start(&exe);
    let src = tempfile::tempdir().unwrap();
    fs::write(src.path().join("fallisci"), "").unwrap();
    assert_eq!(run(&gui, src.path().to_str().unwrap(), "C:\\dst"), 202);

    let snapshot = wait_output(&gui);
    assert_eq!(snapshot["summary"]["failed"][0]["file"], "rotto.docx");
    assert_eq!(snapshot["summary"]["failed"][0]["reason"], "docx non valido");

    let dom = dump_dom(&gui.url("/#doc-ripper"));
    assert!(dom.contains("Conversione terminata con errori (codice di uscita 1)."), "{dom}");
    assert!(dom.contains("<li>rotto.docx — docx non valido</li>"), "{dom}");
}
