// gui usa il selettore di cartelle, Esplora file e i percorsi di Windows (REQ-025).
#[cfg(not(windows))]
compile_error!("gui supporta solo Windows 10 e Windows 11");

fn main() {
    println!("Hello, world!");
}
