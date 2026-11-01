//! Ingresso e uscita comuni ai classificatori.

use serde_json::{Map, Value};

use crate::markdown::Kind;

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
