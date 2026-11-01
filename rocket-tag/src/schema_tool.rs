//! Manutenzione dello schema: impatto di una modifica sulle etichette e sulle regole.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::labels;
use crate::rules::RuleSet;
use crate::schema::Schema;

/// Differenze tra due versioni dello schema.
pub fn diff(old: &Schema, new: &Schema) -> Vec<String> {
    let mut lines = vec![format!("v{} -> v{}", old.version, new.version)];
    if old.version == new.version {
        lines.push("  ATTENZIONE: la versione non è cambiata".to_string());
    }
    for facet in new.facets.iter().filter(|f| old.facet(&f.name).is_none()) {
        lines.push(format!("  + faccetta {}: {:?}", facet.name, facet.values));
    }
    for facet in old.facets.iter().filter(|f| new.facet(&f.name).is_none()) {
        lines.push(format!("  - faccetta {}", facet.name));
    }
    for facet in &new.facets {
        let Some(before) = old.facet(&facet.name) else { continue };
        lines.extend(values_change(&facet.name, &before.values, &facet.values));
        if before.required != facet.required {
            lines.push(format!("  ~ {}: obbligatoria {} -> {}", facet.name, before.required, facet.required));
        }
    }
    lines.extend(values_change("documento.side", &old.sides, &new.sides));
    lines.extend(values_change("documento.doc_type", &old.doc_types, &new.doc_types));
    lines
}

fn values_change(name: &str, old: &[String], new: &[String]) -> Option<String> {
    let added: Vec<&String> = new.iter().filter(|v| !old.contains(v)).collect();
    let removed: Vec<&String> = old.iter().filter(|v| !new.contains(v)).collect();
    (!added.is_empty() || !removed.is_empty()).then(|| format!("  ~ {name}: +{added:?} -{removed:?}"))
}

/// Etichette di paragrafo valide e da rivedere per uno schema.
#[derive(Debug, PartialEq)]
pub struct Status {
    pub valid: usize,
    pub to_review: usize,
    pub documents: usize,
}

/// Conta l'ultima etichetta di ogni unità.
pub fn status(schema: &Schema, records: &[Value]) -> Status {
    let documents = labels::documents(records);
    let mut latest = BTreeMap::new();
    for record in records.iter().filter(|r| r["level"] == "paragraph") {
        let key = (record["document"].to_string(), record["line_start"].to_string(), record["line_end"].to_string());
        latest.insert(key, record);
    }
    let to_review = latest
        .values()
        .filter(|record| {
            let side = record["document"].as_str().and_then(|d| documents.get(d)).and_then(|info| info.side.as_deref());
            let facets = record["facets"].as_object().cloned().unwrap_or_default();
            !schema.label_errors(&facets, side).is_empty()
        })
        .count();
    Status { valid: latest.len() - to_review, to_review, documents: documents.len() }
}

/// Rapporto d'impatto: differenze, etichette da rivedere e incoerenze delle regole (REQ-122).
pub fn impact(old: &Schema, new: &Schema, records: Option<&[Value]>, rules: Option<&RuleSet>) -> Vec<String> {
    let mut lines = vec!["== Differenze".to_string()];
    lines.extend(diff(old, new));
    if let Some(records) = records {
        let status = status(new, records);
        lines.push(String::new());
        lines.push("== Etichette con il nuovo schema (senza migrazione)".to_string());
        lines.push(format!("  {} valide, {} da rivedere", status.valid, status.to_review));
    }
    if let Some(rules) = rules {
        let check = rules.check(new);
        lines.push(String::new());
        lines.push("== Regole".to_string());
        lines.extend(check.warnings.iter().map(|warning| format!("  avviso: {warning}")));
        lines.extend(check.errors.iter().map(|error| format!("  ERRORE: {error}")));
        if check.errors.is_empty() {
            lines.push("  rules.yaml coerente".to_string());
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const SCHEMA: &str = include_str!("../config/label_schema.json");

    fn schema() -> Schema {
        Schema::parse(SCHEMA).unwrap()
    }

    fn changed() -> Schema {
        let text = SCHEMA
            .replace("\"version\": \"0.1.0\"", "\"version\": \"0.2.0\"")
            .replace("\"bidder_instruction\",\n        \"context\"", "\"instruction\",\n        \"context\"")
            .replace("\"flags\"\n      ],", "\"flags\",\n        \"nuova\"\n      ],")
            .replace("\"nature\": {\n          \"$ref\": \"#/$defs/Nature\"\n        },", "\"nature\": {\n          \"$ref\": \"#/$defs/Nature\"\n        },\n        \"nuova\": {\n          \"$ref\": \"#/$defs/Side\"\n        },")
            .replace("\"extraction_quality\",\n        \"flags\",", "\"flags\",")
            .replace("\"schedule\",\n        \"deliverables\",", "\"deliverables\",");
        Schema::parse(&text).unwrap()
    }

    fn label(nature: &str) -> Value {
        json!({"level": "paragraph", "document": "a.md", "line_start": 1, "line_end": 1,
               "facets": {"nature": nature, "domain": "technical", "destination": "P",
                          "required_action": "none", "extraction_quality": "ok", "flags": []}})
    }

    #[test]
    fn differenze_tra_due_versioni() {
        let lines = diff(&schema(), &changed());
        assert_eq!(lines[0], "v0.1.0 -> v0.2.0");
        let has = |text: &str| lines.iter().any(|l| l.contains(text));
        assert!(has("+ faccetta nuova"), "{lines:?}");
        assert!(has("~ nature: +[\"instruction\"] -[\"bidder_instruction\"]"), "{lines:?}");
        assert!(has("~ domain: +[] -[\"schedule\"]"), "{lines:?}");
        assert!(has("~ extraction_quality: obbligatoria true -> false"), "{lines:?}");
    }

    #[test]
    fn stessa_versione_e_faccetta_tolta() {
        let text = SCHEMA.replace("\"flags\": {\n          \"type\": \"array\",\n          \"items\": {\n            \"$ref\": \"#/$defs/Flag\"\n          },\n          \"uniqueItems\": true,\n          \"x-default\": []\n        }", "\"altra\": {\n          \"$ref\": \"#/$defs/Flag\"\n        }");
        let lines = diff(&schema(), &Schema::parse(&text).unwrap());
        assert!(lines.iter().any(|l| l.contains("la versione non è cambiata")));
        assert!(lines.iter().any(|l| l.contains("- faccetta flags")));
    }

    #[test]
    fn etichette_da_rivedere_con_il_nuovo_schema() {
        let records = [label("requirement"), json!({"level": "document", "document": "a.md", "side": "prime"})];
        assert_eq!(status(&schema(), &records), Status { valid: 1, to_review: 0, documents: 1 });
        assert_eq!(status(&changed(), &records), Status { valid: 0, to_review: 1, documents: 1 }, "manca la faccetta nuova");
    }

    #[test]
    fn rapporto_d_impatto_completo() {
        let rules = RuleSet::parse(include_str!("../config/rules.yaml")).unwrap();
        let lines = impact(&schema(), &changed(), Some(&[label("requirement")]), Some(&rules));
        let text = lines.join("\n");
        assert!(text.contains("== Differenze\nv0.1.0 -> v0.2.0"));
        assert!(text.contains("== Etichette con il nuovo schema (senza migrazione)\n  0 valide, 1 da rivedere"));
        assert!(text.contains("ERRORE: nature: valore 'bidder_instruction' non esiste"), "{text}");
        assert!(text.contains("ERRORE: domain: valore 'schedule' non esiste"), "{text}");
    }
}
