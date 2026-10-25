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
    assert_eq!(args, format!("query\n--index\n{}\n--q\nclock RCC\n--top\n10", index.display()));
}

#[test]
fn numero_di_risultati_passato_come_top_e_ricordato() {
    let (dir, exe) = install_with_fake_search();
    let fake = common::gui_exe().parent().unwrap().join("examples").join("fake_doc_ripper.exe");
    fs::copy(&fake, dir.path().join("doc-ripper.exe")).unwrap();
    let appdata = tempfile::tempdir().unwrap();
    let src = tempfile::tempdir().unwrap();
    let src_path = src.path().to_str().unwrap();
    let query = |gui: &Gui, top: serde_json::Value| {
        post_json(gui, "/api/search/query", &json!({ "src": src_path, "q": "clock", "top": top }))
    };

    {
        let gui = Gui::start_with_appdata(&exe, appdata.path());
        let dom = dump_dom(&gui.url("/#search"));
        assert!(dom.contains(r#"id="search-top" class="top" type="number" min="1" step="1" required="" value="10""#), "{dom}");

        assert_eq!(post_json(&gui, "/api/search/index", &json!({ "src": src_path })).0, 204);
        for invalid in [json!(0), json!(-2), json!(2.5), json!("tre")] {
            assert_eq!(query(&gui, invalid.clone()).0, 400, "{invalid}");
        }
        let (status, body) = query(&gui, json!(3));
        assert_eq!(status, 200, "{body}");
        let args = fs::read_to_string(src.path().join(".index/argomenti-query.txt")).unwrap();
        assert!(args.ends_with("--top\n3"), "{args}");

        // Una conversione salva le cartelle senza perdere il numero di risultati.
        assert_eq!(common::run(&gui, src_path, r"C:\Uscita"), 202);
        common::wait_output(&gui);
    }

    // Dopo il riavvio del server viene proposto l'ultimo valore usato.
    let gui = Gui::start_with_appdata(&exe, appdata.path());
    let body = ureq::get(&gui.url("/api/settings")).call().unwrap().into_body().read_to_string().unwrap();
    let settings: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!((settings["top"].as_u64(), settings["dst"].as_str()), (Some(3), Some(r"C:\Uscita")));
    let dom = dump_dom(&gui.url("/#search"));
    assert!(dom.contains(r#"id="search-top" class="top" type="number" min="1" step="1" required="" value="3""#), "{dom}");
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
fn ricaricando_la_pagina_torna_l_ultima_ricerca() {
    let (_dir, exe) = install_with_fake_search();
    let appdata = tempfile::tempdir().unwrap();
    let src = tempfile::tempdir().unwrap();
    let src_path = src.path().to_str().unwrap();

    {
        let gui = Gui::start_with_appdata(&exe, appdata.path());
        let dom = dump_dom(&gui.url("/#search"));
        assert!(!dom.contains(r#"class="hit""#), "nessuna ricerca ancora: {dom}");
        assert_eq!(ureq::get(&gui.url("/api/search/last")).call().unwrap().into_body().read_to_string().unwrap(), "null");

        assert_eq!(post_json(&gui, "/api/search/index", &json!({ "src": src_path })).0, 204);
        let (status, body) = post_json(&gui, "/api/search/query", &json!({ "src": src_path, "q": "clock RCC", "top": 2 }));
        assert_eq!(status, 200, "{body}");

        // Risultato reso (REQ-072) e ricerca ripristinata dopo il ricaricamento (REQ-077).
        let dom = dump_dom(&gui.url("/#search"));
        assert!(dom.contains(&format!(r#"value="{src_path}""#)), "{dom}");
        assert!(dom.contains(r#"<input id="search-q" type="search" autocomplete="off" value="clock RCC">"#), "{dom}");
        assert!(dom.contains(r#"required="" value="2""#), "{dom}");
        assert!(dom.contains(r##"<a href="#" class="hit-file">manuali/rm0008.md</a>"##), "{dom}");
        assert!(dom.contains("<span>riga 12</span><span>punteggio 4.250</span>"), "{dom}");
        assert!(dom.contains(r#"<h2 class="hit-heading">7.3.7 Clock control</h2>"#), "{dom}");
        assert!(dom.contains("<p>Il registro <strong>RCC_APB2ENR</strong> abilita i clock.</p>"), "{dom}");
    }

    // Dopo il riavvio del server la schermata riparte senza risultati.
    let gui = Gui::start_with_appdata(&exe, appdata.path());
    assert_eq!(ureq::get(&gui.url("/api/search/last")).call().unwrap().into_body().read_to_string().unwrap(), "null");
    let dom = dump_dom(&gui.url("/#search"));
    assert!(!dom.contains(r#"class="hit""#), "{dom}");
}

fn stale(gui: &Gui, src: &str) -> bool {
    let src = percent_encoding::utf8_percent_encode(src, percent_encoding::NON_ALPHANUMERIC);
    let body = ureq::get(&gui.url(&format!("/api/search/stale?src={src}"))).call().unwrap().into_body().read_to_string().unwrap();
    serde_json::from_str::<serde_json::Value>(&body).unwrap()["stale"].as_bool().unwrap()
}

#[test]
fn indicizza_in_rosso_dopo_una_conversione() {
    let (dir, exe) = install_with_fake_search();
    let fake = common::gui_exe().parent().unwrap().join("examples").join("fake_doc_ripper.exe");
    fs::copy(&fake, dir.path().join("doc-ripper.exe")).unwrap();
    let appdata = tempfile::tempdir().unwrap();
    let src = tempfile::tempdir().unwrap();
    let dst = tempfile::tempdir().unwrap();
    let dst_path = dst.path().to_str().unwrap();
    let stale_button = r#"<button id="search-index" type="button" class="stale" title="L'indice di questa cartella va rifatto">"#;

    {
        let gui = Gui::start_with_appdata(&exe, appdata.path());
        // Entrambe indicizzate: poi la conversione segna solo la destinazione (REQ-078 a parte).
        for folder in [dst_path, src.path().to_str().unwrap()] {
            assert_eq!(post_json(&gui, "/api/search/index", &json!({ "src": folder })).0, 204);
        }
        assert!(!stale(&gui, dst_path));

        assert_eq!(common::run(&gui, src.path().to_str().unwrap(), dst_path), 202);
        common::wait_output(&gui);
        assert!(stale(&gui, dst_path));
        assert!(stale(&gui, &format!("{}\\", dst_path.to_uppercase())), "stessa cartella scritta in modo diverso");
        assert!(stale(&gui, &dst_path.replace('\\', "/")));
        assert!(!stale(&gui, src.path().to_str().unwrap()));

        // La cartella proposta è la destinazione della conversione: pulsante rosso.
        let dom = dump_dom(&gui.url("/#search"));
        assert!(dom.contains(stale_button), "{dom}");
    }

    // Lo stato resta dopo il riavvio del server e dopo un'indicizzazione fallita.
    let gui = Gui::start_with_appdata(&exe, appdata.path());
    assert!(stale(&gui, dst_path));
    fs::write(dst.path().join("fallisci"), "").unwrap();
    assert_eq!(post_json(&gui, "/api/search/index", &json!({ "src": dst_path })).0, 500);
    assert!(stale(&gui, dst_path));

    // Torna normale solo dopo un'indicizzazione riuscita.
    fs::remove_file(dst.path().join("fallisci")).unwrap();
    assert_eq!(post_json(&gui, "/api/search/index", &json!({ "src": dst_path })).0, 204);
    assert!(!stale(&gui, dst_path));
    let dom = dump_dom(&gui.url("/#search"));
    assert!(dom.contains(r#"<button id="search-index" type="button">Indicizza</button>"#), "{dom}");
}

#[test]
fn indicizza_in_rosso_per_le_cartelle_mai_indicizzate() {
    let (_dir, exe) = install_with_fake_search();
    let appdata = tempfile::tempdir().unwrap();
    let src = tempfile::tempdir().unwrap();
    let src_path = src.path().to_str().unwrap();
    fs::create_dir(appdata.path().join("rocketdata-gui")).unwrap();
    let settings = json!({ "src": "", "dst": src_path });
    fs::write(appdata.path().join("rocketdata-gui/settings.json"), settings.to_string()).unwrap();
    let gui = Gui::start_with_appdata(&exe, appdata.path());

    // .index assente, poi vuota: la cartella non è mai stata indicizzata.
    assert!(stale(&gui, src_path));
    let dom = dump_dom(&gui.url("/#search"));
    assert!(dom.contains(r#"<button id="search-index" type="button" class="stale""#), "{dom}");
    fs::create_dir(src.path().join(".index")).unwrap();
    assert!(stale(&gui, src_path));

    assert_eq!(post_json(&gui, "/api/search/index", &json!({ "src": src_path })).0, 204);
    assert!(!stale(&gui, src_path));

    // Una cartella che non esiste non ha niente da indicizzare: nessun segnale.
    assert!(!stale(&gui, &format!("{src_path}\\manca")));
    assert!(!stale(&gui, ""));
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
