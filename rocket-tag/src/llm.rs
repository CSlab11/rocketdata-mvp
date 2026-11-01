//! Classificatore LLM: prompt, formato di uscita e chiamate al server locale (appendice D).

use serde_json::Value;

use crate::classifier::Context;
use crate::markdown::Kind;
use crate::schema::Schema;

/// Lunghezza massima in caratteri del contesto e dell'unità nel messaggio utente.
const CONTEXT_CHARS: usize = 400;
const UNIT_CHARS: usize = 2500;

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

/// Messaggio utente, con le unità vicine quando sono fornite (REQ-094).
pub fn user_prompt(document: &str, text: &str, context: &Context) -> String {
    let unknown = |value: &Option<String>| value.clone().unwrap_or_else(|| "unknown".to_string());
    let headings = if context.heading_path.is_empty() { "(none)".to_string() } else { context.heading_path.join(" > ") };
    let mut lines = vec![
        format!("Document: {document}"),
        format!("Document side: {}; type: {}", unknown(&context.side), unknown(&context.doc_type)),
        format!("Heading path: {headings}"),
        format!("Unit kind: {}", context.kind.map_or("unknown", Kind::as_str)),
    ];
    if let Some(prev) = &context.prev {
        lines.push(format!("Previous unit (context): {}", first_chars(prev, CONTEXT_CHARS)));
    }
    lines.push(format!("UNIT TO LABEL: {}", first_chars(text, UNIT_CHARS)));
    if let Some(next) = &context.next {
        lines.push(format!("Next unit (context): {}", first_chars(next, CONTEXT_CHARS)));
    }
    lines.join("\n")
}

fn first_chars(text: &str, count: usize) -> String {
    text.chars().take(count).collect()
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
    fn messaggio_utente_con_le_unita_vicine() {
        let context = Context {
            side: Some("prime".into()),
            doc_type: Some("pa_plan".into()),
            kind: Some(Kind::TableRow),
            heading_path: vec!["4 Verification".into(), "4.2 Tests".into()],
            prev: Some("prima".into()),
            next: Some("dopo".into()),
            ..Context::default()
        };
        let user = user_prompt("a.md", "testo", &context);
        assert_eq!(
            user.lines().collect::<Vec<_>>(),
            [
                "Document: a.md",
                "Document side: prime; type: pa_plan",
                "Heading path: 4 Verification > 4.2 Tests",
                "Unit kind: table_row",
                "Previous unit (context): prima",
                "UNIT TO LABEL: testo",
                "Next unit (context): dopo",
            ]
        );
    }

    #[test]
    fn messaggio_utente_senza_contesto_e_con_testo_lungo() {
        let long = "è".repeat(3000);
        let user = user_prompt("a.md", &long, &Context { prev: Some(long.clone()), ..Context::default() });
        let lines: Vec<_> = user.lines().collect();
        assert_eq!(lines[1], "Document side: unknown; type: unknown");
        assert_eq!(lines[2], "Heading path: (none)");
        assert_eq!(lines[3], "Unit kind: unknown");
        assert_eq!(lines[4].chars().count(), "Previous unit (context): ".len() + 400);
        assert_eq!(lines[5].chars().count(), "UNIT TO LABEL: ".len() + 2500);
        assert_eq!(lines.len(), 6);
    }

    #[test]
    fn senza_esempi_la_sezione_manca() {
        assert!(!system_prompt(&schema(), "Guida", &[]).contains("Examples"));
    }
}
