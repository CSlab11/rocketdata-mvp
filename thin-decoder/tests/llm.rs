//! Integrazione LLM: configurazione, backend, cache ed errori.

use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_thin-decoder"))
        .args(args)
        .output()
        .expect("esecuzione del programma")
}

fn write_temp(name: &str, content: &str) -> String {
    let path = std::env::temp_dir().join(name);
    std::fs::write(&path, content).unwrap();
    path.to_str().unwrap().to_string()
}

/// REQ-070: con una configurazione che dichiara un server irraggiungibile ma
/// nessuna funzione attiva, l'esecuzione non chiama l'LLM e dà lo stesso
/// output di quella senza configurazione.
#[test]
fn config_without_llm_functions_gives_rules_output() {
    let cfg = write_temp("td_cfg_rules.json", r#"{"llm": {"url": "http://127.0.0.1:9", "model": "m"}}"#);
    let plain = std::env::temp_dir().join("td_plain.json");
    let with_cfg = std::env::temp_dir().join("td_with_cfg.json");
    let a = run(&["examples/satellite.json", plain.to_str().unwrap()]);
    let b = run(&["examples/satellite.json", with_cfg.to_str().unwrap(), "--config", &cfg]);
    assert!(b.status.success(), "{}", String::from_utf8_lossy(&b.stderr));
    assert_eq!(a.stdout, b.stdout);
    assert_eq!(std::fs::read(plain).unwrap(), std::fs::read(with_cfg).unwrap());
}

#[test]
fn invalid_config_is_rejected() {
    let cfg = write_temp("td_cfg_bad.json", r#"{"unknown": 1}"#);
    let out = std::env::temp_dir().join("td_bad.json");
    let res = run(&["examples/satellite.json", out.to_str().unwrap(), "--config", &cfg]);
    assert!(!res.status.success());
    assert!(String::from_utf8_lossy(&res.stderr).contains("configurazione non valida"));
}

/// Finto server compatibile con OpenAI: risponde con il contenuto calcolato da `reply`.
fn fake_server(reply: fn(&str) -> serde_json::Value) -> String {
    use std::io::{BufRead, BufReader, Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut len = 0;
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" || line.is_empty() {
                    break;
                }
                if let Some(v) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                    len = v.trim().parse().unwrap();
                }
            }
            let mut body = vec![0; len];
            reader.read_exact(&mut body).unwrap();
            let req: serde_json::Value = serde_json::from_slice(&body).unwrap();
            let content = reply(req["messages"][0]["content"].as_str().unwrap()).to_string();
            let resp = serde_json::json!({"choices": [{"message": {"content": content}}]}).to_string();
            let head = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                resp.len()
            );
            stream.write_all(head.as_bytes()).unwrap();
            stream.write_all(resp.as_bytes()).unwrap();
        }
    });
    url
}

fn llm_config(name: &str, url: &str, extra: &str) -> String {
    let cache = std::env::temp_dir().join(format!("{name}_cache"));
    let _ = std::fs::remove_dir_all(&cache);
    let cache = cache.to_str().unwrap().replace(std::path::MAIN_SEPARATOR, "/");
    write_temp(
        &format!("{name}.json"),
        &format!(r#"{{"llm": {{"url": "{url}", "model": "m", "cache_dir": "{cache}"}}, {extra}}}"#),
    )
}

const SMALL_INPUT: &str = r#"{"questions": ["What is the mass of the RW?"],
    "text": "The RW, which is heavy, weighs in at 2 kg."}"#;

fn mass_claim(_prompt: &str) -> serde_json::Value {
    serde_json::json!({"claims": [{"kind": "attribute", "attribute": "mass", "subject": "RW",
        "relation": "weighs", "object": "2 kg", "span": "RW, which is heavy, weighs in at 2 kg",
        "negated": false}]})
}

/// REQ-074: con `extract: llm` i claim vengono dall'LLM e sono rossi.
#[test]
fn llm_extraction_end_to_end() {
    let url = fake_server(mass_claim);
    let cfg = llm_config("td_cfg_extract_llm", &url, r#""extract": "llm""#);
    let input = write_temp("td_small.json", SMALL_INPUT);
    let out = std::env::temp_dir().join("td_extract_llm_out.json");
    let res = run(&[&input, out.to_str().unwrap(), "--config", &cfg]);
    assert!(res.status.success(), "{}", String::from_utf8_lossy(&res.stderr));
    let answers: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(out).unwrap()).unwrap();
    assert_eq!(answers[0]["results"][0]["answer"], "2 kg");
    assert_eq!(answers[0]["results"][0]["evidence"][0]["claim_color"], "rosso");
}

/// REQ-068: server spento e risposta non in cache → errore e uscita diversa da zero.
#[test]
fn unreachable_server_stops_with_error() {
    let cfg = llm_config("td_cfg_down", "http://127.0.0.1:9", r#""extract": "both", "extract_scope": "all""#);
    let input = write_temp("td_small_down.json", SMALL_INPUT);
    let out = std::env::temp_dir().join("td_down_out.json");
    let res = run(&[&input, out.to_str().unwrap(), "--config", &cfg]);
    assert!(!res.status.success());
    assert!(String::from_utf8_lossy(&res.stderr).contains("server LLM non raggiungibile"));
}

fn llm_answer(_prompt: &str) -> serde_json::Value {
    serde_json::json!({"answer": "2 kg", "premises": ["C1"]})
}

/// REQ-076: con `reason: both` ogni domanda ha un risultato per motore.
#[test]
fn both_engines_give_one_entry_with_two_results() {
    let url = fake_server(llm_answer);
    let cfg = llm_config("td_cfg_reason_both", &url, r#""reason": "both""#);
    let input = write_temp("td_small_both.json", SMALL_INPUT);
    let out = std::env::temp_dir().join("td_reason_both_out.json");
    let res = run(&[&input, out.to_str().unwrap(), "--config", &cfg]);
    assert!(res.status.success(), "{}", String::from_utf8_lossy(&res.stderr));
    let answers: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(out).unwrap()).unwrap();
    let results = answers[0]["results"].as_array().unwrap();
    let engines: Vec<&str> = results.iter().map(|r| r["engine"].as_str().unwrap()).collect();
    assert_eq!(engines, ["rules", "llm"]);
    assert_eq!(results[1]["answer"], "2 kg");
    assert!(String::from_utf8_lossy(&res.stdout).contains("(llm)"));
}

fn llm_pattern(_prompt: &str) -> serde_json::Value {
    serde_json::json!({"subject": "RW", "kind": "attribute", "attribute": "mass", "relation": "",
        "object": "", "variable": "object", "expected": "quantity"})
}

/// REQ-063, REQ-083: domanda non riconosciuta dalle regole, pattern dall'LLM,
/// risultato rosso anche se il claim usato è blu.
#[test]
fn llm_question_pattern_gives_red_result() {
    let url = fake_server(llm_pattern);
    let cfg = llm_config("td_cfg_question", &url, r#""question_llm": true"#);
    let input = write_temp(
        "td_small_question.json",
        r#"{"questions": ["Tell me the mass of the RW."], "text": "The RW has a mass of 2 kg."}"#,
    );
    let out = std::env::temp_dir().join("td_question_out.json");
    let res = run(&[&input, out.to_str().unwrap(), "--config", &cfg]);
    assert!(res.status.success(), "{}", String::from_utf8_lossy(&res.stderr));
    let answers: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(out).unwrap()).unwrap();
    let r = &answers[0]["results"][0];
    assert_eq!((r["answer"].as_str(), r["color"].as_str()), (Some("2 kg"), Some("rosso")));
    assert_eq!(r["evidence"][0]["claim_color"], "blu");
}

fn llm_ontology(_prompt: &str) -> serde_json::Value {
    serde_json::json!({"synonyms": [["spacecraft", "satellite"]], "subclass_of": [["RW", "actuator"]],
        "attribute_verbs": [["draw", "power"]]})
}

/// REQ-064: il comando separato scrive un'ontologia a priori riutilizzabile;
/// le domande non chiamano l'LLM per l'ontologia.
#[test]
fn propose_ontology_writes_a_reusable_file() {
    let url = fake_server(llm_ontology);
    let cfg = llm_config("td_cfg_ontology", &url, r#""ontology_llm": true"#);
    let onto = std::env::temp_dir().join("td_proposed_ontology.json");
    let res = run(&["propose-ontology", "examples/satellite.json", onto.to_str().unwrap(), "--config", &cfg]);
    assert!(res.status.success(), "{}", String::from_utf8_lossy(&res.stderr));
    let written: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&onto).unwrap()).unwrap();
    assert_eq!(written["subclass_of"][0], serde_json::json!(["RW", "actuator"]));
    // Domande con ontology_llm attivo ma server spento: nessuna chiamata LLM.
    let down = llm_config("td_cfg_ontology_down", "http://127.0.0.1:9", r#""ontology_llm": true"#);
    let out = std::env::temp_dir().join("td_ontology_run.json");
    let res = run(&["examples/satellite.json", out.to_str().unwrap(), "--ontology", onto.to_str().unwrap(), "--config", &down]);
    assert!(res.status.success(), "{}", String::from_utf8_lossy(&res.stderr));
}

#[test]
fn propose_ontology_requires_the_flag() {
    let cfg = llm_config("td_cfg_ontology_off", "http://127.0.0.1:9", r#""extract": "rules""#);
    let onto = std::env::temp_dir().join("td_proposed_off.json");
    let res = run(&["propose-ontology", "examples/satellite.json", onto.to_str().unwrap(), "--config", &cfg]);
    assert!(!res.status.success());
    assert!(String::from_utf8_lossy(&res.stderr).contains("ontology_llm"));
}
