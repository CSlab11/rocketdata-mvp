mod common;

use std::fs;
use std::io::{BufRead, BufReader};

use common::{Gui, install_with_fake, run};

fn running(gui: &Gui) -> bool {
    let body = ureq::get(&gui.url("/api/doc-ripper/output")).call().unwrap().into_body().read_to_string().unwrap();
    serde_json::from_str::<serde_json::Value>(&body).unwrap()["running"] == true
}

#[test]
fn le_righe_arrivano_prima_della_fine() {
    let (_dir, exe) = install_with_fake();
    let gui = Gui::start(&exe);
    let src = tempfile::tempdir().unwrap();
    fs::write(src.path().join("lento"), "").unwrap();
    assert_eq!(run(&gui, src.path().to_str().unwrap(), "C:\\dst"), 202);

    let response = ureq::get(&gui.url("/api/doc-ripper/events")).call().unwrap();
    let mut reader = BufReader::new(response.into_body().into_reader());
    let mut event = String::new();
    let mut lines = Vec::new();
    let mut seen_while_running = false;
    loop {
        let mut raw = String::new();
        assert!(reader.read_line(&mut raw).unwrap() > 0, "flusso chiuso prima della fine");
        let raw = raw.trim_end();
        if let Some(name) = raw.strip_prefix("event: ") {
            event = name.to_string();
        } else if let Some(data) = raw.strip_prefix("data: ") {
            match event.as_str() {
                "line" => {
                    if data == "Riepilogo" && running(&gui) {
                        seen_while_running = true;
                    }
                    lines.push(data.to_string());
                }
                "end" => {
                    assert_eq!(data, "0");
                    break;
                }
                _ => {}
            }
        }
    }
    assert!(seen_while_running, "nessuna riga ricevuta durante l'esecuzione");
    assert!(lines.contains(&"  Falliti: 0".to_string()), "{lines:?}");
}
