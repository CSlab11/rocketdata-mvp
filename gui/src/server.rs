use axum::Router;
use axum::extract::{Request, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;

/// Interfaccia web incorporata nell'eseguibile (REQ-026).
const INDEX_HTML: &str = include_str!("../web/index.html");
const STYLE_CSS: &str = include_str!("../web/style.css");
const APP_JS: &str = include_str!("../web/app.js");

/// Applicazione HTTP della gui, per un server in ascolto sulla porta `port` di 127.0.0.1.
pub fn router(port: u16) -> Router {
    Router::new()
        .route("/", get(|| async { Html(INDEX_HTML) }))
        .route("/style.css", get(|| async { asset("text/css; charset=utf-8", STYLE_CSS) }))
        .route("/app.js", get(|| async { asset("text/javascript; charset=utf-8", APP_JS) }))
        .layer(middleware::from_fn_with_state(port, local_only))
}

fn asset(content_type: &'static str, body: &'static str) -> impl IntoResponse {
    ([(header::CONTENT_TYPE, content_type)], body)
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

    async fn get(path: &str, host: &str, origin: Option<&str>) -> Response {
        let mut request = Request::builder().uri(path).header(header::HOST, host);
        if let Some(origin) = origin {
            request = request.header(header::ORIGIN, origin);
        }
        router(4000).oneshot(request.body(Body::empty()).unwrap()).await.unwrap()
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
}
