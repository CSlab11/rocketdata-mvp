use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output};

use rust_xlsxwriter::{ExcelDateTime, Format, Formula, Workbook};

fn run(src: &Path, dst: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_doc-ripper"))
        .args(["--src", src.to_str().unwrap(), "--dst", dst.to_str().unwrap()])
        .output()
        .unwrap()
}

fn xlsx(path: &Path) {
    let mut wb = Workbook::new();
    let jan = wb.add_worksheet();
    jan.set_name("Gennaio").unwrap();
    jan.write_string(0, 0, "Cliente").unwrap();
    jan.write_string(0, 1, "Importo").unwrap();
    jan.write_string(0, 2, "Doppio").unwrap();
    jan.write_string(0, 3, "Data").unwrap();
    jan.write_string(1, 0, "Rossi, Mario").unwrap();
    jan.write_number(1, 1, 1234.5).unwrap();
    jan.write_formula(1, 2, Formula::new("=B2*2").set_result("2469")).unwrap();
    let date = ExcelDateTime::from_ymd(2026, 10, 21).unwrap();
    jan.write_datetime_with_format(1, 3, &date, &Format::new().set_num_format("dd/mm/yyyy")).unwrap();

    let hidden = wb.add_worksheet();
    hidden.set_name("Nascosto").unwrap();
    hidden.write_string(0, 0, "segreto").unwrap();
    hidden.set_hidden(true);

    wb.add_worksheet().set_name("Vuoto").unwrap();

    let odd = wb.add_worksheet();
    odd.set_name("Q1 <bozza>").unwrap();
    odd.write_number(2, 1, 7.0).unwrap();

    wb.save(path).unwrap();
}

/// ODS minimo: mimetype non compresso e content.xml con un foglio.
fn ods(path: &Path) {
    let mut zip = zip::ZipWriter::new(fs::File::create(path).unwrap());
    let stored = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    zip.start_file("mimetype", stored).unwrap();
    zip.write_all(b"application/vnd.oasis.opendocument.spreadsheet").unwrap();
    zip.start_file("META-INF/manifest.xml", zip::write::SimpleFileOptions::default()).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8"?>
<manifest:manifest xmlns:manifest="urn:oasis:names:tc:opendocument:xmlns:manifest:1.0" manifest:version="1.2">
 <manifest:file-entry manifest:full-path="/" manifest:media-type="application/vnd.oasis.opendocument.spreadsheet"/>
 <manifest:file-entry manifest:full-path="content.xml" manifest:media-type="text/xml"/>
</manifest:manifest>"#,
    )
    .unwrap();
    zip.start_file("content.xml", zip::write::SimpleFileOptions::default()).unwrap();
    zip.write_all(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0"
  xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0"
  xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" office:version="1.2">
<office:body><office:spreadsheet><table:table table:name="Foglio1"><table:table-row><table:table-cell office:value-type="string"><text:p>Città</text:p></table:table-cell><table:table-cell office:value-type="float" office:value="3.5"><text:p>3,5</text:p></table:table-cell><table:table-cell office:value-type="date" office:date-value="2026-10-21"><text:p>21/10/26</text:p></table:table-cell></table:table-row></table:table></office:spreadsheet></office:body></office:document-content>"#
            .as_bytes(),
    )
    .unwrap();
    zip.finish().unwrap();
}

#[test]
fn converte_xlsx_un_csv_per_foglio() {
    let src = tempfile::tempdir().unwrap();
    let dst = tempfile::tempdir().unwrap();
    fs::create_dir(src.path().join("conti")).unwrap();
    xlsx(&src.path().join("conti/vendite.xlsx"));

    let out = run(src.path(), dst.path());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{stdout}");

    let dir = dst.path().join("conti");
    let gennaio = fs::read_to_string(dir.join("vendite_xlsx_Gennaio.csv")).unwrap();
    assert_eq!(
        gennaio,
        "Cliente,Importo,Doppio,Data\r\n\"Rossi, Mario\",1234.5,2469,2026-10-21\r\n"
    );
    assert_eq!(fs::read_to_string(dir.join("vendite_xlsx_Nascosto.csv")).unwrap(), "segreto\r\n");
    assert_eq!(fs::read_to_string(dir.join("vendite_xlsx_Q1 _bozza_.csv")).unwrap(), ",\r\n,\r\n,7\r\n");
    assert!(!dir.join("vendite_xlsx_Vuoto.csv").exists());
    assert!(stdout.contains("foglio vuoto \"Vuoto\""), "{stdout}");
    assert!(stdout.contains("Convertiti: 1"), "{stdout}");
}

#[test]
fn converte_ods() {
    let src = tempfile::tempdir().unwrap();
    let dst = tempfile::tempdir().unwrap();
    ods(&src.path().join("dati.ods"));

    let out = run(src.path(), dst.path());
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stdout));
    let csv = fs::read_to_string(dst.path().join("dati_ods_Foglio1.csv")).unwrap();
    assert_eq!(csv, "Città,3.5,2026-10-21\r\n");
}

#[test]
fn foglio_corrotto_fallisce_senza_fermare_gli_altri() {
    let src = tempfile::tempdir().unwrap();
    let dst = tempfile::tempdir().unwrap();
    fs::write(src.path().join("rotto.xlsx"), "non sono un xlsx").unwrap();
    xlsx(&src.path().join("buono.xlsx"));

    let out = run(src.path(), dst.path());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(1), "{stdout}");
    assert!(stdout.contains("Falliti: 1"), "{stdout}");
    assert!(stdout.contains("rotto.xlsx - "), "{stdout}");
    assert!(dst.path().join("buono_xlsx_Gennaio.csv").exists());
}
