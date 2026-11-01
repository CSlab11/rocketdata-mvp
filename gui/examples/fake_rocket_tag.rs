//! rocket-tag finto per i test di gui: viene copiato come `rocket-tag.exe` accanto a `gui.exe`.
//! Il comportamento dipende da file marcatore nella cartella di ingresso:
//! `lento` → righe distanziate di 1 s, `fallisci` → messaggio su stderr e codice di uscita 3.
//! Accanto al file di uscita scrive `<uscita>.args.json` con gli argomenti e la chiave ricevuti.
//! Le faccette sono inventate: gui non deve conoscerle.

use std::path::{Path, PathBuf};
use std::thread::sleep;
use std::time::Duration;

use serde_json::json;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let value = |flag: &str| args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).cloned();
    let (Some("classify"), Some(classifier), Some(input), Some(output)) =
        (args.first().map(String::as_str), args.get(1), args.get(2), value("-o"))
    else {
        eprintln!("uso: rocket-tag classify rules|llm <INGRESSO> -o <FILE>");
        std::process::exit(2);
    };
    let input = Path::new(input);
    let output = PathBuf::from(output);
    let slow = input.join("lento").exists();

    std::fs::create_dir_all(output.parent().unwrap()).unwrap();
    let received = json!({"args": args, "key": std::env::var("ROCKET_TAG_API_KEY").ok()});
    let mut record = output.as_os_str().to_os_string();
    record.push(".args.json");
    std::fs::write(record, received.to_string()).unwrap();

    for line in ["[1/3] a.md", "[2/3] a.md", "[3/3] sotto/b.md"] {
        println!("{line}");
        if slow {
            sleep(Duration::from_secs(1));
        }
    }
    if input.join("fallisci").exists() {
        eprintln!("il server LLM non risponde");
        std::process::exit(3);
    }
    let result = json!({
        "schema_version": "0.1.0",
        "classifier": if classifier == "rules" { "rules-v1" } else { "llm:modello" },
        "created_at": "2026-11-02T10:00:00Z",
        "items": [
            {"document": "a.md", "line_start": 1, "line_end": 1, "kind": "paragraph", "text_hash": "aaaaaaaaaaaaaaaa",
             "labels": {"tono": "formale", "temi": ["spazio", "mare"]},
             "confidence": {"tono": 0.9, "temi": 0.6}, "unit_confidence": 0.6},
            {"document": "a.md", "line_start": 3, "line_end": 4, "kind": "paragraph", "text_hash": "bbbbbbbbbbbbbbbb",
             "labels": {"temi": []}, "confidence": {"temi": 0.8}, "unit_confidence": 0.8},
            {"document": "sotto/b.md", "line_start": 1, "line_end": 1, "kind": "list_item", "text_hash": "cccccccccccccccc",
             "labels": {"tono": "informale", "temi": ["mare"]},
             "confidence": {"tono": 0.5, "temi": 0.9}, "unit_confidence": 0.5}
        ],
        "skipped": [{"document": "a.md", "line_start": 6, "line_end": 6, "error": "tono: valore non ammesso"}]
    });
    std::fs::write(&output, result.to_string()).unwrap();
}
