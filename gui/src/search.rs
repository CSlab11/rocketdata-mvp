use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use tokio::process::Command;

/// Il processo non ha una console propria: niente finestre che si aprono.
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Cartella dell'indice di `src`.
pub fn index_dir(src: &Path) -> PathBuf {
    src.join(".index")
}

/// `search-rm3 index --src <src> --index <src>\.index` (REQ-069).
pub async fn index(exe: &Path, src: &Path) -> Result<(), String> {
    let dir = index_dir(src);
    run(exe, &["index".as_ref(), "--src".as_ref(), src.as_os_str(), "--index".as_ref(), dir.as_os_str()]).await?;
    Ok(())
}

/// `search-rm3 query --index <src>\.index --q <q>`: il JSON scritto su stdout (REQ-069).
pub async fn query(exe: &Path, src: &Path, q: &str) -> Result<serde_json::Value, String> {
    let dir = index_dir(src);
    let stdout = run(exe, &["query".as_ref(), "--index".as_ref(), dir.as_os_str(), "--q".as_ref(), q.as_ref()]).await?;
    serde_json::from_str(&stdout).map_err(|e| format!("risposta di search-rm3 non valida: {e}"))
}

/// Esegue search-rm3 come processo separato: stdout se il codice di uscita è 0, altrimenti il
/// messaggio di errore scritto su stderr.
async fn run(exe: &Path, args: &[&OsStr]) -> Result<String, String> {
    let output = Command::new(exe)
        .args(args)
        .stdin(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .kill_on_drop(true)
        .output()
        .await
        .map_err(|e| format!("impossibile avviare search-rm3: {e}"))?;
    if output.status.success() {
        return String::from_utf8(output.stdout).map_err(|_| "risposta di search-rm3 non in UTF-8".to_string());
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Err(match output.status.code() {
        _ if !stderr.is_empty() => stderr,
        Some(code) => format!("search-rm3 terminato con codice di uscita {code}"),
        None => "search-rm3 terminato senza codice di uscita".to_string(),
    })
}
