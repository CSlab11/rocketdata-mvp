//! Classificatore a regole: `rules.yaml` è un file di dati (appendice C).

use std::fs;
use std::path::Path;

use regex::{Regex, RegexBuilder};
use serde::Deserialize;
use serde_yaml::{Mapping, Value as Yaml};

#[derive(Deserialize)]
struct RawFile {
    version: Yaml,
    #[serde(default)]
    documents: Vec<RawDocument>,
    #[serde(default)]
    facets: Mapping,
}

#[derive(Deserialize)]
struct RawDocument {
    #[serde(rename = "match")]
    pattern: String,
    side: String,
    doc_type: String,
}

#[derive(Deserialize)]
struct RawFacet {
    mode: Option<String>,
    #[serde(default)]
    rules: Vec<RawRule>,
    then_map: Option<RawMap>,
    default: Option<String>,
    default_confidence: Option<f64>,
}

#[derive(Deserialize)]
struct RawMap {
    from: String,
    map: Mapping,
    confidence: Option<f64>,
}

#[derive(Deserialize)]
struct RawRule {
    id: Option<String>,
    value: String,
    #[serde(default)]
    any: Vec<String>,
    #[serde(default)]
    all: Vec<String>,
    #[serde(default)]
    unless: Vec<String>,
    count_min: Option<usize>,
    min_len: Option<usize>,
    max_len: Option<usize>,
    #[serde(default)]
    case: bool,
    #[serde(default)]
    when: Mapping,
    confidence: Option<f64>,
}

/// Lato e tipo assegnati ai documenti il cui nome corrisponde.
#[derive(Debug)]
pub struct DocumentRule {
    pub pattern: Regex,
    pub side: String,
    pub doc_type: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    First,
    Score,
    Multi,
}

/// Regole di una faccetta.
#[derive(Debug)]
pub struct FacetRules {
    pub name: String,
    pub mode: Mode,
    pub rules: Vec<Rule>,
    pub then_map: Option<ThenMap>,
    pub default: Option<String>,
    pub default_confidence: Option<f64>,
}

#[derive(Debug)]
pub struct ThenMap {
    pub from: String,
    pub map: Vec<(String, String)>,
    pub confidence: f64,
}

/// Condizione di contesto: i valori `None` valgono per "sconosciuto".
#[derive(Debug)]
pub struct Condition {
    pub key: String,
    pub allowed: Vec<Option<String>>,
    /// Solo per `heading`.
    pub patterns: Vec<Regex>,
}

#[derive(Debug)]
pub struct Rule {
    pub id: String,
    pub value: String,
    pub any: Vec<Regex>,
    pub all: Vec<Regex>,
    pub unless: Vec<Regex>,
    pub count_min: usize,
    pub min_len: Option<usize>,
    pub max_len: Option<usize>,
    pub when: Vec<Condition>,
    pub confidence: f64,
}

#[derive(Debug)]
pub struct RuleSet {
    pub version: String,
    pub documents: Vec<DocumentRule>,
    /// Nell'ordine del file, che è l'ordine di valutazione.
    pub facets: Vec<FacetRules>,
}

impl RuleSet {
    /// Regole lette dal file indicato (REQ-099).
    pub fn load(path: &Path) -> Result<RuleSet, String> {
        let text = fs::read_to_string(path).map_err(|e| format!("impossibile leggere {}: {e}", path.display()))?;
        RuleSet::parse(&text)
    }

    pub fn parse(text: &str) -> Result<RuleSet, String> {
        let raw: RawFile = serde_yaml::from_str(text).map_err(|e| format!("rules.yaml: {e}"))?;
        let mut documents = Vec::new();
        for document in raw.documents {
            let pattern = compile(&document.pattern, false).map_err(|e| format!("rules.yaml: documents: {e}"))?;
            documents.push(DocumentRule { pattern, side: document.side, doc_type: document.doc_type });
        }
        let mut facets = Vec::new();
        for (name, spec) in raw.facets {
            let name = name.as_str().ok_or("rules.yaml: nome di faccetta non valido")?.to_string();
            let spec: RawFacet = serde_yaml::from_value(spec).map_err(|e| format!("rules.yaml: {name}: {e}"))?;
            facets.push(facet_rules(name, spec)?);
        }
        Ok(RuleSet { version: yaml_string(&raw.version).unwrap_or_else(|| "0".to_string()), documents, facets })
    }
}

fn facet_rules(name: String, spec: RawFacet) -> Result<FacetRules, String> {
    let mode = match spec.mode.as_deref() {
        None | Some("first") => Mode::First,
        Some("score") => Mode::Score,
        Some("multi") => Mode::Multi,
        Some(other) => return Err(format!("rules.yaml: {name}: modalità sconosciuta '{other}'")),
    };
    let mut rules = Vec::new();
    for (index, rule) in spec.rules.into_iter().enumerate() {
        let id = rule.id.clone().unwrap_or_else(|| format!("{name}#{index}"));
        rules.push(compile_rule(id, rule).map_err(|e| format!("rules.yaml: {name}: {e}"))?);
    }
    let then_map = spec.then_map.map(|m| ThenMap {
        from: m.from,
        map: m.map.iter().filter_map(|(k, v)| Some((yaml_string(k)?, yaml_string(v)?))).collect(),
        confidence: m.confidence.unwrap_or(0.5),
    });
    Ok(FacetRules { name, mode, rules, then_map, default: spec.default, default_confidence: spec.default_confidence })
}

fn compile_rule(id: String, rule: RawRule) -> Result<Rule, String> {
    let list = |patterns: &[String]| patterns.iter().map(|p| compile(p, rule.case)).collect::<Result<Vec<_>, _>>();
    let mut when = Vec::new();
    for (key, values) in &rule.when {
        let key = yaml_string(key).ok_or("when: chiave non valida")?;
        let allowed: Vec<Option<String>> = match values {
            Yaml::Sequence(values) => values.iter().map(yaml_string).collect(),
            value => vec![yaml_string(value)],
        };
        let patterns = if key == "heading" {
            allowed.iter().flatten().map(|p| compile(p, false)).collect::<Result<Vec<_>, _>>()?
        } else {
            Vec::new()
        };
        when.push(Condition { key, allowed, patterns });
    }
    Ok(Rule {
        any: list(&rule.any)?,
        all: list(&rule.all)?,
        unless: list(&rule.unless)?,
        id,
        value: rule.value,
        count_min: rule.count_min.unwrap_or(1),
        min_len: rule.min_len,
        max_len: rule.max_len,
        when,
        confidence: rule.confidence.unwrap_or(0.5),
    })
}

/// Le espressioni non distinguono maiuscole e minuscole, salvo `case: true`.
fn compile(pattern: &str, case: bool) -> Result<Regex, String> {
    RegexBuilder::new(pattern)
        .case_insensitive(!case)
        .build()
        .map_err(|e| format!("espressione non valida '{pattern}': {e}"))
}

fn yaml_string(value: &Yaml) -> Option<String> {
    match value {
        Yaml::String(s) => Some(s.clone()),
        Yaml::Number(n) => Some(n.to_string()),
        Yaml::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RULES: &str = include_str!("../config/rules.yaml");

    #[test]
    fn legge_il_file_di_regole() {
        let rules = RuleSet::parse(RULES).unwrap();
        assert_eq!(rules.version, "0.1.0");
        assert_eq!(rules.documents.len(), 9);
        let names: Vec<_> = rules.facets.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["extraction_quality", "nature", "domain", "destination", "required_action", "flags"]);
        let nature = &rules.facets[1];
        assert_eq!(nature.mode, Mode::First);
        assert_eq!(nature.rules[0].id, "def-table");
        assert_eq!(rules.facets[2].mode, Mode::Score);
        assert_eq!(rules.facets[2].rules[0].id, "domain#0");
        assert_eq!(rules.facets[5].mode, Mode::Multi);
        assert_eq!(rules.facets[3].then_map.as_ref().unwrap().from, "domain");
    }

    #[test]
    fn il_file_viene_dal_percorso_indicato() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("regole.yaml");
        fs::write(&path, "version: \"2\"\nfacets:\n  a:\n    rules:\n      - {value: x}\n").unwrap();
        let rules = RuleSet::load(&path).unwrap();
        assert_eq!(rules.version, "2");
        assert_eq!(rules.facets[0].rules[0].id, "a#0");
    }

    #[test]
    fn modalita_sconosciuta() {
        let error = RuleSet::parse("version: \"1\"\nfacets:\n  a:\n    mode: boh\n").unwrap_err();
        assert!(error.contains("modalità sconosciuta"));
    }
}
