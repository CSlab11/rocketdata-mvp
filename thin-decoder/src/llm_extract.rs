//! Estrazione dei claim con l'LLM, frase per frase, con output vincolato da
//! schema JSON. L'LLM propone, le regole verificano: lo span deve comparire
//! letteralmente nella frase, soggetto e oggetto devono stare nello span.

use serde::Deserialize;
use serde_json::{json, Value};

use crate::color::Color;
use crate::config::Scope;
use crate::extract::{Claim, Phrase, Rel};
use crate::lexicon::{is_aux, is_in, MODALS, PREPOSITIONS};
use crate::llm::Llm;
use crate::text::{tokenize, Kind, Sentence};

#[derive(Debug, Deserialize)]
struct LlmClaim {
    kind: String,
    attribute: String,
    subject: String,
    relation: String,
    object: String,
    span: String,
    negated: bool,
}

#[derive(Debug, Deserialize)]
struct LlmClaims {
    claims: Vec<LlmClaim>,
}

fn schema() -> Value {
    let s = json!({"type": "string"});
    json!({
        "type": "object",
        "properties": {"claims": {"type": "array", "items": {
            "type": "object",
            "properties": {
                "kind": {"type": "string", "enum": ["attribute", "is_a", "value", "verb"]},
                "attribute": s, "subject": s, "relation": s, "object": s, "span": s,
                "negated": {"type": "boolean"},
            },
            "required": ["kind", "attribute", "subject", "relation", "object", "span", "negated"],
        }}},
        "required": ["claims"],
    })
}

fn prompt(sentence: &str) -> String {
    format!(
        "Extract every factual claim from the sentence as (subject, relation, object).\n\
         Copy subject, object and span verbatim from the sentence; the span is the shortest \
         part of the sentence containing subject, relation and object.\n\
         kind: \"attribute\" for \"the A of X is V\" or \"X has an A of V\" (put A in attribute, \
         V in object); \"is_a\" for \"X is a Y\"; \"value\" for \"X is V\"; \"verb\" otherwise \
         (put the verb and its preposition in relation, e.g. \"mounted on\").\n\
         Leave attribute empty unless kind is \"attribute\". Set negated for \"not\" or \"never\".\n\
         Example: \"The battery has a capacity of 40 Ah and is not connected to the heater.\" → \
         {{\"kind\":\"attribute\",\"attribute\":\"capacity\",\"subject\":\"battery\",\"relation\":\"has\",\
         \"object\":\"40 Ah\",\"span\":\"battery has a capacity of 40 Ah\",\"negated\":false}}, \
         {{\"kind\":\"verb\",\"attribute\":\"\",\"subject\":\"battery\",\"relation\":\"connected to\",\
         \"object\":\"heater\",\"span\":\"battery has a capacity of 40 Ah and is not connected to the heater\",\
         \"negated\":true}}\n\n\
         Sentence: {sentence}"
    )
}

/// Relazione proposta dall'LLM nello stesso formato delle regole, per
/// confrontarla con claim e pattern.
pub fn relation_of(kind: &str, attribute: &str, relation: &str) -> Option<Rel> {
    match kind {
        "attribute" => {
            let attr = Phrase::from_tokens(attribute, &tokenize(attribute));
            (!attr.key.is_empty()).then(|| Rel::Attr(attr.key_str()))
        }
        "is_a" => Some(Rel::IsA),
        "value" => Some(Rel::Be),
        _ => {
            let toks: Vec<_> = tokenize(relation).into_iter().filter(|t| t.kind == Kind::Word).collect();
            let verb = toks.iter().find(|t| !is_aux(&t.lower) && !is_in(MODALS, &t.lower) && t.lower != "not")?;
            let prep = toks.last().filter(|t| t.start > verb.start && is_in(PREPOSITIONS, &t.lower));
            Some(Rel::verb(&verb.stem, prep.map(|t| t.lower.as_str())))
        }
    }
}

/// Sintagma dai token della frase compresi nel tratto `text` che inizia in `from`.
fn phrase_at(src: &str, s: &Sentence, text: &str, from: usize) -> Option<Phrase> {
    let start = from + src[from..s.end].find(text)?;
    let end = start + text.len();
    let toks: Vec<_> = s.tokens.iter().filter(|t| t.start >= start && t.end <= end).cloned().collect();
    let p = Phrase::from_tokens(src, &toks);
    (!p.is_empty()).then_some(p)
}

/// Controlli deterministici su un claim proposto dall'LLM.
fn check(src: &str, s: &Sentence, c: &LlmClaim) -> Result<(usize, usize, Phrase, Rel, Phrase), String> {
    let sentence = &src[s.start..s.end];
    let at = sentence
        .find(&c.span)
        .filter(|_| !c.span.is_empty())
        .ok_or_else(|| format!("span «{}» non presente nella frase", c.span))?;
    let (start, end) = (s.start + at, s.start + at + c.span.len());
    let missing = |what: &str, t: &str| format!("{what} «{t}» non presente nello span «{}»", c.span);
    if c.subject.is_empty() || !c.span.contains(&c.subject) {
        return Err(missing("soggetto", &c.subject));
    }
    if c.object.is_empty() || !c.span.contains(&c.object) {
        return Err(missing("oggetto", &c.object));
    }
    let subj = phrase_at(src, s, &c.subject, start).ok_or_else(|| missing("soggetto", &c.subject))?;
    let obj = phrase_at(src, s, &c.object, start).ok_or_else(|| missing("oggetto", &c.object))?;
    let rel = relation_of(&c.kind, &c.attribute, &c.relation).ok_or_else(|| format!("relazione «{}» non riconosciuta", c.relation))?;
    Ok((start, end, subj, rel, obj))
}

/// Frasi da inviare all'LLM: tutte, oppure solo quelle senza claim delle regole.
pub fn sentences_for<'a>(scope: Scope, sentences: &'a [Sentence], rules: &[Claim]) -> Vec<&'a Sentence> {
    sentences
        .iter()
        .filter(|s| scope == Scope::All || !rules.iter().any(|c| c.sentence == s.id))
        .collect()
}

/// Claim estratti dall'LLM dalle frasi indicate, colorati di rosso e numerati
/// a partire da `next_id`.
pub fn extract(
    llm: &Llm,
    src: &str,
    sentences: &[&Sentence],
    doc: &str,
    next_id: usize,
) -> Result<Vec<Claim>, String> {
    let mut claims = Vec::new();
    for s in sentences {
        let raw = llm.chat(&prompt(&src[s.start..s.end]), &schema())?;
        let proposed: LlmClaims = serde_json::from_value(raw)
            .map_err(|e| format!("claim LLM non conformi allo schema (frase {}): {e}", s.id))?;
        for c in &proposed.claims {
            match check(src, s, c) {
                Ok((start, end, subj, rel, obj)) => claims.push(Claim {
                    id: format!("C{}", next_id + claims.len()),
                    subj,
                    rel,
                    obj,
                    negated: c.negated,
                    modality: None,
                    coref_from: None,
                    doc: doc.to_string(),
                    sentence: s.id,
                    start,
                    end,
                    color: Color::Rosso,
                }),
                Err(why) => eprintln!("claim LLM scartato (frase {}): {why}", s.id),
            }
        }
    }
    Ok(claims)
}

/// Unisce i claim delle regole e dell'LLM: un claim LLM uguale a uno delle
/// regole è scartato; l'ordine è quello del testo e, a parità, vince il claim
/// a regole.
pub fn merge(rules: Vec<Claim>, llm: Vec<Claim>) -> Vec<Claim> {
    let same = |a: &Claim, b: &Claim| {
        a.subj.key == b.subj.key && a.rel == b.rel && a.obj.key == b.obj.key && a.negated == b.negated
    };
    let mut all = rules.clone();
    all.extend(llm.into_iter().filter(|l| !rules.iter().any(|r| same(r, l))));
    all.sort_by_key(|c| (c.start, c.color));
    all
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract;
    use crate::llm::{fake, test_config};
    use crate::text::split_sentences;

    fn claim(kind: &str, attr: &str, subj: &str, rel: &str, obj: &str, span: &str) -> Value {
        json!({"kind": kind, "attribute": attr, "subject": subj, "relation": rel,
               "object": obj, "span": span, "negated": false})
    }

    #[test]
    fn llm_claims_are_checked_red_and_comparable_with_rules() {
        let text = "The RW, which is heavy, weighs in at 2 kg. It is mounted on the +X panel.";
        let server = fake::start(|req| {
            let p = req["messages"][0]["content"].as_str().unwrap();
            if p.contains("weighs") {
                json!({"claims": [
                    claim("attribute", "mass", "RW", "weighs", "2 kg", "RW, which is heavy, weighs in at 2 kg"),
                    claim("value", "", "RW", "is", "light", "RW is light"),
                ]})
            } else {
                json!({"claims": [claim("verb", "", "It", "is mounted on", "+X panel", "It is mounted on the +X panel")]})
            }
        });
        let llm = Llm::new(test_config(&server.url, "td_cache_extract"));
        let sentences = split_sentences(text, tokenize(text));
        let refs: Vec<&Sentence> = sentences.iter().collect();
        let claims = extract(&llm, text, &refs, "t", 1).unwrap();
        let triples: Vec<String> =
            claims.iter().map(|c| format!("{} | {} | {}", c.subj.text, c.rel.label(), c.obj.text)).collect();
        // Il secondo claim è scartato: lo span non compare nel testo.
        assert_eq!(triples, ["RW | attr:mass | 2 kg", "It | mount_on | +X panel"]);
        assert!(claims.iter().all(|c| c.color == Color::Rosso));
        assert_eq!((claims[0].id.as_str(), claims[1].id.as_str()), ("C1", "C2"));
        assert_eq!(&text[claims[1].start..claims[1].end], "It is mounted on the +X panel");
    }

    #[test]
    fn scope_selects_uncovered_or_all_sentences() {
        let text = "The RW is mounted on the panel. Thermal margins: TBD. The OBC is connected to the bus.";
        let s = split_sentences(text, tokenize(text));
        let rules = extract::extract(text, &s, "t");
        let ids = |v: Vec<&Sentence>| v.iter().map(|s| s.id).collect::<Vec<_>>();
        assert_eq!(ids(sentences_for(Scope::Uncovered, &s, &rules)), [2]);
        assert_eq!(ids(sentences_for(Scope::All, &s, &rules)), [1, 2, 3]);
    }

    #[test]
    fn merge_drops_duplicates_and_prefers_rules_on_ties() {
        let text = "The RW is mounted on the panel. The OBC is connected to the bus.";
        let s = split_sentences(text, tokenize(text));
        let rules = extract::extract(text, &s, "t");
        let mut dup = rules[0].clone();
        dup.id = "C3".into();
        dup.color = Color::Rosso;
        let mut other = rules[1].clone();
        other.id = "C4".into();
        other.color = Color::Rosso;
        other.obj.key = vec!["power".into()];
        let merged = merge(rules, vec![other, dup]);
        let ids: Vec<&str> = merged.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, ["C1", "C2", "C4"]);
    }
}
