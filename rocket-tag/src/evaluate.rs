//! Valutazione di un file di uscita contro le etichette umane (appendici F e G).

use std::collections::{BTreeMap, HashMap};
use std::time::SystemTime;

use serde_json::{Map, Value, json};

use crate::labels;
use crate::output::{Item, Output};
use crate::schema::{Facet, Schema};

/// Etichetta umana abbinata all'elemento del file di uscita.
#[derive(Debug, Clone, PartialEq)]
pub struct Matched {
    pub gold: Map<String, Value>,
    pub item: Item,
}

/// Esito dell'abbinamento tra etichette e file di uscita.
#[derive(Debug, Default, PartialEq)]
pub struct Pairing {
    pub matched: Vec<Matched>,
    /// Stessa unità con `text_hash` diverso: il testo è cambiato.
    pub misaligned: usize,
    /// Etichette senza un elemento corrispondente.
    pub missing: usize,
    /// Etichette non valide per lo schema corrente, da rivedere (REQ-120).
    pub invalid: usize,
}

/// Metriche di un valore.
#[derive(Debug, Clone, PartialEq)]
pub struct ValueMetrics {
    pub value: String,
    pub precision: f64,
    pub recall: f64,
    pub f1: f64,
    pub support: usize,
    pub predicted: usize,
}

/// Una coppia (umano, predetto) con il numero di occorrenze.
#[derive(Debug, Clone, PartialEq)]
pub struct Confusion {
    pub gold: String,
    pub predicted: String,
    pub count: usize,
}

/// Accuratezza in una fascia di confidenza.
#[derive(Debug, Clone, PartialEq)]
pub struct Band {
    pub range: &'static str,
    pub count: usize,
    pub accuracy: f64,
}

/// Metriche di una faccetta (appendice G).
#[derive(Debug, Clone, PartialEq)]
pub struct FacetMetrics {
    pub facet: String,
    pub count: usize,
    pub accuracy: f64,
    pub macro_f1: f64,
    pub coverage: f64,
    pub values: Vec<ValueMetrics>,
    pub confusions: Vec<Confusion>,
    pub bands: Vec<Band>,
}

const BANDS: [(f64, f64, &str); 4] =
    [(0.0, 0.5, "[0; 0,5)"), (0.5, 0.7, "[0,5; 0,7)"), (0.7, 0.85, "[0,7; 0,85)"), (0.85, f64::INFINITY, "[0,85; 1]")];
const MAX_CONFUSIONS: usize = 8;

/// Un'unità per una faccetta: valori come insiemi ordinati, così faccette singole e multiple si
/// misurano allo stesso modo; `predicted` è `None` per l'astensione.
struct Sample {
    gold: Vec<String>,
    predicted: Option<Vec<String>>,
    confidence: Option<f64>,
}

impl Sample {
    fn correct(&self) -> bool {
        self.predicted.as_ref() == Some(&self.gold)
    }

    fn has_predicted(&self, value: &str) -> bool {
        self.predicted.iter().flatten().any(|v| v == value)
    }

    fn has_gold(&self, value: &str) -> bool {
        self.gold.iter().any(|v| v == value)
    }
}

fn values_of(value: Option<&Value>) -> Vec<String> {
    let mut values: Vec<String> = match value {
        Some(Value::Array(list)) => list.iter().filter_map(Value::as_str).map(str::to_string).collect(),
        Some(Value::String(text)) => vec![text.clone()],
        _ => Vec::new(),
    };
    values.sort();
    values
}

fn ratio(part: usize, whole: usize) -> f64 {
    if whole == 0 { 0.0 } else { part as f64 / whole as f64 }
}

/// Accuratezza, macro-F1, copertura, metriche per valore, confusioni e fasce di una faccetta
/// (REQ-119).
pub fn facet_metrics(facet: &Facet, matched: &[Matched]) -> FacetMetrics {
    let samples: Vec<Sample> = matched
        .iter()
        .map(|m| Sample {
            gold: values_of(m.gold.get(&facet.name)),
            predicted: m.item.labels.get(&facet.name).map(|value| values_of(Some(value))),
            confidence: m.item.confidence.get(&facet.name).and_then(Value::as_f64),
        })
        .collect();
    let values: Vec<ValueMetrics> =
        facet.values.iter().map(|value| value_metrics(value, &samples)).filter(|v| v.support > 0 || v.predicted > 0).collect();
    let supported: Vec<&ValueMetrics> = values.iter().filter(|v| v.support > 0).collect();
    let covered = samples.iter().filter(|s| s.predicted.is_some()).count();
    FacetMetrics {
        facet: facet.name.clone(),
        count: samples.len(),
        accuracy: ratio(samples.iter().filter(|s| s.correct()).count(), samples.len()),
        macro_f1: if supported.is_empty() { 0.0 } else { supported.iter().map(|v| v.f1).sum::<f64>() / supported.len() as f64 },
        coverage: ratio(covered, samples.len()),
        values,
        confusions: confusions(&samples),
        bands: bands(&samples),
    }
}

fn value_metrics(value: &str, samples: &[Sample]) -> ValueMetrics {
    let count = |test: &dyn Fn(&Sample) -> bool| samples.iter().filter(|s| test(s)).count();
    let hits = count(&|s| s.has_gold(value) && s.has_predicted(value));
    let predicted = count(&|s| s.has_predicted(value));
    let support = count(&|s| s.has_gold(value));
    let (precision, recall) = (ratio(hits, predicted), ratio(hits, support));
    let f1 = if precision + recall > 0.0 { 2.0 * precision * recall / (precision + recall) } else { 0.0 };
    ValueMetrics { value: value.to_string(), precision, recall, f1, support, predicted }
}

fn show(values: &[String]) -> String {
    if values.is_empty() { "(nessuno)".to_string() } else { values.join(", ") }
}

fn confusions(samples: &[Sample]) -> Vec<Confusion> {
    let mut found: Vec<Confusion> = Vec::new();
    for sample in samples.iter().filter(|s| !s.correct()) {
        let gold = show(&sample.gold);
        let predicted = sample.predicted.as_ref().map_or_else(|| "(astensione)".to_string(), |values| show(values));
        match found.iter_mut().find(|c| c.gold == gold && c.predicted == predicted) {
            Some(confusion) => confusion.count += 1,
            None => found.push(Confusion { gold, predicted, count: 1 }),
        }
    }
    found.sort_by(|a, b| b.count.cmp(&a.count));
    found.truncate(MAX_CONFUSIONS);
    found
}

fn bands(samples: &[Sample]) -> Vec<Band> {
    BANDS
        .iter()
        .filter_map(|&(low, high, range)| {
            let inside: Vec<&Sample> = samples.iter().filter(|s| s.confidence.is_some_and(|c| low <= c && c < high)).collect();
            (!inside.is_empty()).then(|| Band {
                range,
                count: inside.len(),
                accuracy: ratio(inside.iter().filter(|s| s.correct()).count(), inside.len()),
            })
        })
        .collect()
}

/// Risultato di una valutazione.
#[derive(Debug, Clone, PartialEq)]
pub struct Evaluation {
    pub timestamp: String,
    pub schema_version: String,
    pub classifier: String,
    pub evaluated: usize,
    pub invalid: usize,
    pub misaligned: usize,
    pub missing: usize,
    pub facets: Vec<FacetMetrics>,
}

impl Evaluation {
    /// Metriche di tutte le faccette dello schema (REQ-119).
    pub fn new(schema: &Schema, records: &[Value], output: &Output) -> Result<Evaluation, String> {
        let pairing = pair(schema, records, output);
        if pairing.matched.is_empty() {
            return Err("nessuna etichetta valida con un elemento corrispondente nel file di uscita".to_string());
        }
        Ok(Evaluation {
            timestamp: humantime::format_rfc3339_seconds(SystemTime::now()).to_string(),
            schema_version: schema.version.clone(),
            classifier: output.classifier.clone(),
            evaluated: pairing.matched.len(),
            invalid: pairing.invalid,
            misaligned: pairing.misaligned,
            missing: pairing.missing,
            facets: schema.facets.iter().map(|facet| facet_metrics(facet, &pairing.matched)).collect(),
        })
    }

    /// Report Markdown: tabella riassuntiva, poi per ogni faccetta valori, confusioni e fasce
    /// (REQ-121).
    pub fn report(&self) -> String {
        let mut lines = vec![
            format!("# Valutazione – {} contro le etichette umane", self.classifier),
            String::new(),
            format!(
                "Schema v{} · {} unità valutate · {} etichette da rivedere escluse · {} non allineate · {} senza elemento · {}",
                self.schema_version, self.evaluated, self.invalid, self.misaligned, self.missing, self.timestamp
            ),
            String::new(),
            "| Faccetta | Accuratezza | Macro-F1 | Copertura | N |".to_string(),
            "| --- | --- | --- | --- | --- |".to_string(),
        ];
        for facet in &self.facets {
            lines.push(format!(
                "| {} | {:.1}% | {:.2} | {:.0}% | {} |",
                facet.facet,
                facet.accuracy * 100.0,
                facet.macro_f1,
                facet.coverage * 100.0,
                facet.count
            ));
        }
        for facet in &self.facets {
            lines.extend([
                String::new(),
                format!("## {}", facet.facet),
                String::new(),
                "| Valore | Precisione | Richiamo | F1 | Supporto | Predetti |".to_string(),
                "| --- | --- | --- | --- | --- | --- |".to_string(),
            ]);
            let mut values: Vec<&ValueMetrics> = facet.values.iter().collect();
            values.sort_by(|a, b| b.support.cmp(&a.support));
            for value in values {
                lines.push(format!(
                    "| {} | {:.2} | {:.2} | {:.2} | {} | {} |",
                    value.value, value.precision, value.recall, value.f1, value.support, value.predicted
                ));
            }
            if !facet.confusions.is_empty() {
                let list: Vec<String> =
                    facet.confusions.iter().map(|c| format!("{} → {} ({})", c.gold, c.predicted, c.count)).collect();
                lines.extend([String::new(), format!("Confusioni più frequenti (umano → predetto): {}", list.join("; "))]);
            }
            if !facet.bands.is_empty() {
                let list: Vec<String> =
                    facet.bands.iter().map(|b| format!("{}: {:.0}% (n={})", b.range, b.accuracy * 100.0, b.count)).collect();
                lines.extend([String::new(), format!("Accuratezza per confidenza: {}", list.join("; "))]);
            }
        }
        lines.join("\n") + "\n"
    }

    /// Riga dello storico JSONL (REQ-121).
    pub fn history_line(&self) -> String {
        let round = |value: f64| (value * 10000.0).round() / 10000.0;
        let facets: Map<String, Value> = self
            .facets
            .iter()
            .map(|f| {
                let metrics = json!({"accuracy": round(f.accuracy), "macro_f1": round(f.macro_f1), "coverage": round(f.coverage)});
                (f.facet.clone(), metrics)
            })
            .collect();
        json!({"timestamp": self.timestamp, "schema_version": self.schema_version, "classifier": self.classifier,
               "n_eval": self.evaluated, "facets": facets})
        .to_string()
    }
}

/// Abbina etichette di paragrafo ed elementi per documento, riga iniziale e riga finale; se la
/// stessa unità compare più volte vale l'ultima riga (REQ-118).
pub fn pair(schema: &Schema, records: &[Value], output: &Output) -> Pairing {
    let documents = labels::documents(records);
    let mut labels = BTreeMap::new();
    for record in records.iter().filter(|r| r["level"] == "paragraph") {
        let (Some(document), Some(start), Some(end)) =
            (record["document"].as_str(), record["line_start"].as_u64(), record["line_end"].as_u64())
        else {
            continue;
        };
        labels.insert((document.to_string(), start as usize, end as usize), record);
    }
    let items: HashMap<_, _> = output.items.iter().map(|i| ((i.document.clone(), i.line_start, i.line_end), i)).collect();
    let mut pairing = Pairing::default();
    for (key, record) in labels {
        let gold = record["facets"].as_object().cloned().unwrap_or_default();
        let side = documents.get(&key.0).and_then(|info| info.side.as_deref());
        if !schema.label_errors(&gold, side).is_empty() {
            pairing.invalid += 1;
            continue;
        }
        let Some(item) = items.get(&key) else {
            pairing.missing += 1;
            continue;
        };
        if record["text_hash"].as_str().is_some_and(|hash| !item.text_hash.is_empty() && hash != item.text_hash) {
            pairing.misaligned += 1;
            continue;
        }
        pairing.matched.push(Matched { gold, item: (*item).clone() });
    }
    pairing
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::Kind;

    fn item(document: &str, line: usize, hash: &str) -> Item {
        Item {
            document: document.into(),
            line_start: line,
            line_end: line + 1,
            kind: Kind::Paragraph,
            text_hash: hash.into(),
            labels: Map::new(),
            confidence: Map::new(),
            unit_confidence: 0.0,
            rule_ids: None,
        }
    }

    fn output(items: Vec<Item>) -> Output {
        Output { items, ..Output::new("0.1.0", "rules-v1".into()) }
    }

    fn schema() -> Schema {
        Schema::parse(include_str!("../config/label_schema.json")).unwrap()
    }

    fn facets(nature: &str) -> Value {
        json!({"nature": nature, "domain": "technical", "destination": "P", "required_action": "none",
               "extraction_quality": "ok", "flags": []})
    }

    fn label(document: &str, line: usize, hash: Option<&str>, nature: &str) -> Value {
        let mut record = json!({"level": "paragraph", "document": document, "line_start": line, "line_end": line + 1,
                                "facets": facets(nature)});
        if let Some(hash) = hash {
            record["text_hash"] = json!(hash);
        }
        record
    }

    /// L'esempio di 10 unità: A 6 volte e B 4; 5+1 astensione, 2+2 errori.
    fn sample(gold: &str, predicted: Option<&str>, confidence: f64) -> Matched {
        let mut unit = item("a.md", 1, "h");
        if let Some(value) = predicted {
            unit.labels.insert("nature".into(), json!(value));
            unit.confidence.insert("nature".into(), json!(confidence));
        }
        Matched { gold: facets(gold).as_object().unwrap().clone(), item: unit }
    }

    fn close(left: f64, right: f64) {
        assert!((left - right).abs() < 0.005, "{left} != {right}");
    }

    #[test]
    fn metriche_di_una_faccetta_singola() {
        let schema = schema();
        let facet = schema.facet("nature").unwrap();
        let (a, b) = ("requirement", "option");
        let mut matched = vec![sample(a, None, 0.0)];
        matched.extend((0..5).map(|_| sample(a, Some(a), 0.9)));
        matched.extend((0..2).map(|_| sample(b, Some(b), 0.6)));
        matched.extend((0..2).map(|_| sample(b, Some(a), 0.4)));
        let metrics = facet_metrics(facet, &matched);
        assert_eq!(metrics.count, 10);
        close(metrics.accuracy, 0.7);
        close(metrics.coverage, 0.9);
        close(metrics.macro_f1, 0.72);
        let value = |name: &str| metrics.values.iter().find(|v| v.value == name).unwrap();
        close(value(a).precision, 0.71);
        close(value(a).recall, 0.83);
        close(value(a).f1, 0.77);
        close(value(b).precision, 1.0);
        close(value(b).recall, 0.5);
        close(value(b).f1, 0.67);
        assert_eq!((value(a).support, value(a).predicted), (6, 7));
        assert_eq!(metrics.values.len(), 2, "i valori mai usati non compaiono");
        assert_eq!(
            metrics.confusions[0],
            Confusion { gold: "option".into(), predicted: "requirement".into(), count: 2 }
        );
        assert_eq!(metrics.confusions[1].predicted, "(astensione)");
        let bands: Vec<_> = metrics.bands.iter().map(|b| (b.range, b.count)).collect();
        assert_eq!(bands, [("[0; 0,5)", 2), ("[0,5; 0,7)", 2), ("[0,85; 1]", 5)]);
        close(metrics.bands[0].accuracy, 0.0);
        close(metrics.bands[2].accuracy, 1.0);
    }

    #[test]
    fn la_macro_f1_esclude_i_valori_senza_supporto() {
        let schema = schema();
        let facet = schema.facet("nature").unwrap();
        let metrics = facet_metrics(facet, &[sample("requirement", Some("requirement"), 1.0), sample("requirement", Some("option"), 1.0)]);
        assert_eq!(metrics.values.len(), 2, "option è predetto ma non ha supporto");
        close(metrics.macro_f1, 2.0 / 3.0);
        assert_eq!(metrics.bands[0].range, "[0,85; 1]", "1 è incluso nell'ultima fascia");
    }

    #[test]
    fn faccette_multiple_come_insiemi() {
        let schema = schema();
        let facet = schema.facet("flags").unwrap();
        let flag = "multiple_requirements";
        let unit = |gold: Value, predicted: Option<Value>| {
            let mut unit = item("a.md", 1, "h");
            if let Some(value) = predicted {
                unit.labels.insert("flags".into(), value);
            }
            Matched { gold: json!({"flags": gold}).as_object().unwrap().clone(), item: unit }
        };
        let matched = [
            unit(json!([flag]), Some(json!([flag]))),
            unit(json!([]), Some(json!([]))),
            unit(json!([flag]), Some(json!([]))),
            unit(json!([]), Some(json!([flag]))),
            unit(json!([flag]), None),
        ];
        let metrics = facet_metrics(facet, &matched);
        close(metrics.accuracy, 0.4);
        close(metrics.coverage, 0.8);
        close(metrics.values[0].precision, 0.5);
        close(metrics.values[0].recall, 1.0 / 3.0);
        assert_eq!(metrics.confusions.len(), 3, "le confusioni valgono anche per le multiple");
        assert!(metrics.confusions.iter().any(|c| c.gold == flag && c.predicted == "(nessuno)"));
    }

    #[test]
    fn abbina_per_documento_e_righe() {
        let records = [
            label("a.md", 1, None, "requirement"),
            label("a.md", 5, Some("h2"), "option"),
            label("b.md", 1, None, "context"),
            json!({"level": "document", "document": "a.md", "side": "prime"}),
        ];
        let pairing = pair(&schema(), &records, &output(vec![item("a.md", 1, "h1"), item("a.md", 5, "h2"), item("a.md", 9, "h3")]));
        assert_eq!(pairing.matched.len(), 2);
        assert_eq!(pairing.matched[0].gold["nature"], "requirement");
        assert_eq!(pairing.matched[1].item.line_start, 5);
        assert_eq!(pairing.missing, 1, "b.md non è nel file di uscita");
        assert_eq!(pairing.misaligned, 0);
    }

    #[test]
    fn text_hash_diverso_esclude_e_conta() {
        let records = [label("a.md", 1, Some("altro"), "option"), label("a.md", 5, None, "option")];
        let pairing = pair(&schema(), &records, &output(vec![item("a.md", 1, "h1"), item("a.md", 5, "h2")]));
        assert_eq!(pairing.misaligned, 1);
        assert_eq!(pairing.matched.len(), 1, "senza text_hash si abbina lo stesso");
    }

    #[test]
    fn vale_l_ultima_riga() {
        let records = [label("a.md", 1, None, "option"), label("a.md", 1, None, "context")];
        let pairing = pair(&schema(), &records, &output(vec![item("a.md", 1, "h")]));
        assert_eq!(pairing.matched.len(), 1);
        assert_eq!(pairing.matched[0].gold["nature"], "context");
    }

    #[test]
    fn esclude_e_conta_le_etichette_non_valide() {
        let mut extra = label("a.md", 3, None, "option");
        extra["facets"]["inventata"] = json!("x");
        let mut supplier = label("b.md", 1, None, "option");
        supplier["facets"]["required_action"] = json!("quote");
        let records = [
            label("a.md", 1, None, "option"),
            label("a.md", 2, None, "valore-mai-visto"),
            extra,
            supplier,
            json!({"level": "document", "document": "b.md", "side": "supplier"}),
        ];
        let items = vec![item("a.md", 1, "h"), item("a.md", 2, "h"), item("a.md", 3, "h"), item("b.md", 1, "h")];
        let pairing = pair(&schema(), &records, &output(items));
        assert_eq!(pairing.matched.len(), 1);
        assert_eq!(pairing.invalid, 3, "valore non ammesso, faccetta in più, vincolo del lato");
    }
}
