//! Classificatore a regole: `rules.yaml` è un file di dati (appendice C).

use std::fs;
use std::path::Path;

use regex::{Regex, RegexBuilder};
use serde::Deserialize;
use serde_json::{Map, Value, json};
use serde_yaml::{Mapping, Value as Yaml};

use crate::classifier::{Context, Decision};
use crate::markdown::Kind;

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

    /// Valuta le faccette nell'ordine del file: ciascuna vede quelle già decise (REQ-100).
    pub fn classify(&self, text: &str, context: &Context) -> Decision {
        let mut decision = Decision::default();
        for facet in &self.facets {
            if let Some((value, confidence, why)) = facet.decide(text, context, &decision.labels) {
                decision.labels.insert(facet.name.clone(), value);
                decision.confidence.insert(facet.name.clone(), json!(confidence));
                decision.rule_ids.insert(facet.name.clone(), why);
            }
        }
        decision
    }
}

/// Valore, confidenza e motivo di una faccetta.
type Decided = (Value, f64, Value);

impl FacetRules {
    fn decide(&self, text: &str, context: &Context, decided: &Map<String, Value>) -> Option<Decided> {
        match self.mode {
            Mode::First => self.first(text, context, decided).or_else(|| self.mapped(decided)).or_else(|| self.fallback()),
            Mode::Score => self.score(text, context, decided).or_else(|| self.fallback()),
            Mode::Multi => Some(self.multi(text, context, decided)),
        }
    }

    /// Vince la prima regola soddisfatta.
    fn first(&self, text: &str, context: &Context, decided: &Map<String, Value>) -> Option<Decided> {
        let hit = self.rules.iter().find(|r| r.when_ok(context, decided) && r.text_ok(text))?;
        Some((json!(hit.value), hit.confidence, json!(hit.id)))
    }

    /// Valore ricavato da una faccetta già decisa.
    fn mapped(&self, decided: &Map<String, Value>) -> Option<Decided> {
        let map = self.then_map.as_ref()?;
        let source = decided.get(&map.from)?.as_str()?;
        let (_, value) = map.map.iter().find(|(from, _)| from == source)?;
        Some((json!(value), map.confidence, json!(format!("map {}", map.from))))
    }

    /// Vince il valore con più occorrenze; a parità, quello che compare prima nelle regole.
    fn score(&self, text: &str, context: &Context, decided: &Map<String, Value>) -> Option<Decided> {
        let mut scores: Vec<(&str, usize)> = Vec::new();
        for rule in self.rules.iter().filter(|r| r.when_ok(context, decided)) {
            let hits = rule.any_hits(text);
            if hits > 0 {
                match scores.iter_mut().find(|(value, _)| *value == rule.value) {
                    Some((_, score)) => *score += hits,
                    None => scores.push((&rule.value, hits)),
                }
            }
        }
        let &(best, top) = scores.iter().reduce(|best, s| if s.1 > best.1 { s } else { best })?;
        let total: usize = scores.iter().map(|(_, score)| score).sum();
        let confidence = ((0.4 + 0.5 * top as f64 / total as f64) * 100.0).round() / 100.0;
        let list: Vec<String> = scores.iter().map(|(value, score)| format!("{value}: {score}")).collect();
        Some((json!(best), confidence, json!(format!("score {{{}}}", list.join(", ")))))
    }

    /// Lista ordinata dei valori di tutte le regole soddisfatte.
    fn multi(&self, text: &str, context: &Context, decided: &Map<String, Value>) -> Decided {
        let hits: Vec<&Rule> = self.rules.iter().filter(|r| r.when_ok(context, decided) && r.text_ok(text)).collect();
        let mut values: Vec<&str> = hits.iter().map(|r| r.value.as_str()).collect();
        values.sort_unstable();
        values.dedup();
        let lowest = hits.iter().map(|r| r.confidence).reduce(f64::min);
        let confidence = lowest.or(self.default_confidence).unwrap_or(0.6);
        (json!(values), confidence, json!(hits.iter().map(|r| r.id.as_str()).collect::<Vec<_>>()))
    }

    fn fallback(&self) -> Option<Decided> {
        let value = self.default.as_ref()?;
        Some((json!(value), self.default_confidence.unwrap_or(0.3), json!("default")))
    }
}

impl Condition {
    fn holds(&self, context: &Context, decided: &Map<String, Value>) -> bool {
        let allows = |value: Option<&str>| self.allowed.iter().any(|a| a.as_deref() == value);
        match self.key.as_str() {
            "side" => allows(context.side.as_deref()),
            "doc_type" => allows(context.doc_type.as_deref()),
            "kind" => allows(context.kind.map(Kind::as_str)),
            "heading" => {
                let path = context.heading_path.join(" › ");
                self.patterns.iter().any(|p| p.is_match(&path))
            }
            facet => match decided.get(facet) {
                Some(Value::Array(values)) => values.iter().any(|v| allows(v.as_str())),
                Some(value) => allows(value.as_str()),
                None => allows(None),
            },
        }
    }
}

impl Rule {
    /// Tutte le condizioni su contesto e faccette già decise (REQ-101).
    fn when_ok(&self, context: &Context, decided: &Map<String, Value>) -> bool {
        self.when.iter().all(|condition| condition.holds(context, decided))
    }

    /// Condizioni sul testo (REQ-101).
    fn text_ok(&self, text: &str) -> bool {
        let length = text.chars().count();
        self.max_len.is_none_or(|max| length <= max)
            && self.min_len.is_none_or(|min| length >= min)
            && !self.unless.iter().any(|p| p.is_match(text))
            && self.all.iter().all(|p| p.is_match(text))
            && (self.any.is_empty() || self.any_hits(text) >= self.count_min)
    }

    /// Occorrenze totali delle espressioni di `any`.
    fn any_hits(&self, text: &str) -> usize {
        self.any.iter().map(|p| p.find_iter(text).count()).sum()
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

    fn labels(yaml: &str, text: &str) -> Map<String, Value> {
        RuleSet::parse(yaml).unwrap().classify(text, &Context::default()).labels
    }

    #[test]
    fn first_vince_la_prima_regola_che_corrisponde() {
        let yaml = r#"
version: "1"
facets:
  a:
    rules:
      - {value: uno, any: ['\bshall\b']}
      - {value: due, any: ['\bshall\b', '\bwill\b']}
    default: tre
"#;
        assert_eq!(labels(yaml, "It shall work")["a"], "uno");
        assert_eq!(labels(yaml, "It will work")["a"], "due");
        assert_eq!(labels(yaml, "Nothing")["a"], "tre");
    }

    #[test]
    fn score_vince_il_valore_con_piu_corrispondenze() {
        let yaml = r#"
version: "1"
facets:
  a:
    mode: score
    rules:
      - {value: uno, any: ['x']}
      - {value: due, any: ['y']}
      - {value: uno, any: ['z']}
"#;
        assert_eq!(labels(yaml, "x y y")["a"], "due");
        assert_eq!(labels(yaml, "x z y y")["a"], "uno", "stesso punteggio: il valore che compare prima");
        assert_eq!(labels(yaml, "x y")["a"], "uno");
        assert!(!labels(yaml, "nulla").contains_key("a"));
    }

    #[test]
    fn multi_restituisce_la_lista_ordinata_dei_valori() {
        let yaml = r#"
version: "1"
facets:
  a:
    mode: multi
    rules:
      - {value: zeta, any: ['z']}
      - {value: alfa, any: ['a']}
      - {value: zeta, any: ['y']}
"#;
        assert_eq!(labels(yaml, "z a y")["a"], json!(["alfa", "zeta"]));
        assert_eq!(labels(yaml, "niente")["a"], json!([]));
    }

    #[test]
    fn then_map_ricava_il_valore_da_una_faccetta_gia_decisa() {
        let yaml = r#"
version: "1"
facets:
  a:
    rules:
      - {value: uno, any: ['x']}
      - {value: due, any: ['y']}
  b:
    rules:
      - {value: diretto, any: ['k']}
    then_map:
      from: a
      map: {uno: P}
    default: Q
"#;
        assert_eq!(labels(yaml, "x")["b"], "P");
        assert_eq!(labels(yaml, "y")["b"], "Q", "valore non mappato");
        assert_eq!(labels(yaml, "x k")["b"], "diretto");
    }

    fn one_rule(rule: &str) -> RuleSet {
        RuleSet::parse(&format!("version: \"1\"\nfacets:\n  a:\n    rules:\n      - {{value: si, {rule}}}\n")).unwrap()
    }

    fn matches(rule: &str, text: &str, context: &Context) -> bool {
        one_rule(rule).classify(text, context).labels.contains_key("a")
    }

    #[test]
    fn condizioni_sul_testo() {
        let none = Context::default();
        assert!(matches("any: ['x', 'y']", "solo y", &none));
        assert!(!matches("all: ['x', 'y']", "solo y", &none));
        assert!(matches("all: ['x', 'y']", "x e y", &none));
        assert!(!matches("any: ['x'], unless: ['y']", "x e y", &none));
        assert!(!matches("any: ['x'], count_min: 2", "x", &none));
        assert!(matches("any: ['x', 'y'], count_min: 2", "x y", &none));
        assert!(!matches("min_len: 5", "città", &Context { ..Context::default() }) == false, "lunghezza in caratteri");
        assert!(!matches("max_len: 4", "città", &none));
        assert!(matches("any: ['X']", "x", &none), "senza distinzione maiuscole");
        assert!(!matches("any: ['X'], case: true", "x", &none));
    }

    #[test]
    fn condizioni_sul_contesto() {
        let context = Context {
            side: Some("prime".into()),
            doc_type: Some("pa_plan".into()),
            kind: Some(Kind::TableRow),
            heading_path: vec!["4 Verification".into(), "Thermal Tests".into()],
            ..Context::default()
        };
        assert!(matches("when: {side: [prime]}", "t", &context));
        assert!(!matches("when: {side: [supplier]}", "t", &context));
        assert!(!matches("when: {side: [prime]}", "t", &Context::default()));
        assert!(matches("when: {side: [prime, null]}", "t", &Context::default()), "null = lato sconosciuto");
        assert!(matches("when: {doc_type: [pa_plan]}", "t", &context));
        assert!(matches("when: {kind: [table_row]}", "t", &context));
        assert!(!matches("when: {kind: [paragraph]}", "t", &context));
        assert!(matches("when: {heading: ['thermal tests']}", "t", &context));
        assert!(matches("when: {heading: ['Verification › Thermal']}", "t", &context));
        assert!(!matches("when: {heading: ['glossary']}", "t", &context));
    }

    #[test]
    fn condizioni_sulle_faccette_gia_decise() {
        let yaml = r#"
version: "1"
facets:
  a:
    rules:
      - {value: uno, any: ['x']}
  b:
    mode: multi
    rules:
      - {value: flag, any: ['y']}
  c:
    rules:
      - {value: da-a, when: {a: [uno]}}
      - {value: da-b, when: {b: [flag]}}
      - {value: sconosciuta, when: {a: [null]}}
"#;
        assert_eq!(labels(yaml, "x")["c"], "da-a");
        assert_eq!(labels(yaml, "y")["c"], "da-b");
        assert_eq!(labels(yaml, "z")["c"], "sconosciuta");
    }

    #[test]
    fn modalita_sconosciuta() {
        let error = RuleSet::parse("version: \"1\"\nfacets:\n  a:\n    mode: boh\n").unwrap_err();
        assert!(error.contains("modalità sconosciuta"));
    }
}
