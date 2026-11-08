//! Finto `python.exe` di `docling\` per i test di doc-ripper. Riceve
//! `-I convert_pdf.py --pdf <file.pdf> --out <cartella>` come il Python vero.
//!
//! - `FAKE_DOCLING=errore`: esce con codice 1 e un messaggio su stderr, come lo script quando Docling fallisce.
//! - altrimenti scrive `<cartella>\documento.json` con un titolo e un paragrafo.
//! - `FAKE_DOCLING_LOG=<file>`: aggiunge al file una riga con il PDF ricevuto.

use std::io::Write;
use std::path::PathBuf;

const DOCUMENT: &str = r##"{
  "body": {"children": [{"$ref": "#/texts/0"}, {"$ref": "#/texts/1"}]},
  "texts": [
    {"label": "section_header", "level": 1, "text": "Titolo da Docling", "prov": [{"page_no": 1}]},
    {"label": "text", "text": "Paragrafo da Docling.", "prov": [{"page_no": 1}]}
  ]
}"##;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let value = |flag: &str| args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).cloned().unwrap();
    let pdf = value("--pdf");
    let out = PathBuf::from(value("--out"));
    if let Ok(log) = std::env::var("FAKE_DOCLING_LOG") {
        let mut file = std::fs::OpenOptions::new().create(true).append(true).open(log).unwrap();
        writeln!(file, "{pdf}").unwrap();
    }
    if std::env::var("FAKE_DOCLING").as_deref() == Ok("errore") {
        eprintln!("avviso qualsiasi");
        eprintln!("RuntimeError: modello di layout non trovato");
        std::process::exit(1);
    }
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("documento.json"), DOCUMENT).unwrap();
}
