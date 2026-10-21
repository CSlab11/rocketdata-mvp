use std::fs;
use std::path::Path;

/// Scrive un file generato, creando le cartelle mancanti.
/// Un file già presente con lo stesso nome viene sovrascritto (REQ-016).
pub fn write_file(path: &Path, contents: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    fs::write(path, contents).map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sovrascrive_un_file_esistente() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sotto/report_pdf.md");
        write_file(&path, b"versione vecchia, piu' lunga").unwrap();
        write_file(&path, b"nuova").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"nuova");
    }
}
