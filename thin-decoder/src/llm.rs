//! Client LLM: Ollama tramite l'endpoint compatibile con OpenAI
//! (`/v1/chat/completions`), con output vincolato da schema JSON.
//! Ogni risposta è salvata in una cache su file, indicizzata dall'hash di
//! modello, versione, prompt e schema.

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::time::Duration;

use crate::config::LlmConfig;

pub struct Llm {
    cfg: LlmConfig,
    agent: ureq::Agent,
}

impl Llm {
    pub fn new(cfg: LlmConfig) -> Llm {
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(5))
            .timeout_read(Duration::from_secs(600))
            .build();
        Llm { cfg, agent }
    }

    fn cache_path(&self, prompt: &str, schema: &Value) -> PathBuf {
        let key = json!([self.cfg.model, self.cfg.version, prompt, schema]).to_string();
        let hash: String = Sha256::digest(key.as_bytes()).iter().map(|b| format!("{b:02x}")).collect();
        PathBuf::from(&self.cfg.cache_dir).join(format!("{hash}.json"))
    }

    /// Restituisce l'oggetto JSON prodotto dal modello per il prompt, vincolato
    /// dallo schema: dalla cache se la chiave coincide, altrimenti dal server.
    pub fn chat(&self, prompt: &str, schema: &Value) -> Result<Value, String> {
        let path = self.cache_path(prompt, schema);
        if let Ok(raw) = std::fs::read_to_string(&path) {
            if let Ok(v) = serde_json::from_str(&raw) {
                return Ok(v);
            }
        }
        let v = self.request(prompt, schema)?;
        std::fs::create_dir_all(&self.cfg.cache_dir)
            .and_then(|_| std::fs::write(&path, v.to_string()))
            .map_err(|e| format!("impossibile scrivere la cache {}: {e}", path.display()))?;
        Ok(v)
    }

    fn request(&self, prompt: &str, schema: &Value) -> Result<Value, String> {
        let url = format!("{}/v1/chat/completions", self.cfg.url.trim_end_matches('/'));
        let body = json!({
            "model": self.cfg.model,
            "messages": [{"role": "user", "content": prompt}],
            "temperature": self.cfg.temperature,
            "seed": self.cfg.seed,
            "stream": false,
            "response_format": {
                "type": "json_schema",
                "json_schema": {"name": "output", "strict": true, "schema": schema},
            },
        });
        let resp: Value = self
            .agent
            .post(&url)
            .send_json(body)
            .map_err(|e| match e {
                ureq::Error::Transport(t) => {
                    format!("server LLM non raggiungibile su {url} e risposta non in cache: {t}")
                }
                ureq::Error::Status(code, _) => format!("il server LLM {url} ha risposto con errore {code}"),
            })?
            .into_json()
            .map_err(|e| format!("risposta del server LLM non valida: {e}"))?;
        let content = resp["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| format!("risposta del server LLM senza contenuto: {resp}"))?;
        serde_json::from_str(content)
            .map_err(|e| format!("il modello non ha prodotto JSON valido ({e}): {content}"))
    }
}

/// Finto server compatibile con OpenAI per i test: risponde a ogni richiesta
/// con il contenuto calcolato da `reply` e registra i corpi ricevuti.
#[cfg(test)]
pub mod fake {
    use serde_json::{json, Value};
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    pub struct Server {
        pub url: String,
        pub requests: Arc<Mutex<Vec<Value>>>,
    }

    pub fn start(reply: impl Fn(&Value) -> Value + Send + 'static) -> Server {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let log = requests.clone();
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
                let req: Value = serde_json::from_slice(&body).unwrap();
                let content = reply(&req).to_string();
                log.lock().unwrap().push(req);
                let resp = json!({"choices": [{"message": {"role": "assistant", "content": content}}]})
                    .to_string();
                let head = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    resp.len()
                );
                stream.write_all(head.as_bytes()).unwrap();
                stream.write_all(resp.as_bytes()).unwrap();
            }
        });
        Server { url, requests }
    }
}

#[cfg(test)]
pub fn test_config(url: &str, cache: &str) -> LlmConfig {
    let dir = std::env::temp_dir().join(cache);
    let _ = std::fs::remove_dir_all(&dir);
    LlmConfig {
        url: url.into(),
        model: "qwen2.5:7b".into(),
        version: "v1".into(),
        cache_dir: dir.to_str().unwrap().into(),
        temperature: 0.0,
        seed: 42,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn posts_openai_request_with_schema_and_reads_json_content() {
        let server = fake::start(|_| json!({"answer": "2.5 kg"}));
        let llm = Llm::new(test_config(&server.url, "td_cache_request"));
        let schema = json!({"type": "object", "properties": {"answer": {"type": "string"}}});
        assert_eq!(llm.chat("What is the mass?", &schema).unwrap(), json!({"answer": "2.5 kg"}));
        let req = server.requests.lock().unwrap()[0].clone();
        assert_eq!(req["model"], "qwen2.5:7b");
        assert_eq!(req["messages"][0]["content"], "What is the mass?");
        assert_eq!((req["temperature"].as_f64(), req["seed"].as_u64()), (Some(0.0), Some(42)));
        assert_eq!(req["response_format"]["json_schema"]["schema"], schema);
    }

    #[test]
    fn cache_is_reused_when_the_key_matches() {
        let server = fake::start(|req| json!({"echo": req["messages"][0]["content"]}));
        let cfg = test_config(&server.url, "td_cache_reuse");
        let llm = Llm::new(cfg.clone());
        let schema = json!({"type": "object"});
        let first = llm.chat("a", &schema).unwrap();
        assert_eq!(llm.chat("a", &schema).unwrap(), first);
        assert_eq!(server.requests.lock().unwrap().len(), 1);
        // Stessa chiave, server spento: risposta dalla cache.
        let offline = Llm::new(LlmConfig { url: "http://127.0.0.1:9".into(), ..cfg.clone() });
        assert_eq!(offline.chat("a", &schema).unwrap(), first);
        // Chiave diversa (prompt, schema, versione): nuova richiesta.
        llm.chat("b", &schema).unwrap();
        llm.chat("a", &json!({"type": "object", "required": []})).unwrap();
        Llm::new(LlmConfig { version: "v2".into(), ..cfg }).chat("a", &schema).unwrap();
        assert_eq!(server.requests.lock().unwrap().len(), 4);
    }

    #[test]
    fn unreachable_server_without_cache_is_an_explicit_error() {
        let llm = Llm::new(test_config("http://127.0.0.1:9", "td_cache_down"));
        let err = llm.chat("a", &json!({"type": "object"})).unwrap_err();
        assert!(err.contains("server LLM non raggiungibile") && err.contains("non in cache"), "{err}");
    }
}
