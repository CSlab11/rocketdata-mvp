use axum::Router;
use axum::extract::{Request, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::routing::get;

/// Applicazione HTTP della gui, per un server in ascolto sulla porta `port` di 127.0.0.1.
pub fn router(port: u16) -> Router {
    Router::new()
        .route("/", get(|| async { "gui" }))
        .layer(middleware::from_fn_with_state(port, local_only))
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
    use axum::body::Body;
    use tower::ServiceExt;

    async fn status(host: &str, origin: Option<&str>) -> StatusCode {
        let mut request = Request::builder().uri("/").header(header::HOST, host);
        if let Some(origin) = origin {
            request = request.header(header::ORIGIN, origin);
        }
        router(4000).oneshot(request.body(Body::empty()).unwrap()).await.unwrap().status()
    }

    #[tokio::test]
    async fn accetta_solo_localhost() {
        assert_eq!(status("127.0.0.1:4000", None).await, StatusCode::OK);
        assert_eq!(status("localhost:4000", Some("http://localhost:4000")).await, StatusCode::OK);
        assert_eq!(status("evil.example:4000", None).await, StatusCode::FORBIDDEN);
        assert_eq!(status("127.0.0.1:5000", None).await, StatusCode::FORBIDDEN);
        assert_eq!(status("127.0.0.1:4000", Some("http://evil.example")).await, StatusCode::FORBIDDEN);
    }
}
