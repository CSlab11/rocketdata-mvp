//! Conversione dei PDF con Docling, con il Python finto di examples/fake_python.rs al posto di quello vero.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use pdfium_render::prelude::*;

fn exe() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_doc-ripper"))
}

/// doc-ripper.exe e pdfium.dll in `dir`, con accanto `docling\` contenente il Python finto e lo script.
fn install(dir: &Path) -> PathBuf {
    let build = exe().parent().unwrap().to_path_buf();
    fs::create_dir_all(dir.join("docling").join("python")).unwrap();
    fs::copy(exe(), dir.join("doc-ripper.exe")).unwrap();
    fs::copy(build.join("pdfium.dll"), dir.join("pdfium.dll")).unwrap();
    fs::copy(build.join("examples").join("fake_python.exe"), dir.join("docling").join("python").join("python.exe")).unwrap();
    fs::write(dir.join("docling").join("convert_pdf.py"), "").unwrap();
    dir.join("doc-ripper.exe")
}

fn run(exe: &Path, src: &Path, dst: &Path, envs: &[(&str, &str)]) -> Output {
    Command::new(exe)
        .args(["--src", src.to_str().unwrap(), "--dst", dst.to_str().unwrap()])
        .envs(envs.iter().copied())
        .output()
        .unwrap()
}

/// PDF di una pagina con un testo, creato con la PDFium di doc-ripper.
fn sample_pdf(path: &Path, text: &str) {
    let dir = exe().parent().unwrap().to_path_buf();
    let pdfium = Pdfium::new(Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(&dir)).unwrap());
    let mut document = pdfium.create_new_pdf().unwrap();
    let font = document.fonts_mut().helvetica();
    let mut page = document.pages_mut().create_page_at_end(PdfPagePaperSize::a4()).unwrap();
    page.objects_mut().create_text_object(PdfPoints::new(72.0), PdfPoints::new(700.0), text, font, PdfPoints::new(12.0)).unwrap();
    document.save_to_file(path).unwrap();
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
    assert_eq!(markdown(dst.path(), "report_pdf.md"), "# Titolo da Docling\n\nParagrafo da Docling.\n");
    assert!(dst.path().join(".doc-ripper").join("report_pdf").join("documento.json").is_file());
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
