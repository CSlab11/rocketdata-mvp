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
    std::fs::write(dir.path().join("a.md"), "Uno.

Due.
").unwrap();
    let out = dir.path().join("uscita/rules.json");
    let run = classify_rules(dir.path(), &out);
    assert!(run.status.success(), "{}", String::from_utf8_lossy(&run.stderr));
    let saved: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
    assert_eq!(saved["schema_version"], "0.1.0");
    assert_eq!(saved["classifier"], "rules-v0.1.0");
    assert_eq!(saved["items"].as_array().unwrap().len(), 2);
    assert_eq!(saved["skipped"], serde_json::json!([]));
}

/// Il crate si compila e si esegue da solo, senza gli altri tool (REQ-079).
#[test]
fn si_esegue_da_solo() {
    let output = rocket_tag().arg("--version").output().unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).starts_with("rocket-tag "));
}
