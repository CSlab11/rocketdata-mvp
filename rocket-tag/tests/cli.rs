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

/// Lato dalle etichette, poi da `rules.yaml`, altrimenti sconosciuto (REQ-089).
#[test]
fn lato_del_documento() {
    let dir = tempfile::tempdir().unwrap();
    let text = "The unit shall operate.\n";
    std::fs::write(dir.path().join("[TD] a.md"), text).unwrap();
    std::fs::write(dir.path().join("b.md"), text).unwrap();
    let labels = dir.path().join("etichette.jsonl");
    std::fs::write(&labels, r#"{"level":"document","document":"[TD] a.md","side":"prime","doc_type":"other"}"#).unwrap();

    let action = |saved: &serde_json::Value, index: usize| saved["items"][index]["labels"]["required_action"].clone();
    let rule = |saved: &serde_json::Value, index: usize| saved["items"][index]["rule_ids"]["required_action"].clone();
    let out = dir.path().join("rules.json");
    assert!(classify_rules(dir.path(), &out).status.success());
    let saved: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
    assert_eq!(rule(&saved, 0), "schema x-fixedWhen", "supplier da rules.yaml");
    assert_eq!(rule(&saved, 1), "default", "lato sconosciuto");

    let run = rocket_tag()
        .args(["classify", "rules"])
        .arg(dir.path())
        .arg("-o")
        .arg(&out)
        .args(["--schema", "config/label_schema.json", "--rules", "config/rules.yaml", "--labels"])
        .arg(&labels)
        .output()
        .unwrap();
    assert!(run.status.success());
    let saved: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
    assert_eq!(action(&saved, 0), "declare_compliance", "le etichette umane vincono su rules.yaml");
}

/// `--exclude` è ripetibile e vale per file e cartelle (REQ-128).
#[test]
fn opzione_exclude() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join(".index")).unwrap();
    std::fs::write(dir.path().join("index.md"), "Indice.\n").unwrap();
    std::fs::write(dir.path().join(".index/x.md"), "Interno.\n").unwrap();
    std::fs::write(dir.path().join("a.md"), "Testo.\n").unwrap();
    let out = dir.path().join("rules.json");
    let run = rocket_tag()
        .args(["classify", "rules"])
        .arg(dir.path())
        .arg("-o")
        .arg(&out)
        .args(["--schema", "config/label_schema.json", "--rules", "config/rules.yaml"])
        .args(["--exclude", "index.md", "--exclude", ".index"])
        .output()
        .unwrap();
    assert!(run.status.success(), "{}", String::from_utf8_lossy(&run.stderr));
    let saved: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
    let items = saved["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["document"], "a.md");
}

/// Una riga di avanzamento su stdout per ogni unità elaborata (REQ-129).
#[test]
fn righe_di_avanzamento() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("sotto")).unwrap();
    std::fs::write(dir.path().join("a.md"), "Uno.\n\nDue.\n").unwrap();
    std::fs::write(dir.path().join("sotto/b.md"), "Tre.\n").unwrap();
    let run = classify_rules(dir.path(), &dir.path().join("rules.json"));
    assert!(run.status.success());
    let stdout = String::from_utf8_lossy(&run.stdout);
    let lines: Vec<_> = stdout.lines().collect();
    assert_eq!(lines, ["[1/3] a.md", "[2/3] a.md", "[3/3] sotto/b.md"]);
}

/// Solo i file Markdown esistenti: niente conversioni, pulizia o scritture nei documenti (REQ-080).
#[test]
fn non_tocca_i_documenti() {
    let dir = tempfile::tempdir().unwrap();
    let docs = dir.path().join("docs");
    std::fs::create_dir(&docs).unwrap();
    std::fs::write(docs.join("a.md"), "Uno.\n").unwrap();
    std::fs::write(docs.join("b.pdf"), "%PDF finto").unwrap();
    std::fs::write(docs.join("c.docx"), "finto").unwrap();
    let out = dir.path().join("rules.json");
    let run = classify_rules(&docs, &out);
    assert!(run.status.success());
    let saved: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
    assert_eq!(saved["items"].as_array().unwrap().len(), 1, "i file non Markdown si ignorano");
    let mut names: Vec<_> = std::fs::read_dir(&docs).unwrap().map(|e| e.unwrap().file_name()).collect();
    names.sort();
    assert_eq!(names, ["a.md", "b.pdf", "c.docx"]);
    assert_eq!(std::fs::read_to_string(docs.join("a.md")).unwrap(), "Uno.\n");
}

/// Iteratore e classificatori sono separati: un testo si classifica senza file Markdown (REQ-084).
#[test]
fn classify_one_con_le_regole() {
    let run = rocket_tag()
        .args(["classify", "one", "--text", "The unit shall operate.", "--side", "prime"])
        .args(["--schema", "config/label_schema.json", "--rules", "config/rules.yaml"])
        .output()
        .unwrap();
    assert!(run.status.success(), "{}", String::from_utf8_lossy(&run.stderr));
    let result: serde_json::Value = serde_json::from_slice(&run.stdout).unwrap();
    assert_eq!(result["labels"]["nature"], "requirement");
    assert_eq!(result["labels"]["required_action"], "declare_compliance");
    assert_eq!(result["confidence"]["nature"], 0.85);
    assert_eq!(result["rule_ids"]["nature"], "req-modal");
}

#[test]
fn classify_one_vuole_un_solo_classificatore() {
    let schema = ["--schema", "config/label_schema.json"];
    let neither = rocket_tag().args(["classify", "one", "--text", "x"]).args(schema).output().unwrap();
    assert!(!neither.status.success());
    let both = rocket_tag()
        .args(["classify", "one", "--text", "x", "--rules", "r", "--base-url", "u", "--model", "m", "--guide", "g", "--examples", "e"])
        .args(schema)
        .output()
        .unwrap();
    assert!(!both.status.success());
}

/// Codice di uscita diverso da 0 e messaggio su stderr, per ogni comando (REQ-130).
#[test]
fn errori_con_codice_di_uscita_e_stderr() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("manca");
    let out = dir.path().join("o.json");
    let schema = ["--schema", "config/label_schema.json"];
    let rules = ["--rules", "config/rules.yaml"];
    let runs = [
        rocket_tag().args(["classify", "rules"]).arg(&missing).arg("-o").arg(&out).args(schema).args(rules).output().unwrap(),
        rocket_tag().args(["classify", "rules", "x.md", "-o", "o.json", "--schema"]).arg(&missing).args(rules).output().unwrap(),
        rocket_tag().args(["rules", "check", "--schema"]).arg(&missing).args(rules).output().unwrap(),
        rocket_tag().args(["rules", "check"]).args(schema).arg("--rules").arg(&missing).output().unwrap(),
        rocket_tag().args(["classify", "one", "--text", "x"]).args(schema).arg("--rules").arg(&missing).output().unwrap(),
        rocket_tag().args(["comando-inesistente"]).output().unwrap(),
    ];
    for run in runs {
        assert!(!run.status.success());
        assert!(!run.stderr.is_empty());
        assert!(run.stdout.is_empty(), "{}", String::from_utf8_lossy(&run.stdout));
    }
}

/// Report Markdown e riga di metriche nello storico JSONL (REQ-121).
#[test]
fn evaluate_scrive_report_e_storico() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.md"), "The unit shall operate.\n\nAltro testo qui.\n").unwrap();
    let out = dir.path().join("rules.json");
    assert!(classify_rules(dir.path(), &out).status.success());
    let labels = dir.path().join("etichette.jsonl");
    let right = serde_json::json!({"level": "paragraph", "document": "a.md", "line_start": 1, "line_end": 1,
        "facets": {"nature": "requirement", "domain": "technical", "destination": "P",
                   "required_action": "none", "extraction_quality": "ok", "flags": []}});
    let wrong = r#"{"level":"paragraph","document":"a.md","line_start":3,"line_end":3,"facets":{"nature":"mai"}}"#;
    std::fs::write(&labels, format!("{right}\n{wrong}\n")).unwrap();
    let (report, metrics) = (dir.path().join("r.md"), dir.path().join("m.jsonl"));
    for _ in 0..2 {
        let run = rocket_tag()
            .arg("evaluate")
            .arg(&labels)
            .arg(&out)
            .args(["--schema", "config/label_schema.json", "--report"])
            .arg(&report)
            .arg("--metrics")
            .arg(&metrics)
            .output()
            .unwrap();
        assert!(run.status.success(), "{}", String::from_utf8_lossy(&run.stderr));
        assert!(String::from_utf8_lossy(&run.stdout).contains("1 unità valutate, 1 etichette da rivedere"));
    }
    let text = std::fs::read_to_string(&report).unwrap();
    assert!(text.starts_with("# Valutazione – rules-v0.1.0"));
    assert!(text.contains("| Faccetta | Accuratezza | Macro-F1 | Copertura | N |"));
    assert!(text.contains("| nature | 100.0% | 1.00 | 100% | 1 |"));
    assert!(text.contains("## nature\n\n| Valore | Precisione"));
    let history = std::fs::read_to_string(&metrics).unwrap();
    let lines: Vec<_> = history.lines().collect();
    assert_eq!(lines.len(), 2, "una riga per esecuzione");
    let first: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(first["classifier"], "rules-v0.1.0");
    assert_eq!(first["n_eval"], 1);
    assert_eq!(first["facets"]["nature"]["accuracy"], 1.0);
}

#[test]
fn evaluate_senza_abbinamenti_e_un_errore() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.md"), "Testo.\n").unwrap();
    let out = dir.path().join("rules.json");
    assert!(classify_rules(dir.path(), &out).status.success());
    let labels = dir.path().join("etichette.jsonl");
    std::fs::write(&labels, "").unwrap();
    let run = rocket_tag().arg("evaluate").arg(&labels).arg(&out).args(["--schema", "config/label_schema.json"]).output().unwrap();
    assert!(!run.status.success());
    assert!(String::from_utf8_lossy(&run.stderr).contains("nessuna etichetta valida"));
}

/// Il crate si compila e si esegue da solo, senza gli altri tool (REQ-079).
#[test]
fn si_esegue_da_solo() {
    let output = rocket_tag().arg("--version").output().unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).starts_with("rocket-tag "));
}
