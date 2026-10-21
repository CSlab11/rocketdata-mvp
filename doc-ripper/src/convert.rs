use std::path::Path;

use crate::report::SkipReason;
use crate::run::Outcome;

/// Tipi di file che doc-ripper sa convertire.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    Pdf,
    Docx,
    Spreadsheet,
}

impl Kind {
    pub fn of(path: &Path) -> Option<Kind> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        match ext.as_str() {
            "pdf" => Some(Kind::Pdf),
            "docx" => Some(Kind::Docx),
            "xlsx" | "xls" | "ods" => Some(Kind::Spreadsheet),
            _ => None,
        }
    }
}

/// Converte il file `rel` (relativo a `src`) scrivendo i risultati sotto `dst`.
/// I file di tipo non supportato non vengono elaborati né copiati (REQ-019).
pub fn convert(_src: &Path, _dst: &Path, rel: &Path) -> Result<Outcome, String> {
    match Kind::of(rel) {
        None => Ok(Outcome::skipped(SkipReason::Unsupported)),
        Some(_) => Ok(Outcome::default()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn riconosce_le_estensioni() {
        assert_eq!(Kind::of(Path::new("a/report.PDF")), Some(Kind::Pdf));
        assert_eq!(Kind::of(Path::new("report.docx")), Some(Kind::Docx));
        assert_eq!(Kind::of(Path::new("vendite.xlsx")), Some(Kind::Spreadsheet));
        assert_eq!(Kind::of(Path::new("vendite.xls")), Some(Kind::Spreadsheet));
        assert_eq!(Kind::of(Path::new("vendite.ods")), Some(Kind::Spreadsheet));
        assert_eq!(Kind::of(Path::new("note.txt")), None);
        assert_eq!(Kind::of(Path::new("vecchio.doc")), None);
        assert_eq!(Kind::of(Path::new("senza_estensione")), None);
    }
}
