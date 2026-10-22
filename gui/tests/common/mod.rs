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

/// DOM della pagina dopo l'esecuzione degli script, ottenuto con Edge headless
/// (presente in Windows 10 e 11).
#[allow(dead_code)]
pub fn dump_dom(url: &str) -> String {
    let edge = [r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe", r"C:\Program Files\Microsoft\Edge\Application\msedge.exe"]
        .into_iter()
        .find(|p| Path::new(p).is_file())
        .expect("Microsoft Edge non trovato");
    let profile = tempfile::tempdir().unwrap();
    let output = Command::new(edge)
        .args(["--headless", "--disable-gpu", "--no-first-run", "--virtual-time-budget=3000"])
        .arg(format!("--user-data-dir={}", profile.path().display()))
        .args(["--dump-dom", url])
        .output()
        .unwrap();
    String::from_utf8_lossy(&output.stdout).into_owned()
}
