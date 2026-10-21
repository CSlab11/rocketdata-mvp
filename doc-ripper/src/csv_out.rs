use calamine::{Data, ExcelDateTime};

/// Serializza le righe di un foglio in CSV secondo RFC 4180: virgola, CRLF, UTF-8 senza BOM,
/// valori grezzi (REQ-021).
pub fn to_csv<'a>(rows: impl Iterator<Item = &'a [Data]>) -> Result<Vec<u8>, String> {
    let mut writer = csv::WriterBuilder::new()
        .terminator(csv::Terminator::CRLF)
        .from_writer(Vec::new());
    for row in rows {
        writer.write_record(row.iter().map(cell_text)).map_err(|e| e.to_string())?;
    }
    writer.into_inner().map_err(|e| e.to_string())
}

/// Valore grezzo di una cella: risultato delle formule, date ISO 8601, numeri senza formattazione.
fn cell_text(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        Data::String(s) | Data::DateTimeIso(s) | Data::DurationIso(s) => s.clone(),
        Data::Int(i) => i.to_string(),
        Data::Float(f) => f.to_string(),
        Data::Bool(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
        Data::DateTime(dt) => datetime_text(dt),
        Data::Error(e) => e.to_string(),
    }
}

fn datetime_text(dt: &ExcelDateTime) -> String {
    if dt.is_duration() {
        let total = (dt.as_f64() * 86_400.0).round() as i64;
        return format!("PT{}H{}M{}S", total / 3600, total % 3600 / 60, total % 60);
    }
    let (y, mo, d, h, mi, s, ms) = dt.to_ymd_hms_milli();
    let time = if ms > 0 {
        format!("{h:02}:{mi:02}:{s:02}.{ms:03}")
    } else {
        format!("{h:02}:{mi:02}:{s:02}")
    };
    if dt.as_f64() < 1.0 {
        time
    } else if (h, mi, s, ms) == (0, 0, 0, 0) {
        format!("{y:04}-{mo:02}-{d:02}")
    } else {
        format!("{y:04}-{mo:02}-{d:02}T{time}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use calamine::{CellErrorType, ExcelDateTimeType};

    fn date(value: f64) -> Data {
        Data::DateTime(ExcelDateTime::new(value, ExcelDateTimeType::DateTime, false))
    }

    #[test]
    fn valori_grezzi() {
        assert_eq!(cell_text(&Data::Float(1234.5)), "1234.5");
        assert_eq!(cell_text(&Data::Float(3.0)), "3");
        assert_eq!(cell_text(&Data::Float(1234567.25)), "1234567.25");
        assert_eq!(cell_text(&Data::Int(-7)), "-7");
        assert_eq!(cell_text(&Data::Bool(true)), "TRUE");
        assert_eq!(cell_text(&Data::Empty), "");
        assert_eq!(cell_text(&Data::Error(CellErrorType::Div0)), "#DIV/0!");
    }

    #[test]
    fn date_in_iso_8601() {
        // 46316 = 2026-10-21 nel sistema di date 1900
        assert_eq!(cell_text(&date(46316.0)), "2026-10-21");
        assert_eq!(cell_text(&date(46316.5)), "2026-10-21T12:00:00");
        assert_eq!(cell_text(&date(0.75)), "18:00:00");
        let duration = Data::DateTime(ExcelDateTime::new(1.4375, ExcelDateTimeType::TimeDelta, false));
        assert_eq!(cell_text(&duration), "PT34H30M0S");
        assert_eq!(cell_text(&Data::DateTimeIso("2026-10-21".into())), "2026-10-21");
    }

    #[test]
    fn csv_rfc_4180() {
        let rows = vec![
            vec![Data::String("Nome".into()), Data::String("Importo".into())],
            vec![Data::String("Rossi, Mario".into()), Data::Float(10.5)],
            vec![Data::String("Detto \"Bianchi\"".into()), Data::Empty],
            vec![Data::String("Città".into()), Data::Int(2)],
        ];
        let bytes = to_csv(rows.iter().map(|r| r.as_slice())).unwrap();
        let text = String::from_utf8(bytes.clone()).unwrap();
        assert_eq!(
            text,
            "Nome,Importo\r\n\"Rossi, Mario\",10.5\r\n\"Detto \"\"Bianchi\"\"\",\r\nCittà,2\r\n"
        );
        assert!(!bytes.starts_with(&[0xEF, 0xBB, 0xBF]), "niente BOM");
    }
}
