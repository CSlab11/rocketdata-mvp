mod common;

use common::{Gui, dump_dom, gui_exe, install_with_fake};

#[test]
fn menu_con_la_funzione_doc_ripper() {
    let gui = Gui::start(gui_exe());
    let dom = dump_dom(&gui.url("/"));
    assert!(dom.contains(r##"<a href="#doc-ripper" aria-current="page">doc-ripper</a>"##), "{dom}");
    assert!(dom.contains("<h1>doc-ripper</h1>"), "{dom}");
}

/// Scelta della modalità di conversione con le opzioni ai e simple, ai proposta (REQ-154).
#[test]
fn schermata_doc_ripper_con_modalita_ai_e_simple() {
    let (_dir, exe) = install_with_fake();
    let gui = Gui::start(&exe);
    let dom = dump_dom(&gui.url("/"));
    assert!(dom.contains(r#"id="mode-ai""#) && dom.contains(r#"id="mode-simple""#), "{dom}");
    assert!(dom.contains(r#"<label class="option" for="mode-ai">"#) && dom.contains(" ai</label>"), "{dom}");
    assert!(dom.contains(" simple</label>"), "{dom}");
}