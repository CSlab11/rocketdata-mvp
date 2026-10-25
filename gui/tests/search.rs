mod common;

use std::fs;

use serde_json::json;

use common::{Gui, dump_dom, install_with_fake_search, post_json};

#[test]
fn indicizza_e_cerca_con_search_rm3() {
    let (_dir, exe) = install_with_fake_search();
    let gui = Gui::start(&exe);
    let src = tempfile::tempdir().unwrap();
    let src_path = src.path().to_str().unwrap();
    let index = src.path().join(".index");

    let (status, body) = post_json(&gui, "/api/search/index", &json!({ "src": src_path }));
    assert_eq!(status, 204, "{body}");
    let args = fs::read_to_string(index.join("argomenti-index.txt")).unwrap();
    assert_eq!(args, format!("index\n--src\n{src_path}\n--index\n{}", index.display()));

    let (status, body) = post_json(&gui, "/api/search/query", &json!({ "src": src_path, "q": "clock RCC" }));
    assert_eq!(status, 200, "{body}");
    let response: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(response["query"], "clock RCC");
    assert_eq!(response["results"][0]["heading"], "7.3.7 Clock control");
    let args = fs::read_to_string(index.join("argomenti-query.txt")).unwrap();
    assert_eq!(args, format!("query\n--index\n{}\n--q\nclock RCC", index.display()));
}

#[test]
fn riporta_gli_errori_di_search_rm3() {
    let (_dir, exe) = install_with_fake_search();
    let gui = Gui::start(&exe);
    let src = tempfile::tempdir().unwrap();
    let src_path = src.path().to_str().unwrap();

    // Codice di uscita diverso da 0: il messaggio di stderr arriva all'interfaccia.
    let (status, body) = post_json(&gui, "/api/search/query", &json!({ "src": src_path, "q": "clock" }));
    assert_eq!(status, 500);
    assert!(body.contains("non contiene un indice"), "{body}");
    fs::write(src.path().join("fallisci"), "").unwrap();
    let (status, body) = post_json(&gui, "/api/search/index", &json!({ "src": src_path }));
    assert_eq!((status, body.as_str()), (500, "errore finto: indice non creato"));

    assert_eq!(post_json(&gui, "/api/search/index", &json!({ "src": " " })).0, 400);
    assert_eq!(post_json(&gui, "/api/search/query", &json!({ "src": src_path, "q": "" })).0, 400);
}

fn hidden(path: &std::path::Path) -> bool {
    use std::os::windows::fs::MetadataExt;
    fs::metadata(path).unwrap().file_attributes() & 0x2 != 0
}

#[test]
fn cartella_index_creata_se_manca_e_non_nascosta() {
    let (_dir, exe) = install_with_fake_search();
    let gui = Gui::start(&exe);
    let src = tempfile::tempdir().unwrap();
    let src_path = src.path().to_str().unwrap();
    let index = src.path().join(".index");

    // La crea gui, anche per una query: il search-rm3 finto non crea nulla se l'indice manca.
    assert_eq!(post_json(&gui, "/api/search/query", &json!({ "src": src_path, "q": "clock" })).0, 500);
    assert!(index.is_dir());
    assert!(!hidden(&index));

    let status = std::process::Command::new("attrib").arg("+h").arg(&index).status().unwrap();
    assert!(status.success() && hidden(&index));
    assert_eq!(post_json(&gui, "/api/search/index", &json!({ "src": src_path })).0, 204);
    assert!(!hidden(&index), "la cartella .index non deve restare nascosta");

    // Una cartella di origine inesistente non viene creata.
    let missing = src.path().join("manca");
    let (status, body) = post_json(&gui, "/api/search/index", &json!({ "src": missing }));
    assert_eq!(status, 500);
    assert!(body.contains("non esiste"), "{body}");
    assert!(!missing.exists());
}

#[test]
fn propone_l_ultima_destinazione_di_doc_ripper() {
    let (_dir, exe) = install_with_fake_search();
    let appdata = tempfile::tempdir().unwrap();
    fs::create_dir(appdata.path().join("rocketdata-gui")).unwrap();
    let settings = json!({ "src": r"C:\Origine", "dst": r"C:\Uscita conversioni" });
    fs::write(appdata.path().join("rocketdata-gui/settings.json"), settings.to_string()).unwrap();
    let gui = Gui::start_with_appdata(&exe, appdata.path());

    let dom = dump_dom(&gui.url("/#search"));
    assert!(dom.contains(r#"<input id="search-src" type="text" spellcheck="false" autocomplete="off" value="C:\Uscita conversioni">"#), "{dom}");
    assert!(dom.contains(r#"<button id="search-src-browse" type="button">Sfoglia…</button>"#), "{dom}");
}

#[test]
fn schermata_di_ricerca() {
    let (_dir, exe) = install_with_fake_search();
    let gui = Gui::start(&exe);
    let dom = dump_dom(&gui.url("/#search"));
    assert!(dom.contains(r##"<a href="#search" aria-current="page">Ricerca</a>"##), "{dom}");
    assert!(dom.contains(r#"<button id="search-index" type="button">Indicizza</button>"#), "{dom}");
    assert!(dom.contains(r#"<button id="search-run" class="primary" type="submit">Cerca</button>"#), "{dom}");
}
