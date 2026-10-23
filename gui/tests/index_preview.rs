mod common;

use common::{Gui, dump_dom, install_with_fake, run, wait_output};

#[test]
fn anteprima_dell_indice_a_fine_conversione() {
    let (_dir, exe) = install_with_fake();
    let gui = Gui::start(&exe);
    let src = tempfile::tempdir().unwrap();
    let dst = tempfile::tempdir().unwrap();
    assert_eq!(run(&gui, src.path().to_str().unwrap(), dst.path().to_str().unwrap()), 202);
    wait_output(&gui);

    let dom = dump_dom(&gui.url("/#doc-ripper"));
    assert!(dom.contains(r#"<section id="preview" class="preview"><h2>Anteprima di index.md</h2>"#), "{dom}");
    assert!(dom.contains("<h3>Indice dei file generati</h3>"), "{dom}");
    assert!(dom.contains("<th>File generato</th><th>Originale</th>"), "{dom}");
    assert!(dom.contains(r#"<td><a href="report_pdf.md">report_pdf.md</a></td>"#), "{dom}");
    assert!(dom.contains(r#"<td><a href="C:/dati/report%20(v1).pdf">report (v1).pdf</a></td>"#), "{dom}");
}

#[test]
fn nessun_indice_se_doc_ripper_non_lo_scrive() {
    let (_dir, exe) = install_with_fake();
    let gui = Gui::start(&exe);
    let src = tempfile::tempdir().unwrap();
    let dst = tempfile::tempdir().unwrap();
    std::fs::write(src.path().join("fallisci"), "").unwrap();
    std::fs::write(dst.path().join("index.md"), "# Vecchio").unwrap();
    assert_eq!(run(&gui, src.path().to_str().unwrap(), dst.path().to_str().unwrap()), 202);
    wait_output(&gui);

    let dom = dump_dom(&gui.url("/#doc-ripper"));
    assert!(dom.contains(r#"<p class="muted">Nessun indice disponibile</p>"#), "{dom}");
    assert!(!dom.contains("Vecchio"), "{dom}");
}
