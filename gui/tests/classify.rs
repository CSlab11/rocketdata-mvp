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
    let received = read_json(&folder.path().join("rules.json.args.json"));
    let args: Vec<&str> = received["args"].as_array().unwrap().iter().map(|a| a.as_str().unwrap()).collect();
    assert_eq!(args[..3], ["classify", "rules", src]);
    let config = dir.path().canonicalize().unwrap().join("rocket-tag-config");
    let schema = args[args.iter().position(|a| *a == "--schema").unwrap() + 1];
    assert!(Path::new(schema.trim_start_matches(r"\\?\")).ends_with("rocket-tag-config/label_schema.json"), "{schema} {config:?}");

    let request = json!({"src": src, "classifier": "llm", "base_url": "http://127.0.0.1:8080/v1", "model": "modello"});
    assert_eq!(post_json(&gui, "/api/classify/run", &request).0, 202);
    assert_eq!(wait_end(&gui)["exit_code"], 0);
    let received = read_json(&folder.path().join("llm.json.args.json"));
    let args: Vec<&str> = received["args"].as_array().unwrap().iter().map(|a| a.as_str().unwrap()).collect();
    assert_eq!(args[..2], ["classify", "llm"]);
    assert!(args.windows(2).any(|w| w == ["--base-url", "http://127.0.0.1:8080/v1"]));
    assert!(args.windows(2).any(|w| w == ["--model", "modello"]));
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
