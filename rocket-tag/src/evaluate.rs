//! Valutazione di un file di uscita contro le etichette umane (appendici F e G).

use std::collections::{BTreeMap, HashMap};

use serde_json::{Map, Value};

use crate::output::{Item, Output};

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
}

/// Abbina etichette di paragrafo ed elementi per documento, riga iniziale e riga finale; se la
/// stessa unità compare più volte vale l'ultima riga (REQ-118).
pub fn pair(records: &[Value], output: &Output) -> Pairing {
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
        let Some(item) = items.get(&key) else {
            pairing.missing += 1;
            continue;
        };
        if record["text_hash"].as_str().is_some_and(|hash| !item.text_hash.is_empty() && hash != item.text_hash) {
            pairing.misaligned += 1;
            continue;
        }
        let gold = record["facets"].as_object().cloned().unwrap_or_default();
        pairing.matched.push(Matched { gold, item: (*item).clone() });
    }
    pairing
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::Kind;
    use serde_json::json;

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

    fn label(document: &str, line: usize, hash: Option<&str>, nature: &str) -> Value {
        let mut record = json!({"level": "paragraph", "document": document, "line_start": line, "line_end": line + 1,
                                "facets": {"nature": nature}});
        if let Some(hash) = hash {
            record["text_hash"] = json!(hash);
        }
        record
    }

    #[test]
    fn abbina_per_documento_e_righe() {
        let records = [
            label("a.md", 1, None, "x"),
            label("a.md", 5, Some("h2"), "y"),
            label("b.md", 1, None, "z"),
            json!({"level": "document", "document": "a.md", "side": "prime"}),
        ];
        let pairing = pair(&records, &output(vec![item("a.md", 1, "h1"), item("a.md", 5, "h2"), item("a.md", 9, "h3")]));
        assert_eq!(pairing.matched.len(), 2);
        assert_eq!(pairing.matched[0].gold["nature"], "x");
        assert_eq!(pairing.matched[1].item.line_start, 5);
        assert_eq!(pairing.missing, 1, "b.md non è nel file di uscita");
        assert_eq!(pairing.misaligned, 0);
    }

    #[test]
    fn text_hash_diverso_esclude_e_conta() {
        let records = [label("a.md", 1, Some("altro"), "x"), label("a.md", 5, None, "y")];
        let pairing = pair(&records, &output(vec![item("a.md", 1, "h1"), item("a.md", 5, "h2")]));
        assert_eq!(pairing.misaligned, 1);
        assert_eq!(pairing.matched.len(), 1, "senza text_hash si abbina lo stesso");
    }

    #[test]
    fn vale_l_ultima_riga() {
        let records = [label("a.md", 1, None, "vecchia"), label("a.md", 1, None, "nuova")];
        let pairing = pair(&records, &output(vec![item("a.md", 1, "h")]));
        assert_eq!(pairing.matched.len(), 1);
        assert_eq!(pairing.matched[0].gold["nature"], "nuova");
    }
}
