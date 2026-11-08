//! REQ-036: due esecuzioni sullo stesso input producono lo stesso output.

use std::process::Command;

fn run(out: &std::path::Path) -> (Vec<u8>, String) {
    let res = Command::new(env!("CARGO_BIN_EXE_thin-decoder"))
        .args(["examples/satellite.json", out.to_str().unwrap(), "--ontology", "examples/ontology.json"])
        .output()
        .expect("esecuzione del programma");
    assert!(res.status.success(), "{}", String::from_utf8_lossy(&res.stderr));
    (res.stdout, std::fs::read_to_string(out).unwrap())
}

#[test]
fn two_runs_give_identical_output() {
    let dir = std::env::temp_dir();
    let (stdout1, json1) = run(&dir.join("td_repro_1.json"));
    let (stdout2, json2) = run(&dir.join("td_repro_2.json"));
    assert_eq!(json1, json2);
    assert_eq!(stdout1, stdout2);
    let answers: serde_json::Value = serde_json::from_str(&json1).unwrap();
    assert_eq!(answers.as_array().unwrap().len(), 17);
}
