//! Confronto tra il pattern della domanda e i claim, e costruzione della
//! risposta da template. Ordine fisso dei livelli di corrispondenza:
//! esatta, per lemma, per acronimo, per sinonimo, per testa del nome composto.
//! A parità vince il claim che compare prima nel testo.

use crate::color::Color;
use crate::extract::{Claim, Phrase, Rel};
use crate::lexicon::HAVE_LIKE;
use crate::ontology::Ontology;
use crate::output::{Engine, Evidence, Status, Task, QA};
use crate::quantity::is_year;
use crate::question::{self, Expect, Pattern, Var};
use crate::lexicon::{is_aux, is_in, DETERMINERS, PREPOSITIONS, PRONOUNS};
use crate::text::{stem, tokenize, Kind, Sentence};

use std::collections::BTreeSet;

pub struct Kb<'a> {
    pub src: &'a str,
    pub sentences: &'a [Sentence],
    pub claims: &'a [Claim],
    pub onto: &'a Ontology,
}

const MONTHS: &[&str] = &[
    "january", "february", "march", "april", "may", "june", "july", "august", "september",
    "october", "november", "december",
];
const LOCATION_PREPS: &[&str] = &[
    "aboard", "above", "at", "behind", "below", "from", "in", "inside", "into", "near", "on",
    "onto", "over", "to", "under", "within",
];

/// Livello di corrispondenza tra due sintagmi (0 = esatta … 4 = testa del composto).
fn entity_level(q: &Phrase, c: &Phrase, onto: &Ontology) -> Option<(u8, Option<String>)> {
    if q.is_empty() {
        return Some((0, None));
    }
    if c.is_empty() {
        return None;
    }
    if q.lower == c.lower {
        return Some((0, None));
    }
    if q.key == c.key {
        return Some((1, None));
    }
    let (qk, ck) = (q.key_str(), c.key_str());
    if let Some((alias, why)) = onto.acronym_alias(&qk) {
        if alias == ck {
            return Some((2, Some(why)));
        }
    }
    if let Some((alias, why)) = onto.acronym_alias(&ck) {
        if alias == qk {
            return Some((2, Some(why)));
        }
    }
    if let Some((_, why)) = onto.synonyms_of(&qk).into_iter().find(|(k, _)| *k == ck) {
        return Some((3, Some(why)));
    }
    let suffix = |a: &[String], b: &[String]| !a.is_empty() && b.len() > a.len() && b.ends_with(a);
    if suffix(&q.key, &c.key) || suffix(&c.key, &q.key) {
        return Some((4, Some(format!("compound head: '{}' ~ '{}'", q.text, c.text))));
    }
    None
}

fn head(key: &str) -> &str {
    key.rsplit(' ').next().unwrap_or(key)
}

/// Attributo `attr` compatibile con un suggerimento (uguale, stessa testa o sinonimo).
fn attr_compatible(hint: &str, attr: &str, onto: &Ontology) -> bool {
    hint == attr
        || head(attr) == hint
        || onto.synonyms_of(hint).iter().any(|(k, _)| k == attr || k == head(attr))
}

fn verb_linked(verb: &str, attr: &str, onto: &Ontology) -> bool {
    onto.verb_attr_linked(verb, attr)
        || onto.verb_attr_linked(verb, head(attr))
        || onto.synonyms_of(head(attr)).iter().any(|(k, _)| onto.verb_attr_linked(verb, k))
}

/// Livello di corrispondenza della relazione (0 = identica … 4 = solo grandezza).
fn rel_level(p: &Pattern, q: &Rel, c: &Claim, onto: &Ontology) -> Option<u8> {
    if q == &c.rel {
        return Some(0);
    }
    match (q, &c.rel) {
        (Rel::Verb { verb: a, .. }, Rel::Verb { verb: b, .. }) if a == b => return Some(1),
        (Rel::Be, Rel::IsA) | (Rel::IsA, Rel::Be) => return Some(1),
        (Rel::Attr(a), Rel::Attr(b)) if attr_compatible(a, b, onto) => return Some(2),
        (Rel::Verb { verb, prep: None }, Rel::Attr(a)) if verb_linked(verb, a, onto) => return Some(2),
        (Rel::Attr(a), Rel::Verb { verb, prep: None }) if verb_linked(verb, a, onto) => return Some(2),
        (Rel::Verb { verb: h, .. }, Rel::Verb { verb: v, .. })
            if h == "hav" && HAVE_LIKE.iter().any(|w| stem(w) == *v) =>
        {
            return Some(3)
        }
        _ => {}
    }
    for hint in &p.attr_hints {
        let ok = match &c.rel {
            Rel::Attr(a) => attr_compatible(hint, a, onto),
            Rel::Verb { verb, prep: None } => verb_linked(verb, hint, onto),
            _ => false,
        };
        if ok {
            return Some(2);
        }
    }
    if let Expect::Quantity(Some(dim)) = p.expect {
        if c.obj.quantity.as_ref().map_or(false, |x| x.dim == dim) {
            return Some(4);
        }
    }
    None
}

fn is_date(p: &Phrase) -> bool {
    p.lower.iter().any(|w| MONTHS.contains(&w.as_str()))
        || tokenize(&p.text).iter().any(is_year)
}

/// Il valore trovato è del tipo atteso dalla domanda?
fn fits(expect: &Expect, c: &Claim, value: &Phrase) -> bool {
    if value.is_empty() {
        return false;
    }
    match expect {
        Expect::Any | Expect::Bool => true,
        Expect::Quantity(dim) => value
            .quantity
            .as_ref()
            .map_or(false, |x| x.dim != "count" && dim.map_or(true, |d| d == x.dim)),
        Expect::Date => is_date(value),
        Expect::Location => {
            value.quantity.is_none()
                && !is_date(value)
                && matches!(&c.rel, Rel::Verb { prep: Some(p), .. } if LOCATION_PREPS.contains(&p.as_str()))
        }
        Expect::Count(noun) => {
            value.quantity.as_ref().map_or(false, |x| x.dim == "count")
                && (noun.is_empty() || value.key.contains(noun))
        }
    }
}

/// La classe richiesta ("which actuator") è soddisfatta dall'entità?
fn class_check(class: &Phrase, e: &Phrase, onto: &Ontology) -> Option<Vec<String>> {
    let ck = class.key_str();
    let mut keys = vec![(e.key_str(), Vec::new())];
    if let Some((alias, why)) = onto.acronym_alias(&e.key_str()) {
        keys.push((alias, vec![why]));
    }
    for (k, mut why) in keys {
        if k == ck || head(&k) == ck {
            return Some(why);
        }
        if let Some(chain) = onto.is_a(&k, &ck) {
            why.extend(chain);
            return Some(why);
        }
    }
    None
}

struct Candidate<'a> {
    score: (u8, u8, u8, u8, usize),
    claim: &'a Claim,
    value: Phrase,
    reasoning: Vec<String>,
}

/// Varianti del pattern: l'originale e, per "X Y is ...", la lettura (X, attributo Y).
fn variants(p: &Pattern) -> Vec<(u8, Phrase, Rel, Phrase)> {
    let main = &p.claims[0];
    let mut out = vec![(0, main.subj.clone(), main.rel.clone(), main.obj.clone())];
    if matches!(main.rel, Rel::Be) && p.var == Var::Obj && main.subj.key.len() >= 2 {
        let mut entity = main.subj.clone();
        let attr = entity.key.pop().unwrap_or_default();
        entity.lower.pop();
        out.push((1, entity, Rel::Attr(attr), Phrase::default()));
    }
    out
}

fn wh_candidates<'a>(kb: &Kb<'a>, p: &Pattern) -> Vec<Candidate<'a>> {
    let mut found = Vec::new();
    for (vi, qs, qr, qo) in variants(p) {
        for (pos, c) in kb.claims.iter().enumerate() {
            if c.negated {
                continue;
            }
            // Lettura diretta e, per la variabile soggetto, passivo "X is V-ed by Y".
            let mut readings: Vec<(Phrase, Phrase, Phrase, Option<Rel>, Option<String>)> = Vec::new();
            match p.var {
                Var::Obj => readings.push((qs.clone(), c.subj.clone(), c.obj.clone(), None, None)),
                Var::Subj => {
                    readings.push((qo.clone(), c.obj.clone(), c.subj.clone(), None, None));
                    if let (Rel::Verb { verb, prep: None }, Rel::Verb { verb: cv, prep: Some(by) }) = (&qr, &c.rel) {
                        if verb == cv && by == "by" {
                            let why = format!("passive {}: '{}' ← by '{}'", c.id, c.subj.text, c.obj.text);
                            readings.push((qo.clone(), c.subj.clone(), c.obj.clone(), Some(qr.clone()), Some(why)));
                        }
                    }
                }
                Var::None => {}
            }
            for (bound_q, bound_c, value, rel_override, extra) in readings {
                let Some((ent, ent_why)) = entity_level(&bound_q, &bound_c, kb.onto) else { continue };
                let rel = match rel_override {
                    Some(_) => Some(1),
                    None => rel_level(p, &qr, c, kb.onto),
                };
                let Some(rel) = rel else { continue };
                if !fits(&p.expect, c, &value) {
                    continue;
                }
                let mut reasoning: Vec<String> = ent_why.into_iter().chain(extra).collect();
                let mut penalty = 0;
                if let (Some(class), false) = (&p.class, matches!(p.expect, Expect::Quantity(Some(_)))) {
                    match class_check(class, &value, kb.onto) {
                        Some(chain) => reasoning.extend(chain),
                        None => penalty = 1,
                    }
                }
                if let Some(m) = &c.modality {
                    reasoning.push(format!("modality {}: '{m}'", c.id));
                }
                if let (Some(from), Var::Obj) = (&c.coref_from, p.var) {
                    reasoning.push(format!("coreference {}: '{from}' = '{}'", c.id, c.subj.text));
                }
                found.push(Candidate { score: (ent, rel, penalty, vi, pos), claim: c, value, reasoning });
            }
        }
    }
    found.sort_by_key(|c| c.score);
    found
}

pub fn evidence(kb: &Kb, c: &Claim) -> Evidence {
    Evidence {
        claim: Some(c.id.clone()),
        claim_color: Some(c.color),
        doc: c.doc.clone(),
        sentence: c.sentence,
        start: c.start,
        end: c.end,
        text: kb.src[c.start..c.end].to_string(),
    }
}

fn answer_text(value: &Phrase) -> String {
    match &value.quantity {
        Some(q) if !q.unit.is_empty() && q.unit != q.si_unit && q.dim != "time" => {
            format!("{} (= {})", value.text, q.normalized())
        }
        _ => value.text.clone(),
    }
}

fn yes_no(kb: &Kb, question: &str, p: &Pattern) -> Option<QA> {
    let mut evidence_list = Vec::new();
    let mut reasoning = Vec::new();
    let mut negated = false;
    for qc in &p.claims {
        if qc.rel == Rel::IsA {
            // "Is X a Y?": sottoclasse nell'ontologia (anche via acronimo).
            let chain = class_check(&qc.obj, &qc.subj, kb.onto)?;
            reasoning.extend(chain);
            continue;
        }
        let best = kb
            .claims
            .iter()
            .enumerate()
            .filter_map(|(pos, c)| {
                let (es, ws) = entity_level(&qc.subj, &c.subj, kb.onto)?;
                let (eo, wo) = entity_level(&qc.obj, &c.obj, kb.onto)?;
                let r = rel_level(p, &qc.rel, c, kb.onto)?;
                Some(((es.max(eo), r, pos), c, ws.into_iter().chain(wo).collect::<Vec<_>>()))
            })
            .min_by_key(|(score, ..)| *score)?;
        negated |= best.1.negated;
        evidence_list.push(evidence(kb, best.1));
        reasoning.extend(best.2);
    }
    Some(QA {
        question: question.to_string(),
        answer: if negated { "No." } else { "Yes." }.to_string(),
        status: Status::Answered,
        evidence: evidence_list,
        reasoning,
        color: Color::Blu,
        engine: Engine::Rules,
        task: Task::question(),
    })
}

/// Quota minima di parole della domanda presenti nella frase: numeratore/denominatore.
const FALLBACK_MIN_OVERLAP: (usize, usize) = (1, 2);
const QUESTION_WORDS: &[&str] = &["how", "many", "much", "what", "when", "where", "which", "who", "whom", "not"];

fn content_stems(toks: &[crate::text::Token]) -> BTreeSet<String> {
    toks.iter()
        .filter(|t| t.kind != Kind::Punct)
        .filter(|t| {
            let l = t.lower.as_str();
            !(is_aux(l) || is_in(DETERMINERS, l) || is_in(PREPOSITIONS, l) || is_in(PRONOUNS, l)
                || is_in(QUESTION_WORDS, l) || l == "of" || l == "and" || l == "or")
        })
        .map(|t| t.stem.clone())
        .collect()
}

/// Nessun claim corrisponde: frase con la maggiore sovrapposizione di radici,
/// se supera la soglia; a parità, la prima nel testo.
fn fallback(kb: &Kb, question: &str) -> QA {
    let q = content_stems(&tokenize(question));
    let (num, den) = FALLBACK_MIN_OVERLAP;
    let best = kb
        .sentences
        .iter()
        .map(|s| (content_stems(&s.tokens).intersection(&q).count(), s))
        .filter(|(n, _)| *n > 0 && n * den >= q.len() * num)
        .fold(None::<(usize, &Sentence)>, |acc, (n, s)| match acc {
            Some((m, _)) if m >= n => acc,
            _ => Some((n, s)),
        });
    let Some((n, s)) = best else {
        return QA::not_found(question);
    };
    let text = kb.src[s.start..s.end].to_string();
    QA {
        question: question.to_string(),
        answer: text.clone(),
        status: Status::TextualEvidence,
        evidence: vec![Evidence {
            claim: None,
            claim_color: None,
            doc: kb.claims.first().map(|c| c.doc.clone()).unwrap_or_default(),
            sentence: s.id,
            start: s.start,
            end: s.end,
            text,
        }],
        reasoning: vec![format!("lexical overlap: {n}/{} question words", q.len())],
        color: Color::Blu,
        engine: Engine::Rules,
        task: Task::question(),
    }
}

/// Ogni risposta `answered`, di qualunque motore, deve citare almeno un claim
/// esistente o un fatto derivato (catena dell'ontologia nella traccia).
/// L'evidenza testuale e la lacuna non affermano fatti tratti dai claim.
#[cfg_attr(not(test), allow(dead_code))]
pub fn cited(qa: &QA, claims: &[Claim]) -> Result<(), String> {
    if qa.status != Status::Answered {
        return Ok(());
    }
    let ids: Vec<&String> = qa.evidence.iter().filter_map(|e| e.claim.as_ref()).collect();
    if let Some(id) = ids.iter().find(|id| !claims.iter().any(|c| &&c.id == *id)) {
        return Err(format!("cita il claim inesistente {id}"));
    }
    if ids.is_empty() && qa.reasoning.is_empty() {
        return Err("non cita alcun claim o fatto derivato".into());
    }
    Ok(())
}

/// Risposta del motore a regole, con il colore calcolato dai claim citati.
#[cfg_attr(not(test), allow(dead_code))]
pub fn answer(kb: &Kb, question: &str) -> QA {
    answer_with(kb, question, question::parse(question))
}

/// Risposta del motore a regole con un pattern già calcolato (dalle regole o
/// dall'LLM); senza pattern, frase più vicina o lacuna. Con un pattern
/// proposto dall'LLM il risultato è rosso.
pub fn answer_with(kb: &Kb, question: &str, pattern: Option<Pattern>) -> QA {
    let from_llm = pattern.as_ref().map_or(false, |p| p.kind == "llm");
    let task = task(pattern.as_ref());
    let mut qa = rules_answer(kb, question, pattern);
    qa.task = task;
    let used = qa
        .evidence
        .iter()
        .filter_map(|e| e.claim.as_ref())
        .filter_map(|id| kb.claims.iter().find(|c| &c.id == id))
        .map(|c| c.color);
    qa.color = if from_llm { Color::Rosso } else { Color::of_rules(used) };
    qa
}

fn expect_label(e: &Expect) -> String {
    match e {
        Expect::Any => "any".into(),
        Expect::Quantity(None) => "quantity".into(),
        Expect::Quantity(Some(d)) => format!("quantity:{d}"),
        Expect::Date => "date".into(),
        Expect::Location => "location".into(),
        Expect::Count(k) => format!("count:{k}"),
        Expect::Bool => "bool".into(),
    }
}

/// Specifica di compito per la traccia: tipo di domanda, tripla con `?` al
/// posto della variabile, tipo atteso e origine del pattern.
fn task(p: Option<&Pattern>) -> Task {
    let mut t = Task::question();
    let Some(p) = p else {
        t.kind = Some("unrecognized".into());
        return t;
    };
    let triples: Vec<String> = p
        .claims
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let side = |ph: &Phrase, var: Var| match (i, p.var == var) {
                (0, true) => "?".to_string(),
                _ => ph.text.clone(),
            };
            format!("{} | {} | {}", side(&c.subj, Var::Subj), c.rel.label(), side(&c.obj, Var::Obj))
        })
        .collect();
    t.kind = Some(p.kind.to_string());
    t.pattern = Some(triples.join("; "));
    t.expected = Some(expect_label(&p.expect));
    t.source = Some(if p.kind == "llm" { "llm" } else { "rules" });
    t
}

fn rules_answer(kb: &Kb, question: &str, pattern: Option<Pattern>) -> QA {
    let Some(p) = pattern else {
        return fallback(kb, question);
    };
    if p.var == Var::None {
        return yes_no(kb, question, &p).unwrap_or_else(|| fallback(kb, question));
    }
    let candidates = wh_candidates(kb, &p);
    let Some(best) = candidates.first() else {
        return fallback(kb, question);
    };
    let mut reasoning = best.reasoning.clone();
    if p.kind == "llm" {
        reasoning.insert(0, "question pattern proposed by the LLM".into());
    }
    QA {
        question: question.to_string(),
        answer: answer_text(&best.value),
        status: Status::Answered,
        evidence: vec![evidence(kb, best.claim)],
        reasoning,
        color: Color::Blu,
        engine: Engine::Rules,
        task: Task::question(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract::extract;
    use crate::ontology;
    use crate::text::split_sentences;

    fn ask(text: &str, q: &str) -> QA {
        let s = split_sentences(text, tokenize(text));
        let claims = extract(text, &s, "t");
        let mut onto = ontology::from_text(text, &s, &claims);
        onto.add_defaults();
        answer(&Kb { src: text, sentences: &s, claims: &claims, onto: &onto }, q)
    }

    #[test]
    fn exact_match_beats_acronym_and_earliest_wins_ties() {
        let text = "The reaction wheel (RW) has a mass of 2 kg. The RW has a mass of 3 kg. \
                    The RW has a mass of 4 kg.";
        assert_eq!(ask(text, "What is the mass of the RW?").answer, "3 kg");
        assert_eq!(ask(text, "What is the mass of the reaction wheel?").answer, "2 kg");
    }

    #[test]
    fn synonym_head_and_verb_attribute_links() {
        let text = "The star tracker has a weight of 1.2 kg. The payload weighs 30 kg.";
        assert_eq!(ask(text, "What is the mass of the star tracker?").answer, "1.2 kg");
        assert_eq!(ask(text, "What is the mass of the tracker?").answer, "1.2 kg");
        assert_eq!(ask(text, "How heavy is the payload?").answer, "30 kg");
    }

    #[test]
    fn expected_type_filters_candidates() {
        let text = "The satellite was launched from Kourou in 2021.";
        assert_eq!(ask(text, "When was the satellite launched?").answer, "2021");
        assert_eq!(ask(text, "Where was the satellite launched?").answer, "Kourou");
    }

    #[test]
    fn fallback_returns_closest_sentence_or_gap() {
        let text = "The payload does not exceed 120 W. The satellite orbits at an altitude of 600 km.";
        let qa = ask(text, "What is the orbit altitude?");
        assert_eq!(qa.status, Status::TextualEvidence);
        assert_eq!(qa.evidence[0].sentence, 2);
        assert_eq!(ask(text, "Who is the prime contractor?").status, Status::NotFound);
    }

    #[test]
    fn rules_result_is_violet_with_a_red_claim() {
        let text = "The RW has a mass of 2 kg. The OBC has a mass of 3 kg.";
        let s = split_sentences(text, tokenize(text));
        let mut claims = extract(text, &s, "t");
        let mut onto = ontology::from_text(text, &s, &claims);
        onto.add_defaults();
        let kb = Kb { src: text, sentences: &s, claims: &claims, onto: &onto };
        assert_eq!(answer(&kb, "What is the mass of the RW?").color, Color::Blu);
        claims[0].color = Color::Rosso;
        let kb = Kb { src: text, sentences: &s, claims: &claims, onto: &onto };
        assert_eq!(answer(&kb, "What is the mass of the RW?").color, Color::Viola);
        assert_eq!(answer(&kb, "What is the mass of the OBC?").color, Color::Blu);
    }

    #[test]
    fn every_rules_answer_is_cited() {
        let raw = std::fs::read_to_string("examples/satellite.json").unwrap();
        let input: serde_json::Value = serde_json::from_str(&raw).unwrap();
        let text = input["text"].as_str().unwrap();
        let s = split_sentences(text, tokenize(text));
        let claims = extract(text, &s, "t");
        let mut onto = ontology::from_text(text, &s, &claims);
        onto.add_defaults();
        let kb = Kb { src: text, sentences: &s, claims: &claims, onto: &onto };
        for q in input["questions"].as_array().unwrap() {
            let qa = answer(&kb, q.as_str().unwrap());
            assert_eq!(cited(&qa, &claims), Ok(()), "{}", qa.question);
        }
    }

    #[test]
    fn uncited_or_unknown_claims_are_rejected() {
        let mut qa = QA::not_found("q");
        qa.status = Status::Answered;
        assert!(cited(&qa, &[]).is_err());
        qa.evidence.push(Evidence {
            claim: Some("C9".into()),
            claim_color: None,
            doc: "t".into(),
            sentence: 1,
            start: 0,
            end: 0,
            text: String::new(),
        });
        assert!(cited(&qa, &[]).unwrap_err().contains("C9"));
    }

    #[test]
    fn trace_includes_the_task_specification() {
        let qa = ask("The RW has a mass of 2 kg.", "What is the mass of the RW?");
        assert_eq!(qa.task.request, "domanda");
        assert_eq!(qa.task.pattern.as_deref(), Some("RW | attr:mass | ?"));
        assert_eq!(qa.task.source, Some("rules"));
        assert_eq!(qa.evidence[0].claim.as_deref(), Some("C1"));
        let qa = ask("The RW has a mass of 2 kg.", "Tell me the mass of the RW.");
        assert_eq!(qa.task.kind.as_deref(), Some("unrecognized"));
    }

    #[test]
    fn yes_no_with_negation() {
        let text = "The RW is not connected to the PCDU.";
        let qa = ask(text, "Is the RW connected to the PCDU?");
        assert_eq!((qa.answer.as_str(), qa.evidence.len()), ("No.", 1));
    }
}
