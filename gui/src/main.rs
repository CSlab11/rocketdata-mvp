// gui usa il selettore di cartelle, Esplora file e i percorsi di Windows (REQ-025).
#[cfg(not(windows))]
compile_error!("gui supporta solo Windows 10 e Windows 11");

mod runner;
mod server;
mod summary;

use std::path::PathBuf;

use clap::Parser;
use tokio::net::TcpListener;

/// Interfaccia web locale dei tool RocketData.
#[derive(Parser)]
#[command(version)]
struct Args {
    /// Non aprire il browser all'avvio
    #[arg(long)]
    no_browser: bool,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    // Solo l'interfaccia di loopback: la gui non è raggiungibile da altri computer (REQ-036).
    let listener = match TcpListener::bind(("127.0.0.1", 0)).await {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("Errore: impossibile avviare il server: {error}");
            std::process::exit(1);
        }
    };
    let port = listener.local_addr().unwrap().port();
    let url = format!("http://127.0.0.1:{port}/");
    println!("gui in ascolto su {url}");

    // Apre l'interfaccia nel browser predefinito (REQ-026).
    if !args.no_browser {
        if let Err(error) = open::that_detached(&url) {
            eprintln!("Impossibile aprire il browser ({error}): aprire {url} manualmente.");
        }
    }

    let state = server::AppState { port, doc_ripper: find_doc_ripper(), runner: Default::default() };
    if let Err(error) = axum::serve(listener, server::router(state)).await {
        eprintln!("Errore del server: {error}");
        std::process::exit(1);
    }
}

/// `doc-ripper.exe` nella cartella di `gui.exe`, se presente (REQ-029).
fn find_doc_ripper() -> Option<PathBuf> {
    let path = std::env::current_exe().ok()?.parent()?.join("doc-ripper.exe");
    path.is_file().then_some(path)
}
