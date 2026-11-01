//! Classificatore LLM contro un server finto compatibile con l'API OpenAI.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{Arc, Mutex};

use serde_json::{Value, json};

/// Richiesta ricevuta dal server finto.
#[derive(Clone)]
struct Request {
    path: String,
    headers: String,
    body: Value,
}

struct Server {
    url: String,
    requests: Arc<Mutex<Vec<Request>>>,
}

impl Server {
    fn requests(&self) -> Vec<Request> {
        self.requests.lock().unwrap().clone()
    }
}

/// Server che risponde con `reply(corpo, numero della richiesta)`: stato HTTP e testo del modello.
fn serve(reply: impl Fn(&Value, usize) -> (u16, String) + Send + Sync + 'static) -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/v1", listener.local_addr().unwrap());
    let requests = Arc::new(Mutex::new(Vec::new()));
    let log = Arc::clone(&requests);
    let reply = Arc::new(reply);
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.unwrap();
            let log = Arc::clone(&log);
            let reply = Arc::clone(&reply);
            std::thread::spawn(move || {
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut first = String::new();
                reader.read_line(&mut first).unwrap();
                let mut headers = String::new();
                let mut length = 0;
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    if line.trim().is_empty() {
                        break;
                    }
                    if let Some(value) = line.to_lowercase().strip_prefix("content-length:") {
                        length = value.trim().parse().unwrap();
                    }
                    headers.push_str(&line);
                }
                let mut body = vec![0; length];
                reader.read_exact(&mut body).unwrap();
                let body: Value = serde_json::from_slice(&body).unwrap();
                let number = {
                    let mut log = log.lock().unwrap();
                    log.push(Request { path: first.split(' ').nth(1).unwrap().to_string(), headers, body: body.clone() });
                    log.len()
                };
                let (status, content) = reply(&body, number);
                let answer = json!({"choices": [{"message": {"content": content}}]}).to_string();
                let response = format!(
                    "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{answer}",
                    answer.len()
                );
                let _ = stream.write_all(response.as_bytes());
            });
        }
    });
    Server { url, requests }
}

fn labels() -> String {
    json!({"nature": "requirement", "domain": "technical", "destination": "P",
           "required_action": "declare_compliance", "extraction_quality": "ok", "flags": []})
    .to_string()
}

fn llm(input: &Path, out: &Path, url: &str) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_rocket-tag"));
    command
        .args(["classify", "llm"])
        .arg(input)
        .arg("-o")
        .arg(out)
        .args(["--schema", "config/label_schema.json", "--guide", "config/prompt_guide.md"])
        .args(["--examples", "config/guide_examples.json", "--base-url", url, "--model", "modello"]);
    command
}

fn run(command: &mut Command) -> Output {
    let output = command.output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    output
}

fn saved(path: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn workspace(text: &str) -> (tempfile::TempDir, PathBuf, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let docs = dir.path().join("docs");
    std::fs::create_dir(&docs).unwrap();
    std::fs::write(docs.join("a.md"), text).unwrap();
    let out = dir.path().join("llm.json");
    (dir, docs, out)
}

/// Qualsiasi server compatibile con chat completions, con URL e modello configurabili (REQ-106).
#[test]
fn chiama_il_server_configurato() {
    let server = serve(|_, _| (200, labels()));
    let (_dir, docs, out) = workspace("The unit shall operate.\n");
    run(&mut llm(&docs, &out, &server.url));

    let requests = server.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].path, "/v1/chat/completions");
    assert!(!requests[0].headers.to_lowercase().contains("authorization"), "la chiave è facoltativa");
    let body = &requests[0].body;
    assert_eq!(body["model"], "modello");
    assert_eq!(body["temperature"], 0.0);
    assert_eq!(body["messages"][0]["role"], "system");
    assert!(body["messages"][0]["content"].as_str().unwrap().contains("Facets and allowed values:"));
    assert_eq!(body["messages"][1]["role"], "user");
    assert!(body["messages"][1]["content"].as_str().unwrap().contains("UNIT TO LABEL: The unit shall operate."));
    assert_eq!(body["response_format"]["type"], "json_schema");

    let result = saved(&out);
    assert_eq!(result["classifier"], "llm:modello");
    assert_eq!(result["prompt_id"].as_str().unwrap().len(), 10);
    assert_eq!(result["items"][0]["labels"]["nature"], "requirement");
    assert!(result["items"][0].get("rule_ids").is_none());
}

/// La risposta può arrivare dentro un blocco di codice o con testo attorno.
#[test]
fn accetta_il_json_dentro_un_blocco_di_codice() {
    let server = serve(|_, _| (200, format!("```json\n{}\n```", labels())));
    let (_dir, docs, out) = workspace("Testo.\n");
    run(&mut llm(&docs, &out, &server.url));
    assert_eq!(saved(&out)["items"][0]["labels"]["domain"], "technical");
}

/// Le modalità del vincolo si scelgono da riga di comando (REQ-107).
#[test]
fn modalita_guided_json_da_riga_di_comando() {
    let server = serve(|_, _| (200, labels()));
    let (_dir, docs, out) = workspace("Testo.\n");
    run(llm(&docs, &out, &server.url).args(["--format", "guided_json"]));
    let body = &server.requests()[0].body;
    assert_eq!(body["guided_json"]["type"], "object");
    assert!(body.get("response_format").is_none());
}
