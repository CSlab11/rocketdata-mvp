mod common;

use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use common::{Gui, dump_dom, install_with_fake_tag, post_json};

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
