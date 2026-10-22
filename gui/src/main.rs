// gui usa il selettore di cartelle, Esplora file e i percorsi di Windows (REQ-025).
#[cfg(not(windows))]
compile_error!("gui supporta solo Windows 10 e Windows 11");

mod server;

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
    let _args = Args::parse();
    // Solo l'interfaccia di loopback: la gui non è raggiungibile da altri computer (REQ-036).
    let listener = match TcpListener::bind(("127.0.0.1", 0)).await {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("Errore: impossibile avviare il server: {error}");
            std::process::exit(1);
        }
    };
    let port = listener.local_addr().unwrap().port();
    println!("gui in ascolto su http://127.0.0.1:{port}/");

    if let Err(error) = axum::serve(listener, server::router(port)).await {
        eprintln!("Errore del server: {error}");
        std::process::exit(1);
    }
}
