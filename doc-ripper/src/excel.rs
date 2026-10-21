use std::path::Path;

use calamine::{open_workbook_auto, Data, Range, Reader, SheetType};

use crate::convert::base_name;
use crate::csv_out::to_csv;
use crate::output::write_file;
use crate::report::SkipReason;
use crate::run::Outcome;

/// Converte un foglio di calcolo in un CSV per foglio di lavoro, nascosti compresi;
/// i fogli vuoti vengono saltati (REQ-012).
pub fn convert(src: &Path, dst: &Path, rel: &Path) -> Result<Outcome, String> {
    let mut workbook = open_workbook_auto(src.join(rel)).map_err(|e| e.to_string())?;
    let sheets: Vec<String> = workbook
        .sheets_metadata()
        .iter()
        .filter(|s| s.typ == SheetType::WorkSheet)
        .map(|s| s.name.clone())
        .collect();

    let mut outcome = Outcome::default();
    for sheet in sheets {
        let range = workbook.worksheet_range(&sheet).map_err(|e| format!("foglio \"{sheet}\": {e}"))?;
        if range.cells().all(|(_, _, cell)| *cell == Data::Empty) {
            outcome.skipped.push(SkipReason::EmptySheet(sheet));
            continue;
        }
        let out_rel = rel.with_file_name(format!("{}_{}.csv", base_name(rel), file_safe(&sheet)));
        let rows = rows_from_a1(&range);
        let bytes = to_csv(rows.iter().map(|r| r.as_slice()))?;
        write_file(&dst.join(&out_rel), &bytes)?;
        outcome.outputs.push(out_rel);
    }
    Ok(outcome)
}

/// Righe del foglio a partire dalla cella A1: calamine parte dalla prima cella usata,
/// quindi si aggiungono le righe e colonne vuote iniziali per mantenere le posizioni.
fn rows_from_a1(range: &Range<Data>) -> Vec<Vec<Data>> {
    let (row0, col0) = range.start().unwrap_or((0, 0));
    let width = col0 as usize + range.width();
    let mut rows = vec![vec![Data::Empty; width]; row0 as usize];
    for row in range.rows() {
        let mut padded = vec![Data::Empty; col0 as usize];
        padded.extend_from_slice(row);
        rows.push(padded);
    }
    rows
}

/// Sostituisce con `_` i caratteri non ammessi nei nomi di file di Windows.
fn file_safe(name: &str) -> String {
    let mut out: String = name
        .chars()
        .map(|c| if c.is_control() || "<>:\"/\\|?*".contains(c) { '_' } else { c })
        .collect();
    // Windows non ammette nomi che terminano con punto o spazio.
    if out.ends_with('.') || out.ends_with(' ') {
        out.pop();
        out.push('_');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nomi_dei_fogli_sicuri() {
        assert_eq!(file_safe("Gennaio"), "Gennaio");
        assert_eq!(file_safe("Q1 <2026>"), "Q1 _2026_");
        assert_eq!(file_safe("fine."), "fine_");
        assert_eq!(file_safe("a|b\"c"), "a_b_c");
    }

    #[test]
    fn mantiene_le_posizioni_da_a1() {
        let mut range = Range::new((2, 1), (2, 2));
        range.set_value((2, 1), Data::Int(1));
        range.set_value((2, 2), Data::Int(2));
        let rows = rows_from_a1(&range);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0], vec![Data::Empty; 3]);
        assert_eq!(rows[2], vec![Data::Empty, Data::Int(1), Data::Int(2)]);
    }
}
