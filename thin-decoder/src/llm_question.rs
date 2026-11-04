//! Interpretazione con LLM delle domande che le regole non riconoscono: il
//! modello propone un pattern vincolato da schema JSON, che si scarta se una
//! sua parola non compare nella domanda né nell'ontologia.

use serde::Deserialize;
use serde_json::{json, Value};

use crate::extract::{Phrase, RawClaim};
use crate::llm::Llm;
use crate::llm_extract::relation_of;
use crate::ontology::Ontology;
use crate::question::{Expect, Pattern, Var};
use crate::text::{tokenize, Kind};

#[derive(Debug, Deserialize)]
struct LlmPattern {
    subject: String,
    kind: String,
    attribute: String,
    relation: String,
    object: String,
    variable: String,
    expected: String,
}

fn schema() -> Value {
    let s = json!({"type": "string"});
    json!({
        "type": "object",
        "properties": {
            "subject": s, "attribute": s, "relation": s, "object": s,
            "kind": {"type": "string", "enum": ["attribute", "is_a", "value", "verb"]},
            "variable": {"type": "string", "enum": ["subject", "object", "none"]},
            "expected": {"type": "string", "enum": ["any", "quantity", "date", "location"]},
        },
        "required": ["subject", "kind", "attribute", "relation", "object", "variable", "expected"],
    })
}

fn prompt(question: &str) -> String {
    format!(
        "Rewrite the question as a triple (subject, relation, object) with one unknown.\n\
         variable: \"subject\" or \"object\" for the unknown part (leave it empty), \"none\" for a \
         yes/no question. kind: \"attribute\" for an attribute of the subject such as mass or \
         voltage (put it in attribute and leave object empty when it is the unknown); \"is_a\", \
         \"value\" or \"verb\" otherwise (put the verb and its preposition in relation).\n\
         expected: the type of the answer. Use only words of the question.\n\
         Example: \"Tell me the supply voltage of the OBC.\" → {{\"subject\":\"OBC\",\"kind\":\"attribute\",\
         \"attribute\":\"supply voltage\",\"relation\":\"\",\"object\":\"\",\"variable\":\"object\",\
         \"expected\":\"quantity\"}}\n\nQuestion: {question}"
    )
}

/// Parole del pattern assenti sia dalla domanda sia dall'ontologia; attributo
/// e relazione contano solo per il `kind` che li usa.
fn unknown_words(p: &LlmPattern, question: &str, onto: &Ontology) -> Vec<String> {
    let q: Vec<String> = tokenize(question).into_iter().map(|t| t.lower).collect();
    let mut parts = vec![&p.subject, &p.object];
    match p.kind.as_str() {
        "attribute" => parts.push(&p.attribute),
        "verb" => parts.push(&p.relation),
        _ => {}
    }
    parts
        .iter()
        .flat_map(|s| tokenize(s))
        .filter(|t| t.kind == Kind::Word && !q.contains(&t.lower) && !onto.knows(&t.stem))
        .map(|t| t.text)
        .collect()
}

fn phrase(text: &str) -> Phrase {
    Phrase::from_tokens(text, &tokenize(text))
}

/// Pattern proposto dall'LLM, oppure `None` se non supera i controlli.
pub fn pattern(llm: &Llm, onto: &Ontology, question: &str) -> Result<Option<Pattern>, String> {
    let raw = llm.chat(&prompt(question), &schema())?;
    let p: LlmPattern =
        serde_json::from_value(raw).map_err(|e| format!("pattern LLM non conforme allo schema: {e}"))?;
    let unknown = unknown_words(&p, question, onto);
    if !unknown.is_empty() {
        eprintln!("pattern LLM scartato («{question}»): parole fuori da domanda e ontologia: {}", unknown.join(", "));
        return Ok(None);
    }
    let Some(rel) = relation_of(&p.kind, &p.attribute, &p.relation) else {
        eprintln!("pattern LLM scartato («{question}»): relazione «{}» non riconosciuta", p.relation);
        return Ok(None);
    };
    let var = match p.variable.as_str() {
        "subject" => Var::Subj,
        "none" => Var::None,
        _ => Var::Obj,
    };
    let expect = match (var, p.expected.as_str()) {
        (Var::None, _) => Expect::Bool,
        (_, "quantity") => Expect::Quantity(None),
        (_, "date") => Expect::Date,
        (_, "location") => Expect::Location,
        _ => Expect::Any,
    };
    let claim = RawClaim {
        subj: if var == Var::Subj { Phrase::default() } else { phrase(&p.subject) },
        rel,
        obj: if var == Var::Obj { Phrase::default() } else { phrase(&p.object) },
        negated: false,
        modality: None,
        start: 0,
        end: 0,
    };
    Ok(Some(Pattern {
        kind: "llm",
        claims: vec![claim],
        var,
        expect,
        attr_hints: Vec::new(),
        class: None,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::answer::{answer_with, Kb};
    use crate::extract::extract;
    use crate::llm::{fake, test_config};
    use crate::ontology;
    use crate::output::Status;
    use crate::question;
    use crate::text::split_sentences;

    const TEXT: &str = "The reaction wheel (RW) has a mass of 2 kg.";
    const QUESTION: &str = "Tell me the mass of the RW.";

    fn reply(attribute: &str, cache: &str) -> (Option<Pattern>, Ontology) {
        let s = split_sentences(TEXT, tokenize(TEXT));
        let claims = extract(TEXT, &s, "t");
        let mut onto = ontology::from_text(TEXT, &s, &claims);
        onto.add_defaults();
        let r = json!({"subject": "RW", "kind": "attribute", "attribute": attribute, "relation": "has",
                       "object": "", "variable": "object", "expected": "quantity"});
        let server = fake::start(move |_| r.clone());
        let llm = Llm::new(test_config(&server.url, cache));
        (pattern(&llm, &onto, QUESTION).unwrap(), onto)
    }

    #[test]
    fn llm_pattern_answers_a_question_the_rules_do_not_parse() {
        assert!(question::parse(QUESTION).is_none());
        let (p, onto) = reply("mass", "td_cache_question_ok");
        let p = p.expect("pattern accettato");
        assert_eq!((p.kind, p.var), ("llm", Var::Obj));
        let s = split_sentences(TEXT, tokenize(TEXT));
        let claims = extract(TEXT, &s, "t");
        let kb = Kb { src: TEXT, sentences: &s, claims: &claims, onto: &onto };
        let qa = answer_with(&kb, QUESTION, Some(p));
        assert_eq!((qa.answer.as_str(), qa.status), ("2 kg", Status::Answered));
        // REQ-083: claim blu, ma pattern dall'LLM → rosso.
        assert_eq!(qa.color, crate::color::Color::Rosso);
    }

    #[test]
    fn words_outside_question_and_ontology_discard_the_pattern() {
        // "weight" non è nella domanda ma è sinonimo predefinito di "mass".
        assert!(reply("weight", "td_cache_question_onto").0.is_some());
        assert!(reply("thrust", "td_cache_question_bad").0.is_none());
    }
}
