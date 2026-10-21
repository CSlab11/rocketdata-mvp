use std::path::Path;

use crate::convert::base_name;
use crate::output::write_file;

/// Immagini estratte da un documento, scritte in `<nome>_<ext>_img/` accanto al `.md` (REQ-020).
pub struct Images {
    dir: String,
    files: Vec<(String, Vec<u8>)>,
}

impl Images {
    pub fn new(rel: &Path) -> Self {
        Images { dir: format!("{}_img", base_name(rel)), files: Vec::new() }
    }

    /// Registra un'immagine e restituisce il riferimento Markdown, relativo al `.md`.
    /// Un nome già registrato non viene duplicato.
    pub fn add(&mut self, name: &str, bytes: &[u8]) -> String {
        if !self.files.iter().any(|(n, _)| n == name) {
            self.files.push((name.to_string(), bytes.to_vec()));
        }
        format!("![{name}]({}/{})", encode(&self.dir), encode(name))
    }

    /// Scrive le immagini nella cartella accanto a `md_path`.
    pub fn write(&self, md_path: &Path) -> Result<(), String> {
        let dir = md_path.with_file_name(&self.dir);
        for (name, bytes) in &self.files {
            write_file(&dir.join(name), bytes)?;
        }
        Ok(())
    }
}

/// Codifica i caratteri che interromperebbero un link Markdown.
fn encode(segment: &str) -> String {
    segment.replace('%', "%25").replace(' ', "%20").replace('(', "%28").replace(')', "%29")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn riferimento_relativo_e_codificato() {
        let mut images = Images::new(Path::new("a/Il report.docx"));
        assert_eq!(images.add("image1.png", b"x"), "![image1.png](Il%20report_docx_img/image1.png)");
        images.add("image1.png", b"x");
        assert_eq!(images.files.len(), 1);
    }
}
