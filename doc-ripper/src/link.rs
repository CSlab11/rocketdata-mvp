use std::path::{Path, PathBuf};

use url::Url;

use crate::docx::escape;

/// URL `file://` assoluto e codificato (spazi, accenti, `#`…) del file.
pub fn file_url(path: &Path) -> String {
    let path = without_verbatim_prefix(path);
    Url::from_file_path(&path).map_or_else(|_| path.display().to_string(), String::from)
}

/// Link navigabile al file originale: percorso assoluto codificato come `file_url`, senza lo
/// schema `file://` (es. `C:/dati/a%20b.pdf`, `//server/condivisa/x.pdf`) (REQ-045).
pub fn file_link(path: &Path) -> String {
    let url = file_url(path);
    url.strip_prefix("file:///").or_else(|| url.strip_prefix("file:")).unwrap_or(&url).to_string()
}

/// Riga da mettere in testa al Markdown con il link al file originale.
pub fn origin_line(original: &Path) -> String {
    let name = original.file_name().unwrap_or_default().to_string_lossy();
    format!("Origine: [{}]({})", escape(&name), file_link(original))
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
    fn link_senza_schema() {
        assert_eq!(
            file_link(Path::new(r"\\?\C:\Dati utente\Città\report #1.pdf")),
            "C:/Dati%20utente/Citt%C3%A0/report%20%231.pdf"
        );
        assert_eq!(file_link(Path::new(r"\\?\UNC\server\condivisa\a b.docx")), "//server/condivisa/a%20b.docx");
    }

    #[test]
    fn riga_di_origine() {
        assert_eq!(
            origin_line(Path::new(r"C:\dati\note_1.pdf")),
            "Origine: [note\\_1.pdf](C:/dati/note_1.pdf)"
        );
    }
}
