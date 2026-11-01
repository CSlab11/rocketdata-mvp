mod common;

use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use common::{Gui, dump_dom, gui_exe, install_with_fake_tag, post_json};

/// Cartella di Markdown con il testo delle unità restituite dal rocket-tag finto.
fn markdown_folder() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("sotto")).unwrap();
    fs::write(dir.path().join("a.md"), "Primo testo.\n\nSecondo testo\nsu due righe.\n").unwrap();
    fs::write(dir.path().join("sotto/b.md"), "- Terzo testo.\n").unwrap();
    dir
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn output(gui: &Gui) -> Value {
    let body = ureq::get(&gui.url("/api/classify/output")).call().unwrap().into_body().read_to_string().unwrap();
    serde_json::from_str(&body).unwrap()
}

/// Attende la fine dell'esecuzione di rocket-tag.
fn wait_end(gui: &Gui) -> Value {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let snapshot = output(gui);
        if snapshot["running"] == false && (snapshot["exit_code"] != Value::Null || snapshot["cancelled"] == true) {
            return snapshot;
        }
        assert!(Instant::now() < deadline, "rocket-tag non è terminato");
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// gui esegue rocket-tag come processo separato, con il comando del classificatore scelto (REQ-133).
#[test]
fn esegue_rocket_tag_con_il_classificatore_scelto() {
    let (dir, exe) = install_with_fake_tag();
    let gui = Gui::start(&exe);
    let folder = markdown_folder();
    let src = folder.path().to_str().unwrap();

    let (status, _) = post_json(&gui, "/api/classify/run", &json!({"src": src, "classifier": "rules"}));
    assert_eq!(status, 202);
    assert_eq!(wait_end(&gui)["exit_code"], 0);
    let received = read_json(&folder.path().join(".classify/rules.json.args.json"));
    let args: Vec<&str> = received["args"].as_array().unwrap().iter().map(|a| a.as_str().unwrap()).collect();
    assert_eq!(args[..3], ["classify", "rules", src]);
    let config = dir.path().canonicalize().unwrap().join("rocket-tag-config");
    let schema = args[args.iter().position(|a| *a == "--schema").unwrap() + 1];
    assert!(Path::new(schema.trim_start_matches(r"\\?\")).ends_with("rocket-tag-config/label_schema.json"), "{schema} {config:?}");

    let request = json!({"src": src, "classifier": "llm", "base_url": "http://127.0.0.1:8080/v1", "model": "modello"});
    assert_eq!(post_json(&gui, "/api/classify/run", &request).0, 202);
    assert_eq!(wait_end(&gui)["exit_code"], 0);
    let received = read_json(&folder.path().join(".classify/llm.json.args.json"));
    let args: Vec<&str> = received["args"].as_array().unwrap().iter().map(|a| a.as_str().unwrap()).collect();
    assert_eq!(args[..2], ["classify", "llm"]);
    assert!(args.windows(2).any(|w| w == ["--base-url", "http://127.0.0.1:8080/v1"]));
    assert!(args.windows(2).any(|w| w == ["--model", "modello"]));
}

/// File di uscita in `.classify` ed esclusione di `index.md` e `.index` (REQ-136).
#[test]
fn file_di_uscita_ed_esclusioni() {
    let (_dir, exe) = install_with_fake_tag();
    let gui = Gui::start(&exe);
    let folder = markdown_folder();
    assert!(!folder.path().join(".classify").exists());
    let request = json!({"src": folder.path().to_str().unwrap(), "classifier": "rules"});
    assert_eq!(post_json(&gui, "/api/classify/run", &request).0, 202);
    wait_end(&gui);
    assert!(folder.path().join(".classify/rules.json").is_file());
    let received = read_json(&folder.path().join(".classify/rules.json.args.json"));
    let args: Vec<&str> = received["args"].as_array().unwrap().iter().map(|a| a.as_str().unwrap()).collect();
    let output = folder.path().join(".classify").join("rules.json");
    assert!(args.windows(2).any(|w| w == ["-o", output.to_str().unwrap()]), "{args:?}");
    assert!(args.windows(2).any(|w| w == ["--exclude", "index.md"]));
    assert!(args.windows(2).any(|w| w == ["--exclude", ".index"]));
}

/// `classify llm` riceve `rules.json` solo se esiste (REQ-139).
#[test]
fn passa_le_regole_all_llm_se_esistono() {
    let (_dir, exe) = install_with_fake_tag();
    let gui = Gui::start(&exe);
    let folder = markdown_folder();
    let src = folder.path().to_str().unwrap();
    let llm = json!({"src": src, "classifier": "llm", "base_url": "http://x/v1", "model": "m"});
    let args = || read_json(&folder.path().join(".classify/llm.json.args.json"))["args"].clone();
    assert_eq!(post_json(&gui, "/api/classify/run", &llm).0, 202);
    wait_end(&gui);
    assert!(!args().as_array().unwrap().contains(&json!("--rules-out")));

    assert_eq!(post_json(&gui, "/api/classify/run", &json!({"src": src, "classifier": "rules"})).0, 202);
    wait_end(&gui);
    assert_eq!(post_json(&gui, "/api/classify/run", &llm).0, 202);
    wait_end(&gui);
    let args = args();
    let args = args.as_array().unwrap();
    let position = args.iter().position(|a| a == "--rules-out").expect("--rules-out");
    assert!(args[position + 1].as_str().unwrap().ends_with(r".classify\rules.json"));
}

/// URL e modello si ricordano; la chiave passa solo dalla variabile d'ambiente (REQ-141).
#[test]
fn server_llm_ricordato_e_chiave_solo_in_ambiente() {
    let (_dir, exe) = install_with_fake_tag();
    let appdata = tempfile::tempdir().unwrap();
    let folder = markdown_folder();
    let request = json!({"src": folder.path().to_str().unwrap(), "classifier": "llm",
                         "base_url": "http://server/v1", "model": "qwen", "api_key": "segreta"});
    {
        let gui = Gui::start_with_appdata(&exe, appdata.path());
        assert_eq!(post_json(&gui, "/api/classify/run", &request).0, 202);
        let snapshot = wait_end(&gui);
        assert!(!snapshot.to_string().contains("segreta"));
    }
    let received = read_json(&folder.path().join(".classify/llm.json.args.json"));
    assert_eq!(received["key"], "segreta");
    assert!(!received["args"].to_string().contains("segreta"), "mai come argomento");
    let settings = fs::read_to_string(appdata.path().join("rocketdata-gui/settings.json")).unwrap();
    assert!(!settings.contains("segreta"), "la chiave non si salva");

    let gui = Gui::start_with_appdata(&exe, appdata.path());
    let dom = dump_dom(&gui.url("/#classify"));
    assert!(dom.contains(r#"value="http://server/v1""#), "{dom}");
    assert!(dom.contains(r#"value="qwen""#), "{dom}");
    assert!(dom.contains(r#"id="classify-key""#) && dom.contains(r#"type="password""#), "{dom}");
}

/// File di configurazione predefiniti accanto a gui.exe, oppure quelli scelti e ricordati (REQ-140).
#[test]
fn file_di_configurazione_modificabili_e_ricordati() {
    let (_dir, exe) = install_with_fake_tag();
    let appdata = tempfile::tempdir().unwrap();
    let folder = markdown_folder();
    let src = folder.path().to_str().unwrap();
    let args = |folder: &Path| read_json(&folder.join(".classify/rules.json.args.json"))["args"].clone();
    {
        let gui = Gui::start_with_appdata(&exe, appdata.path());
        assert_eq!(post_json(&gui, "/api/classify/run", &json!({"src": src, "classifier": "rules"})).0, 202);
        wait_end(&gui);
        let received = args(folder.path());
        let list = received.as_array().unwrap();
        let position = list.iter().position(|a| a == "--rules").unwrap();
        assert!(list[position + 1].as_str().unwrap().ends_with(r"rocket-tag-config\rules.yaml"));

        let request = json!({"src": src, "classifier": "rules",
                             "config": {"schema": "D:\\mio\\schema.json", "rules": "D:\\mio\\regole.yaml"}});
        assert_eq!(post_json(&gui, "/api/classify/run", &request).0, 202);
        wait_end(&gui);
        let received = args(folder.path());
        let list: Vec<_> = received.as_array().unwrap().iter().map(|a| a.as_str().unwrap()).collect();
        assert!(list.windows(2).any(|w| w == ["--schema", r"D:\mio\schema.json"]));
        assert!(list.windows(2).any(|w| w == ["--rules", r"D:\mio\regole.yaml"]));
    }
    let gui = Gui::start_with_appdata(&exe, appdata.path());
    let dom = dump_dom(&gui.url("/#classify"));
    assert!(dom.contains(r#"value="D:\mio\regole.yaml""#), "{dom}");
    assert!(dom.contains(r#"placeholder=""#) && dom.contains(r"rocket-tag-config\prompt_guide.md"), "{dom}");
}

/// Conversione e classificazione non girano insieme sulla stessa cartella (REQ-147).
#[test]
fn conversione_e_classificazione_si_escludono() {
    let (dir, exe) = install_with_fake_tag();
    fs::copy(gui_exe().parent().unwrap().join("examples/fake_doc_ripper.exe"), dir.path().join("doc-ripper.exe")).unwrap();
    let gui = Gui::start(&exe);
    let folder = markdown_folder();
    fs::write(folder.path().join("lento"), "").unwrap();
    let dst = folder.path().to_str().unwrap();
    let other = tempfile::tempdir().unwrap();
    let classify = json!({"src": dst, "classifier": "rules"});

    assert_eq!(post_json(&gui, "/api/classify/run", &classify).0, 202);
    let same = json!({"src": dst, "dst": dst.to_uppercase()});
    let (status, message) = post_json(&gui, "/api/doc-ripper/run", &same);
    assert_eq!(status, 409);
    assert!(message.contains("in corso una classificazione"), "{message}");
    assert_eq!(post_json(&gui, "/api/doc-ripper/run", &json!({"src": dst, "dst": other.path().to_str().unwrap()})).0, 202, "altra cartella");
    assert_eq!(post_json(&gui, "/api/doc-ripper/cancel", &json!({})).0, 202);
    wait_end(&gui);
    common::wait_output(&gui);

    assert_eq!(post_json(&gui, "/api/doc-ripper/run", &json!({"src": dst, "dst": dst})).0, 202);
    let (status, message) = post_json(&gui, "/api/classify/run", &classify);
    assert_eq!(status, 409);
    assert!(message.contains("in corso una conversione"), "{message}");
    assert_eq!(post_json(&gui, "/api/doc-ripper/cancel", &json!({})).0, 202);
}

fn get(gui: &Gui, path: &str) -> (u16, String) {
    let agent = ureq::Agent::config_builder().http_status_as_error(false).build().new_agent();
    let mut response = agent.get(&gui.url(path)).call().unwrap();
    (response.status().as_u16(), response.body_mut().read_to_string().unwrap())
}

fn encoded(text: &str) -> String {
    text.bytes().map(|b| format!("%{b:02X}")).collect()
}

fn classified_folder(gui: &Gui) -> tempfile::TempDir {
    let folder = markdown_folder();
    let request = json!({"src": folder.path().to_str().unwrap(), "classifier": "rules"});
    assert_eq!(post_json(gui, "/api/classify/run", &request).0, 202);
    wait_end(gui);
    folder
}

/// La lettura dei risultati si completa nella richiesta, senza avviare processi (REQ-138).
#[test]
fn risultati_nella_richiesta() {
    let (dir, exe) = install_with_fake_tag();
    let gui = Gui::start(&exe);
    let folder = classified_folder(&gui);
    let src = encoded(folder.path().to_str().unwrap());
    fs::remove_file(dir.path().join("rocket-tag.exe")).unwrap();

    let (status, body) = get(&gui, &format!("/api/classify/results?src={src}&classifier=rules"));
    assert_eq!(status, 200, "{body}");
    let result: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(result["items"].as_array().unwrap().len(), 3);
    assert_eq!(result["classifier"], "rules-v1");

    let (status, body) = get(&gui, &format!("/api/classify/results?src={src}&classifier=llm"));
    assert_eq!(status, 404);
    assert!(body.contains("nessun risultato"));
}

/// Un `document` che esce dalla cartella non si legge (REQ-146).
#[test]
fn percorsi_fuori_dalla_cartella_rifiutati() {
    let (_dir, exe) = install_with_fake_tag();
    let gui = Gui::start(&exe);
    let parent = tempfile::tempdir().unwrap();
    let folder = parent.path().join("documenti");
    fs::create_dir(&folder).unwrap();
    fs::write(folder.join("a.md"), "Interno.\n").unwrap();
    fs::write(parent.path().join("segreto.md"), "Segreto.\n").unwrap();
    let src = encoded(folder.to_str().unwrap());
    let text = |document: &str| get(&gui, &format!("/api/classify/text?src={src}&document={}", encoded(document)));

    assert_eq!(text("a.md"), (200, "Interno.\n".to_string()));
    for document in ["../segreto.md", r"..\segreto.md", parent.path().join("segreto.md").to_str().unwrap(), "a.txt"] {
        let (status, body) = text(document);
        assert_eq!(status, 403, "{document}");
        assert!(body.contains("fuori dalla cartella"), "{body}");
        assert!(!body.contains("Segreto."));
    }
    assert!(text("manca.md").1.contains("non esiste"));

    let link = folder.join("uscita");
    let junction = Command::new("cmd").args(["/c", "mklink", "/J"]).arg(&link).arg(parent.path()).output().unwrap();
    assert!(junction.status.success());
    let (status, body) = text("uscita/segreto.md");
    assert_eq!(status, 403, "giunzione verso l'esterno");
    assert!(!body.contains("Segreto."));
}

/// Documenti con i conteggi, unità con righe, confidenza, testo ed etichette; le faccette non sono
/// scritte nel codice e l'astensione compare come "—" (REQ-142).
#[test]
fn risultati_nella_pagina() {
    let (_dir, exe) = install_with_fake_tag();
    let gui = Gui::start(&exe);
    let _folder = classified_folder(&gui);
    let page = |dom: &str, text: &str| assert!(dom.contains(text), "{text}: {dom}");
    let dom = dump_dom(&gui.url("/#classify"));
    page(&dom, "a.md — 2 unità (1 a bassa confidenza)");
    page(&dom, "sotto/b.md — 1 unità (1 a bassa confidenza)");
    page(&dom, "righe 1–1");
    page(&dom, "confidenza 0,60");
    page(&dom, "Primo testo.");
    page(&dom, "Secondo testo su due righe.");
    page(&dom, "tono: formale");
    page(&dom, "temi: spazio, mare");
    page(&dom, "tono: —");
    page(&dom, "temi: nessuno");
    assert!(!dom.contains("Terzo testo."), "solo il documento scelto");
}

/// Filtri ricavati dal file di uscita, con soglia predefinita 0,7 (REQ-143).
#[test]
fn filtri_dei_risultati() {
    let (_dir, exe) = install_with_fake_tag();
    let gui = Gui::start(&exe);
    let _folder = classified_folder(&gui);
    let dom = dump_dom(&gui.url("/#classify"));
    assert!(dom.contains(r#"<option value="tono=formale">tono: formale</option>"#), "{dom}");
    assert!(dom.contains(r#"<option value="temi=mare">temi: mare</option>"#), "{dom}");
    assert!(dom.contains(r#"id="classify-threshold""#) && dom.contains(r#"value="0.7""#), "{dom}");
    let source = include_str!("../web/app.js");
    assert!(source.contains("item.unit_confidence <= Number(threshold.value)"), "minore o uguale alla soglia");
}

/// Unità scartate con documento, righe e motivo (REQ-144).
#[test]
fn unita_scartate_nella_pagina() {
    let (_dir, exe) = install_with_fake_tag();
    let gui = Gui::start(&exe);
    let _folder = classified_folder(&gui);
    let dom = dump_dom(&gui.url("/#classify"));
    assert!(dom.contains("Unità scartate: 1"), "{dom}");
    assert!(dom.contains("a.md, righe 6–6: tono: valore non ammesso"), "{dom}");
}

/// Niente nomi di faccette o valori nel codice di gui (REQ-142).
#[test]
fn nessuna_faccetta_nel_codice() {
    for source in [include_str!("../web/app.js"), include_str!("../src/server.rs"), include_str!("../src/classify.rs")] {
        for name in ["nature", "extraction_quality", "required_action", "requirement"] {
            assert!(!source.contains(name), "{name}");
        }
    }
}

#[test]
fn richieste_non_valide() {
    let (_dir, exe) = install_with_fake_tag();
    let gui = Gui::start(&exe);
    let (status, message) = post_json(&gui, "/api/classify/run", &json!({"src": "C:\\non\\esiste", "classifier": "rules"}));
    assert_eq!(status, 400);
    assert!(message.contains("cartella"), "{message}");
    let folder = markdown_folder();
    let request = json!({"src": folder.path().to_str().unwrap(), "classifier": "llm"});
    let (status, message) = post_json(&gui, "/api/classify/run", &request);
    assert_eq!(status, 400);
    assert!(message.contains("URL e modello"), "{message}");
}

#[test]
fn la_schermata_e_nel_menu() {
    let (_dir, exe) = install_with_fake_tag();
    let gui = Gui::start(&exe);
    let dom = dump_dom(&gui.url("/#classify"));
    assert!(dom.contains(r##"<a href="#classify" aria-current="page">Classificazione</a>"##), "{dom}");
    assert!(dom.contains(r#"id="classify-kind""#), "{dom}");
}

/// La richiesta di avvio non attende la fine; l'avanzamento arriva in tempo reale (REQ-137).
#[test]
fn esecuzione_in_background_con_avanzamento() {
    let (_dir, exe) = install_with_fake_tag();
    let gui = Gui::start(&exe);
    let folder = markdown_folder();
    fs::write(folder.path().join("lento"), "").unwrap();
    let request = json!({"src": folder.path().to_str().unwrap(), "classifier": "rules"});
    let started = Instant::now();
    assert_eq!(post_json(&gui, "/api/classify/run", &request).0, 202);
    assert!(started.elapsed() < Duration::from_millis(600), "la richiesta di avvio non deve attendere");
    assert_eq!(output(&gui)["running"], true);
    let (status, message) = post_json(&gui, "/api/classify/run", &request);
    assert_eq!(status, 409, "uno alla volta");
    assert!(message.contains("rocket-tag è già in esecuzione"), "{message}");

    let body = ureq::get(&gui.url("/api/classify/events")).call().unwrap().into_body().into_reader();
    let mut lines = Vec::new();
    for line in std::io::BufRead::lines(std::io::BufReader::new(body)) {
        let line = line.unwrap();
        let finished = line == "event: end";
        lines.push(line);
        if finished {
            break;
        }
    }
    assert!(lines.contains(&"data: [3/3] sotto/b.md".to_string()), "{lines:?}");
}

#[test]
fn annulla_interrompe_il_processo() {
    let (_dir, exe) = install_with_fake_tag();
    let gui = Gui::start(&exe);
    let folder = markdown_folder();
    fs::write(folder.path().join("lento"), "").unwrap();
    let request = json!({"src": folder.path().to_str().unwrap(), "classifier": "rules"});
    assert_eq!(post_json(&gui, "/api/classify/run", &request).0, 202);
    assert_eq!(post_json(&gui, "/api/classify/cancel", &json!({})).0, 202);
    assert_eq!(wait_end(&gui)["cancelled"], true);
    assert!(!folder.path().join(".classify/rules.json").exists());
    assert_eq!(post_json(&gui, "/api/classify/cancel", &json!({})).0, 409, "nessuna esecuzione in corso");
}

/// Al termine: esito dal codice di uscita, barra piena e messaggio di stderr (REQ-137).
#[test]
fn esito_nella_pagina() {
    let (_dir, exe) = install_with_fake_tag();
    let gui = Gui::start(&exe);
    let folder = markdown_folder();
    let request = json!({"src": folder.path().to_str().unwrap(), "classifier": "rules"});
    assert_eq!(post_json(&gui, "/api/classify/run", &request).0, 202);
    wait_end(&gui);
    let dom = dump_dom(&gui.url("/#classify"));
    assert!(dom.contains("Classificazione completata."), "{dom}");
    assert!(dom.contains("Unità 3 di 3 (100%)"), "{dom}");

    fs::write(folder.path().join("fallisci"), "").unwrap();
    assert_eq!(post_json(&gui, "/api/classify/run", &request).0, 202);
    assert_eq!(wait_end(&gui)["exit_code"], 3);
    let dom = dump_dom(&gui.url("/#classify"));
    assert!(dom.contains("Classificazione terminata con errori (codice di uscita 3)."), "{dom}");
    assert!(dom.contains("il server LLM non risponde"), "{dom}");
    assert!(!dom.contains("[3/3] sotto/b.md"), "le righe di avanzamento non compaiono nell'output");
}

/// La schermata propone l'ultima destinazione di doc-ripper, modificabile con "Sfoglia" (REQ-135).
#[test]
fn propone_l_ultima_destinazione_di_doc_ripper() {
    let (_dir, exe) = install_with_fake_tag();
    let appdata = tempfile::tempdir().unwrap();
    let settings = appdata.path().join("rocketdata-gui");
    fs::create_dir(&settings).unwrap();
    fs::write(settings.join("settings.json"), json!({"src": "C:\\origine", "dst": "C:\\convertiti"}).to_string()).unwrap();
    let gui = Gui::start_with_appdata(&exe, appdata.path());
    let dom = dump_dom(&gui.url("/#classify"));
    assert!(dom.contains(r#"id="classify-src" type="text" spellcheck="false" autocomplete="off" value="C:\convertiti""#), "{dom}");
    assert!(dom.contains(r#"<button id="classify-src-browse" type="button">Sfoglia…</button>"#), "{dom}");
}

/// I test usano un rocket-tag finto, senza dipendere dal tool reale (REQ-148).
#[test]
fn il_rocket_tag_finto_scrive_il_file_di_uscita() {
    let (dir, _) = install_with_fake_tag();
    let folder = markdown_folder();
    let out = folder.path().join("uscita/rules.json");
    let status = Command::new(dir.path().join("rocket-tag.exe"))
        .args(["classify", "rules"])
        .arg(folder.path())
        .arg("-o")
        .arg(&out)
        .output()
        .unwrap()
        .status;
    assert!(status.success());
    assert_eq!(read_json(&out)["items"].as_array().unwrap().len(), 3);
}
