use std::path::Path;

use url::Url;

use crate::docx::escape;
use crate::link::{file_link, file_url};
use crate::output::write_file;
use crate::report::Report;

/// Scrive `index.md` nella radice di `dst`: ogni file generato con il link relativo
/// e il link all'originale come percorso assoluto senza `file://` (REQ-046).
pub fn write_index(src: &Path, dst: &Path, report: &Report) -> Result<(), String> {
    let base = Url::parse(&file_url(dst)).map_err(|e| e.to_string())?;
    let base = Url::parse(&format!("{}/", base.as_str().trim_end_matches('/'))).map_err(|e| e.to_string())?;

    let mut out = String::from("# Indice dei file generati\n\n| File generato | Originale |\n|---|---|\n");
    for (original, outputs) in &report.converted {
        let original_link = file_link(&src.join(original));
        for output in outputs {
            let target = Url::parse(&file_url(&dst.join(output))).map_err(|e| e.to_string())?;
            let relative = base.make_relative(&target).unwrap_or_else(|| target.to_string());
            out.push_str(&format!(
                "| [{}]({relative}) | [{}]({original_link}) |\n",
                cell(output),
                cell(original),
            ));
        }
    }
    write_file(&dst.join("index.md"), out.as_bytes())
}

fn cell(path: &Path) -> String {
    let text = path.to_string_lossy().replace('\\', "/");
    escape(&text).replace('|', "\\|")
}
