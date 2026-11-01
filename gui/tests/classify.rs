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
