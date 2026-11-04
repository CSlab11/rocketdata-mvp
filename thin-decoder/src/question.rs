//! Analisi delle domande con regole: tipo (what, which, who, when, where,
//! how much, how many, how + aggettivo, sì/no) → pattern di tripla con una
//! variabile e un tipo di risposta atteso. La tripla si ottiene riportando la
//! domanda in forma dichiarativa e applicando le stesse regole dell'estrazione.

use crate::extract::{clause_claims, verb_group_end, Phrase, RawClaim, Rel};
use crate::lexicon::{is_aux, is_in, BE, DO, HAVE, MODALS, PREPOSITIONS};
use crate::text::{is_participle, tokenize, verb_form, Kind, Token, VForm};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Var {
    Subj,
    Obj,
    /// Domanda sì/no: nessuna variabile.
    None,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expect {
    Any,
    /// Quantità, eventualmente di una grandezza precisa.
    Quantity(Option<&'static str>),
    Date,
    Location,
    /// Numero di oggetti del tipo indicato (chiave del nome).
    Count(String),
    Bool,
}

#[derive(Debug, Clone)]
pub struct Pattern {
    /// Tipo di domanda, per la traccia.
    pub kind: &'static str,
    /// Tripla principale (prima) ed eventuali triple aggiuntive.
    pub claims: Vec<RawClaim>,
    pub var: Var,
    pub expect: Expect,
    /// Attributi suggeriti dalla domanda ("how heavy" → mass, weight).
    pub attr_hints: Vec<String>,
    /// Classe richiesta ("which actuator", "what voltage").
    pub class: Option<Phrase>,
}

const NOUN_DIMS: &[(&str, &str)] = &[
    ("altitude", "length"), ("capacity", "energy"), ("consumption", "power"),
    ("current", "current"), ("depth", "length"), ("diameter", "length"),
    ("distance", "length"), ("duration", "time"), ("energy", "energy"), ("force", "force"),
    ("frequency", "frequency"), ("height", "length"), ("length", "length"), ("life", "time"),
    ("lifetime", "time"), ("mass", "mass"), ("period", "time"), ("power", "power"),
    ("radius", "length"), ("rate", "datarate"), ("speed", "speed"),
    ("temperature", "temperature"), ("thickness", "length"), ("thrust", "force"),
    ("velocity", "speed"), ("voltage", "voltage"), ("weight", "mass"), ("width", "length"),
];

/// "how ADJ": (aggettivo, grandezza, attributi suggeriti).
const ADJECTIVES: &[(&str, Option<&str>, &[&str])] = &[
    ("big", None, &["size"]), ("cold", Some("temperature"), &["temperature"]),
    ("deep", Some("length"), &["depth"]), ("far", Some("length"), &["distance"]),
    ("fast", Some("speed"), &["speed", "velocity"]), ("heavy", Some("mass"), &["mass", "weight"]),
    ("high", Some("length"), &["height", "altitude"]), ("hot", Some("temperature"), &["temperature"]),
    ("large", None, &["size"]), ("light", Some("mass"), &["mass", "weight"]),
    ("long", None, &["length", "lifetime", "duration"]), ("old", Some("time"), &["age"]),
    ("tall", Some("length"), &["height"]), ("thick", Some("length"), &["thickness"]),
    ("warm", Some("temperature"), &["temperature"]), ("wide", Some("length"), &["width"]),
];

pub fn noun_dim(lower: &str) -> Option<&'static str> {
    NOUN_DIMS.iter().find(|(n, _)| *n == lower).map(|(_, d)| *d)
}

fn aux_kind(t: &Token) -> &'static str {
    if is_in(BE, &t.lower) {
        "be"
    } else if is_in(HAVE, &t.lower) {
        "have"
    } else if is_in(DO, &t.lower) {
        "do"
    } else if is_in(MODALS, &t.lower) {
        "modal"
    } else {
        ""
    }
}

/// Primo verbo dopo il soggetto che, con l'ausiliare iniziale, forma il gruppo verbale.
fn find_inner_verb(aux: &Token, after: &[Token]) -> Option<usize> {
    let kind = aux_kind(aux);
    (1..after.len()).find(|&i| {
        let t = &after[i];
        match kind {
            "be" => matches!(verb_form(t), Some(VForm::Ed) | Some(VForm::Ing)),
            "have" => verb_form(t) == Some(VForm::Ed),
            "do" | "modal" => {
                verb_form(t) == Some(VForm::Base) || is_in(BE, &t.lower) || is_in(HAVE, &t.lower)
            }
            _ => false,
        }
    })
}

/// Domanda sì/no con "be": il soggetto finisce prima di un participio,
/// una preposizione, un articolo indeterminativo o un numero.
fn be_split(after: &[Token]) -> Option<usize> {
    (1..after.len()).find(|&i| {
        let t = &after[i];
        is_participle(&t.lower)
            || (is_in(PREPOSITIONS, &t.lower) && !t.is("of"))
            || t.is("a")
            || t.is("an")
            || t.kind == Kind::Number
    })
}

fn group_of(first: &Token, rest: &[Token]) -> Vec<Token> {
    std::iter::once(first.clone()).chain(rest.iter().cloned()).collect()
}

/// Sintagma nominale dopo la parola interrogativa ("which actuator", "how many arrays").
fn noun_after(toks: &[Token], start: usize) -> usize {
    let mut j = start;
    while j < toks.len()
        && toks[j].kind == Kind::Word
        && !is_aux(&toks[j].lower)
        && !matches!(verb_form(&toks[j]), Some(VForm::S) | Some(VForm::Ed))
    {
        j += 1;
    }
    j
}

/// Proposizione dopo la parte interrogativa: restituisce le triple e la variabile.
fn wh_clause(q: &str, toks: &[Token]) -> (Vec<RawClaim>, Var) {
    let Some(aux) = toks.first() else {
        return (Vec::new(), Var::Obj);
    };
    if is_aux(&aux.lower) {
        let after = &toks[1..];
        // "what is mounted on X": il participio segue subito → variabile soggetto.
        if aux_kind(aux) == "be" && after.first().map_or(false, |t| verb_form(t) == Some(VForm::Ed)) {
            let g = group_of(aux, &after[..1]);
            return (clause_claims(q, &[], &g, &after[1..]), Var::Subj);
        }
        if let Some(v) = find_inner_verb(aux, after) {
            let end = verb_group_end(after, v).max(v + 1);
            let g = group_of(aux, &after[v..end]);
            return (clause_claims(q, &after[..v], &g, &after[end..]), Var::Obj);
        }
        let g = group_of(aux, &[]);
        return (clause_claims(q, after, &g, &[]), Var::Obj);
    }
    if verb_form(aux).is_some() {
        let end = verb_group_end(toks, 0);
        return (clause_claims(q, &[], &toks[..end], &toks[end..]), Var::Subj);
    }
    (Vec::new(), Var::Obj)
}

/// Tripla principale: quella con lo slot della variabile vuoto. Se manca (es.
/// "what does X transmit to Y"), si ricava dalla prima liberando l'oggetto.
fn main_first(mut claims: Vec<RawClaim>, var: Var) -> Vec<RawClaim> {
    let empty = |c: &RawClaim| match var {
        Var::Subj => c.subj.is_empty(),
        Var::Obj => c.obj.is_empty(),
        Var::None => true,
    };
    if let Some(i) = claims.iter().position(empty) {
        let c = claims.remove(i);
        claims.insert(0, c);
    } else if let Some(first) = claims.first().cloned() {
        let mut c = first;
        if let Rel::Verb { prep, .. } = &mut c.rel {
            *prep = None;
        }
        match var {
            Var::Subj => c.subj = Phrase::default(),
            _ => c.obj = Phrase::default(),
        }
        claims.insert(0, c);
    }
    claims
}

pub fn parse(q: &str) -> Option<Pattern> {
    let mut toks = tokenize(q);
    while toks.last().map_or(false, |t| t.kind == Kind::Punct) {
        toks.pop();
    }
    toks.retain(|t| !(t.is("not") || t.is("never")) || t.start == 0);
    let first = toks.first()?.clone();
    let mut p = Pattern {
        kind: "",
        claims: Vec::new(),
        var: Var::Obj,
        expect: Expect::Any,
        attr_hints: Vec::new(),
        class: None,
    };

    if is_aux(&first.lower) {
        // Domanda sì/no: si riporta in forma dichiarativa.
        let after = &toks[1..];
        p.kind = "yes_no";
        p.var = Var::None;
        p.expect = Expect::Bool;
        p.claims = if aux_kind(&first) == "be" {
            match be_split(after) {
                Some(s) if is_participle(&after[s].lower) => {
                    clause_claims(q, &after[..s], &group_of(&first, &after[s..s + 1]), &after[s + 1..])
                }
                Some(s) => clause_claims(q, &after[..s], &group_of(&first, &[]), &after[s..]),
                None => clause_claims(q, after, &group_of(&first, &[]), &[]),
            }
        } else {
            match find_inner_verb(&first, after) {
                Some(v) => {
                    let end = verb_group_end(after, v).max(v + 1);
                    clause_claims(q, &after[..v], &group_of(&first, &after[v..end]), &after[end..])
                }
                None => Vec::new(),
            }
        };
        return (!p.claims.is_empty()).then_some(p);
    }

    let mut i = 1;
    match first.lower.as_str() {
        "how" => {
            let w = toks.get(1)?.lower.clone();
            if w == "many" || w == "much" {
                let end = noun_after(&toks, 2);
                let noun = Phrase::from_tokens(q, &toks[2..end]);
                if w == "many" {
                    p.kind = "how_many";
                    p.expect = Expect::Count(noun.key.last().cloned().unwrap_or_default());
                } else {
                    p.kind = "how_much";
                    let dim = noun.lower.last().and_then(|n| noun_dim(n));
                    p.expect = Expect::Quantity(dim);
                    if !noun.is_empty() {
                        p.attr_hints.push(noun.key_str());
                    }
                }
                i = end;
            } else if let Some((_, dim, hints)) = ADJECTIVES.iter().find(|(a, ..)| *a == w) {
                p.kind = "how_adjective";
                p.expect = Expect::Quantity(*dim);
                p.attr_hints = hints.iter().map(|h| crate::ontology::key_of(h)).collect();
                i = 2;
            } else {
                return None;
            }
        }
        "what" | "which" => {
            p.kind = if first.lower == "what" { "what" } else { "which" };
            let end = noun_after(&toks, 1);
            if end > 1 {
                let class = Phrase::from_tokens(q, &toks[1..end]);
                if let Some(dim) = class.lower.last().and_then(|n| noun_dim(n)) {
                    p.expect = Expect::Quantity(Some(dim));
                    p.attr_hints.push(class.key_str());
                }
                p.class = Some(class);
            }
            i = end;
        }
        "who" | "whom" => p.kind = "who",
        "when" => {
            p.kind = "when";
            p.expect = Expect::Date;
        }
        "where" => {
            p.kind = "where";
            p.expect = Expect::Location;
        }
        _ => return None,
    }

    let (claims, var) = wh_clause(q, &toks[i.min(toks.len())..]);
    if claims.is_empty() {
        return None;
    }
    p.var = var;
    p.claims = main_first(claims, var);
    Some(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn show(q: &str) -> String {
        let p = parse(q).expect(q);
        let c = &p.claims[0];
        format!(
            "{} {:?} {:?} | {} | {} | {}",
            p.kind,
            p.var,
            p.expect,
            c.subj.key_str(),
            c.rel.label(),
            c.obj.key_str()
        )
    }

    #[test]
    fn wh_questions() {
        assert_eq!(show("What is the mass of the reaction wheel?"), "what Obj Any | reaction wheel | attr:mass | ");
        assert_eq!(show("Where is the RW mounted?"), "where Obj Location | rw | mount | ");
        assert_eq!(show("What is mounted on the +X panel?"), "what Subj Any |  | mount_on | +x panel");
        assert_eq!(show("When was the satellite launched?"), "when Obj Date | satellit | launch | ");
        assert_eq!(show("Who manufactures the reaction wheels?"), "who Subj Any |  | manufactur | reaction wheel");
        assert_eq!(
            show("What voltage does the OBC operate at?"),
            "what Obj Quantity(Some(\"voltage\")) | obc | operat_at | "
        );
        assert_eq!(show("What does the PCDU distribute?"), "what Obj Any | pcdu | distribut | ");
    }

    #[test]
    fn how_questions() {
        assert_eq!(show("How much does the satellite weigh?"), "how_much Obj Quantity(None) | satellit | weigh | ");
        assert_eq!(
            show("How many solar arrays does the satellite have?"),
            "how_many Obj Count(\"arrai\") | satellit | hav | "
        );
        assert_eq!(show("How heavy is the RW?"), "how_adjective Obj Quantity(Some(\"mass\")) | rw | be | ");
    }

    #[test]
    fn yes_no_questions() {
        assert_eq!(show("Is the reaction wheel an actuator?"), "yes_no None Bool | reaction wheel | is_a | actuator");
        assert_eq!(show("Is the RW mounted on the +X panel?"), "yes_no None Bool | rw | mount_on | +x panel");
        assert_eq!(show("Does the payload exceed 300 W?"), "yes_no None Bool | payload | exceed | 300 w");
    }
}
