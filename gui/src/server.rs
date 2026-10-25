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

use std::sync::atomic::{AtomicBool, Ordering};

use crate::dialogs::{FolderPicker, Opener, Opening};
use crate::search;
use crate::settings::{self, Settings};
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
    /// `search-rm3.exe` accanto a `gui.exe`, se presente (REQ-069).
    pub search_rm3: Option<PathBuf>,
    pub runner: Arc<Runner>,
    /// Segnala al server di spegnersi (REQ-039).
    pub shutdown: Arc<tokio::sync::Notify>,
    /// Selettore di cartelle (REQ-030); sostituibile nei test.
    pub folder_picker: FolderPicker,
    /// Un solo selettore aperto alla volta.
    pub picking: AtomicBool,
    /// File delle impostazioni (REQ-035); `None` se `%APPDATA%` non è definita.
    pub settings_path: Option<PathBuf>,
    /// Apertura dei risultati sul desktop (REQ-033); sostituibile nei test.
    pub opener: Opener,
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
        .route("/api/doc-ripper/index", get(doc_ripper_index))
        .route("/api/search/index", post(search_index))
        .route("/api/search/query", post(search_query))
        .route("/api/shutdown", post(shutdown))
        .route("/api/pick-folder", post(pick_folder))
        .route("/api/settings", get(get_settings))
        .route("/api/open", post(open_result))
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
    let (src, dst) = (request.src.trim(), request.dst.trim());
    state.runner.start(exe, src, dst).map_err(|e| (StatusCode::CONFLICT, e))?;
    // Ultime cartelle usate, riproposte ai prossimi avvii (REQ-035).
    if let Some(path) = &state.settings_path {
        if let Err(error) = settings::save(path, &Settings { src: src.into(), dst: dst.into() }) {
            eprintln!("Impossibile salvare le impostazioni in {}: {error}", path.display());
        }
    }
    Ok(StatusCode::ACCEPTED)
}

#[derive(Deserialize)]
struct SearchIndexRequest {
    src: String,
}

#[derive(Deserialize)]
struct SearchQueryRequest {
    src: String,
    q: String,
}

fn search_rm3(state: &AppState) -> Result<&std::path::Path, (StatusCode, String)> {
    state.search_rm3.as_deref().ok_or_else(|| (StatusCode::SERVICE_UNAVAILABLE, "search-rm3.exe non disponibile".into()))
}

fn search_src(src: &str) -> Result<&std::path::Path, (StatusCode, String)> {
    match src.trim() {
        "" => Err((StatusCode::BAD_REQUEST, "indicare la cartella da indicizzare".into())),
        src => Ok(std::path::Path::new(src)),
    }
}

/// Indicizza la cartella con `search-rm3 index` (REQ-069).
async fn search_index(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SearchIndexRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let exe = search_rm3(&state)?;
    let src = search_src(&request.src)?;
    search::index(exe, src).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(StatusCode::NO_CONTENT)
}

/// Cerca nell'indice della cartella con `search-rm3 query`: il JSON del sotto-tool (REQ-069).
async fn search_query(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SearchQueryRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let exe = search_rm3(&state)?;
    let src = search_src(&request.src)?;
    if request.q.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "scrivere una domanda".into()));
    }
    let response = search::query(exe, src, request.q.trim()).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(response))
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum OpenTarget {
    Destination,
    Index,
    /// Un link dell'anteprima di `index.md` (REQ-048).
    Link,
}

#[derive(Deserialize)]
struct OpenRequest {
    target: OpenTarget,
    href: Option<String>,
}

/// Testo di `index.md` dell'ultima esecuzione terminata, per l'anteprima (REQ-048).
async fn doc_ripper_index(State(state): State<Arc<AppState>>) -> Result<String, (StatusCode, String)> {
    let snapshot = state.runner.snapshot();
    let dst = match (&snapshot.dst, snapshot.running) {
        (Some(dst), false) => PathBuf::from(dst),
        _ => return Err((StatusCode::CONFLICT, "nessuna conversione terminata".into())),
    };
    std::fs::read_to_string(dst.join("index.md")).map_err(|_| (StatusCode::NOT_FOUND, "nessun indice disponibile".into()))
}

/// File a cui punta un link di `index.md`: solo i link presenti nell'indice, decodificati
/// (`%20` → spazio) e risolti rispetto alla destinazione se relativi (REQ-048).
fn index_link(dst: &std::path::Path, href: &str) -> Result<PathBuf, (StatusCode, String)> {
    let index = std::fs::read_to_string(dst.join("index.md")).unwrap_or_default();
    if href.is_empty() || !index.contains(&format!("]({href})")) {
        return Err((StatusCode::FORBIDDEN, "link non presente in index.md".into()));
    }
    let decoded = percent_encoding::percent_decode_str(href)
        .decode_utf8()
        .map_err(|_| (StatusCode::UNPROCESSABLE_ENTITY, "link non valido".into()))?;
    Ok(dst.join(decoded.as_ref()))
}

/// Apre la cartella di destinazione o `index.md` dell'ultima esecuzione (REQ-033). Il percorso
/// non arriva dal browser: una pagina non può far aprire al server file arbitrari.
async fn open_result(
    State(state): State<Arc<AppState>>,
    Json(request): Json<OpenRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let snapshot = state.runner.snapshot();
    let dst = match (&snapshot.dst, snapshot.running) {
        (Some(dst), false) => PathBuf::from(dst),
        _ => return Err((StatusCode::CONFLICT, "nessuna conversione terminata".into())),
    };
    let opening = match request.target {
        OpenTarget::Destination => Opening::Folder(dst),
        OpenTarget::Index => Opening::File(dst.join("index.md")),
        OpenTarget::Link => Opening::File(index_link(&dst, request.href.as_deref().unwrap_or_default())?),
    };
    let exists = match &opening {
        Opening::Folder(path) => path.is_dir(),
        Opening::File(path) => path.is_file(),
    };
    if !exists {
        return Err((StatusCode::NOT_FOUND, "il risultato non esiste più o non è stato generato".into()));
    }
    (state.opener)(opening).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(StatusCode::NO_CONTENT)
}

/// Ultime cartelle usate (REQ-035).
async fn get_settings(State(state): State<Arc<AppState>>) -> Json<Settings> {
    Json(state.settings_path.as_deref().map(settings::load).unwrap_or_default())
}

/// Interrompe l'esecuzione in corso (REQ-034).
async fn doc_ripper_cancel(State(state): State<Arc<AppState>>) -> Result<StatusCode, (StatusCode, String)> {
    state.runner.cancel().map_err(|e| (StatusCode::CONFLICT, e))?;
    Ok(StatusCode::ACCEPTED)
}

#[derive(Deserialize)]
struct PickRequest {
    title: String,
    initial: Option<String>,
}

#[derive(Serialize)]
struct PickResponse {
    path: Option<String>,
}

/// Apre il selettore di cartelle di Windows dal server (REQ-030): il browser non può fornire
/// il percorso completo di una cartella.
async fn pick_folder(
    State(state): State<Arc<AppState>>,
    Json(request): Json<PickRequest>,
) -> Result<Json<PickResponse>, (StatusCode, String)> {
    if state.picking.swap(true, Ordering::SeqCst) {
        return Err((StatusCode::CONFLICT, "un selettore di cartelle è già aperto".into()));
    }
    let picker = Arc::clone(&state.folder_picker);
    let initial = request.initial.filter(|s| !s.trim().is_empty()).map(PathBuf::from);
    let chosen = tokio::task::spawn_blocking(move || picker(&request.title, initial)).await;
    state.picking.store(false, Ordering::SeqCst);
    let chosen = chosen.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(PickResponse { path: chosen.map(|p| p.display().to_string()) }))
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

    fn state(folder_picker: FolderPicker) -> AppState {
        AppState {
            port: 4000,
            doc_ripper: None,
            search_rm3: None,
            runner: Arc::default(),
            shutdown: Arc::default(),
            folder_picker,
            picking: AtomicBool::new(false),
            settings_path: None,
            opener: Arc::new(|_| Ok(())),
        }
    }

    fn app() -> Router {
        router(state(Arc::new(|_, _| None)))
    }

    async fn pick(app: Router, body: &str) -> (StatusCode, String) {
        let request = Request::builder()
            .method("POST")
            .uri("/api/pick-folder")
            .header(header::HOST, "127.0.0.1:4000")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, String::from_utf8_lossy(&body).into_owned())
    }

    async fn open(app: Router, target: &str) -> StatusCode {
        let request = Request::builder()
            .method("POST")
            .uri("/api/open")
            .header(header::HOST, "127.0.0.1:4000")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(format!(r#"{{"target":"{target}"}}"#)))
            .unwrap();
        app.oneshot(request).await.unwrap().status()
    }

    #[tokio::test]
    async fn apre_destinazione_e_indice() {
        let dst = tempfile::tempdir().unwrap();
        let opened = Arc::new(std::sync::Mutex::new(Vec::new()));
        let record = Arc::clone(&opened);
        let app = |runner: Runner| {
            let record = Arc::clone(&record);
            let mut s = state(Arc::new(|_, _| None));
            s.runner = Arc::new(runner);
            s.opener = Arc::new(move |o| {
                record.lock().unwrap().push(o);
                Ok(())
            });
            router(s)
        };
        let finished = || Runner::finished(dst.path().to_str().unwrap());

        assert_eq!(open(app(Runner::default()), "destination").await, StatusCode::CONFLICT);
        assert_eq!(open(app(finished()), "destination").await, StatusCode::NO_CONTENT);
        assert_eq!(open(app(finished()), "index").await, StatusCode::NOT_FOUND, "index.md non ancora generato");
        std::fs::write(dst.path().join("index.md"), "# Indice").unwrap();
        assert_eq!(open(app(finished()), "index").await, StatusCode::NO_CONTENT);
        assert_eq!(open(app(finished()), "C:/Windows/system32").await, StatusCode::UNPROCESSABLE_ENTITY);

        assert_eq!(
            *opened.lock().unwrap(),
            vec![Opening::Folder(dst.path().to_path_buf()), Opening::File(dst.path().join("index.md"))]
        );
    }

    #[tokio::test]
    async fn apre_solo_i_link_dell_indice() {
        let dst = tempfile::tempdir().unwrap();
        let original = tempfile::tempdir().unwrap();
        std::fs::create_dir(dst.path().join("conti 2026")).unwrap();
        std::fs::write(dst.path().join("conti 2026/a_pdf.md"), "").unwrap();
        std::fs::write(original.path().join("a b.pdf"), "").unwrap();
        let original_link = format!("{}/a%20b.pdf", original.path().display().to_string().replace('\\', "/"));
        std::fs::write(
            dst.path().join("index.md"),
            format!("| [a\\_pdf.md](conti%202026/a_pdf.md) | [a b.pdf]({original_link}) |\n| [x](manca.md) | |\n"),
        )
        .unwrap();
        let opened = Arc::new(std::sync::Mutex::new(Vec::new()));
        let record = Arc::clone(&opened);
        let app = || {
            let record = Arc::clone(&record);
            let mut s = state(Arc::new(|_, _| None));
            s.runner = Arc::new(Runner::finished(dst.path().to_str().unwrap()));
            s.opener = Arc::new(move |o| {
                record.lock().unwrap().push(o);
                Ok(())
            });
            router(s)
        };
        let link = |href: &str| {
            let body = serde_json::json!({ "target": "link", "href": href }).to_string();
            let app = app();
            async move {
                let request = Request::builder()
                    .method("POST")
                    .uri("/api/open")
                    .header(header::HOST, "127.0.0.1:4000")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(body))
                    .unwrap();
                app.oneshot(request).await.unwrap().status()
            }
        };

        assert_eq!(link("conti%202026/a_pdf.md").await, StatusCode::NO_CONTENT);
        assert_eq!(link(&original_link).await, StatusCode::NO_CONTENT);
        assert_eq!(link("manca.md").await, StatusCode::NOT_FOUND, "nell'indice ma inesistente");
        assert_eq!(link("C:/Windows/notepad.exe").await, StatusCode::FORBIDDEN, "non presente nell'indice");
        assert_eq!(link("").await, StatusCode::FORBIDDEN);

        assert_eq!(
            *opened.lock().unwrap(),
            vec![
                Opening::File(dst.path().join("conti 2026/a_pdf.md")),
                Opening::File(PathBuf::from(format!("{}/a b.pdf", original.path().display().to_string().replace('\\', "/")))),
            ]
        );
    }

    #[tokio::test]
    async fn testo_dell_indice() {
        let dst = tempfile::tempdir().unwrap();
        let app = || {
            let mut s = state(Arc::new(|_, _| None));
            s.runner = Arc::new(Runner::finished(dst.path().to_str().unwrap()));
            router(s)
        };
        let fetch = |app: Router| async move {
            let request = Request::builder()
                .uri("/api/doc-ripper/index")
                .header(header::HOST, "127.0.0.1:4000")
                .body(Body::empty())
                .unwrap();
            let response = app.oneshot(request).await.unwrap();
            let status = response.status();
            let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
            (status, String::from_utf8_lossy(&body).into_owned())
        };
        assert_eq!(fetch(app()).await.0, StatusCode::NOT_FOUND);
        std::fs::write(dst.path().join("index.md"), "# Indice").unwrap();
        assert_eq!(fetch(app()).await, (StatusCode::OK, "# Indice".to_string()));
        assert_eq!(fetch(router(state(Arc::new(|_, _| None)))).await.0, StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn selettore_di_cartelle() {
        let seen = Arc::new(std::sync::Mutex::new(None));
        let record = Arc::clone(&seen);
        let picker: FolderPicker = Arc::new(move |title, initial| {
            *record.lock().unwrap() = Some((title.to_string(), initial));
            Some(PathBuf::from(r"C:\Dati\Città"))
        });
        let (status, body) =
            pick(router(state(picker)), r#"{"title":"Cartella di origine","initial":"C:\\Prima"}"#).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, r#"{"path":"C:\\Dati\\Città"}"#);
        assert_eq!(
            *seen.lock().unwrap(),
            Some(("Cartella di origine".to_string(), Some(PathBuf::from(r"C:\Prima"))))
        );

        let (status, body) = pick(app(), r#"{"title":"x","initial":""}"#).await;
        assert_eq!((status, body.as_str()), (StatusCode::OK, r#"{"path":null}"#));
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
