use serde::Serialize;

use crate::analyze::Analyzer;
use crate::bm25;
use crate::index::Index;
use crate::rm3::{self, Params};

/// Risultato del comando `query` (REQ-056).
#[derive(Serialize)]
pub struct Response<'a> {
    pub query: &'a str,
    pub terms: Vec<Term>,
    pub results: Vec<Hit<'a>>,
}

/// Termine della query effettivamente usata.
#[derive(Serialize)]
pub struct Term {
    pub term: String,
    pub weight: f64,
    pub source: &'static str,
}

#[derive(Serialize)]
pub struct Hit<'a> {
    pub file: &'a str,
    pub heading: &'a str,
    pub line: usize,
    pub score: f64,
    pub text: &'a str,
}

/// Cerca la domanda nell'indice con espansione RM3 e restituisce al più `top` sezioni.
pub fn run<'a>(
    index: &'a Index,
    analyzer: &Analyzer,
    q: &'a str,
    top: usize,
    params: &Params,
    heading_weight: f64,
) -> Response<'a> {
    let keywords = keywords(analyzer, q);
    let first = bm25::search(index, &weighted(&keywords), heading_weight);
    let terms = rm3::expand(index, analyzer, keywords, &first, params);
    let results = bm25::search(index, &weighted(&terms), heading_weight)
        .into_iter()
        .take(top)
        .map(|(id, score)| {
            let section = &index.sections[id as usize];
            Hit { file: &section.file, heading: &section.heading, line: section.line, score, text: &section.text }
        })
        .collect();
    Response { query: q, terms, results }
}

fn weighted(terms: &[Term]) -> Vec<(String, f64)> {
    terms.iter().map(|t| (t.term.clone(), t.weight)).collect()
}

/// Keyword della domanda, nell'ordine in cui compaiono, con peso pari alla loro frequenza relativa.
fn keywords(analyzer: &Analyzer, q: &str) -> Vec<Term> {
    let tokens = analyzer.tokens(q);
    let mut terms: Vec<Term> = Vec::new();
    for token in &tokens {
        match terms.iter_mut().find(|t| &t.term == token) {
            Some(term) => term.weight += 1.0,
            None => terms.push(Term { term: token.clone(), weight: 1.0, source: "query" }),
        }
    }
    for term in &mut terms {
        term.weight /= tokens.len() as f64;
    }
    terms
}
