//! Ingresso e uscita comuni ai classificatori.

use serde_json::{Map, Value, json};

use crate::markdown::Kind;
use crate::schema::Schema;

/// Contesto facoltativo di un'unità.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Context {
    pub side: Option<String>,
    pub doc_type: Option<String>,
    pub kind: Option<Kind>,
    pub heading_path: Vec<String>,
    pub clause: Option<String>,
    pub prev: Option<String>,
    pub next: Option<String>,
}

/// Etichette assegnate a un'unità; una faccetta in astensione è assente.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Decision {
    pub labels: Map<String, Value>,
    pub confidence: Map<String, Value>,
    pub rule_ids: Map<String, Value>,
}

impl Decision {
    /// Applica i vincoli `x-fixedWhen` e dispone le faccette nell'ordine dello schema (REQ-096).
    pub fn finish(mut self, schema: &Schema, side: Option<&str>) -> Decision {
        for facet in &schema.facets {
            if let Some(fixed) = facet.fixed.as_ref().filter(|f| Some(f.side.as_str()) == side) {
                self.labels.insert(facet.name.clone(), json!(fixed.value));
                self.confidence.insert(facet.name.clone(), json!(1.0));
                self.rule_ids.insert(facet.name.clone(), json!("schema x-fixedWhen"));
            }
        }
        Decision {
            labels: in_schema_order(self.labels, schema),
            confidence: in_schema_order(self.confidence, schema),
            rule_ids: in_schema_order(self.rule_ids, schema),
        }
    }
}

fn in_schema_order(mut values: Map<String, Value>, schema: &Schema) -> Map<String, Value> {
    let mut ordered: Map<String, Value> =
        schema.facets.iter().filter_map(|f| Some((f.name.clone(), values.shift_remove(&f.name)?))).collect();
    ordered.append(&mut values);
    ordered
}
