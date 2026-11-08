//! Conversione dei PDF con Docling, con il Python finto di examples/fake_python.rs al posto di quello vero.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn exe() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_doc-ripper"))
}

/// doc-ripper.exe e pdfium.dll in `dir`, con accanto `docling\` contenente il Python finto, lo script e la
/// cartella `docling-1.0.0.dist-info` da cui doc-ripper ricava la versione di Docling.
fn install(dir: &Path) -> PathBuf {
    let build = exe().parent().unwrap().to_path_buf();
    fs::create_dir_all(dir.join("docling").join("python")).unwrap();
    fs::copy(exe(), dir.join("doc-ripper.exe")).unwrap();
    fs::copy(build.join("pdfium.dll"), dir.join("pdfium.dll")).unwrap();
    fs::copy(build.join("examples").join("fake_python.exe"), dir.join("docling").join("python").join("python.exe")).unwrap();
    fs::write(dir.join("docling").join("convert_pdf.py"), "").unwrap();
    fs::create_dir_all(dir.join("docling/python/Lib/site-packages/docling-1.0.0.dist-info")).unwrap();
    dir.join("doc-ripper.exe")
}

fn run(exe: &Path, src: &Path, dst: &Path, envs: &[(&str, &str)]) -> Output {
    Command::new(exe)
        .args(["--src", src.to_str().unwrap(), "--dst", dst.to_str().unwrap()])
        .envs(envs.iter().copied())
        .output()
        .unwrap()
}

/// PDF di una pagina con un testo in Helvetica, scritto a mano: nei test in parallelo PDFium non si può
/// collegare alla libreria da più thread insieme, quindi qui non si usa.
fn sample_pdf(path: &Path, text: &str) {
    let stream = format!("BT /F1 12 Tf 72 700 Td ({text}) Tj ET");
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>".to_string(),
        format!("<< /Length {} >>\nstream\n{stream}\nendstream", stream.len()),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
    ];
    let mut pdf = String::from("%PDF-1.4\n");
    let mut offsets = Vec::new();
    for (i, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.push_str(&format!("{} 0 obj\n{object}\nendobj\n", i + 1));
    }
    let xref = pdf.len();
    pdf.push_str(&format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1));
    for offset in offsets {
        pdf.push_str(&format!("{offset:010} 00000 n \n"));
    }
    pdf.push_str(&format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", objects.len() + 1));
    fs::write(path, pdf).unwrap();
}
fn markdown(dst: &Path, name: &str) -> String {
    let md = fs::read_to_string(dst.join(name)).unwrap();
    md.split_once("\n\n").unwrap().1.to_string()
}

#[test]
fn converte_con_docling_quando_e_installato() {
    let bin = tempfile::tempdir().unwrap();
    let src = tempfile::tempdir().unwrap();
    let dst = tempfile::tempdir().unwrap();
    sample_pdf(&src.path().join("report.pdf"), "Testo PDFium");
    let output = run(&install(bin.path()), src.path(), dst.path(), &[]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{stdout}");
    assert!(stdout.contains("report.pdf -> report_pdf.md\n"), "{stdout}");
    assert_eq!(markdown(dst.path(), "report_pdf.md"), "# Titolo da Docling\n\nParagrafo da Docling.\n\nSeconda parte.\n");
    assert!(dst.path().join(".doc-ripper").join("report_pdf").join("blocco-0001.json").is_file());
}

/// Avanzamento per blocco di pagine dei PDF convertiti con Docling (REQ-054).
#[test]
fn avanzamento_per_blocco_di_pagine() {
    let bin = tempfile::tempdir().unwrap();
    let src = tempfile::tempdir().unwrap();
    let dst = tempfile::tempdir().unwrap();
    sample_pdf(&src.path().join("report.pdf"), "Testo");
    let output = run(&install(bin.path()), src.path(), dst.path(), &[]);
    let stdout = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
    assert!(stdout.starts_with("[1/1] report.pdf\n[1/1] pagina 10/12\n[1/1] pagina 12/12\n"), "{stdout}");
}

#[test]
fn ripiega_su_pdfium_se_docling_fallisce() {
    let bin = tempfile::tempdir().unwrap();
    let src = tempfile::tempdir().unwrap();
    let dst = tempfile::tempdir().unwrap();
    sample_pdf(&src.path().join("report.pdf"), "Testo PDFium");
    let output = run(&install(bin.path()), src.path(), dst.path(), &[("FAKE_DOCLING", "errore")]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{stdout}");
    assert!(
        stdout.contains("report.pdf -> report_pdf.md (ripiego PDFium: Docling: RuntimeError: modello di layout non trovato)"),
        "{stdout}"
    );
    assert!(markdown(dst.path(), "report_pdf.md").contains("Testo PDFium"));
}

#[test]
fn ripiega_su_pdfium_se_docling_non_e_installato() {
    let src = tempfile::tempdir().unwrap();
    let dst = tempfile::tempdir().unwrap();
    sample_pdf(&src.path().join("report.pdf"), "Testo PDFium");
    let output = run(&exe(), src.path(), dst.path(), &[]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{stdout}");
    assert!(stdout.contains("report.pdf -> report_pdf.md (ripiego PDFium: Docling non disponibile)"), "{stdout}");
}

/// Solo i `.pdf` passano da Docling: `.docx` e fogli di calcolo restano ai loro convertitori (REQ-049).
#[test]
fn docling_solo_per_i_pdf() {
    let bin = tempfile::tempdir().unwrap();
    let src = tempfile::tempdir().unwrap();
    let dst = tempfile::tempdir().unwrap();
    let log = bin.path().join("chiamate.txt");
    sample_pdf(&src.path().join("report.pdf"), "Testo");
    let docx = docx_rs::Docx::new().add_paragraph(docx_rs::Paragraph::new().add_run(docx_rs::Run::new().add_text("Testo DOCX")));
    docx.build().pack(fs::File::create(src.path().join("nota.docx")).unwrap()).unwrap();
    let mut workbook = rust_xlsxwriter::Workbook::new();
    workbook.add_worksheet().write_string(0, 0, "cella").unwrap();
    workbook.save(src.path().join("dati.xlsx")).unwrap();

    let output = run(&install(bin.path()), src.path(), dst.path(), &[("FAKE_DOCLING_LOG", log.to_str().unwrap())]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{stdout}");
    assert!(stdout.contains("Convertiti: 3"), "{stdout}");
    let calls = fs::read_to_string(&log).unwrap();
    let calls: Vec<&str> = calls.lines().collect();
    assert_eq!(calls.len(), 1, "{calls:?}");
    assert!(calls[0].ends_with("report.pdf"), "{calls:?}");
    assert!(fs::read_to_string(dst.path().join("nota_docx.md")).unwrap().contains("Testo DOCX"));
}
/// Il JSON di Docling si riusa se il PDF e la versione di Docling non sono cambiati (REQ-069).
#[test]
fn riusa_il_json_se_pdf_e_versione_non_cambiano() {
    let bin = tempfile::tempdir().unwrap();
    let src = tempfile::tempdir().unwrap();
    let dst = tempfile::tempdir().unwrap();
    let log = bin.path().join("chiamate.txt");
    let exe = install(bin.path());
    let pdf = src.path().join("report.pdf");
    let envs = [("FAKE_DOCLING_LOG", log.to_str().unwrap())];
    let calls = || fs::read_to_string(&log).unwrap().lines().count();

    sample_pdf(&pdf, "Prima versione");
    assert!(run(&exe, src.path(), dst.path(), &envs).status.success());
    assert_eq!(calls(), 1);

    // Stesso PDF e stessa versione: nessuna nuova esecuzione, nessuna riga di avanzamento per pagina.
    let output = run(&exe, src.path(), dst.path(), &envs);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(calls(), 1);
    assert!(!stdout.contains("pagina"), "{stdout}");
    assert!(markdown(dst.path(), "report_pdf.md").contains("Seconda parte."));

    // PDF cambiato: Docling di nuovo.
    sample_pdf(&pdf, "Seconda versione");
    assert!(run(&exe, src.path(), dst.path(), &envs).status.success());
    assert_eq!(calls(), 2);

    // Versione di Docling cambiata: Docling di nuovo.
    let packages = bin.path().join("docling/python/Lib/site-packages");
    fs::rename(packages.join("docling-1.0.0.dist-info"), packages.join("docling-1.1.0.dist-info")).unwrap();
    assert!(run(&exe, src.path(), dst.path(), &envs).status.success());
    assert_eq!(calls(), 3);
}