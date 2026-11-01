//! Classificatore LLM contro un server finto compatibile con l'API OpenAI.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rocket_tag::classifier::Context;
use rocket_tag::llm::{Format, Llm, Settings};
use rocket_tag::markdown::Kind;
use rocket_tag::schema::Schema;
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
    with_model(input, out, url, "modello")
}

fn with_model(input: &Path, out: &Path, url: &str, model: &str) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_rocket-tag"));
    command
        .args(["classify", "llm"])
        .arg(input)
        .arg("-o")
        .arg(out)
        .args(["--schema", "config/label_schema.json", "--guide", "config/prompt_guide.md"])
        .args(["--examples", "config/guide_examples.json", "--base-url", url, "--model", model]);
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

/// La chiave arriva solo dalla variabile d'ambiente (REQ-132).
#[test]
fn chiave_dalla_variabile_d_ambiente() {
    let server = serve(|_, _| (200, labels()));
    let (_dir, docs, out) = workspace("Testo.\n");
    run(llm(&docs, &out, &server.url).env("ROCKET_TAG_API_KEY", "segreta"));
    assert!(server.requests()[0].headers.to_lowercase().contains("authorization: bearer segreta"));

    let rejected = llm(&docs, &out, &server.url).args(["--api-key", "segreta"]).output().unwrap();
    assert!(!rejected.status.success(), "nessuna opzione da riga di comando");
}

/// Una risposta non valida si ritenta una volta; al secondo fallimento l'unità è scartata con il
/// motivo (REQ-109).
#[test]
fn risposta_non_valida_ritentata_poi_scartata() {
    let bad = json!({"nature": "inventata"}).to_string();
    let wrong = bad.clone();
    let server = serve(move |body, _| {
        let user = body["messages"][1]["content"].as_str().unwrap();
        if user.contains("UNIT TO LABEL: Sempre") { (200, wrong.clone()) } else { (200, labels()) }
    });
    let (_dir, docs, out) = workspace("Giusta.\n\nSempre sbagliata.\n");
    run(&mut llm(&docs, &out, &server.url));
    assert_eq!(server.requests().len(), 3);
    let result = saved(&out);
    assert_eq!(result["items"].as_array().unwrap().len(), 1);
    let skipped = &result["skipped"][0];
    assert_eq!((skipped["document"].as_str(), skipped["line_start"].as_u64()), (Some("a.md"), Some(3)));
    assert!(skipped["error"].as_str().unwrap().contains("nature: valore non ammesso"), "{skipped}");
}

#[test]
fn la_seconda_risposta_puo_essere_valida() {
    let server = serve(|_, number| if number == 1 { (200, "non json".to_string()) } else { (200, labels()) });
    let (_dir, docs, out) = workspace("Testo.\n");
    run(&mut llm(&docs, &out, &server.url));
    let result = saved(&out);
    assert_eq!(result["items"].as_array().unwrap().len(), 1);
    assert_eq!(result["skipped"], json!([]));
}

/// Prima della validazione valgono i vincoli dello schema e il default delle faccette multiple.
#[test]
fn vincoli_e_default_prima_della_validazione() {
    let server = serve(|_, _| {
        (200, json!({"nature": "commitment", "domain": "technical", "destination": "P",
                     "required_action": "quote", "extraction_quality": "ok"}).to_string())
    });
    let (dir, docs, out) = workspace("Testo.\n");
    std::fs::rename(docs.join("a.md"), docs.join("[TD] a.md")).unwrap();
    let rules = dir.path().join("rules.yaml");
    std::fs::write(&rules, "version: \"1\"\ndocuments:\n  - {match: 'TD', side: supplier, doc_type: technical_description}\n").unwrap();
    run(llm(&docs, &out, &server.url).arg("--rules").arg(&rules));
    let labels = &saved(&out)["items"][0]["labels"];
    assert_eq!(labels["required_action"], "none");
    assert_eq!(labels["flags"], json!([]));
    assert_eq!(saved(&out)["skipped"], json!([]));
}

/// Confidenza 0,9 se concorda con le regole, 0,5 se no, 0,7 senza il loro file (REQ-113).
#[test]
fn confidenza_dall_accordo_con_le_regole() {
    let server = serve(|_, _| (200, labels()));
    let (dir, docs, out) = workspace("The unit shall operate.\n");
    run(&mut llm(&docs, &out, &server.url));
    let without = &saved(&out)["items"][0];
    assert_eq!(without["confidence"]["nature"], 0.7);
    assert_eq!(without["unit_confidence"], 0.7);

    let rules_out = dir.path().join("rules.json");
    let rules = Command::new(env!("CARGO_BIN_EXE_rocket-tag"))
        .args(["classify", "rules"])
        .arg(&docs)
        .arg("-o")
        .arg(&rules_out)
        .args(["--schema", "config/label_schema.json", "--rules", "config/rules.yaml"])
        .output()
        .unwrap();
    assert!(rules.status.success());
    let requests = server.requests().len();
    run(llm(&docs, &out, &server.url).arg("--rules-out").arg(&rules_out));
    assert_eq!(server.requests().len(), requests, "alla ripresa si ricalcolano solo le confidenze");
    let with = &saved(&out)["items"][0];
    assert_eq!(with["confidence"]["nature"], 0.9, "regole: requirement");
    assert_eq!(with["confidence"]["required_action"], 0.5, "regole: none; LLM: declare_compliance");
    assert_eq!(with["unit_confidence"], 0.5);
}

/// Richieste parallele configurabili e limite al numero di unità (REQ-112).
#[test]
fn richieste_parallele_e_limite() {
    let running = Arc::new(AtomicUsize::new(0));
    let peak = Arc::new(AtomicUsize::new(0));
    let (now, top) = (Arc::clone(&running), Arc::clone(&peak));
    let server = serve(move |_, _| {
        top.fetch_max(now.fetch_add(1, Ordering::SeqCst) + 1, Ordering::SeqCst);
        std::thread::sleep(std::time::Duration::from_millis(150));
        now.fetch_sub(1, Ordering::SeqCst);
        (200, labels())
    });
    let (_dir, docs, out) = workspace("Uno.\n\nDue.\n\nTre.\n\nQuattro.\n\nCinque.\n\nSei.\n");
    run(llm(&docs, &out, &server.url).args(["--workers", "3"]));
    assert_eq!(peak.load(Ordering::SeqCst), 3);
    let lines: Vec<_> = saved(&out)["items"].as_array().unwrap().iter().map(|i| i["line_start"].as_u64().unwrap()).collect();
    assert_eq!(lines, [1, 3, 5, 7, 9, 11], "in ordine di documento e riga");

    let (_dir, docs, out) = workspace("Uno.\n\nDue.\n\nTre.\n");
    let requests = server.requests().len();
    let limited = run(llm(&docs, &out, &server.url).args(["--limit", "2"]));
    assert_eq!(server.requests().len() - requests, 2);
    assert_eq!(saved(&out)["items"].as_array().unwrap().len(), 2);
    assert_eq!(String::from_utf8_lossy(&limited.stdout).lines().last(), Some("[2/2] a.md"));
}

/// La ripresa salta le unità con stessa versione dello schema, `prompt_id` e `text_hash` (REQ-110).
#[test]
fn riprende_dopo_un_interruzione() {
    let server = serve(|_, _| (200, labels()));
    let (_dir, docs, out) = workspace("Uno.\n\nDue.\n\nTre.\n");
    run(llm(&docs, &out, &server.url).args(["--limit", "2"]));
    assert_eq!(server.requests().len(), 2);

    let resumed = run(&mut llm(&docs, &out, &server.url));
    assert_eq!(server.requests().len(), 3, "solo l'unità mancante");
    assert_eq!(saved(&out)["items"].as_array().unwrap().len(), 3);
    assert_eq!(String::from_utf8_lossy(&resumed.stdout).trim(), "[3/3] a.md");

    std::fs::write(docs.join("a.md"), "Uno.\n\nDue cambiato.\n\nTre.\n").unwrap();
    run(&mut llm(&docs, &out, &server.url));
    assert_eq!(server.requests().len(), 4, "testo cambiato: si rifà solo quell'unità");

    run(&mut with_model(&docs, &out, &server.url, "altro"));
    assert_eq!(server.requests().len(), 7, "prompt_id diverso: si rifà tutto");
}

/// Il file si aggiorna ogni 20 unità: dopo un'interruzione forzata resta valido e la ripresa
/// trova le unità già classificate (REQ-131).
#[test]
fn file_aggiornato_durante_l_esecuzione() {
    let server = serve(|_, _| {
        std::thread::sleep(std::time::Duration::from_millis(20));
        (200, labels())
    });
    let text: String = (0..100).map(|i| format!("Unità {i}.\n\n")).collect();
    let (_dir, docs, out) = workspace(&text);
    let mut child = llm(&docs, &out, &server.url).args(["--workers", "1"]).stdout(std::process::Stdio::null()).spawn().unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    while !out.exists() {
        assert!(std::time::Instant::now() < deadline, "nessun aggiornamento durante l'esecuzione");
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    child.kill().unwrap();
    child.wait().unwrap();

    let partial = saved(&out)["items"].as_array().unwrap().len();
    assert!((20..100).contains(&partial) && partial % 20 == 0, "{partial}");
    let before = server.requests().len();
    run(&mut llm(&docs, &out, &server.url));
    assert_eq!(saved(&out)["items"].as_array().unwrap().len(), 100);
    assert_eq!(server.requests().len() - before, 100 - partial);
}

#[test]
fn le_unita_sparite_non_restano_nel_file() {
    let server = serve(|_, _| (200, labels()));
    let (_dir, docs, out) = workspace("Uno.\n\nDue.\n");
    run(&mut llm(&docs, &out, &server.url));
    std::fs::write(docs.join("a.md"), "Uno.\n").unwrap();
    run(&mut llm(&docs, &out, &server.url));
    assert_eq!(saved(&out)["items"].as_array().unwrap().len(), 1);
    assert_eq!(server.requests().len(), 2);
}

/// 5 errori di connessione senza alcun successo fermano l'esecuzione con un messaggio (REQ-111).
#[test]
fn si_ferma_se_il_server_non_risponde() {
    let server = serve(|_, _| (500, "guasto".to_string()));
    let (_dir, docs, out) = workspace("Uno.\n\nDue.\n\nTre.\n\nQuattro.\n\nCinque.\n\nSei.\n\nSette.\n");
    let failed = llm(&docs, &out, &server.url).args(["--workers", "1"]).output().unwrap();
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("il server LLM non risponde"));
    assert_eq!(server.requests().len(), 5);

    let closed = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/v1", closed.local_addr().unwrap());
    drop(closed);
    let (_dir, docs, out) = workspace("Unica.\n");
    let failed = llm(&docs, &out, &url).output().unwrap();
    assert!(!failed.status.success(), "anche con meno di 5 unità, se nessuna è riuscita");
    assert!(!out.exists());
}

/// Dopo almeno un successo gli errori del server scartano solo le unità coinvolte.
#[test]
fn dopo_un_successo_gli_errori_scartano_le_unita() {
    let server = serve(|_, number| if number == 1 { (200, labels()) } else { (500, "guasto".to_string()) });
    let (_dir, docs, out) = workspace("Uno.\n\nDue.\n\nTre.\n\nQuattro.\n\nCinque.\n\nSei.\n\nSette.\n");
    run(llm(&docs, &out, &server.url).args(["--workers", "1"]));
    let result = saved(&out);
    assert_eq!(result["items"].as_array().unwrap().len(), 1);
    assert_eq!(result["skipped"].as_array().unwrap().len(), 6);
    assert!(result["skipped"][0]["error"].as_str().unwrap().starts_with("server:"));
}

/// Un testo da riga di comando anche con l'LLM, con le unità vicine nel prompt (REQ-084).
#[test]
fn classify_one_con_l_llm() {
    let server = serve(|_, _| (200, labels()));
    let output = Command::new(env!("CARGO_BIN_EXE_rocket-tag"))
        .args(["classify", "one", "--text", "Testo.", "--prev", "Prima.", "--next", "Dopo."])
        .args(["--schema", "config/label_schema.json", "--guide", "config/prompt_guide.md"])
        .args(["--examples", "config/guide_examples.json", "--base-url", &server.url, "--model", "modello"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["labels"]["nature"], "requirement");
    let user = server.requests()[0].body["messages"][1]["content"].as_str().unwrap().to_string();
    assert!(user.contains("Previous unit (context): Prima.") && user.contains("Next unit (context): Dopo."));
}

fn client(url: &str) -> (Schema, Llm) {
    let schema = Schema::load(Path::new("config/label_schema.json")).unwrap();
    let settings = Settings {
        base_url: url.to_string(),
        model: "modello".to_string(),
        api_key: None,
        format: Format::JsonSchema,
        temperature: 0.0,
        timeout: Duration::from_secs(5),
    };
    let llm = Llm::new(&schema, "Guida", &[], settings);
    (schema, llm)
}

/// Bastano lo schema e il testo dell'unità, senza il documento intero (REQ-091).
#[test]
fn l_llm_bastano_schema_e_testo() {
    let server = serve(|_, _| (200, labels()));
    let (schema, llm) = client(&server.url);
    let decision = llm.classify(&schema, "Testo.", &Context::default(), None).unwrap();
    assert_eq!(decision.labels["nature"], "requirement");
}

/// Unità vicine e contesto sono facoltativi: con e senza, il classificatore funziona (REQ-093).
#[test]
fn contesto_facoltativo() {
    let server = serve(|_, _| (200, labels()));
    let (schema, llm) = client(&server.url);
    let context = Context {
        heading_path: vec!["4 Verification".to_string()],
        clause: Some("4".to_string()),
        kind: Some(Kind::Paragraph),
        side: Some("prime".to_string()),
        doc_type: Some("statement_of_work".to_string()),
        prev: Some("Prima.".to_string()),
        next: Some("Dopo.".to_string()),
        ..Context::default()
    };
    assert!(llm.classify(&schema, "Testo.", &context, None).is_ok());
    assert!(llm.classify(&schema, "Testo.", &Context::default(), None).is_ok());
    let requests = server.requests();
    let user = |index: usize| requests[index].body["messages"][1]["content"].as_str().unwrap().to_string();
    assert!(user(0).contains("Heading path: 4 Verification"));
    assert!(user(0).contains("Document side: prime; type: statement_of_work"));
    assert!(user(1).contains("Heading path: (none)"));
}

/// Regole e LLM girano ciascuno da solo, con il proprio file di uscita (REQ-082).
#[test]
fn classificatori_indipendenti_con_file_propri() {
    let server = serve(|_, _| (200, labels()));
    let (dir, docs, out) = workspace("Testo.\n");
    let rules_out = dir.path().join("rules.json");
    let rules = Command::new(env!("CARGO_BIN_EXE_rocket-tag"))
        .args(["classify", "rules"])
        .arg(&docs)
        .arg("-o")
        .arg(&rules_out)
        .args(["--schema", "config/label_schema.json", "--rules", "config/rules.yaml"])
        .output()
        .unwrap();
    assert!(rules.status.success());
    assert!(server.requests().is_empty(), "le regole non chiamano il server");
    assert!(!out.exists());

    std::fs::remove_file(&rules_out).unwrap();
    run(&mut llm(&docs, &out, &server.url));
    assert!(!rules_out.exists(), "l'LLM non ha bisogno delle regole");
    assert_eq!(saved(&out)["classifier"], "llm:modello");
}

/// Il testo dei documenti va solo al server configurato, anche con un proxy nell'ambiente (REQ-081).
#[test]
fn nessun_invio_a_servizi_diversi_dal_server_configurato() {
    let proxy = serve(|_, _| (200, labels()));
    let server = serve(|_, _| (200, labels()));
    let (_dir, docs, out) = workspace("Testo riservato.\n");
    let address = proxy.url.trim_end_matches("/v1").to_string();
    run(llm(&docs, &out, &server.url).env("HTTP_PROXY", &address).env("http_proxy", &address).env("ALL_PROXY", &address));
    assert_eq!(server.requests().len(), 1);
    assert!(proxy.requests().is_empty());
}

/// La rete serve solo al client LLM.
#[test]
fn solo_il_client_llm_usa_la_rete() {
    for source in [
        include_str!("../src/classifier.rs"),
        include_str!("../src/iterate.rs"),
        include_str!("../src/labels.rs"),
        include_str!("../src/markdown.rs"),
        include_str!("../src/output.rs"),
        include_str!("../src/rules.rs"),
        include_str!("../src/schema.rs"),
    ] {
        assert!(!source.contains("ureq") && !source.contains("TcpStream"));
    }
}

/// Uno schema con altre faccette passa da iteratore, LLM e valutazione senza toccare il codice
/// (REQ-097).
#[test]
fn uno_schema_diverso_non_richiede_modifiche_al_codice() {
    let schema = json!({
        "version": "9.0.0",
        "$defs": {
            "Tono": {"type": "string", "enum": ["formale", "informale"], "x-enumDescriptions": {"formale": "Lettera"}},
            "Tema": {"type": "string", "enum": ["spazio", "mare"]},
            "Side": {"type": "string", "enum": ["a"]},
            "DocType": {"type": "string", "enum": ["b"]},
            "ParagraphFacets": {
                "properties": {
                    "tono": {"$ref": "#/$defs/Tono"},
                    "temi": {"type": "array", "items": {"$ref": "#/$defs/Tema"}, "uniqueItems": true}
                },
                "required": ["tono", "temi"]
            },
            "DocumentLabel": {"properties": {"side": {"$ref": "#/$defs/Side"}, "doc_type": {"$ref": "#/$defs/DocType"}}}
        }
    });
    let server = serve(|body, _| {
        assert!(body["messages"][0]["content"].as_str().unwrap().contains("tono (one of):\n  - formale: Lettera"));
        assert_eq!(body["response_format"]["json_schema"]["schema"]["required"], json!(["tono", "temi"]));
        (200, json!({"tono": "formale", "temi": ["spazio"]}).to_string())
    });
    let (dir, docs, out) = workspace("Testo.\n");
    let schema_file = dir.path().join("schema.json");
    std::fs::write(&schema_file, schema.to_string()).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_rocket-tag"));
    command
        .args(["classify", "llm"])
        .arg(&docs)
        .arg("-o")
        .arg(&out)
        .arg("--schema")
        .arg(&schema_file)
        .args(["--guide", "config/prompt_guide.md", "--examples", "config/guide_examples.json"])
        .args(["--base-url", &server.url, "--model", "modello"]);
    run(&mut command);
    let result = saved(&out);
    assert_eq!(result["schema_version"], "9.0.0");
    assert_eq!(result["items"][0]["labels"], json!({"tono": "formale", "temi": ["spazio"]}));

    let labels = dir.path().join("etichette.jsonl");
    let label = json!({"level": "paragraph", "document": "a.md", "line_start": 1, "line_end": 1,
                       "facets": {"tono": "formale", "temi": ["mare"]}});
    std::fs::write(&labels, label.to_string()).unwrap();
    let mut evaluate = Command::new(env!("CARGO_BIN_EXE_rocket-tag"));
    evaluate.arg("evaluate").arg(&labels).arg(&out).arg("--schema").arg(&schema_file);
    let stdout = String::from_utf8_lossy(&run(&mut evaluate).stdout).into_owned();
    assert!(stdout.contains("tono") && stdout.contains("accuratezza 100.0%"), "{stdout}");
    assert!(stdout.contains("temi") && stdout.contains("accuratezza   0.0%"), "{stdout}");
}

/// Nessun nome di faccetta o valore nel codice dei componenti (REQ-095, REQ-097).
#[test]
fn nessuna_faccetta_scritta_nel_codice() {
    let without_tests = |source: &'static str| source.split("#[cfg(test)]").next().unwrap();
    for source in [
        without_tests(include_str!("../src/llm.rs")),
        without_tests(include_str!("../src/iterate.rs")),
        without_tests(include_str!("../src/evaluate.rs")),
        without_tests(include_str!("../src/classifier.rs")),
    ] {
        for name in ["\"nature\"", "\"domain\"", "\"destination\"", "\"required_action\"", "\"extraction_quality\"", "\"flags\""] {
            assert!(!source.contains(name), "{name}");
        }
    }
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
