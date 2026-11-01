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

/// Il crate si compila e si esegue da solo, senza gli altri tool (REQ-079).
#[test]
fn si_esegue_da_solo() {
    let output = rocket_tag().arg("--version").output().unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).starts_with("rocket-tag "));
}
