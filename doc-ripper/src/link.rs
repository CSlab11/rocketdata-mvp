use std::path::{Path, PathBuf};

use url::Url;

use crate::docx::escape;

/// URL `file://` assoluto e codificato (spazi, accenti, `#`…) del file originale (REQ-014).
pub fn file_url(path: &Path) -> String {
    let path = without_verbatim_prefix(path);
    Url::from_file_path(&path).map_or_else(|_| path.display().to_string(), String::from)
}

/// Riga da mettere in testa al Markdown con il link al file originale.
pub fn origin_line(original: &Path) -> String {
    let name = original.file_name().unwrap_or_default().to_string_lossy();
    format!("Origine: [{}]({})", escape(&name), file_url(original))
}

/// Toglie il prefisso `\\?\` che `canonicalize` aggiunge su Windows: gli URL non lo prevedono.
fn without_verbatim_prefix(path: &Path) -> PathBuf {
    let text = path.to_string_lossy();
    if let Some(unc) = text.strip_prefix(r"\\?\UNC\") {
        PathBuf::from(format!(r"\\{unc}"))
    } else if let Some(local) = text.strip_prefix(r"\\?\") {
        PathBuf::from(local)
    } else {
        path.to_path_buf()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_assoluto_codificato() {
        assert_eq!(
            file_url(Path::new(r"\\?\C:\Dati utente\Città\report #1.pdf")),
            "file:///C:/Dati%20utente/Citt%C3%A0/report%20%231.pdf"
        );
        assert_eq!(file_url(Path::new(r"C:\dati\report.pdf")), "file:///C:/dati/report.pdf");
        assert_eq!(file_url(Path::new(r"\\?\UNC\server\condivisa\a b.docx")), "file://server/condivisa/a%20b.docx");
    }

    #[test]
    fn riga_di_origine() {
        assert_eq!(
            origin_line(Path::new(r"C:\dati\note_1.pdf")),
            "Origine: [note\\_1.pdf](file:///C:/dati/note_1.pdf)"
        );
    }
}
