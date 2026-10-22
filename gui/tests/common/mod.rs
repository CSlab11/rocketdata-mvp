use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Child, Command, Stdio};

/// gui.exe avviato per un test; il processo viene terminato alla fine.
pub struct Gui {
    pub child: Child,
    pub port: u16,
}

impl Gui {
    /// Avvia `exe` senza browser e attende la riga con l'indirizzo del server.
    pub fn start(exe: &Path) -> Gui {
        let mut child = Command::new(exe)
            .arg("--no-browser")
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let mut line = String::new();
        BufReader::new(child.stdout.as_mut().unwrap()).read_line(&mut line).unwrap();
        let port = line
            .trim()
            .trim_end_matches('/')
            .rsplit(':')
            .next()
            .and_then(|p| p.parse().ok())
            .unwrap_or_else(|| panic!("indirizzo non trovato in {line:?}"));
        Gui { child, port }
    }

    pub fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{path}", self.port)
    }
}

impl Drop for Gui {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub fn gui_exe() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_gui"))
}
