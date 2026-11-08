//! REQ-079: il colore di claim e risultati compare nel JSON e a terminale.

use std::process::Command;

#[test]
fn colors_in_json_and_terminal() {
    let out = std::env::temp_dir().join("td_colors.json");
    let res = Command::new(env!("CARGO_BIN_EXE_thin-decoder"))
        .args(["examples/satellite.json", out.to_str().unwrap()])
        .output()
        .expect("esecuzione del programma");
    assert!(res.status.success(), "{}", String::from_utf8_lossy(&res.stderr));
    let answers: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
    let answers = answers.as_array().unwrap();
    let results: Vec<&serde_json::Value> = answers.iter().map(|a| &a["results"][0]).collect();
    assert!(results.iter().all(|r| r["color"] == "blu" && r["engine"] == "rules"));
    let ev = results.iter().flat_map(|r| r["evidence"].as_array().cloned().unwrap_or_default());
    assert!(ev.filter(|e| e.get("claim").is_some()).all(|e| e["claim_color"] == "blu"));
    let stdout = String::from_utf8_lossy(&res.stdout);
    assert!(stdout.contains("[blu]"), "{stdout}");
}
