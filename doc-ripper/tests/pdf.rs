use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn exe() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_doc-ripper"))
}

fn run(src: &Path, dst: &Path) -> Output {
    Command::new(exe())
        .args(["--src", src.to_str().unwrap(), "--dst", dst.to_str().unwrap()])
        .output()
        .unwrap()
}

/// Copia in `dest` un PDF di prova di tests/fixtures/, generato una volta da examples/pdf_fixtures.rs:
/// PDFium non si usa nei test perché non regge più collegamenti nello stesso processo (BUG-001).
fn fixture(name: &str, dest: &Path) {
    fs::copy(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name), dest).unwrap();
}

/// Separa la riga "Origine" dal resto del Markdown e verifica che il link porti al file originale.
fn split_origin(md: &str, original: &Path) -> String {
    let (first, rest) = md.split_once("

").unwrap();
    assert!(first.starts_with("Origine: ["), "{first}");
    let url = first.split_once("](").unwrap().1.strip_suffix(')').unwrap();
    assert!(!url.starts_with("file:"), "{url}");
    let path = url::Url::parse(&format!("file:///{url}")).unwrap().to_file_path().unwrap();
    assert_eq!(fs::canonicalize(path).unwrap(), fs::canonicalize(original).unwrap());
    rest.to_string()
}

#[test]
fn converte_pdf_in_markdown_strutturato() {
    let src = tempfile::tempdir().unwrap();
    let dst = tempfile::tempdir().unwrap();
    fs::create_dir(src.path().join("pdf")).unwrap();
    fixture("report.pdf", &src.path().join("pdf/report.pdf"));

    let out = run(src.path(), dst.path());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{stdout}{}", String::from_utf8_lossy(&out.stderr));
    assert!(stdout.contains("Convertiti: 1"), "{stdout}");

    let md = fs::read_to_string(dst.path().join("pdf/report_pdf.md")).unwrap();
    let md = split_origin(&md, &src.path().join("pdf/report.pdf"));
    let expected = concat!(
        "# Relazione annuale\n\n",
        "## Introduzione\n\n",
        "Questo paragrafo continua sulla riga successiva.\n\n",
        "- Primo punto\n- Secondo punto\n\n",
        "| Voce | Q1 | Q2 |\n|---|---|---|\n| Affitto | 100 | 110 |\n\n",
        "Fine del documento.\n",
    );
    assert_eq!(md, expected);
}

#[test]
fn voci_di_sommario_con_puntini_guida_restano_testo() {
    let src = tempfile::tempdir().unwrap();
    let dst = tempfile::tempdir().unwrap();
    fixture("manuale.pdf", &src.path().join("manuale.pdf"));

    let out = run(src.path(), dst.path());
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let md = fs::read_to_string(dst.path().join("manuale_pdf.md")).unwrap();
    let md = split_origin(&md, &src.path().join("manuale.pdf"));
    assert!(md.starts_with("# Contents\n\n"), "{md}");
    assert!(!md.contains("# 1 Overview"), "{md}");
    assert!(!md.contains("# ETH"), "{md}");
    assert!(md.contains("\n1 Overview of the manual . . . . . 40"), "{md}");
    assert!(md.contains("\nETH\\_MMCTIR . . . . .1050"), "{md}");
}

#[test]
fn estrae_le_immagini_dei_pdf() {
    let src = tempfile::tempdir().unwrap();
    let dst = tempfile::tempdir().unwrap();
    fixture("foto.pdf", &src.path().join("foto.pdf"));

    let out = run(src.path(), dst.path());
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stdout));

    let md = fs::read_to_string(dst.path().join("foto_pdf.md")).unwrap();
    let md = split_origin(&md, &src.path().join("foto.pdf"));
    assert_eq!(
        md,
        "Sopra.\n\n![pagina1_immagine1.png](foto_pdf_img/pagina1_immagine1.png)\n\nSotto.\n"
    );
    let png = image::open(dst.path().join("foto_pdf_img/pagina1_immagine1.png")).unwrap();
    assert_eq!((png.width(), png.height()), (4, 3));
}

#[test]
fn salta_i_pdf_scansionati() {
    let src = tempfile::tempdir().unwrap();
    let dst = tempfile::tempdir().unwrap();
    fixture("scansione.pdf", &src.path().join("scansione.pdf"));

    let out = run(src.path(), dst.path());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(stdout.contains("scansione.pdf - scansionato"), "{stdout}");
    assert!(stdout.contains("Convertiti: 0"), "{stdout}");
    assert!(!dst.path().join("scansione_pdf.md").exists());
    assert!(!dst.path().join("scansione_pdf_img").exists());
}

#[test]
fn pdf_corrotto_fallisce() {
    let src = tempfile::tempdir().unwrap();
    let dst = tempfile::tempdir().unwrap();
    fs::write(src.path().join("rotto.pdf"), "non sono un pdf").unwrap();

    let out = run(src.path(), dst.path());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(1), "{stdout}");
    assert!(stdout.contains("rotto.pdf - pdf non valido"), "{stdout}");
}

#[test]
fn scrive_una_riga_di_avanzamento_per_pagina() {
    let src = tempfile::tempdir().unwrap();
    let dst = tempfile::tempdir().unwrap();
    fixture("doc.pdf", &src.path().join("doc.pdf"));

    let out = run(src.path(), dst.path());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines[..4], ["[1/1] doc.pdf", "[1/1] pagina 1/2", "[1/1] pagina 2/2", "Riepilogo"], "{stdout}");
}
