use std::ffi::OsStr;
use std::fs;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use tokio::process::Command;
use windows_sys::Win32::Storage::FileSystem::{FILE_ATTRIBUTE_HIDDEN, SetFileAttributesW};

/// Il processo non ha una console propria: niente finestre che si aprono.
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Cartella dell'indice di `src`: la sottocartella `.index`, creata se manca e mai nascosta
/// (REQ-075).
pub fn index_dir(src: &Path) -> Result<PathBuf, String> {
    if !src.is_dir() {
        return Err(format!("la cartella da indicizzare non esiste: {}", src.display()));
    }
    let dir = src.join(".index");
    match fs::create_dir(&dir) {
        Err(e) if e.kind() != std::io::ErrorKind::AlreadyExists => {
            return Err(format!("impossibile creare {}: {e}", dir.display()));
        }
        _ => {}
    }
    let attributes = fs::metadata(&dir).map_err(|e| format!("impossibile leggere {}: {e}", dir.display()))?.file_attributes();
    if attributes & FILE_ATTRIBUTE_HIDDEN != 0 {
        let wide: Vec<u16> = dir.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
        if unsafe { SetFileAttributesW(wide.as_ptr(), attributes & !FILE_ATTRIBUTE_HIDDEN) } == 0 {
            return Err(format!("impossibile rendere visibile {}", dir.display()));
        }
    }
    Ok(dir)
}

/// `search-rm3 index --src <src> --index <src>\.index` (REQ-069, REQ-075).
pub async fn index(exe: &Path, src: &Path) -> Result<(), String> {
    let dir = index_dir(src)?;
    run(exe, &["index".as_ref(), "--src".as_ref(), src.as_os_str(), "--index".as_ref(), dir.as_os_str()]).await?;
    Ok(())
}

/// `search-rm3 query --index <src>\.index --q <q>`: il JSON scritto su stdout (REQ-069, REQ-075).
pub async fn query(exe: &Path, src: &Path, q: &str) -> Result<serde_json::Value, String> {
    let dir = index_dir(src)?;
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
