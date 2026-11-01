use std::process::Command;

fn rocket_tag() -> Command {
    Command::new(env!("CARGO_BIN_EXE_rocket-tag"))
}

/// `rules check` riesce sulle regole attuali e blocca un valore inesistente (REQ-103).
#[test]
fn rules_check() {
    let ok = rocket_tag()
        .args(["rules", "check", "--schema", "config/label_schema.json", "--rules", "config/rules.yaml"])
        .output()
        .unwrap();
    assert!(ok.status.success(), "{}", String::from_utf8_lossy(&ok.stderr));
    assert!(String::from_utf8_lossy(&ok.stdout).contains("coerente con lo schema v0.1.0"));

    let dir = tempfile::tempdir().unwrap();
    let rules = dir.path().join("rules.yaml");
    std::fs::write(&rules, "version: \"1\"\nfacets:\n  nature:\n    rules:\n      - {value: mai}\n").unwrap();
    let bad = rocket_tag()
        .args(["rules", "check", "--schema", "config/label_schema.json", "--rules"])
        .arg(&rules)
        .output()
        .unwrap();
    assert!(!bad.status.success());
    assert!(String::from_utf8_lossy(&bad.stderr).contains("valore 'mai' non esiste"));
}

fn classify_rules(input: &std::path::Path, output: &std::path::Path) -> std::process::Output {
    rocket_tag()
        .args(["classify", "rules"])
        .arg(input)
        .arg("-o")
        .arg(output)
        .args(["--schema", "config/label_schema.json", "--rules", "config/rules.yaml"])
        .output()
        .unwrap()
}

/// Ogni esecuzione scrive un file JSON nel formato dell'appendice E (REQ-114).
#[test]
fn classify_rules_scrive_il_file_di_uscita() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.md"), "Uno.\n\nDue.\n").unwrap();
    let out = dir.path().join("uscita/rules.json");
    let run = classify_rules(dir.path(), &out);
    assert!(run.status.success(), "{}", String::from_utf8_lossy(&run.stderr));
    let saved: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
    assert_eq!(saved["schema_version"], "0.1.0");
    assert_eq!(saved["classifier"], "rules-v0.1.0");
    assert_eq!(saved["items"].as_array().unwrap().len(), 2);
    assert_eq!(saved["skipped"], serde_json::json!([]));
}

/// Ogni elemento riporta documento, righe e un'etichetta per faccetta (REQ-115).
#[test]
fn elementi_con_documento_righe_ed_etichette() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("sotto")).unwrap();
    std::fs::write(dir.path().join("sotto/a.md"), "# 1 Titolo\n\nThe unit shall operate\nat 30 °C.\n").unwrap();
    let out = dir.path().join("rules.json");
    assert!(classify_rules(dir.path(), &out).status.success());
    let saved: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
    let item = &saved["items"][0];
    assert_eq!(item["document"], "sotto/a.md");
    assert_eq!((item["line_start"].as_u64(), item["line_end"].as_u64()), (Some(3), Some(4)));
    assert_eq!(item["labels"]["nature"], "requirement");
    assert_eq!(item["labels"]["flags"], serde_json::json!([]));
}

/// Tipo di unità, `text_hash`, confidenze e regole (REQ-116).
#[test]
fn elementi_con_tipo_hash_e_confidenze() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.md"), "- The unit shall operate.\n").unwrap();
    let out = dir.path().join("rules.json");
    assert!(classify_rules(dir.path(), &out).status.success());
    let saved: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
    let item = &saved["items"][0];
    assert_eq!(item["kind"], "list_item");
    assert_eq!(item["text_hash"].as_str().unwrap().len(), 16);
    let confidences = item["confidence"].as_object().unwrap();
    assert_eq!(confidences.len(), 6);
    let lowest = confidences.values().map(|c| c.as_f64().unwrap()).fold(1.0, f64::min);
    assert_eq!(item["unit_confidence"].as_f64().unwrap(), lowest);
    assert_eq!(item["rule_ids"]["nature"], "req-modal");
}

/// Il crate si compila e si esegue da solo, senza gli altri tool (REQ-079).
#[test]
fn si_esegue_da_solo() {
    let output = rocket_tag().arg("--version").output().unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).starts_with("rocket-tag "));
}
