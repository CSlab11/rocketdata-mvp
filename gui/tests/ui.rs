mod common;

use common::{Gui, dump_dom, gui_exe};

#[test]
fn menu_con_la_funzione_doc_ripper() {
    let gui = Gui::start(gui_exe());
    let dom = dump_dom(&gui.url("/"));
    assert!(dom.contains(r##"<a href="#doc-ripper" aria-current="page">doc-ripper</a>"##), "{dom}");
    assert!(dom.contains("<h1>doc-ripper</h1>"), "{dom}");
}
