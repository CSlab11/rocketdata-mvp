//! Motore LLM del secondo passaggio: risponde alla domanda a partire da tutti
//! i claim del primo passaggio, citando gli ID usati come premesse. Una
//! risposta che non cita claim esistenti è scartata e diventa una lacuna.

use serde::Deserialize;
use serde_json::{json, Value};

use crate::answer::{cited, evidence, Kb};
use crate::color::Color;
use crate::llm::Llm;
use crate::output::{Engine, Evidence, Status, QA};

#[derive(Debug, Deserialize)]
struct LlmAnswer {
    answer: String,
    premises: Vec<String>,
}

fn schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "answer": {"type": "string"},
            "premises": {"type": "array", "items": {"type": "string"}},
        },
        "required": ["answer", "premises"],
    })
}

fn prompt(kb: &Kb, question: &str) -> String {
    let claims: Vec<String> = kb
        .claims
        .iter()
        .map(|c| {
            let neg = if c.negated { " (negated)" } else { "" };
            format!("{}: {} | {} | {}{neg} — \"{}\"", c.id, c.subj.text, c.rel.label(), c.obj.text, &kb.src[c.start..c.end])
        })
        .collect();
    format!(
        "Answer the question using only the claims below. Put in premises the IDs of every claim \
         the answer relies on. If the claims do not contain the answer, return an empty answer \
         and no premises.\n\nClaims:\n{}\n\nQuestion: {question}",
        claims.join("\n")
    )
}

pub fn answer(llm: &Llm, kb: &Kb, question: &str) -> Result<QA, String> {
    let raw = llm.chat(&prompt(kb, question), &schema())?;
    let proposed: LlmAnswer =
        serde_json::from_value(raw).map_err(|e| format!("risposta LLM non conforme allo schema: {e}"))?;
    let mut qa = QA::not_found(question);
    qa.engine = Engine::Llm;
    qa.color = Color::Rosso;
    if proposed.answer.trim().is_empty() {
        return Ok(qa);
    }
    let mut candidate = qa.clone();
    candidate.answer = proposed.answer.trim().to_string();
    candidate.status = Status::Answered;
    for id in &proposed.premises {
        // Un ID sconosciuto resta come evidenza vuota: `cited` lo rifiuta.
        candidate.evidence.push(match kb.claims.iter().find(|c| &c.id == id) {
            Some(c) => evidence(kb, c),
            None => Evidence { claim: Some(id.clone()), ..Evidence::default() },
        });
    }
    if !proposed.premises.is_empty() {
        candidate.reasoning = vec![format!("llm engine: premises {}", proposed.premises.join(", "))];
    }
    match cited(&candidate, kb.claims) {
        Ok(()) => {
            let used = candidate.evidence.iter().filter_map(|e| e.claim_color);
            candidate.color = Color::of_llm(used);
            Ok(candidate)
        }
        Err(why) => {
            qa.reasoning = vec![format!("llm answer «{}» discarded: {why}", candidate.answer)];
            Ok(qa)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract::extract;
    use crate::llm::{fake, test_config};
    use crate::ontology;
    use crate::text::{split_sentences, tokenize};

    fn ask(reply: Value, cache: &str) -> (QA, Value) {
        let text = "The RW has a mass of 2 kg. The OBC is connected to the bus.";
        let s = split_sentences(text, tokenize(text));
        let claims = extract(text, &s, "t");
        let onto = ontology::from_text(text, &s, &claims);
        let server = fake::start(move |_| reply.clone());
        let llm = Llm::new(test_config(&server.url, cache));
        let kb = Kb { src: text, sentences: &s, claims: &claims, onto: &onto };
        let qa = answer(&llm, &kb, "What is the mass of the RW?").unwrap();
        let req = server.requests.lock().unwrap()[0].clone();
        (qa, req)
    }

    #[test]
    fn answer_with_premises_receives_all_claims() {
        let (qa, req) = ask(json!({"answer": "2 kg", "premises": ["C1"]}), "td_cache_answer_ok");
        assert_eq!((qa.answer.as_str(), qa.status, qa.engine), ("2 kg", Status::Answered, Engine::Llm));
        assert_eq!(qa.evidence[0].claim.as_deref(), Some("C1"));
        assert_eq!(qa.color, Color::Giallo);
        let prompt = req["messages"][0]["content"].as_str().unwrap();
        assert!(prompt.contains("C1: RW | attr:mass | 2 kg") && prompt.contains("C2: OBC | connect_to | bus"));
    }

    #[test]
    fn uncited_answers_become_gaps() {
        let (qa, _) = ask(json!({"answer": "3 kg", "premises": []}), "td_cache_answer_none");
        assert_eq!(qa.status, Status::NotFound);
        assert!(qa.reasoning[0].contains("discarded"));
        let (qa, _) = ask(json!({"answer": "3 kg", "premises": ["C7"]}), "td_cache_answer_bad");
        assert_eq!(qa.status, Status::NotFound);
        assert!(qa.reasoning[0].contains("C7"));
        let (qa, _) = ask(json!({"answer": "", "premises": []}), "td_cache_answer_empty");
        assert_eq!(qa.status, Status::NotFound);
    }
}
