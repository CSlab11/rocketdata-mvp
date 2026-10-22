use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use pdfium_render::prelude::*;

fn exe() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_doc-ripper"))
}

fn run(src: &Path, dst: &Path) -> Output {
    Command::new(exe())
        .args(["--src", src.to_str().unwrap(), "--dst", dst.to_str().unwrap()])
        .output()
        .unwrap()
}

/// PDFium caricata dalla stessa cartella di doc-ripper.exe (vi è copiata da build.rs).
fn pdfium() -> Pdfium {
    let dir = exe().parent().unwrap().to_path_buf();
    Pdfium::new(Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(&dir)).unwrap())
}

/// Crea un PDF di una pagina con i testi indicati: (x, y, dimensione, testo).
fn sample(pdfium: &Pdfium, path: &Path, texts: &[(f32, f32, f32, &str)]) {
    let mut document = pdfium.create_new_pdf().unwrap();
    let font = document.fonts_mut().helvetica();
    let mut page = document.pages_mut().create_page_at_end(PdfPagePaperSize::a4()).unwrap();
    for &(x, y, size, text) in texts {
        page.objects_mut()
            .create_text_object(PdfPoints::new(x), PdfPoints::new(y), text, font, PdfPoints::new(size))
            .unwrap();
    }
    document.save_to_file(path).unwrap();
}

#[test]
fn converte_pdf_in_markdown_strutturato() {
    let src = tempfile::tempdir().unwrap();
    let dst = tempfile::tempdir().unwrap();
    fs::create_dir(src.path().join("pdf")).unwrap();
    sample(
        &pdfium(),
        &src.path().join("pdf/report.pdf"),
        &[
            (72.0, 780.0, 24.0, "Relazione annuale"),
            (72.0, 740.0, 16.0, "Introduzione"),
            (72.0, 715.0, 11.0, "Questo paragrafo continua"),
            (72.0, 702.0, 11.0, "sulla riga successiva."),
            (72.0, 675.0, 11.0, "- Primo punto"),
            (72.0, 661.0, 11.0, "- Secondo punto"),
            (72.0, 630.0, 11.0, "Voce"),
            (250.0, 630.0, 11.0, "Q1"),
            (350.0, 630.0, 11.0, "Q2"),
            (72.0, 616.0, 11.0, "Affitto"),
            (250.0, 616.0, 11.0, "100"),
            (350.0, 616.0, 11.0, "110"),
            (72.0, 590.0, 11.0, "Fine del documento."),
        ],
    );

    let out = run(src.path(), dst.path());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{stdout}{}", String::from_utf8_lossy(&out.stderr));
    assert!(stdout.contains("Convertiti: 1"), "{stdout}");

    let md = fs::read_to_string(dst.path().join("pdf/report_pdf.md")).unwrap();
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
fn estrae_le_immagini_dei_pdf() {
    let src = tempfile::tempdir().unwrap();
    let dst = tempfile::tempdir().unwrap();
    let pdfium = pdfium();
    let mut document = pdfium.create_new_pdf().unwrap();
    let font = document.fonts_mut().helvetica();
    let mut page = document.pages_mut().create_page_at_end(PdfPagePaperSize::a4()).unwrap();
    let objects = page.objects_mut();
    objects.create_text_object(PdfPoints::new(72.0), PdfPoints::new(780.0), "Sopra.", font, PdfPoints::new(11.0)).unwrap();
    let picture = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(4, 3, image::Rgb([200, 30, 30])));
    objects
        .create_image_object(PdfPoints::new(72.0), PdfPoints::new(600.0), &picture, Some(PdfPoints::new(40.0)), None)
        .unwrap();
    objects.create_text_object(PdfPoints::new(72.0), PdfPoints::new(500.0), "Sotto.", font, PdfPoints::new(11.0)).unwrap();
    document.save_to_file(&src.path().join("foto.pdf")).unwrap();

    let out = run(src.path(), dst.path());
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stdout));

    let md = fs::read_to_string(dst.path().join("foto_pdf.md")).unwrap();
    assert_eq!(
        md,
        "Sopra.\n\n![pagina1_immagine1.png](foto_pdf_img/pagina1_immagine1.png)\n\nSotto.\n"
    );
    let png = image::open(dst.path().join("foto_pdf_img/pagina1_immagine1.png")).unwrap();
    assert_eq!((png.width(), png.height()), (4, 3));
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
