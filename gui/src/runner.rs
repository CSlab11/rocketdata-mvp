use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::Command;
use tokio::sync::{broadcast, oneshot};

use crate::summary::{self, Summary};

/// Stato dell'ultima esecuzione di un tool.
#[derive(Debug, Default, Clone, Serialize)]
pub struct Snapshot {
    pub running: bool,
    pub lines: Vec<String>,
    pub exit_code: Option<i32>,
    /// Riepilogo letto dall'output a fine esecuzione (REQ-032).
    pub summary: Option<Summary>,
    /// L'esecuzione è stata interrotta con "Annulla" (REQ-034).
    pub cancelled: bool,
    /// Cartella di destinazione dell'esecuzione (REQ-033).
    pub dst: Option<String>,
}

/// Evento dell'esecuzione in corso, inoltrato in tempo reale (REQ-031).
#[derive(Debug, Clone)]
pub enum Event {
    Line(String),
    End(Option<i32>),
}

/// Esegue un tool come processo separato, una esecuzione alla volta (REQ-028, REQ-133).
pub struct Runner {
    /// Nome del tool nei messaggi di errore.
    name: &'static str,
    state: Mutex<Snapshot>,
    events: broadcast::Sender<Event>,
    /// Segnale per interrompere l'esecuzione in corso (REQ-034).
    cancel: Mutex<Option<oneshot::Sender<()>>>,
}

impl Default for Runner {
    fn default() -> Self {
        Runner::new("doc-ripper")
    }
}

/// Il processo non ha una console propria: niente finestre che si aprono.
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

impl Runner {
    pub fn new(name: &'static str) -> Runner {
        Runner { name, state: Mutex::default(), events: broadcast::channel(1024).0, cancel: Mutex::default() }
    }

    /// Runner con un'esecuzione già terminata su `dst`, per i test.
    #[cfg(test)]
    pub fn finished(dst: &str) -> Runner {
        let runner = Runner::default();
        *runner.state.lock().unwrap() = Snapshot { dst: Some(dst.into()), exit_code: Some(0), ..Snapshot::default() };
        runner
    }

    pub fn snapshot(&self) -> Snapshot {
        self.state.lock().unwrap().clone()
    }

    /// Stato attuale e iscrizione agli eventi successivi, presi insieme: nessuna riga va persa
    /// né arriva due volte.
    pub fn subscribe(&self) -> (Snapshot, broadcast::Receiver<Event>) {
        let state = self.state.lock().unwrap();
        (state.clone(), self.events.subscribe())
    }

    /// Avvia `exe` con `args` e le variabili d'ambiente `envs`; `dst` è la cartella su cui lavora.
    /// Errore se un'esecuzione è già in corso.
    pub fn start(self: &Arc<Self>, exe: &Path, args: &[OsString], envs: &[(&str, String)], dst: &str) -> Result<(), String> {
        {
            let mut state = self.state.lock().unwrap();
            if state.running {
                return Err(format!("{} è già in esecuzione", self.name));
            }
            *state = Snapshot { running: true, dst: Some(dst.to_string()), ..Snapshot::default() };
        }
        let spawned = Command::new(exe)
            .args(args)
            .envs(envs.iter().map(|(name, value)| (*name, value)))
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
                return Err(format!("impossibile avviare {}: {error}", self.name));
            }
        };

        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        let (cancel_tx, cancel_rx) = oneshot::channel();
        *self.cancel.lock().unwrap() = Some(cancel_tx);
        let runner = Arc::clone(self);
        tokio::spawn(async move {
            let output = async { tokio::join!(runner.collect(stdout), runner.collect(stderr)) };
            let process = async {
                let mut status = None;
                let cancelled = tokio::select! {
                    result = child.wait() => {
                        status = result.ok();
                        false
                    }
                    _ = cancel_rx => true,
                };
                if cancelled {
                    let _ = child.kill().await;
                    status = child.wait().await.ok();
                }
                (status.and_then(|s| s.code()), cancelled)
            };
            let (_, (code, cancelled)) = tokio::join!(output, process);
            runner.cancel.lock().unwrap().take();
            let mut state = runner.state.lock().unwrap();
            state.exit_code = code;
            state.cancelled = cancelled;
            state.summary = summary::parse(&state.lines);
            state.running = false;
            let _ = runner.events.send(Event::End(code));
        });
        Ok(())
    }

    /// Interrompe l'esecuzione in corso (REQ-034); errore se non ce n'è una.
    pub fn cancel(&self) -> Result<(), String> {
        let signal = self.cancel.lock().unwrap().take();
        match signal.map(|s| s.send(())) {
            Some(Ok(())) => Ok(()),
            _ => Err(format!("nessuna esecuzione di {} in corso", self.name)),
        }
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
