//! Ontologia costruita al volo dal testo: acronimi, sottoclassi da pattern di
//! Hearst, da copula ("X is a Y") e dalle teste dei nomi composti.
//! Le chiavi sono le radici dei sintagmi unite da spazio. Ogni relazione
//! conserva la propria provenienza, che finisce nel ragionamento della risposta.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::extract::{Claim, Phrase, Rel};
use crate::lexicon::{is_aux, is_in, DETERMINERS, PREPOSITIONS};
use crate::text::{stem, tokenize, verb_form, Kind, Sentence, Token, VForm};

#[derive(Debug, Default)]
pub struct Ontology {
    /// acronimo → (forma estesa, provenienza)
    pub acronyms: BTreeMap<String, (String, String)>,
    /// sottoclasse: figlio → {genitore → provenienza}
    pub parents: BTreeMap<String, BTreeMap<String, String>>,
    /// sinonimi simmetrici: chiave → {chiave → provenienza}
    pub synonyms: BTreeMap<String, BTreeMap<String, String>>,
    /// verbo (radice) → attributi (chiavi): "weigh" → "mass"
    pub attr_verbs: BTreeMap<String, BTreeSet<String>>,
}

/// Ontologia a priori in JSON. Tutti i campi sono opzionali.
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct OntologyFile {
    pub synonyms: Vec<Vec<String>>,
    pub subclass_of: Vec<(String, String)>,
    pub attribute_verbs: Vec<(String, String)>,
}

/// Lessico predefinito, sempre attivo.
const DEFAULT_SYNONYMS: &[&[&str]] = &[
    &["mass", "weight"],
    &["lifetime", "life", "duration"],
    &["cost", "price"],
    &["speed", "velocity"],
    &["manufacturer", "maker", "supplier"],
];
const DEFAULT_ATTR_VERBS: &[(&str, &str)] = &[
    ("weigh", "mass"), ("weigh", "weight"), ("cost", "cost"), ("cost", "price"),
    ("last", "lifetime"), ("last", "duration"), ("consume", "power"),
    ("consume", "power consumption"), ("consume", "consumption"), ("measure", "size"),
    ("measure", "length"), ("orbit", "altitude"), ("travel", "speed"),
];

/// Chiave di un testo libero (radici dei token di contenuto).
pub fn key_of(text: &str) -> String {
    Phrase::from_tokens(text, &tokenize(text)).key_str()
}

impl Ontology {
    pub fn add_parent(&mut self, child: &str, parent: &str, why: String) {
        if child.is_empty() || parent.is_empty() || child == parent {
            return;
        }
        self.parents
            .entry(child.to_string())
            .or_default()
            .entry(parent.to_string())
            .or_insert(why);
    }

    pub fn add_synonyms(&mut self, words: &[String], why: &str) {
        let keys: Vec<(String, &String)> =
            words.iter().map(|w| (key_of(w), w)).filter(|(k, _)| !k.is_empty()).collect();
        for (a, wa) in &keys {
            for (b, wb) in &keys {
                if a != b {
                    let msg = format!("{why}: {wa} ~ {wb}");
                    self.synonyms.entry(a.clone()).or_default().entry(b.clone()).or_insert(msg);
                }
            }
        }
    }

    pub fn add_attr_verb(&mut self, verb: &str, attr: &str) {
        self.attr_verbs.entry(key_of(verb)).or_default().insert(key_of(attr));
    }

    pub fn add_defaults(&mut self) {
        for group in DEFAULT_SYNONYMS {
            let words: Vec<String> = group.iter().map(|w| w.to_string()).collect();
            self.add_synonyms(&words, "default lexicon");
        }
        for (v, a) in DEFAULT_ATTR_VERBS {
            self.add_attr_verb(v, a);
        }
    }

    /// Carica un'ontologia a priori da file JSON.
    pub fn load_file(&mut self, path: &str) -> Result<(), String> {
        let raw = std::fs::read_to_string(path)
            .map_err(|e| format!("impossibile leggere l'ontologia {path}: {e}"))?;
        let file: OntologyFile = serde_json::from_str(&raw)
            .map_err(|e| format!("ontologia JSON non valida in {path}: {e}"))?;
        for group in &file.synonyms {
            self.add_synonyms(group, "ontology file");
        }
        for (child, parent) in &file.subclass_of {
            let why = format!("ontology file: {child} ⊂ {parent}");
            self.add_parent(&key_of(child), &key_of(parent), why);
        }
        for (verb, attr) in &file.attribute_verbs {
            self.add_attr_verb(verb, attr);
        }
        Ok(())
    }

    /// Vero se la radice compare in un concetto dell'ontologia.
    pub fn knows(&self, stem: &str) -> bool {
        let has = |key: &str| key.split(' ').any(|w| w == stem);
        self.acronyms.keys().any(|k| has(k))
            || self.parents.iter().any(|(k, ps)| has(k) || ps.keys().any(|p| has(p)))
            || self.synonyms.keys().any(|k| has(k))
            || self.attr_verbs.iter().any(|(v, attrs)| has(v) || attrs.iter().any(|a| has(a)))
    }

    pub fn synonyms_of(&self, key: &str) -> Vec<(String, String)> {
        self.synonyms
            .get(key)
            .map(|m| m.iter().map(|(k, w)| (k.clone(), w.clone())).collect())
            .unwrap_or_default()
    }

    /// Attributi associati al verbo (e viceversa, verbi associati all'attributo).
    pub fn verb_attr_linked(&self, verb: &str, attr: &str) -> bool {
        self.attr_verbs.get(verb).map_or(false, |a| a.contains(attr))
    }

    /// Forma estesa di un acronimo, o forma breve di una forma estesa.
    pub fn acronym_alias(&self, key: &str) -> Option<(String, String)> {
        if let Some((long, why)) = self.acronyms.get(key) {
            return Some((long.clone(), why.clone()));
        }
        self.acronyms
            .iter()
            .find(|(_, (long, _))| long == key)
            .map(|(short, (_, why))| (short.clone(), why.clone()))
    }

    /// Antenati di `key` (chiusura transitiva), ciascuno con la catena di provenienze.
    pub fn ancestors(&self, key: &str) -> Vec<(String, Vec<String>)> {
        let mut out: Vec<(String, Vec<String>)> = Vec::new();
        let mut seen: BTreeSet<String> = BTreeSet::from([key.to_string()]);
        let mut frontier: Vec<(String, Vec<String>)> = vec![(key.to_string(), Vec::new())];
        while !frontier.is_empty() {
            let mut next = Vec::new();
            for (k, chain) in &frontier {
                for (p, why) in self.parents.get(k).into_iter().flatten() {
                    if seen.insert(p.clone()) {
                        let mut c = chain.clone();
                        c.push(why.clone());
                        out.push((p.clone(), c.clone()));
                        next.push((p.clone(), c));
                    }
                }
            }
            frontier = next;
        }
        out
    }

    /// Catena che dimostra `child` is_a `ancestor`, se esiste.
    pub fn is_a(&self, child: &str, ancestor: &str) -> Option<Vec<String>> {
        self.ancestors(child)
            .into_iter()
            .find(|(a, _)| a == ancestor)
            .map(|(_, chain)| chain)
    }
}

const FUNCTION_WORDS: &[&str] = &["and", "for", "in", "of", "on", "the", "to", "&"];
const TYPE_WORDS: &[&str] = &["typ", "kind", "form", "class", "sort"];

fn initials(word: &str) -> Vec<char> {
    word.split('-')
        .filter_map(|p| p.chars().next())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

fn is_acronym(t: &Token) -> bool {
    t.kind == Kind::Word
        && (2..=8).contains(&t.text.chars().count())
        && t.text.chars().filter(|c| c.is_ascii_uppercase()).count() >= 2
        && t.text.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-')
}

/// "Power Conditioning and Distribution Unit (PCDU)": le iniziali, lette a ritroso,
/// devono coprire tutte le lettere dell'acronimo.
fn acronyms(src: &str, s: &Sentence, onto: &mut Ontology) {
    let t = &s.tokens;
    for i in 1..t.len().saturating_sub(2) {
        if t[i].text != "(" || t[i + 2].text != ")" || !is_acronym(&t[i + 1]) {
            continue;
        }
        let letters: Vec<char> = t[i + 1]
            .text
            .chars()
            .filter(|c| c.is_ascii_alphabetic())
            .map(|c| c.to_ascii_lowercase())
            .collect();
        let mut j = letters.len();
        let mut k = i;
        while k > 0 && j > 0 {
            let w = &t[k - 1];
            if w.kind != Kind::Word {
                break;
            }
            let ini = initials(&w.text);
            if j >= ini.len() && letters[j - ini.len()..j] == ini[..] {
                j -= ini.len();
            } else if !is_in(FUNCTION_WORDS, &w.lower) {
                break;
            }
            k -= 1;
        }
        if j == 0 {
            let long = Phrase::from_tokens(src, &t[k..i]);
            let why = format!("acronym {} = {} (sentence {})", t[i + 1].text, long.text, s.id);
            onto.acronyms.insert(stem(&t[i + 1].lower), (long.key_str(), why));
        }
    }
}

fn is_np_word(t: &Token) -> bool {
    t.kind == Kind::Word
        && !is_aux(&t.lower)
        && !is_in(PREPOSITIONS, &t.lower)
        && !is_in(DETERMINERS, &t.lower)
        && !matches!(verb_form(t), Some(VForm::S) | Some(VForm::Ed))
        && !matches!(t.lower.as_str(), "and" | "or" | "other" | "such" | "including")
}

/// Sintagma nominale che termina subito prima di `end` (escluso).
fn np_before(toks: &[Token], end: usize) -> &[Token] {
    let mut a = end;
    while a > 0 && is_np_word(&toks[a - 1]) {
        a -= 1;
    }
    &toks[a..end]
}

/// Elenco "A, B and C" a partire da `start`, fino a punteggiatura finale o verbo.
fn list_after(toks: &[Token], start: usize) -> Vec<&[Token]> {
    let mut items = Vec::new();
    let mut cur = start;
    let mut k = start;
    while k <= toks.len() {
        let stop = k == toks.len()
            || matches!(toks[k].text.as_str(), "." | ";" | ":" | "?" | "!")
            || is_aux(&toks[k].lower)
            || matches!(verb_form(&toks[k]), Some(VForm::S));
        let sep = !stop && (toks[k].text == "," || toks[k].is("and") || toks[k].is("or"));
        if stop || sep {
            if k > cur {
                items.push(&toks[cur..k]);
            }
            if stop {
                break;
            }
            cur = k + 1;
        }
        k += 1;
    }
    items
}

fn hearst(src: &str, s: &Sentence, onto: &mut Ontology) {
    let t = &s.tokens;
    for i in 0..t.len() {
        // "X such as A, B and C" / "X, including A and B"
        let (class_end, list_start, name) = if t[i].is("such") && t.get(i + 1).map_or(false, |n| n.is("as")) {
            (i, i + 2, "such as")
        } else if t[i].is("including") {
            (if i > 0 && t[i - 1].text == "," { i - 1 } else { i }, i + 1, "including")
        } else if t[i].is("other") && i > 0 && (t[i - 1].is("and") || t[i - 1].is("or")) {
            // "A, B and other X"
            let mut j = i + 1;
            while j < t.len() && is_np_word(&t[j]) {
                j += 1;
            }
            let class = Phrase::from_tokens(src, &t[i + 1..j]);
            let mut a = i - 1;
            while a > 0 && (is_np_word(&t[a - 1]) || t[a - 1].text == "," || t[a - 1].is("and")) {
                a -= 1;
            }
            for item in list_after(&t[..i - 1], a) {
                let child = Phrase::from_tokens(src, item);
                let why = format!("Hearst 'and other' (sentence {}): {} ⊂ {}", s.id, child.text, class.text);
                onto.add_parent(&child.key_str(), &class.key_str(), why);
            }
            continue;
        } else {
            continue;
        };
        let class = Phrase::from_tokens(src, np_before(t, class_end));
        for item in list_after(t, list_start) {
            let child = Phrase::from_tokens(src, item);
            let why = format!("Hearst '{name}' (sentence {}): {} ⊂ {}", s.id, child.text, class.text);
            onto.add_parent(&child.key_str(), &class.key_str(), why);
        }
    }
}

/// "a type of actuator" → "actuator".
fn strip_type_of(key: &[String]) -> String {
    match key.iter().position(|k| k == "of") {
        Some(i) if i > 0 && is_in(TYPE_WORDS, &key[i - 1]) => key[i + 1..].join(" "),
        _ => key.join(" "),
    }
}

fn copula(c: &Claim, onto: &mut Ontology) {
    let plural_class = matches!(c.rel, Rel::Be)
        && c.obj.quantity.is_none()
        && c.obj.lower.last().map_or(false, |w| w.ends_with('s'))
        && !c.obj.text.to_lowercase().starts_with("the ");
    if matches!(c.rel, Rel::IsA) || plural_class {
        let why = format!("copula {} (sentence {}): {} ⊂ {}", c.id, c.sentence, c.subj.text, c.obj.text);
        onto.add_parent(&c.subj.key_str(), &strip_type_of(&c.obj.key), why);
    }
}

/// "attitude control system" → "control system" → "system".
fn compound_heads(onto: &mut Ontology, keys: &BTreeSet<String>) {
    for key in keys {
        let words: Vec<&str> = key.split(' ').collect();
        if words.len() < 2
            || words.iter().any(|w| *w == "of" || !w.chars().all(|c| c.is_alphabetic() || c == '-'))
        {
            continue;
        }
        for k in 1..words.len() {
            if is_in(FUNCTION_WORDS, words[k]) {
                continue;
            }
            let parent = words[k..].join(" ");
            onto.add_parent(key, &parent, format!("compound head: {key} ⊂ {parent}"));
            break;
        }
    }
}

pub fn from_text(src: &str, sentences: &[Sentence], claims: &[Claim]) -> Ontology {
    let mut onto = Ontology::default();
    for s in sentences {
        acronyms(src, s, &mut onto);
        hearst(src, s, &mut onto);
    }
    for c in claims {
        copula(c, &mut onto);
    }
    let mut keys: BTreeSet<String> = BTreeSet::new();
    for c in claims {
        for p in [&c.subj, &c.obj] {
            if p.quantity.is_none() && !p.key.is_empty() {
                keys.insert(p.key_str());
            }
        }
    }
    keys.extend(onto.parents.keys().cloned());
    keys.extend(onto.parents.values().flat_map(|m| m.keys().cloned()).collect::<Vec<_>>());
    keys.extend(onto.acronyms.values().map(|(l, _)| l.clone()).collect::<Vec<_>>());
    // Le teste dei composti generano nuove chiavi: si ripete fino al punto fisso.
    loop {
        let before = onto.parents.values().map(|m| m.len()).sum::<usize>();
        compound_heads(&mut onto, &keys);
        keys.extend(onto.parents.values().flat_map(|m| m.keys().cloned()).collect::<Vec<_>>());
        if onto.parents.values().map(|m| m.len()).sum::<usize>() == before {
            break;
        }
    }
    onto
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract::extract;
    use crate::text::{split_sentences, tokenize};

    fn build(text: &str) -> Ontology {
        let s = split_sentences(text, tokenize(text));
        let c = extract(text, &s, "t");
        from_text(text, &s, &c)
    }

    #[test]
    fn a_priori_file_and_defaults() {
        let path = std::env::temp_dir().join("td_onto_test.json");
        std::fs::write(
            &path,
            r#"{"synonyms": [["spacecraft", "satellite"]], "subclass_of": [["magnetorquer", "actuator"]],
                "attribute_verbs": [["draw", "power"]]}"#,
        )
        .unwrap();
        let mut o = Ontology::default();
        o.add_defaults();
        o.load_file(path.to_str().unwrap()).unwrap();
        assert!(o.synonyms_of("spacecraft").iter().any(|(k, _)| k == "satellit"));
        assert!(o.synonyms_of("weight").iter().any(|(k, _)| k == "mass"));
        assert!(o.is_a("magnetorquer", "actuator").is_some());
        assert!(o.verb_attr_linked("draw", "power") && o.verb_attr_linked("weigh", "mass"));
        assert!(o.load_file("/nonexistent.json").is_err());
    }

    #[test]
    fn acronyms_from_initials() {
        let o = build(
            "The Power Conditioning and Distribution Unit (PCDU) feeds the On-Board Computer (OBC).",
        );
        assert_eq!(o.acronyms["pcdu"].0, "power condition and distribution unit");
        assert_eq!(o.acronyms["obc"].0, "on-board computer");
        assert_eq!(o.acronym_alias("on-board computer").unwrap().0, "obc");
    }

    #[test]
    fn hearst_copula_and_compound_heads() {
        let o = build(
            "The AOCS uses actuators such as reaction wheels and magnetorquers. \
             The star tracker is a type of sensor.",
        );
        assert!(o.is_a("reaction wheel", "actuator").is_some());
        assert!(o.is_a("magnetorquer", "actuator").is_some());
        assert!(o.is_a("star tracker", "sensor").is_some());
        assert!(o.is_a("reaction wheel", "wheel").is_some());
    }
}
