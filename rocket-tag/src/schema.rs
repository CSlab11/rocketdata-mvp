//! Schema delle etichette (appendice B): unica fonte di faccette e valori.

use std::collections::HashMap;

use serde_json::Value;

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
        Schema::read(&raw)
    }

    fn read(raw: &Value) -> Result<Schema, String> {
        let facets_node = raw.pointer("/$defs/ParagraphFacets").ok_or("schema: $defs/ParagraphFacets mancante")?;
        let required: Vec<&str> = facets_node["required"].as_array().into_iter().flatten().filter_map(Value::as_str).collect();
        let properties = facets_node["properties"].as_object().ok_or("schema: ParagraphFacets senza properties")?;
        let mut facets = Vec::new();
        for (name, property) in properties {
            facets.push(facet(raw, name, property, required.contains(&name.as_str()))?);
        }
        Ok(Schema {
            version: raw["version"].as_str().unwrap_or("0").to_string(),
            facets,
            sides: document_values(raw, "side"),
            doc_types: document_values(raw, "doc_type"),
        })
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
