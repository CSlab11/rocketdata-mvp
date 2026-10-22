use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::Command;
use tokio::sync::broadcast;

use crate::summary::{self, Summary};

/// Stato dell'ultima esecuzione di doc-ripper.
#[derive(Debug, Default, Clone, Serialize)]
pub struct Snapshot {
    pub running: bool,
    pub lines: Vec<String>,
    pub exit_code: Option<i32>,
    /// Riepilogo letto dall'output a fine esecuzione (REQ-032).
    pub summary: Option<Summary>,
}

/// Evento dell'esecuzione in corso, inoltrato in tempo reale (REQ-031).
#[derive(Debug, Clone)]
pub enum Event {
    Line(String),
    End(Option<i32>),
}

/// Esegue doc-ripper come processo separato, uno alla volta (REQ-028).
pub struct Runner {
    state: Mutex<Snapshot>,
    events: broadcast::Sender<Event>,
}

impl Default for Runner {
    fn default() -> Self {
        Runner { state: Mutex::default(), events: broadcast::channel(1024).0 }
    }
}

/// Il processo non ha una console propria: niente finestre che si aprono.
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

impl Runner {
    pub fn snapshot(&self) -> Snapshot {
        self.state.lock().unwrap().clone()
    }

    /// Stato attuale e iscrizione agli eventi successivi, presi insieme: nessuna riga va persa
    /// né arriva due volte.
    pub fn subscribe(&self) -> (Snapshot, broadcast::Receiver<Event>) {
        let state = self.state.lock().unwrap();
        (state.clone(), self.events.subscribe())
    }

    /// Avvia `exe --src <src> --dst <dst>`; errore se un'esecuzione è già in corso.
    pub fn start(self: &Arc<Self>, exe: &Path, src: &str, dst: &str) -> Result<(), String> {
        {
            let mut state = self.state.lock().unwrap();
            if state.running {
                return Err("doc-ripper è già in esecuzione".into());
            }
            *state = Snapshot { running: true, ..Snapshot::default() };
        }
        let spawned = Command::new(exe)
            .args(["--src", src, "--dst", dst])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .creation_flags(CREATE_NO_WINDOW)
            .kill_on_drop(true)
            .spawn();
        let mut child = match spawned {
            Ok(child) => child,
            Err(error) => {
                self.state.lock().unwrap().running = false;
                return Err(format!("impossibile avviare doc-ripper: {error}"));
            }
        };

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        let runner = Arc::clone(self);
        tokio::spawn(async move {
            tokio::join!(runner.collect(stdout), runner.collect(stderr));
            let code = child.wait().await.ok().and_then(|s| s.code());
            let mut state = runner.state.lock().unwrap();
            state.exit_code = code;
            state.summary = summary::parse(&state.lines);
            state.running = false;
            let _ = runner.events.send(Event::End(code));
        });
        Ok(())
    }

    async fn collect(&self, stream: impl AsyncRead + Unpin) {
        let mut reader = BufReader::new(stream);
        let mut buf = Vec::new();
        while reader.read_until(b'\n', &mut buf).await.is_ok_and(|n| n > 0) {
            let line = String::from_utf8_lossy(&buf).trim_end_matches(['\r', '\n']).to_string();
            let mut state = self.state.lock().unwrap();
            state.lines.push(line.clone());
            let _ = self.events.send(Event::Line(line));
            drop(state);
            buf.clear();
        }
    }
}

/// Percorso di doc-ripper oppure errore se non è disponibile (REQ-029).
pub fn require(exe: &Option<PathBuf>) -> Result<&Path, String> {
    exe.as_deref().ok_or_else(|| "doc-ripper.exe non disponibile".to_string())
}
