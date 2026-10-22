mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use common::{install_with_fake, plain_canonical, running_from, wait_running};

/// Esegue uno script della gui con %LOCALAPPDATA% e %APPDATA% di prova; restituisce l'output.
fn script(dir: &Path, name: &str, args: &[&str], local: &Path) -> String {
    let out = Command::new("cmd")
        .arg("/c")
        .arg(dir.join(name))
        .args(args)
        .env("LOCALAPPDATA", local)
        .env("APPDATA", local)
        .output()
        .unwrap();
    assert!(out.status.success(), "{name}: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn listening_port(pid: u32) -> u16 {
    let script = format!("(Get-NetTCPConnection -OwningProcess {pid} -State Listen | Select-Object -First 1).LocalPort");
    for _ in 0..30 {
        let out = Command::new("powershell").args(["-NoProfile", "-Command", &script]).output().unwrap();
        if let Ok(port) = String::from_utf8_lossy(&out.stdout).trim().parse() {
            return port;
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    panic!("la gui non è in ascolto");
}

#[test]
fn avvio_e_arresto_con_gli_script() {
    let (dir, _) = install_with_fake();
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for name in ["start.cmd", "stop.cmd"] {
        fs::copy(manifest.join(name), dir.path().join(name)).unwrap();
    }
    let local = tempfile::tempdir().unwrap();
    let pid_file = local.path().join("rocketdata-gui/gui.pid");
    let gui = plain_canonical(dir.path()).join("gui.exe");
    let doc_ripper = plain_canonical(dir.path()).join("doc-ripper.exe");

    let started = script(dir.path(), "start.cmd", &["--no-browser"], local.path());
    let pid: u32 = started
        .strip_prefix("gui avviato (PID ")
        .and_then(|s| s.strip_suffix(")."))
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| panic!("{started}"));
    assert!(pid_file.is_file());
    assert!(wait_running(&gui, 1));

    let again = script(dir.path(), "start.cmd", &["--no-browser"], local.path());
    assert_eq!(again, format!("gui e' gia' in esecuzione (PID {pid})."));
    assert_eq!(running_from(&gui), 1, "avviata una seconda gui");

    // Un doc-ripper in corso viene terminato insieme al server.
    let src = tempfile::tempdir().unwrap();
    fs::write(src.path().join("lento"), "").unwrap();
    let port = listening_port(pid);
    let body = serde_json::json!({ "src": src.path(), "dst": "C:\\dst" }).to_string();
    let status = ureq::post(&format!("http://127.0.0.1:{port}/api/doc-ripper/run"))
        .header("Content-Type", "application/json")
        .send(body.as_str())
        .unwrap()
        .status();
    assert_eq!(status, 202);
    assert!(wait_running(&doc_ripper, 1));

    let stopped = script(dir.path(), "stop.cmd", &[], local.path());
    assert_eq!(stopped, format!("gui arrestato (PID {pid})."));
    assert!(wait_running(&gui, 0));
    assert!(wait_running(&doc_ripper, 0));
    assert!(!pid_file.exists());

    assert_eq!(script(dir.path(), "stop.cmd", &[], local.path()), "gui non risulta in esecuzione.");
}
