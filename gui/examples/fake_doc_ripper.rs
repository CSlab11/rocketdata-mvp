//! doc-ripper finto per i test di gui: viene copiato come `doc-ripper.exe` accanto a `gui.exe`.
//! Il comportamento dipende da file marcatore nella cartella di origine:
//! `lento` → righe distanziate di 300 ms, `fallisci` → codice di uscita 1.
//! Prima del riepilogo scrive le righe di avanzamento per file e per pagina (REQ-043).

use std::io::Write;
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let value = |flag: &str| args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).cloned();
    let (Some(src), Some(dst)) = (value("--src"), value("--dst")) else {
        eprintln!("uso: doc-ripper --src <ORIGINE> --dst <DESTINAZIONE>");
        std::process::exit(2);
    };
    let src = PathBuf::from(src);
    eprintln!("argomenti: --src {} --dst {}", src.display(), dst);

    let slow = src.join("lento").exists();
    let fail = src.join("fallisci").exists();
    let lines = [
        "[1/2] report.pdf".to_string(),
        "[1/2] pagina 1/2".to_string(),
        "[1/2] pagina 2/2".to_string(),
        "[2/2] vendite.xlsx".to_string(),
        "Riepilogo".to_string(),
        "  Convertiti: 2".to_string(),
        "    report.pdf -> report_pdf.md".to_string(),
        "    vendite.xlsx -> vendite_xlsx_Gennaio.csv, vendite_xlsx_Febbraio.csv".to_string(),
        "  Saltati: 1".to_string(),
        "    note.txt - non supportato".to_string(),
        format!("  Falliti: {}", if fail { 1 } else { 0 }),
    ];
    let mut out = std::io::stdout();
    for line in lines {
        writeln!(out, "{line}").unwrap();
        out.flush().unwrap();
        if slow {
            sleep(Duration::from_millis(300));
        }
    }
    if fail {
        println!("    rotto.docx - docx non valido");
        std::process::exit(1);
    }
}
