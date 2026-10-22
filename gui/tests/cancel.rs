mod common;

use std::fs;
use std::time::{Duration, Instant};

use common::{Gui, dump_dom, install_with_fake, run, wait_output};

fn cancel(gui: &Gui) -> u16 {
    let agent = ureq::Agent::config_builder().http_status_as_error(false).build().new_agent();
    agent.post(&gui.url("/api/doc-ripper/cancel")).send_empty().unwrap().status().as_u16()
}

#[test]
fn annulla_l_esecuzione_in_corso() {
    let (_dir, exe) = install_with_fake();
    let gui = Gui::start(&exe);
    assert_eq!(cancel(&gui), 409, "niente da annullare");

    let src = tempfile::tempdir().unwrap();
    fs::write(src.path().join("lento"), "").unwrap();
    let started = Instant::now();
    assert_eq!(run(&gui, src.path().to_str().unwrap(), "C:\\dst"), 202);
    std::thread::sleep(Duration::from_millis(500));
    assert_eq!(cancel(&gui), 202);

    let snapshot = wait_output(&gui);
    // Il doc-ripper finto lento impiega oltre 2 s: annullato, termina molto prima.
    assert!(started.elapsed() < Duration::from_millis(1800), "{:?}", started.elapsed());
    assert_eq!(snapshot["cancelled"], true);
    let lines = snapshot["lines"].as_array().unwrap();
    assert!(!lines.iter().any(|l| l == "  Falliti: 0"), "{lines:?}");

    let dom = dump_dom(&gui.url("/#doc-ripper"));
    assert!(dom.contains("Conversione annullata"), "{dom}");

    // Dopo l'annullamento si può ripartire.
    fs::remove_file(src.path().join("lento")).unwrap();
    assert_eq!(run(&gui, src.path().to_str().unwrap(), "C:\\dst"), 202);
    assert_eq!(wait_output(&gui)["cancelled"], false);
}
