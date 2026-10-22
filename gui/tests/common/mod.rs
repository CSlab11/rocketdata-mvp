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

/// gui.exe in una cartella temporanea con il doc-ripper finto (examples/fake_doc_ripper.rs)
/// copiato accanto come doc-ripper.exe.
#[allow(dead_code)]
pub fn install_with_fake() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let exe = dir.path().join("gui.exe");
    std::fs::copy(gui_exe(), &exe).unwrap();
    let fake = gui_exe().parent().unwrap().join("examples").join("fake_doc_ripper.exe");
    std::fs::copy(&fake, dir.path().join("doc-ripper.exe")).unwrap();
    (dir, exe)
}

/// Avvia doc-ripper dalla gui; restituisce lo stato HTTP della risposta.
#[allow(dead_code)]
pub fn run(gui: &Gui, src: &str, dst: &str) -> u16 {
    let body = format!(r#"{{"src":{},"dst":{}}}"#, json_string(src), json_string(dst));
    let agent = ureq::Agent::config_builder().http_status_as_error(false).build().new_agent();
    agent
        .post(&gui.url("/api/doc-ripper/run"))
        .header("Content-Type", "application/json")
        .send(body.as_str())
        .unwrap()
        .status()
        .as_u16()
}

/// Attende la fine dell'esecuzione e restituisce lo stato finale.
#[allow(dead_code)]
pub fn wait_output(gui: &Gui) -> serde_json::Value {
    for _ in 0..100 {
        let body = ureq::get(&gui.url("/api/doc-ripper/output")).call().unwrap().into_body().read_to_string().unwrap();
        let snapshot: serde_json::Value = serde_json::from_str(&body).unwrap();
        if snapshot["running"] == false {
            return snapshot;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    panic!("doc-ripper non è terminato");
}

fn json_string(s: &str) -> String {
    serde_json::Value::String(s.to_string()).to_string()
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
