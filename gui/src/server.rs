use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::middleware::{self, Next};
use std::convert::Infallible;

use axum::response::sse::{self, KeepAlive, Sse};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::{Stream, StreamExt};

use crate::runner::{self, Event, Runner, Snapshot};

/// Interfaccia web incorporata nell'eseguibile (REQ-026).
const INDEX_HTML: &str = include_str!("../web/index.html");
const STYLE_CSS: &str = include_str!("../web/style.css");
const APP_JS: &str = include_str!("../web/app.js");

pub struct AppState {
    /// Porta di 127.0.0.1 su cui ascolta il server.
    pub port: u16,
    /// `doc-ripper.exe` accanto a `gui.exe`, se presente (REQ-029).
    pub doc_ripper: Option<PathBuf>,
    pub runner: Arc<Runner>,
    /// Segnala al server di spegnersi (REQ-039).
    pub shutdown: Arc<tokio::sync::Notify>,
}

/// Applicazione HTTP della gui.
pub fn router(state: AppState) -> Router {
    let state = Arc::new(state);
    Router::new()
        .route("/", get(|| async { Html(INDEX_HTML) }))
        .route("/style.css", get(|| async { asset("text/css; charset=utf-8", STYLE_CSS) }))
        .route("/app.js", get(|| async { asset("text/javascript; charset=utf-8", APP_JS) }))
        .route("/api/doc-ripper/status", get(doc_ripper_status))
        .route("/api/doc-ripper/run", post(doc_ripper_run))
        .route("/api/doc-ripper/output", get(doc_ripper_output))
        .route("/api/doc-ripper/events", get(doc_ripper_events))
        .route("/api/doc-ripper/cancel", post(doc_ripper_cancel))
        .route("/api/shutdown", post(shutdown))
        .layer(middleware::from_fn_with_state(state.port, local_only))
        .with_state(state)
}

fn asset(content_type: &'static str, body: &'static str) -> impl IntoResponse {
    ([(header::CONTENT_TYPE, content_type)], body)
}

#[derive(Serialize)]
struct DocRipperStatus {
    available: bool,
    message: Option<String>,
}

async fn doc_ripper_status(State(state): State<Arc<AppState>>) -> Json<DocRipperStatus> {
    Json(match &state.doc_ripper {
        Some(_) => DocRipperStatus { available: true, message: None },
        None => DocRipperStatus {
            available: false,
            message: Some("doc-ripper.exe non trovato nella cartella di gui.exe: copiarlo lì e riavviare la gui.".into()),
        },
    })
}

#[derive(Deserialize)]
struct RunRequest {
    src: String,
    dst: String,
}

/// Avvia doc-ripper con le cartelle indicate (REQ-028).
async fn doc_ripper_run(
    State(state): State<Arc<AppState>>,
    Json(request): Json<RunRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let exe = runner::require(&state.doc_ripper).map_err(|e| (StatusCode::SERVICE_UNAVAILABLE, e))?;
    if request.src.trim().is_empty() || request.dst.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "indicare la cartella di origine e quella di destinazione".into()));
    }
    state
        .runner
        .start(exe, request.src.trim(), request.dst.trim())
        .map_err(|e| (StatusCode::CONFLICT, e))?;
    Ok(StatusCode::ACCEPTED)
}

/// Interrompe l'esecuzione in corso (REQ-034).
async fn doc_ripper_cancel(State(state): State<Arc<AppState>>) -> Result<StatusCode, (StatusCode, String)> {
    state.runner.cancel().map_err(|e| (StatusCode::CONFLICT, e))?;
    Ok(StatusCode::ACCEPTED)
}

/// Pulsante "Esci" (REQ-039): termina un eventuale doc-ripper in corso, poi spegne il server.
async fn shutdown(State(state): State<Arc<AppState>>) -> StatusCode {
    if state.runner.cancel().is_ok() {
        for _ in 0..50 {
            if !state.runner.snapshot().running {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }
    state.shutdown.notify_one();
    StatusCode::ACCEPTED
}

async fn doc_ripper_output(State(state): State<Arc<AppState>>) -> Json<Snapshot> {
    Json(state.runner.snapshot())
}

/// Output dell'esecuzione in tempo reale (REQ-031): prima le righe già prodotte, poi le nuove.
/// Eventi `line` (una riga di output) e `end` (codice di uscita).
async fn doc_ripper_events(State(state): State<Arc<AppState>>) -> Sse<impl Stream<Item = Result<sse::Event, Infallible>>> {
    let (snapshot, receiver) = state.runner.subscribe();
    let mut past: Vec<Event> = snapshot.lines.into_iter().map(Event::Line).collect();
    if !snapshot.running && (snapshot.exit_code.is_some() || !past.is_empty()) {
        past.push(Event::End(snapshot.exit_code));
    }
    let live = BroadcastStream::new(receiver).filter_map(Result::ok);
    let stream = tokio_stream::iter(past).chain(live).map(|event| {
        Ok(match event {
            Event::Line(line) => sse::Event::default().event("line").data(line),
            Event::End(code) => sse::Event::default()
                .event("end")
                .data(code.map_or("null".to_string(), |c| c.to_string())),
        })
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// Accetta solo richieste indirizzate a localhost (REQ-036): un sito esterno aperto nel browser
/// non può comandare la gui puntando il proprio dominio a 127.0.0.1 (DNS rebinding) né inviarle
/// richieste da un'altra origine.
async fn local_only(State(port): State<u16>, request: Request, next: Next) -> Result<Response, StatusCode> {
    if is_local(request.headers(), port) {
        Ok(next.run(request).await)
    } else {
        Err(StatusCode::FORBIDDEN)
    }
}

fn is_local(headers: &HeaderMap, port: u16) -> bool {
    let allowed_hosts = [format!("127.0.0.1:{port}"), format!("localhost:{port}")];
    let host_ok = headers
        .get(header::HOST)
        .and_then(|h| h.to_str().ok())
        .is_some_and(|h| allowed_hosts.iter().any(|a| a.eq_ignore_ascii_case(h)));
    let origin_ok = match headers.get(header::ORIGIN) {
        None => true,
        Some(origin) => origin
            .to_str()
            .ok()
            .is_some_and(|o| allowed_hosts.iter().any(|a| o.eq_ignore_ascii_case(&format!("http://{a}")))),
    };
    host_ok && origin_ok
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{Body, to_bytes};
    use tower::ServiceExt;

    fn app() -> Router {
        router(AppState { port: 4000, doc_ripper: None, runner: Arc::default(), shutdown: Arc::default() })
    }

    async fn get(path: &str, host: &str, origin: Option<&str>) -> Response {
        let mut request = Request::builder().uri(path).header(header::HOST, host);
        if let Some(origin) = origin {
            request = request.header(header::ORIGIN, origin);
        }
        app().oneshot(request.body(Body::empty()).unwrap()).await.unwrap()
    }

    async fn status(host: &str, origin: Option<&str>) -> StatusCode {
        get("/", host, origin).await.status()
    }

    #[tokio::test]
    async fn accetta_solo_localhost() {
        assert_eq!(status("127.0.0.1:4000", None).await, StatusCode::OK);
        assert_eq!(status("localhost:4000", Some("http://localhost:4000")).await, StatusCode::OK);
        assert_eq!(status("evil.example:4000", None).await, StatusCode::FORBIDDEN);
        assert_eq!(status("127.0.0.1:5000", None).await, StatusCode::FORBIDDEN);
        assert_eq!(status("127.0.0.1:4000", Some("http://evil.example")).await, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn serve_pagina_stili_e_script() {
        for (path, content_type, needle) in [
            ("/", "text/html; charset=utf-8", "<title>RocketData</title>"),
            ("/style.css", "text/css; charset=utf-8", "--accent"),
            ("/app.js", "text/javascript; charset=utf-8", "use strict"),
        ] {
            let response = get(path, "127.0.0.1:4000", None).await;
            assert_eq!(response.status(), StatusCode::OK, "{path}");
            assert_eq!(response.headers()[header::CONTENT_TYPE], content_type, "{path}");
            let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
            assert!(String::from_utf8_lossy(&body).contains(needle), "{path}");
        }
    }

    #[test]
    fn menu_delle_funzioni_con_doc_ripper() {
        assert!(INDEX_HTML.contains(r#"<nav class="sidebar" aria-label="Funzioni">"#));
        assert!(APP_JS.contains("const FUNCTIONS = ["));
        assert!(APP_JS.contains(r#"id: "doc-ripper""#));
    }
}
