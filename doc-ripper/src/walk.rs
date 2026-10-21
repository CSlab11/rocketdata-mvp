use std::path::{Path, PathBuf};

use walkdir::WalkDir;

/// Contenuto della cartella di origine, con percorsi relativi a essa.
#[derive(Debug, Default)]
pub struct Tree {
    pub dirs: Vec<PathBuf>,
    pub files: Vec<PathBuf>,
    /// Voci che non è stato possibile leggere, con l'errore.
    pub errors: Vec<(PathBuf, String)>,
}

/// Elenca ricorsivamente `src` (REQ-015). Se `dst` è una sottocartella di `src`, viene esclusa,
/// così i file generati da esecuzioni precedenti non vengono rielaborati.
/// Entrambi i percorsi devono essere canonici.
pub fn scan(src: &Path, dst: &Path) -> Tree {
    let mut tree = Tree::default();
    let skip_dst = dst != src && dst.starts_with(src);
    let walker = WalkDir::new(src).min_depth(1).sort_by_file_name().into_iter();
    for entry in walker.filter_entry(|e| !(skip_dst && e.path() == dst)) {
        match entry {
            Ok(entry) => {
                let rel = entry.path().strip_prefix(src).unwrap().to_path_buf();
                if entry.file_type().is_dir() {
                    tree.dirs.push(rel);
                } else if entry.file_type().is_file() {
                    tree.files.push(rel);
                }
            }
            Err(error) => {
                let path = error.path().unwrap_or(src);
                let rel = path.strip_prefix(src).unwrap_or(path).to_path_buf();
                tree.errors.push((rel, error.to_string()));
            }
        }
    }
    tree
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn elenca_file_e_cartelle_ricorsivamente() {
        let root = tempfile::tempdir().unwrap();
        let src = root.path().canonicalize().unwrap();
        fs::create_dir_all(src.join("a/b")).unwrap();
        fs::write(src.join("uno.pdf"), "").unwrap();
        fs::write(src.join("a/b/due.docx"), "").unwrap();
        let dst = root.path().parent().unwrap().to_path_buf();

        let tree = scan(&src, &dst);
        assert_eq!(tree.dirs, vec![PathBuf::from("a"), PathBuf::from("a/b")]);
        assert_eq!(tree.files, vec![PathBuf::from("a/b/due.docx"), PathBuf::from("uno.pdf")]);
        assert!(tree.errors.is_empty());
    }

    #[test]
    fn esclude_la_destinazione_dentro_l_origine() {
        let root = tempfile::tempdir().unwrap();
        let src = root.path().canonicalize().unwrap();
        fs::create_dir_all(src.join("out")).unwrap();
        fs::write(src.join("out/vecchio_pdf.md"), "").unwrap();
        fs::write(src.join("uno.pdf"), "").unwrap();

        let tree = scan(&src, &src.join("out"));
        assert!(tree.dirs.is_empty());
        assert_eq!(tree.files, vec![PathBuf::from("uno.pdf")]);
    }
}
