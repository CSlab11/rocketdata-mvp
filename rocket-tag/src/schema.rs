//! Schema delle etichette (appendice B): unica fonte di faccette e valori.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde_json::{Map, Value};

/// Valore obbligato quando il documento ha un certo lato (`x-fixedWhen`).
#[derive(Debug, Clone, PartialEq)]
pub struct Fixed {
    pub side: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Facet {
    pub name: String,
    /// Scelta multipla: il valore è una lista.
    pub multi: bool,
    pub values: Vec<String>,
    pub descriptions: HashMap<String, String>,
    pub required: bool,
    /// Valore proposto (`x-default`).
    pub default: Option<Value>,
    pub fixed: Option<Fixed>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Schema {
    pub version: String,
    /// Nell'ordine in cui vanno valutate e mostrate.
    pub facets: Vec<Facet>,
    pub sides: Vec<String>,
    pub doc_types: Vec<String>,
}

impl Schema {
    /// Faccette, valori, descrizioni e vincoli ricavati dal testo dello schema (REQ-095).
    pub fn parse(text: &str) -> Result<Schema, String> {
        let raw: Value = serde_json::from_str(text).map_err(|e| format!("schema: JSON non valido: {e}"))?;
        Schema::from_value(&raw)
    }

    /// Schema letto dal file indicato, mai da un percorso fisso (REQ-092).
    pub fn load(path: &Path) -> Result<Schema, String> {
        let text = fs::read_to_string(path).map_err(|e| format!("impossibile leggere {}: {e}", path.display()))?;
        Schema::parse(&text)
    }

    /// Schema da un oggetto già caricato (REQ-092).
    pub fn from_value(raw: &Value) -> Result<Schema, String> {
        let facets_node = raw.pointer("/$defs/ParagraphFacets").ok_or("schema: $defs/ParagraphFacets mancante")?;
        let required: Vec<&str> = facets_node["required"].as_array().into_iter().flatten().filter_map(Value::as_str).collect();
        let properties = facets_node["properties"].as_object().ok_or("schema: ParagraphFacets senza properties")?;
        let mut facets = Vec::new();
        for (name, property) in properties {
            facets.push(facet(raw, name, property, required.contains(&name.as_str()))?);
        }
        let schema = Schema {
            version: raw["version"].as_str().unwrap_or("0").to_string(),
            facets,
            sides: document_values(raw, "side"),
            doc_types: document_values(raw, "doc_type"),
        };
        schema.check()?;
        Ok(schema)
    }

    /// Vincoli e valori proposti devono essere ammessi dallo schema stesso (REQ-098).
    fn check(&self) -> Result<(), String> {
        for facet in &self.facets {
            if let Some(fixed) = &facet.fixed {
                if !self.sides.contains(&fixed.side) {
                    return Err(format!("schema: {}: x-fixedWhen con lato non ammesso '{}'", facet.name, fixed.side));
                }
                if !facet.values.contains(&fixed.value) {
                    return Err(format!("schema: {}: x-fixedWhen con valore non ammesso '{}'", facet.name, fixed.value));
                }
            }
            let defaults = match &facet.default {
                Some(Value::Array(values)) => values.clone(),
                Some(value) => vec![value.clone()],
                None => Vec::new(),
            };
            for value in defaults {
                if !value.as_str().is_some_and(|v| facet.values.iter().any(|allowed| allowed == v)) {
                    return Err(format!("schema: {}: x-default con valore non ammesso {value}", facet.name));
                }
            }
        }
        Ok(())
    }

    /// Errori di un insieme di etichette rispetto allo schema: faccette obbligatorie mancanti,
    /// valori non ammessi, faccette in più e vincoli per il lato del documento.
    pub fn label_errors(&self, labels: &Map<String, Value>, side: Option<&str>) -> Vec<String> {
        let mut errors = Vec::new();
        for facet in &self.facets {
            let Some(value) = labels.get(&facet.name).filter(|v| !v.is_null()) else {
                if facet.required {
                    errors.push(format!("{}: mancante", facet.name));
                }
                continue;
            };
            let values = match (facet.multi, value) {
                (true, Value::Array(values)) => values.iter().collect(),
                (true, _) => {
                    errors.push(format!("{}: deve essere una lista", facet.name));
                    continue;
                }
                (false, value) => vec![value],
            };
            for value in values {
                if !value.as_str().is_some_and(|v| facet.values.iter().any(|allowed| allowed == v)) {
                    errors.push(format!("{}: valore non ammesso {value}", facet.name));
                }
            }
            if let Some(fixed) = facet.fixed.as_ref().filter(|f| Some(f.side.as_str()) == side) {
                if value.as_str() != Some(fixed.value.as_str()) {
                    errors.push(format!("{}: per side={} deve essere '{}'", facet.name, fixed.side, fixed.value));
                }
            }
        }
        for name in labels.keys().filter(|name| self.facet(name).is_none()) {
            errors.push(format!("faccetta non prevista: {name}"));
        }
        errors
    }

    pub fn facet(&self, name: &str) -> Option<&Facet> {
        self.facets.iter().find(|f| f.name == name)
    }
}

fn facet(raw: &Value, name: &str, property: &Value, required: bool) -> Result<Facet, String> {
    let multi = property["type"] == "array";
    let node = if multi { &property["items"] } else { property };
    let not_enum = || format!("schema: la faccetta '{name}' non punta a un enum");
    let enum_node = resolve(raw, node).filter(|n| n["enum"].is_array()).ok_or_else(not_enum)?;
    let values = strings(&enum_node["enum"]);
    let descriptions = enum_node["x-enumDescriptions"]
        .as_object()
        .into_iter()
        .flatten()
        .filter_map(|(k, v)| Some((k.clone(), v.as_str()?.to_string())))
        .collect();
    let fixed = property.get("x-fixedWhen").map(|f| Fixed {
        side: f["side"].as_str().unwrap_or_default().to_string(),
        value: f["value"].as_str().unwrap_or_default().to_string(),
    });
    Ok(Facet { name: name.to_string(), multi, values, descriptions, required, default: property.get("x-default").cloned(), fixed })
}

/// Il nodo stesso, o l'enum a cui punta il suo `$ref`.
fn resolve<'a>(raw: &'a Value, node: &'a Value) -> Option<&'a Value> {
    match node["$ref"].as_str() {
        Some(reference) => raw.pointer(reference.strip_prefix('#')?),
        None => Some(node),
    }
}

fn document_values(raw: &Value, field: &str) -> Vec<String> {
    let node = raw.pointer(&format!("/$defs/DocumentLabel/properties/{field}"));
    node.and_then(|n| resolve(raw, n)).map_or_else(Vec::new, |n| strings(&n["enum"]))
}

fn strings(values: &Value) -> Vec<String> {
    values.as_array().into_iter().flatten().filter_map(Value::as_str).map(str::to_string).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCHEMA: &str = include_str!("../config/label_schema.json");

    #[test]
    fn legge_le_faccette_nell_ordine_dello_schema() {
        let schema = Schema::parse(SCHEMA).unwrap();
        assert_eq!(schema.version, "0.1.0");
        let names: Vec<_> = schema.facets.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["nature", "domain", "destination", "required_action", "extraction_quality", "flags"]);
        assert_eq!(schema.sides, ["prime", "supplier"]);
        assert!(schema.doc_types.contains(&"pa_plan".to_string()));
    }

    #[test]
    fn schema_da_file_o_da_oggetto_gia_caricato() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("altro_nome.json");
        fs::write(&path, SCHEMA).unwrap();
        let from_file = Schema::load(&path).unwrap();
        let raw: Value = serde_json::from_str(SCHEMA).unwrap();
        assert_eq!(from_file, Schema::from_value(&raw).unwrap());
        assert!(Schema::load(&dir.path().join("manca.json")).unwrap_err().contains("impossibile leggere"));
    }

    fn broken(from: &str, to: &str) -> String {
        assert!(SCHEMA.contains(from), "{from}");
        Schema::parse(&SCHEMA.replace(from, to)).unwrap_err()
    }

    #[test]
    fn segnala_gli_schemi_non_conformi() {
        assert!(Schema::parse("{ non json").unwrap_err().contains("JSON non valido"));
        assert!(broken("\"ParagraphFacets\": {", "\"Altro\": {").contains("ParagraphFacets mancante"));
        assert!(broken("\"$ref\": \"#/$defs/Nature\"", "\"type\": \"string\"").contains("'nature' non punta a un enum"));
        assert!(broken("\"side\": \"supplier\",\n            \"value\": \"none\"", "\"side\": \"altro\",\n            \"value\": \"none\"").contains("lato non ammesso"));
        assert!(broken("\"side\": \"supplier\",\n            \"value\": \"none\"", "\"side\": \"supplier\",\n            \"value\": \"mai\"").contains("valore non ammesso"));
        assert!(broken("\"x-default\": \"ok\"", "\"x-default\": \"mai\"").contains("x-default"));
    }

    #[test]
    fn errori_delle_etichette() {
        let schema = Schema::parse(SCHEMA).unwrap();
        let good = serde_json::json!({"nature": "requirement", "domain": "technical", "destination": "P",
            "required_action": "declare_compliance", "extraction_quality": "ok", "flags": ["multiple_requirements"]});
        let labels = |change: &dyn Fn(&mut Value)| {
            let mut value = good.clone();
            change(&mut value);
            value.as_object().unwrap().clone()
        };
        let errors = |change: &dyn Fn(&mut Value), side| schema.label_errors(&labels(change), side);
        assert!(errors(&|_| {}, None).is_empty());
        assert_eq!(errors(&|v| { v.as_object_mut().unwrap().remove("nature"); }, None), ["nature: mancante"]);
        assert_eq!(errors(&|v| v["domain"] = "mai".into(), None), ["domain: valore non ammesso \"mai\""]);
        assert_eq!(errors(&|v| v["flags"] = "x".into(), None), ["flags: deve essere una lista"]);
        assert_eq!(errors(&|v| v["flags"] = serde_json::json!(["mai"]), None), ["flags: valore non ammesso \"mai\""]);
        assert_eq!(errors(&|v| v["altra"] = 1.into(), None), ["faccetta non prevista: altra"]);
        assert_eq!(errors(&|_| {}, Some("supplier")), ["required_action: per side=supplier deve essere 'none'"]);
    }

    #[test]
    fn legge_valori_descrizioni_default_e_vincoli() {
        let schema = Schema::parse(SCHEMA).unwrap();
        let nature = schema.facet("nature").unwrap();
        assert!(nature.required && !nature.multi);
        assert!(nature.values.contains(&"commitment".to_string()));
        assert!(nature.descriptions["definition"].starts_with("Acronym"));
        let flags = schema.facet("flags").unwrap();
        assert!(flags.multi);
        assert_eq!(flags.default, Some(serde_json::json!([])));
        assert_eq!(schema.facet("extraction_quality").unwrap().default, Some(serde_json::json!("ok")));
        let fixed = schema.facet("required_action").unwrap().fixed.clone().unwrap();
        assert_eq!((fixed.side.as_str(), fixed.value.as_str()), ("supplier", "none"));
    }
}
