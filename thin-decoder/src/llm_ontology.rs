//! Proposta di ontologia con LLM, fuori linea: dal testo, sinonimi,
//! sottoclassi e corrispondenze verbo-attributo nel formato dell'ontologia a
//! priori. Il file va rivisto a mano prima di usarlo con `--ontology`.

use serde::Deserialize;
use serde_json::{json, Value};

use crate::llm::Llm;
use crate::ontology::OntologyFile;

#[derive(Debug, Deserialize)]
struct Proposal {
    synonyms: Vec<Vec<String>>,
    subclass_of: Vec<Vec<String>>,
    attribute_verbs: Vec<Vec<String>>,
}

fn schema() -> Value {
    let list = json!({"type": "array", "items": {"type": "string"}, "minItems": 2});
    let pair = json!({"type": "array", "items": {"type": "string"}, "minItems": 2, "maxItems": 2});
    json!({
        "type": "object",
        "properties": {
            "synonyms": {"type": "array", "items": list},
            "subclass_of": {"type": "array", "items": pair},
            "attribute_verbs": {"type": "array", "items": pair},
        },
        "required": ["synonyms", "subclass_of", "attribute_verbs"],
    })
}

fn prompt(text: &str) -> String {
    format!(
        "Propose a small ontology for the text below.\n\
         synonyms: groups of terms with the same meaning (e.g. [\"spacecraft\", \"satellite\"]).\n\
         subclass_of: [child, parent] pairs (e.g. [\"magnetorquer\", \"actuator\"]).\n\
         attribute_verbs: [verb, attribute] pairs where the verb expresses the attribute \
         (e.g. [\"draw\", \"power\"]). Use lowercase base forms.\n\nText:\n{text}"
    )
}

fn clean(groups: Vec<Vec<String>>) -> Vec<Vec<String>> {
    groups
        .into_iter()
        .map(|g| g.into_iter().map(|w| w.trim().to_string()).filter(|w| !w.is_empty()).collect::<Vec<_>>())
        .filter(|g| g.len() >= 2)
        .collect()
}

fn pairs(groups: Vec<Vec<String>>) -> Vec<(String, String)> {
    clean(groups).into_iter().filter(|g| g.len() == 2).map(|g| (g[0].clone(), g[1].clone())).collect()
}

pub fn propose(llm: &Llm, text: &str) -> Result<OntologyFile, String> {
    let raw = llm.chat(&prompt(text), &schema())?;
    let p: Proposal =
        serde_json::from_value(raw).map_err(|e| format!("ontologia LLM non conforme allo schema: {e}"))?;
    Ok(OntologyFile {
        synonyms: clean(p.synonyms),
        subclass_of: pairs(p.subclass_of),
        attribute_verbs: pairs(p.attribute_verbs),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::{fake, test_config};

    #[test]
    fn proposal_is_cleaned_into_the_ontology_file_format() {
        let server = fake::start(|_| {
            json!({"synonyms": [["spacecraft", "satellite"], ["alone"]],
                   "subclass_of": [["RW", "actuator"], ["a", "b", "c"]],
                   "attribute_verbs": [["draw", " power "]]})
        });
        let llm = Llm::new(test_config(&server.url, "td_cache_ontology"));
        let o = propose(&llm, "The RW is an actuator.").unwrap();
        assert_eq!(o.synonyms, [vec!["spacecraft".to_string(), "satellite".to_string()]]);
        assert_eq!(o.subclass_of, [("RW".to_string(), "actuator".to_string())]);
        assert_eq!(o.attribute_verbs, [("draw".to_string(), "power".to_string())]);
    }
}
