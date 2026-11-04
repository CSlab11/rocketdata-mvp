//! Estrazione dei claim (soggetto, relazione, oggetto) con regole sulle frasi:
//! copula, possesso con attributo, "the A of X", soggetto-verbo-oggetto,
//! passivo e complementi preposizionali. Indipendente dalle domande.

use crate::lexicon::{
    is_adverb, is_attribute_noun, is_aux, is_in, BE, CLAUSE_BREAKERS, DETERMINERS, DO, HAVE,
    MODALS, PREPOSITIONS, PRONOUNS,
};
use crate::color::Color;
use crate::quantity::{self, Quantity};
use crate::text::{verb_form, Kind, Sentence, Token, VForm};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rel {
    /// "X is V" (valore o stato).
    Be,
    /// "X is a Y".
    IsA,
    /// Attributo: "the mass of X is V", "X has a mass of V". Radici unite da spazio.
    Attr(String),
    /// Verbo lessicale (radice), con l'eventuale preposizione del complemento.
    Verb { verb: String, prep: Option<String> },
}

impl Rel {
    pub fn verb(verb: &str, prep: Option<&str>) -> Rel {
        Rel::Verb {
            verb: verb.to_string(),
            prep: prep.map(str::to_string),
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn label(&self) -> String {
        match self {
            Rel::Be => "be".into(),
            Rel::IsA => "is_a".into(),
            Rel::Attr(a) => format!("attr:{a}"),
            Rel::Verb { verb, prep: None } => verb.clone(),
            Rel::Verb { verb, prep: Some(p) } => format!("{verb}_{p}"),
        }
    }
}

/// Sintagma: testo originale, forme minuscole e radici dei token di contenuto.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Phrase {
    pub text: String,
    pub lower: Vec<String>,
    pub key: Vec<String>,
    /// Quantità normalizzata contenuta nel sintagma, se c'è.
    pub quantity: Option<Quantity>,
    pub start: usize,
    pub end: usize,
}

impl Phrase {
    pub fn from_tokens(src: &str, toks: &[Token]) -> Phrase {
        let (mut a, mut b) = (0, toks.len());
        while a < b
            && (is_in(DETERMINERS, &toks[a].lower)
                || (toks[a].kind == Kind::Punct && toks[a].text != "("))
        {
            a += 1;
        }
        while b > a && toks[b - 1].kind == Kind::Punct && toks[b - 1].text != ")" {
            b -= 1;
        }
        if a >= b {
            // "This is ...": il determinante è l'unico contenuto.
            match toks.iter().rposition(|t| t.kind == Kind::Word) {
                Some(w) => (a, b) = (w, w + 1),
                None => return Phrase::default(),
            }
        }
        let t = &toks[a..b];
        let (mut lower, mut key, mut depth) = (Vec::new(), Vec::new(), 0usize);
        for tok in t {
            match tok.text.as_str() {
                "(" => depth += 1,
                ")" => depth = depth.saturating_sub(1),
                _ if depth == 0
                    && tok.kind != Kind::Punct
                    && (!is_in(DETERMINERS, &tok.lower) || t.len() == 1) =>
                {
                    lower.push(tok.lower.clone());
                    key.push(tok.stem.clone());
                }
                _ => {}
            }
        }
        let (start, end) = (t[0].start, t[t.len() - 1].end);
        Phrase {
            text: src[start..end].to_string(),
            lower,
            key,
            quantity: quantity::parse(t),
            start,
            end,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    pub fn key_str(&self) -> String {
        self.key.join(" ")
    }
}

/// Claim prima della numerazione.
#[derive(Debug, Clone)]
pub struct RawClaim {
    pub subj: Phrase,
    pub rel: Rel,
    pub obj: Phrase,
    pub negated: bool,
    pub modality: Option<String>,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone)]
pub struct Claim {
    pub id: String,
    pub subj: Phrase,
    pub rel: Rel,
    pub obj: Phrase,
    /// Negazione nel gruppo verbale ("not", "never").
    pub negated: bool,
    /// Modalità: shall, must, may, should, can, will, … o il participio
    /// che regge l'infinito ("required", "designed").
    pub modality: Option<String>,
    /// Pronome sostituito dal soggetto della frase precedente.
    pub coref_from: Option<String>,
    /// Documento di provenienza (file di input).
    pub doc: String,
    /// Frase di provenienza (da 1).
    pub sentence: usize,
    /// Span in byte nel testo di input.
    pub start: usize,
    pub end: usize,
    /// Origine: blu per le regole, rosso per l'LLM.
    pub color: Color,
}

fn is_plural_noun(t: &Token) -> bool {
    let l = &t.lower;
    t.kind == Kind::Word
        && l.len() > 3
        && l.ends_with('s')
        && !l.ends_with("ss")
        && !l.ends_with("us")
        && !l.ends_with("is")
        && !is_aux(l)
}

fn blocks_verb(prev: &Token) -> bool {
    is_in(DETERMINERS, &prev.lower) || is_in(PREPOSITIONS, &prev.lower) || prev.lower == "of"
}

fn month_may(toks: &[Token], i: usize) -> bool {
    toks[i].lower == "may" && toks.get(i + 1).map_or(false, |n| n.kind == Kind::Number)
}

/// Inizio del gruppo verbale principale (il soggetto ha almeno un token).
pub fn find_verb(toks: &[Token]) -> Option<usize> {
    let mut depth = 0i32;
    for i in 0..toks.len() {
        let t = &toks[i];
        match t.text.as_str() {
            "(" => depth += 1,
            ")" => depth -= 1,
            _ => {}
        }
        if depth > 0 || i == 0 || t.kind != Kind::Word {
            continue;
        }
        let prev = &toks[i - 1];
        if is_aux(&t.lower) {
            if month_may(toks, i) || blocks_verb(prev) {
                continue;
            }
            return Some(i);
        }
        match verb_form(t) {
            Some(VForm::Ed) if !blocks_verb(prev) => return Some(i),
            Some(VForm::S) if !blocks_verb(prev) => {
                // "test results show the ...": il plurale è un nome se segue un verbo base
                // che regge un determinante, un numero o "that".
                let next_is_verb = toks.get(i + 1).map_or(false, |n| {
                    is_aux(&n.lower)
                        || (verb_form(n) == Some(VForm::Base)
                            && toks.get(i + 2).map_or(false, |m| {
                                is_in(DETERMINERS, &m.lower) || m.kind == Kind::Number || m.is("that")
                            }))
                });
                if !next_is_verb {
                    return Some(i);
                }
            }
            Some(VForm::Base) if is_in(PRONOUNS, &prev.lower) || is_plural_noun(prev) => {
                return Some(i)
            }
            _ => {}
        }
    }
    None
}

/// Fine (esclusa) del gruppo verbale che inizia in `start`:
/// ausiliari, modali, negazioni, avverbi e al più un verbo lessicale.
pub fn verb_group_end(toks: &[Token], start: usize) -> usize {
    let mut j = start;
    let mut last_aux: Option<&str> = None;
    while j < toks.len() {
        let t = &toks[j];
        if t.kind != Kind::Word {
            break;
        }
        if is_aux(&t.lower) && !month_may(toks, j) {
            last_aux = Some(if is_in(BE, &t.lower) {
                "be"
            } else if is_in(HAVE, &t.lower) {
                "have"
            } else {
                "modal"
            });
            j += 1;
            continue;
        }
        if j > start && (t.is("not") || t.is("never") || is_adverb(&t.lower)) {
            j += 1;
            continue;
        }
        let ok = match (verb_form(t), last_aux) {
            (Some(_), None) => j == start,
            (Some(VForm::Ed) | Some(VForm::Ing), Some("be")) => true,
            (Some(VForm::Ed), Some("have")) => true,
            (Some(VForm::Base), Some("modal")) => true,
            _ => false,
        };
        if ok {
            j += 1;
        }
        break;
    }
    j
}

/// Il verbo lessicale del gruppo, se c'è.
fn lexical(group: &[Token]) -> Option<&Token> {
    group
        .iter()
        .rev()
        .find(|t| !is_aux(&t.lower) && !t.is("not") && !t.is("never") && verb_form(t).is_some())
}

fn starts_predicate(toks: &[Token]) -> bool {
    toks.first().map_or(false, |t| {
        is_aux(&t.lower) || matches!(verb_form(t), Some(VForm::Ed) | Some(VForm::S))
    })
}

/// Taglia il resto della proposizione a virgole e subordinate; separa una
/// proposizione coordinata ("and the PCDU controls ...", "and connected to ...").
fn split_coordination(rest: &[Token]) -> (&[Token], Option<&[Token]>) {
    let mut depth = 0i32;
    let mut c = 0;
    while c < rest.len() {
        let t = &rest[c];
        match t.text.as_str() {
            "(" => depth += 1,
            ")" => depth -= 1,
            _ => {}
        }
        if depth == 0 {
            let conj = |k: usize| rest.get(k).map_or(false, |x| matches!(x.lower.as_str(), "and" | "but" | "or"));
            let clause_after = |k: usize| {
                let tail = &rest[(k + 1).min(rest.len())..];
                (matches!(find_verb(tail), Some(_)) || starts_predicate(tail)).then_some(tail)
            };
            if t.text == "," {
                if conj(c + 1) {
                    if let Some(tail) = clause_after(c + 1) {
                        return (&rest[..c], Some(tail));
                    }
                }
                return (&rest[..c], None);
            }
            if conj(c) && c > 0 {
                if let Some(tail) = clause_after(c) {
                    return (&rest[..c], Some(tail));
                }
            }
            let such_as = t.is("such") && rest.get(c + 1).map_or(false, |n| n.is("as"));
            if is_in(CLAUSE_BREAKERS, &t.lower) || such_as || t.is("including") || t.is(";") {
                return (&rest[..c], None);
            }
        }
        c += 1;
    }
    (rest, None)
}

/// Divide il resto in oggetto diretto e complementi preposizionali ("of" resta nel sintagma).
fn split_rest(rest: &[Token]) -> (&[Token], Vec<(String, &[Token])>) {
    let mut cuts: Vec<usize> = Vec::new();
    let mut depth = 0i32;
    for (i, t) in rest.iter().enumerate() {
        match t.text.as_str() {
            "(" => depth += 1,
            ")" => depth -= 1,
            _ => {}
        }
        if depth == 0 && is_in(PREPOSITIONS, &t.lower) {
            cuts.push(i);
        }
    }
    let first_end = cuts.first().copied().unwrap_or(rest.len());
    let mut pps = Vec::new();
    for (k, &c) in cuts.iter().enumerate() {
        let end = cuts.get(k + 1).copied().unwrap_or(rest.len());
        pps.push((rest[c].lower.clone(), &rest[c + 1..end]));
    }
    (&rest[..first_end], pps)
}

/// Posizione di "of" (o "'s") che separa attributo e proprietario.
fn attribute_split(toks: &[Token]) -> Option<(usize, bool)> {
    let mut depth = 0i32;
    for (i, t) in toks.iter().enumerate() {
        match t.text.as_str() {
            "(" => depth += 1,
            ")" => depth -= 1,
            _ => {}
        }
        if depth == 0 && i > 0 && i + 1 < toks.len() {
            if t.is("of") {
                return Some((i, false));
            }
            if t.is("'s") {
                return Some((i, true));
            }
        }
    }
    None
}

fn has_number(p: &[Token]) -> bool {
    p.iter().any(|t| t.kind == Kind::Number)
}

fn qualifies_as_attribute(attr: &Phrase, value: &[Token]) -> bool {
    !attr.is_empty()
        && (has_number(value) || attr.key.last().map_or(false, |h| is_attribute_noun(h)))
}

/// "an altitude of 600 km" → (attributo, valore), se il valore è numerico o l'attributo noto.
fn np_attribute(src: &str, np: &[Token]) -> Option<(Phrase, Phrase)> {
    let (i, possessive) = attribute_split(np)?;
    if possessive {
        return None;
    }
    let ap = Phrase::from_tokens(src, &np[..i]);
    let value = &np[i + 1..];
    (has_number(value) && qualifies_as_attribute(&ap, value)).then(|| (ap, Phrase::from_tokens(src, value)))
}

/// Claim di una proposizione con soggetto, gruppo verbale e resto già separati.
/// Il soggetto può essere vuoto (domande con variabile soggetto).
pub fn clause_claims(src: &str, subj: &[Token], group: &[Token], rest: &[Token]) -> Vec<RawClaim> {
    claims_with_mode(src, subj, group, rest, false, None)
}

fn claims_with_mode(
    src: &str,
    subj: &[Token],
    group: &[Token],
    rest: &[Token],
    outer_neg: bool,
    outer_modality: Option<String>,
) -> Vec<RawClaim> {
    let s = Phrase::from_tokens(src, subj);
    let lex = lexical(group);
    let negated = outer_neg || group.iter().any(|t| t.is("not") || t.is("never"));
    let modality = group
        .iter()
        .find(|t| is_in(MODALS, &t.lower))
        .map(|t| t.lower.clone())
        .or(outer_modality);
    let has_be = group.iter().any(|t| is_in(BE, &t.lower));
    let has_have = group.iter().any(|t| is_in(HAVE, &t.lower));
    let only_do_or_modal = lex.is_none()
        && group.iter().all(|t| is_in(DO, &t.lower) || is_in(MODALS, &t.lower) || t.is("not"));
    if group.is_empty() || only_do_or_modal {
        return Vec::new();
    }

    // "is designed to operate ...": il verbo all'infinito diventa la relazione.
    let k = usize::from(rest.first().map_or(false, |t| t.is("able") || t.is("unable")));
    if rest.get(k).map_or(false, |t| t.is("to"))
        && rest.get(k + 1).map_or(false, |t| verb_form(t) == Some(VForm::Base))
    {
        let governor = if k == 1 { Some(&rest[0]) } else { lex };
        let modality = modality.or_else(|| governor.map(|t| t.lower.clone()));
        return claims_with_mode(src, subj, &rest[k + 1..k + 2], &rest[k + 2..], negated, modality);
    }

    let (first, pps) = split_rest(rest);
    let first_p = Phrase::from_tokens(src, first);
    let mut out: Vec<(Phrase, Rel, Phrase)> = Vec::new();

    match lex {
        None if has_be => {
            if !first_p.is_empty() || pps.is_empty() {
                let attr = attribute_split(subj).and_then(|(i, possessive)| {
                    let (a, o) = if possessive {
                        (&subj[i + 1..], &subj[..i])
                    } else {
                        (&subj[..i], &subj[i + 1..])
                    };
                    let ap = Phrase::from_tokens(src, a);
                    qualifies_as_attribute(&ap, first).then(|| (ap, Phrase::from_tokens(src, o)))
                });
                if let Some((ap, owner)) = attr {
                    out.push((owner, Rel::Attr(ap.key_str()), first_p.clone()));
                } else if first.first().map_or(false, |t| t.is("a") || t.is("an")) && !has_number(first) {
                    out.push((s.clone(), Rel::IsA, first_p.clone()));
                } else {
                    out.push((s.clone(), Rel::Be, first_p.clone()));
                }
            }
            for (p, np) in &pps {
                out.push((s.clone(), Rel::verb("be", Some(p)), Phrase::from_tokens(src, np)));
            }
        }
        None if has_have => {
            if !first_p.is_empty() || pps.is_empty() {
                let attr = attribute_split(first).and_then(|(i, possessive)| {
                    if possessive {
                        return None;
                    }
                    let ap = Phrase::from_tokens(src, &first[..i]);
                    let value = &first[i + 1..];
                    qualifies_as_attribute(&ap, value).then(|| (ap, Phrase::from_tokens(src, value)))
                });
                match attr {
                    Some((ap, value)) => out.push((s.clone(), Rel::Attr(ap.key_str()), value)),
                    None => out.push((s.clone(), Rel::verb("hav", None), first_p.clone())),
                }
            }
            for (p, np) in &pps {
                out.push((s.clone(), Rel::verb("hav", Some(p)), Phrase::from_tokens(src, np)));
            }
        }
        None => {}
        Some(v) => {
            if !first_p.is_empty() || pps.is_empty() {
                out.push((s.clone(), Rel::verb(&v.stem, None), first_p.clone()));
            }
            for (p, np) in &pps {
                out.push((s.clone(), Rel::verb(&v.stem, Some(p)), Phrase::from_tokens(src, np)));
            }
        }
    }

    // Attributi impliciti negli oggetti: "orbits at an altitude of 600 km".
    if !matches!(lex, None) || !has_have {
        for np in std::iter::once(first).chain(pps.iter().map(|(_, np)| *np)) {
            if let Some((ap, value)) = np_attribute(src, np) {
                out.push((s.clone(), Rel::Attr(ap.key_str()), value));
            }
        }
    }

    let g_start = group[0].start;
    let g_end = group[group.len() - 1].end;
    out.into_iter()
        .map(|(subj, rel, obj)| {
            let parts = [&subj, &obj].into_iter().filter(|p| !p.is_empty());
            let start = parts.clone().map(|p| p.start).fold(g_start, usize::min);
            let end = parts.map(|p| p.end).fold(g_end, usize::max);
            RawClaim { subj, rel, obj, negated, modality: modality.clone(), start, end }
        })
        .collect()
}

/// Separa un complemento iniziale ("In 2020, the satellite ...") dal soggetto.
fn split_leading(subj: &[Token]) -> (Option<&[Token]>, &[Token]) {
    match subj.iter().rposition(|t| t.text == ",") {
        Some(c) if subj.first().map_or(false, |t| is_in(PREPOSITIONS, &t.lower)) => {
            (Some(&subj[..c]), &subj[c + 1..])
        }
        Some(c) => (None, &subj[c + 1..]),
        None => (None, subj),
    }
}

fn predicate_claims(src: &str, subj: &[Token], toks: &[Token]) -> Vec<RawClaim> {
    let g = verb_group_end(toks, 0);
    if g == 0 {
        return Vec::new();
    }
    let (rest, tail) = split_coordination(&toks[g..]);
    let mut out = clause_claims(src, subj, &toks[..g], rest);
    if let Some(tail) = tail {
        if find_verb(tail).is_some() {
            out.extend(sentence_claims(src, tail));
        } else if starts_predicate(tail) {
            out.extend(predicate_claims(src, subj, tail));
        }
    }
    out
}

fn sentence_claims(src: &str, toks: &[Token]) -> Vec<RawClaim> {
    let Some(v) = find_verb(toks) else {
        return Vec::new();
    };
    let (lead, subj) = split_leading(&toks[..v]);
    let mut out = predicate_claims(src, subj, &toks[v..]);
    if let Some(lead) = lead {
        let g = verb_group_end(toks, v);
        out.extend(clause_claims(src, subj, &toks[v..g], lead));
    }
    out
}

/// Lo span deve essere una porzione letterale del testo, contenere l'oggetto,
/// e il soggetto deve comparire letteralmente nel testo.
fn check_literal(src: &str, c: &RawClaim) -> Result<(), String> {
    let span = src
        .get(c.start..c.end)
        .ok_or_else(|| format!("span {}..{} fuori dal testo", c.start, c.end))?;
    if !c.obj.is_empty() && !span.contains(&c.obj.text) {
        return Err(format!("oggetto «{}» non presente nello span «{span}»", c.obj.text));
    }
    if !c.subj.is_empty() && !src.contains(&c.subj.text) {
        return Err(format!("soggetto «{}» non presente nel testo", c.subj.text));
    }
    Ok(())
}

/// Estrae i claim da tutte le frasi del testo, nell'ordine in cui compaiono.
const SUBJECT_PRONOUNS: &[&str] = &["it", "they", "this", "these"];

pub fn extract(src: &str, sentences: &[Sentence], doc: &str) -> Vec<Claim> {
    let mut claims: Vec<Claim> = Vec::new();
    let mut antecedent: Option<Phrase> = None;
    for s in sentences {
        let first_of_sentence = claims.len();
        for clause in s.tokens.split(|t| t.text == ";") {
            for raw in sentence_claims(src, clause) {
                if let Err(why) = check_literal(src, &raw) {
                    eprintln!("claim scartato (frase {}): {why}", s.id);
                    continue;
                }
                let mut subj = raw.subj;
                let mut coref_from = None;
                let is_pronoun = subj.lower.len() == 1 && is_in(SUBJECT_PRONOUNS, &subj.lower[0]);
                if let (true, Some(ante)) = (is_pronoun, &antecedent) {
                    coref_from = Some(subj.text.clone());
                    subj = ante.clone();
                }
                claims.push(Claim {
                    id: format!("C{}", claims.len() + 1),
                    subj,
                    coref_from,
                    rel: raw.rel,
                    obj: raw.obj,
                    negated: raw.negated,
                    modality: raw.modality,
                    doc: doc.to_string(),
                    sentence: s.id,
                    start: raw.start,
                    end: raw.end,
                    color: Color::Blu,
                });
            }
        }
        if let Some(c) = claims[first_of_sentence..].first() {
            antecedent = Some(c.subj.clone());
        }
    }
    claims
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::{split_sentences, tokenize};

    fn triples(text: &str) -> Vec<String> {
        let claims = extract(text, &split_sentences(text, tokenize(text)), "t");
        claims
            .iter()
            .map(|c| format!("{} | {} | {}", c.subj.text, c.rel.label(), c.obj.text))
            .collect()
    }

    #[test]
    fn claim_carries_id_span_and_provenance() {
        let text = "Intro. The RW is mounted on the panel.";
        let c = &extract(text, &split_sentences(text, tokenize(text)), "doc.json")[0];
        assert_eq!((c.id.as_str(), c.doc.as_str(), c.sentence), ("C1", "doc.json", 2));
        assert_eq!(&text[c.start..c.end], "RW is mounted on the panel");
    }

    #[test]
    fn rule_claims_are_blue() {
        let text = "The RW is mounted on the panel. The OBC has a mass of 2 kg.";
        let claims = extract(text, &split_sentences(text, tokenize(text)), "t");
        assert!(!claims.is_empty() && claims.iter().all(|c| c.color == Color::Blu));
    }

    #[test]
    fn rejects_non_literal_spans() {
        let text = "The RW is mounted on the panel.";
        let mut raw = sentence_claims(text, &tokenize(text)).remove(0);
        assert!(check_literal(text, &raw).is_ok());
        raw.obj.text = "the bus".into();
        assert!(check_literal(text, &raw).is_err());
        raw.end = text.len() + 5;
        assert!(check_literal(text, &raw).is_err());
    }

    #[test]
    fn negation_and_modality() {
        let text = "The OBC shall not consume more than 15 W. The PCDU is required to supply 100 W.";
        let c = extract(text, &split_sentences(text, tokenize(text)), "t");
        assert_eq!((c[0].negated, c[0].modality.as_deref()), (true, Some("shall")));
        assert_eq!((c[1].negated, c[1].modality.as_deref()), (false, Some("required")));
        assert_eq!(c[1].rel.label(), "suppli");
    }

    #[test]
    fn pronoun_subject_takes_previous_subject() {
        let text = "The satellite has two solar arrays. It was launched in 2021.";
        let c = extract(text, &split_sentences(text, tokenize(text)), "t");
        assert_eq!(c[1].subj.text, "satellite");
        assert_eq!(c[1].coref_from.as_deref(), Some("It"));
    }

    #[test]
    fn copula_attribute_and_isa() {
        assert_eq!(triples("The mass of the satellite is 250 kg."), ["satellite | attr:mass | 250 kg"]);
        assert_eq!(triples("The reaction wheel is an actuator."), ["reaction wheel | is_a | actuator"]);
    }

    #[test]
    fn possession_with_attribute() {
        assert_eq!(
            triples("The reaction wheel (RW) has a mass of 2.5 kg."),
            ["reaction wheel (RW) | attr:mass | 2.5 kg"]
        );
    }

    #[test]
    fn passive_with_prepositions_and_coordination() {
        assert_eq!(
            triples("The RW is mounted on the +X panel and connected to the OBC."),
            ["RW | mount_on | +X panel", "RW | connect_to | OBC"]
        );
        assert_eq!(
            triples("In 2020, the satellite was launched from Kourou."),
            ["satellite | launch_from | Kourou", "satellite | launch_in | 2020"]
        );
    }

    #[test]
    fn svo_with_modal_and_infinitive() {
        assert_eq!(
            triples("The OBC shall operate at 28 V; the PCDU is designed to supply 100 W."),
            ["OBC | operat_at | 28 V", "PCDU | suppli | 100 W"]
        );
    }
}
