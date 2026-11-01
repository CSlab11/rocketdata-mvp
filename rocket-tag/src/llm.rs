//! Classificatore LLM: prompt, formato di uscita e chiamate al server locale (appendice D).

use serde_json::Value;

use crate::schema::Schema;

/// Messaggio di sistema, ricavato dallo schema, da `prompt_guide.md` e da `guide_examples.json`
/// (REQ-108).
pub fn system_prompt(schema: &Schema, guide: &str, examples: &[Value]) -> String {
    let mut lines = vec![guide.trim().to_string(), String::new(), "Facets and allowed values:".to_string()];
    for facet in &schema.facets {
        let choice = if facet.multi { "list of" } else { "one of" };
        lines.push(format!("\n{} ({choice}):", facet.name));
        for value in &facet.values {
            lines.push(match facet.descriptions.get(value) {
                Some(description) => format!("  - {value}: {description}"),
                None => format!("  - {value}"),
            });
        }
    }
    if !examples.is_empty() {
        lines.push(String::new());
        lines.push("Examples (side | text -> labels):".to_string());
        for example in examples {
            let side = example["side"].as_str().unwrap_or_default();
            let text = example["text"].as_str().unwrap_or_default();
            lines.push(format!("  [{side}] {text} -> {}", example_labels(schema, &example["labels"])));
        }
    }
    let names: Vec<&str> = schema.facets.iter().map(|f| f.name.as_str()).collect();
    lines.push(String::new());
    lines.push(format!("Answer with a single JSON object containing exactly these keys: {}.", names.join(", ")));
    lines.join("\n")
}

/// Solo le coppie faccetta/valore presenti nello schema corrente.
fn example_labels(schema: &Schema, labels: &Value) -> String {
    let pairs = labels.as_object().into_iter().flatten().filter_map(|(name, value)| {
        let value = value.as_str()?;
        schema.facet(name)?.values.iter().any(|v| v == value).then(|| format!("{name}={value}"))
    });
    pairs.collect::<Vec<_>>().join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const SCHEMA: &str = include_str!("../config/label_schema.json");
    const GUIDE: &str = include_str!("../config/prompt_guide.md");
    const EXAMPLES: &str = include_str!("../config/guide_examples.json");

    fn schema() -> Schema {
        Schema::parse(SCHEMA).unwrap()
    }

    #[test]
    fn messaggio_di_sistema_nell_ordine_dell_appendice_d() {
        let examples: Vec<Value> = serde_json::from_str(EXAMPLES).unwrap();
        let system = system_prompt(&schema(), GUIDE, &examples);
        let at = |text: &str| system.find(text).unwrap_or_else(|| panic!("{text}"));
        assert!(system.starts_with(GUIDE.trim()));
        assert!(at("Facets and allowed values:") < at("nature (one of):"));
        assert!(at("nature (one of):") < at("flags (list of):"));
        assert!(at("  - requirement: Binding obligation") > at("nature (one of):"));
        assert!(at("flags (list of):") < at("Examples (side | text -> labels):"));
        assert!(at("  [prime] The unit shall operate between") > at("Examples (side | text -> labels):"));
        assert!(system.contains("-> nature=requirement, domain=technical, destination=P, required_action=declare_compliance"));
        assert!(system.ends_with(
            "Answer with a single JSON object containing exactly these keys: nature, domain, destination, required_action, extraction_quality, flags."
        ));
    }

    #[test]
    fn gli_esempi_ignorano_faccette_e_valori_assenti_dallo_schema() {
        let examples = [json!({"side": "prime", "text": "T", "labels": {"nature": "requirement", "domain": "inventato", "vecchia": "x"}})];
        let system = system_prompt(&schema(), "Guida", &examples);
        assert!(system.contains("  [prime] T -> nature=requirement\n"), "{system}");
    }

    #[test]
    fn senza_esempi_la_sezione_manca() {
        assert!(!system_prompt(&schema(), "Guida", &[]).contains("Examples"));
    }
}
