use std::collections::HashMap;

use crate::analyze::Analyzer;
use crate::index::Index;
use crate::query::Term;

/// Parametri dell'espansione RM3.
pub struct Params {
    /// Sezioni della prima ricerca usate per l'espansione.
    pub fb_docs: usize,
    /// Termini di espansione.
    pub fb_terms: usize,
    /// Peso della query originale.
    pub lambda: f64,
}

impl Default for Params {
    fn default() -> Self {
        Params { fb_docs: 10, fb_terms: 10, lambda: 0.5 }
    }
}

/// Query finale RM3 (REQ-065): le keyword con peso λ·P(t|Q) e i termini di espansione con
/// peso (1 − λ)·P(t|R), dove P(t|R) somma su ogni sezione della prima ricerca la frequenza
/// relativa del termine nella sezione, pesata con il punteggio normalizzato della sezione.
/// Senza termini di espansione resta la query originale.
pub fn expand(index: &Index, analyzer: &Analyzer, keywords: Vec<Term>, first: &[(u32, f64)], params: &Params) -> Vec<Term> {
    let feedback = &first[..first.len().min(params.fb_docs)];
    let total: f64 = feedback.iter().map(|&(_, score)| score).sum();
    let mut relevance: HashMap<String, f64> = HashMap::new();
    for &(id, score) in feedback {
        let tokens = analyzer.tokens(&index.sections[id as usize].text);
        for token in &tokens {
            if !is_number(token) && !keywords.iter().any(|k| &k.term == token) {
                *relevance.entry(token.clone()).or_default() += score / total / tokens.len() as f64;
            }
        }
    }
    let mut expansion: Vec<(String, f64)> = relevance.into_iter().collect();
    expansion.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    expansion.truncate(params.fb_terms);
    let mass: f64 = expansion.iter().map(|(_, p)| p).sum();
    if expansion.is_empty() || params.lambda >= 1.0 {
        return keywords;
    }

    let mut terms: Vec<Term> =
        keywords.into_iter().map(|k| Term { weight: params.lambda * k.weight, ..k }).filter(|k| k.weight > 0.0).collect();
    terms.extend(expansion.into_iter().map(|(term, p)| Term { term, weight: (1.0 - params.lambda) * p / mass, source: "rm3" }));
    terms
}

/// Numeri puri (es. numeri di pagina, `0x40021000`), esclusi dai termini di espansione.
fn is_number(token: &str) -> bool {
    let hex = token.strip_prefix("0x").filter(|h| !h.is_empty() && h.chars().all(|c| c.is_ascii_hexdigit()));
    hex.is_some() || token.chars().all(|c| c.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::IndexedSection;

    fn index(texts: &[&str]) -> Index {
        let sections = texts
            .iter()
            .map(|t| IndexedSection { file: String::new(), heading: String::new(), line: 1, text: t.to_string() })
            .collect();
        Index { sections, postings: HashMap::new(), lengths: Vec::new(), avg_length: 0.0 }
    }

    fn keyword(term: &str, weight: f64) -> Term {
        Term { term: term.to_string(), weight, source: "query" }
    }

    #[test]
    fn pesi_lambda_e_uno_meno_lambda() {
        let index = index(&["clock bus bus timer", "clock bus"]);
        let terms = expand(&index, &Analyzer::new(), vec![keyword("clock", 1.0)], &[(0, 2.0), (1, 1.0)], &Params::default());
        assert_eq!(terms[0].term, "clock");
        assert_eq!(terms[0].source, "query");
        assert!((terms[0].weight - 0.5).abs() < 1e-12);
        let expanded: Vec<&str> = terms[1..].iter().map(|t| t.term.as_str()).collect();
        assert_eq!(expanded, ["bus", "timer"]);
        assert!(terms[1..].iter().all(|t| t.source == "rm3"));
        let sum: f64 = terms.iter().map(|t| t.weight).sum();
        assert!((sum - 1.0).abs() < 1e-12);
    }

    #[test]
    fn rispetta_fb_docs_e_fb_terms() {
        let index = index(&["clock bus timer", "clock register"]);
        let params = Params { fb_docs: 1, fb_terms: 1, lambda: 0.5 };
        let terms = expand(&index, &Analyzer::new(), vec![keyword("clock", 1.0)], &[(0, 2.0), (1, 1.0)], &params);
        assert_eq!(terms.len(), 2);
        assert_ne!(terms[1].term, "regist");
    }

    #[test]
    fn esclude_keyword_e_numeri_puri() {
        let index = index(&["clock 42 0x40021000 apb2"]);
        let terms = expand(&index, &Analyzer::new(), vec![keyword("clock", 1.0)], &[(0, 1.0)], &Params::default());
        let expanded: Vec<&str> = terms[1..].iter().map(|t| t.term.as_str()).collect();
        assert_eq!(expanded, ["apb2"]);
    }

    #[test]
    fn senza_espansione_resta_la_query_originale() {
        let index = index(&["clock"]);
        let terms = expand(&index, &Analyzer::new(), vec![keyword("clock", 1.0)], &[(0, 1.0)], &Params::default());
        assert_eq!(terms.len(), 1);
        assert_eq!(terms[0].weight, 1.0);
        assert!(expand(&index, &Analyzer::new(), Vec::new(), &[], &Params::default()).is_empty());
    }
}
