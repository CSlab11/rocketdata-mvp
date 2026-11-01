//! Classificatore a regole: `rules.yaml` è un file di dati (appendice C).

use std::fs;
use std::path::Path;

use regex::{Regex, RegexBuilder};
use serde::Deserialize;
use serde_json::{Map, Value, json};
use serde_yaml::{Mapping, Value as Yaml};

use crate::classifier::{Context, Decision, DocInfo};
use crate::markdown::Kind;
use crate::schema::{Facet, Schema};

/// Chiavi di `when` che non sono faccette.
const CONTEXT_KEYS: [&str; 4] = ["side", "doc_type", "kind", "heading"];

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
    /// Espressioni che il crate `regex` non compila, tutte, per il controllo (REQ-102).
    pub pattern_errors: Vec<String>,
}

impl RuleSet {
    /// Regole lette dal file indicato (REQ-099).
    pub fn load(path: &Path) -> Result<RuleSet, String> {
        let text = fs::read_to_string(path).map_err(|e| format!("impossibile leggere {}: {e}", path.display()))?;
        RuleSet::parse(&text)
    }

    pub fn parse(text: &str) -> Result<RuleSet, String> {
        let raw: RawFile = serde_yaml::from_str(text).map_err(|e| format!("rules.yaml: {e}"))?;
        let mut pattern_errors = Vec::new();
        let mut documents = Vec::new();
        for document in raw.documents {
            match compile(&document.pattern, false) {
                Ok(pattern) => documents.push(DocumentRule { pattern, side: document.side, doc_type: document.doc_type }),
                Err(error) => pattern_errors.push(format!("documents: {error}")),
            }
        }
        let mut facets = Vec::new();
        for (name, spec) in raw.facets {
            let name = name.as_str().ok_or("rules.yaml: nome di faccetta non valido")?.to_string();
            let spec: RawFacet = serde_yaml::from_value(spec).map_err(|e| format!("rules.yaml: {name}: {e}"))?;
            facets.push(facet_rules(name, spec, &mut pattern_errors)?);
        }
        Ok(RuleSet { version: yaml_string(&raw.version).unwrap_or_else(|| "0".to_string()), documents, facets, pattern_errors })
    }

    /// Lato e tipo dalla prima regola `documents` che corrisponde al nome del documento (REQ-089).
    pub fn document_info(&self, document: &str) -> Option<DocInfo> {
        let rule = self.documents.iter().find(|d| d.pattern.is_match(document))?;
        Some(DocInfo { side: Some(rule.side.clone()), doc_type: Some(rule.doc_type.clone()) })
    }

    /// Controllo delle regole contro lo schema (REQ-103).
    pub fn check(&self, schema: &Schema) -> Check {
        let mut check = Check { errors: self.pattern_errors.clone(), warnings: Vec::new() };
        for rules in &self.facets {
            match schema.facet(&rules.name) {
                Some(facet) => rules.check(facet, schema, &mut check),
                None => check.errors.push(format!("rules.yaml: faccetta '{}' non esiste nello schema", rules.name)),
            }
        }
        for facet in &schema.facets {
            if !self.facets.iter().any(|f| f.name == facet.name) {
                check.warnings.push(format!("faccetta '{}' senza regole: le regole si astengono", facet.name));
            }
        }
        for document in &self.documents {
            if !schema.sides.contains(&document.side) {
                check.errors.push(format!("documents: side '{}' non valido", document.side));
            }
            if !schema.doc_types.contains(&document.doc_type) {
                check.errors.push(format!("documents: doc_type '{}' non valido", document.doc_type));
            }
        }
        check
    }

    /// Etichette di un'unità, con i vincoli dello schema applicati (REQ-096).
    pub fn classify(&self, schema: &Schema, text: &str, context: &Context) -> Decision {
        self.apply(text, context).finish(schema, context.side.as_deref())
    }

    /// Valuta le faccette nell'ordine del file: ciascuna vede quelle già decise (REQ-100).
    fn apply(&self, text: &str, context: &Context) -> Decision {
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

/// Esito del controllo: gli errori bloccano l'esecuzione, gli avvisi no.
#[derive(Debug, Default, PartialEq)]
pub struct Check {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Valore, confidenza e motivo di una faccetta.
type Decided = (Value, f64, Value);

impl FacetRules {
    fn check(&self, facet: &Facet, schema: &Schema, check: &mut Check) {
        let name = &self.name;
        let mut produced: Vec<&str> = self.rules.iter().map(|r| r.value.as_str()).collect();
        produced.extend(self.default.as_deref());
        if let Some(map) = &self.then_map {
            produced.extend(map.map.iter().map(|(_, value)| value.as_str()));
            match schema.facet(&map.from) {
                Some(source) => {
                    let missing: Vec<&str> =
                        source.values.iter().map(String::as_str).filter(|v| !map.map.iter().any(|(k, _)| k == v)).collect();
                    if !missing.is_empty() {
                        check.warnings.push(format!("{name}.then_map: valori di {} non mappati {missing:?}", map.from));
                    }
                }
                None => check.errors.push(format!("{name}.then_map.from: faccetta '{}' non esiste", map.from)),
            }
        }
        for value in &produced {
            if !facet.values.iter().any(|v| v == value) {
                check.errors.push(format!("{name}: valore '{value}' non esiste nello schema"));
            }
        }
        for rule in &self.rules {
            for condition in rule.when.iter().filter(|c| !CONTEXT_KEYS.contains(&c.key.as_str())) {
                match schema.facet(&condition.key) {
                    Some(source) => {
                        for value in condition.allowed.iter().flatten() {
                            if !source.values.contains(value) {
                                check.errors.push(format!("{name}/{}: when.{} valore sconosciuto '{value}'", rule.id, condition.key));
                            }
                        }
                    }
                    None => check.errors.push(format!("{name}/{}: when.{} non è una faccetta", rule.id, condition.key)),
                }
            }
        }
        let unused: Vec<&str> = facet.values.iter().map(String::as_str).filter(|v| !produced.contains(v)).collect();
        if !unused.is_empty() {
            check.warnings.push(format!("{name}: valori mai prodotti dalle regole {unused:?}"));
        }
    }

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

fn facet_rules(name: String, spec: RawFacet, errors: &mut Vec<String>) -> Result<FacetRules, String> {
    let mode = match spec.mode.as_deref() {
        None | Some("first") => Mode::First,
        Some("score") => Mode::Score,
        Some("multi") => Mode::Multi,
        Some(other) => return Err(format!("rules.yaml: {name}: modalità sconosciuta '{other}'")),
    };
    let mut rules = Vec::new();
    for (index, rule) in spec.rules.into_iter().enumerate() {
        let id = rule.id.clone().unwrap_or_else(|| format!("{name}#{index}"));
        rules.push(compile_rule(&name, id, rule, errors));
    }
    let then_map = spec.then_map.map(|m| ThenMap {
        from: m.from,
        map: m.map.iter().filter_map(|(k, v)| Some((yaml_string(k)?, yaml_string(v)?))).collect(),
        confidence: m.confidence.unwrap_or(0.5),
    });
    Ok(FacetRules { name, mode, rules, then_map, default: spec.default, default_confidence: spec.default_confidence })
}

fn compile_rule(facet: &str, id: String, rule: RawRule, errors: &mut Vec<String>) -> Rule {
    let mut list = |patterns: &[String], case: bool| -> Vec<Regex> {
        let mut compiled = Vec::new();
        for pattern in patterns {
            match compile(pattern, case) {
                Ok(regex) => compiled.push(regex),
                Err(error) => errors.push(format!("{facet}/{id}: {error}")),
            }
        }
        compiled
    };
    let mut when = Vec::new();
    for (key, values) in &rule.when {
        let Some(key) = yaml_string(key) else { continue };
        let allowed: Vec<Option<String>> = match values {
            Yaml::Sequence(values) => values.iter().map(yaml_string).collect(),
            value => vec![yaml_string(value)],
        };
        let headings: Vec<String> = if key == "heading" { allowed.iter().flatten().cloned().collect() } else { Vec::new() };
        when.push(Condition { key, allowed, patterns: list(&headings, false) });
    }
    Rule {
        any: list(&rule.any, rule.case),
        all: list(&rule.all, rule.case),
        unless: list(&rule.unless, rule.case),
        id,
        value: rule.value,
        count_min: rule.count_min.unwrap_or(1),
        min_len: rule.min_len,
        max_len: rule.max_len,
        when,
        confidence: rule.confidence.unwrap_or(0.5),
    }
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
        RuleSet::parse(yaml).unwrap().apply(text, &Context::default()).labels
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
        one_rule(rule).apply(text, context).labels.contains_key("a")
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
        assert!(matches("min_len: 5", "città", &none), "lunghezza in caratteri, non in byte");
        assert!(!matches("min_len: 6", "città", &none));
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
    fn astensione_se_nessuna_regola_si_applica_e_manca_il_valore_predefinito() {
        let rules = RuleSet::parse(
            "version: \"1\"\nfacets:\n  a:\n    rules:\n      - {value: x, any: ['k']}\n    default: null\n  b:\n    rules:\n      - {value: y, any: ['k']}\n",
        )
        .unwrap();
        let decision = rules.apply("niente", &Context::default());
        assert_eq!(decision, Decision::default(), "faccette assenti da etichette, confidenza e regole");
    }

    #[test]
    fn registra_regola_e_confidenza_di_ogni_faccetta() {
        let yaml = r#"
version: "1"
facets:
  regola:
    rules:
      - {id: mia, value: x, any: ['k'], confidence: 0.8}
      - {value: y, any: ['j']}
  predefinito:
    rules:
      - {value: x, any: ['mai']}
    default: z
    default_confidence: 0.4
  senza_conf:
    rules:
      - {value: x, any: ['mai']}
    default: z
  mappa:
    rules:
      - {value: x, any: ['mai']}
    then_map: {from: regola, map: {x: P}, confidence: 0.6}
  punteggio:
    mode: score
    rules:
      - {value: a, any: ['k']}
      - {value: b, any: ['p', 'q']}
  multipla:
    mode: multi
    rules:
      - {id: m1, value: u, any: ['k'], confidence: 0.7}
      - {id: m2, value: v, any: ['p'], confidence: 0.6}
  multipla_vuota:
    mode: multi
    rules:
      - {value: u, any: ['mai']}
    default_confidence: 0.2
"#;
        let decision = RuleSet::parse(yaml).unwrap().apply("k p q", &Context::default());
        let (confidence, why) = (&decision.confidence, &decision.rule_ids);
        assert_eq!((&confidence["regola"], &why["regola"]), (&json!(0.8), &json!("mia")));
        assert_eq!((&confidence["predefinito"], &why["predefinito"]), (&json!(0.4), &json!("default")));
        assert_eq!(confidence["senza_conf"], json!(0.3));
        assert_eq!((&confidence["mappa"], &why["mappa"]), (&json!(0.6), &json!("map regola")));
        assert_eq!(decision.labels["punteggio"], "b");
        assert_eq!(confidence["punteggio"], json!(0.73), "0,4 + 0,5 × 2 ÷ 3");
        assert_eq!(why["punteggio"], "score {a: 1, b: 2}");
        assert_eq!((&confidence["multipla"], &why["multipla"]), (&json!(0.6), &json!(["m1", "m2"])));
        assert_eq!(confidence["multipla_vuota"], json!(0.2));

        let plain = RuleSet::parse(yaml).unwrap().apply("j", &Context::default());
        assert_eq!((&plain.confidence["regola"], &plain.rule_ids["regola"]), (&json!(0.5), &json!("regola#1")));
    }

    const SCHEMA: &str = include_str!("../config/label_schema.json");

    fn check(yaml: &str) -> Check {
        RuleSet::parse(yaml).unwrap().check(&Schema::parse(SCHEMA).unwrap())
    }

    #[test]
    fn le_regole_attuali_sono_coerenti_con_lo_schema() {
        let result = check(RULES);
        assert!(result.errors.is_empty(), "{:?}", result.errors);
    }

    #[test]
    fn il_controllo_blocca_faccette_e_valori_inesistenti() {
        let errors = check(
            r#"
version: "1"
documents:
  - {match: 'x', side: altro, doc_type: pa_plan}
  - {match: 'y', side: prime, doc_type: boh}
facets:
  inventata:
    rules:
      - {value: x}
  nature:
    rules:
      - {value: mai, when: {domain: [mai], inesistente: [x]}}
    then_map: {from: manca, map: {x: requirement}}
"#,
        )
        .errors;
        let expected = [
            "faccetta 'inventata' non esiste",
            "side 'altro' non valido",
            "doc_type 'boh' non valido",
            "nature: valore 'mai' non esiste",
            "when.domain valore sconosciuto 'mai'",
            "when.inesistente non è una faccetta",
            "then_map.from: faccetta 'manca' non esiste",
        ];
        for text in expected {
            assert!(errors.iter().any(|e| e.contains(text)), "{text}: {errors:?}");
        }
    }

    #[test]
    fn il_controllo_avvisa_senza_bloccare() {
        let result = check(
            r#"
version: "1"
facets:
  destination:
    rules:
      - {value: P}
    then_map: {from: domain, map: {technical: P}}
"#,
        );
        assert!(result.errors.is_empty(), "{:?}", result.errors);
        let warned = |text: &str| result.warnings.iter().any(|w| w.contains(text));
        assert!(warned("destination: valori mai prodotti"), "{:?}", result.warnings);
        assert!(warned("destination.then_map: valori di domain non mappati"));
        assert!(warned("faccetta 'nature' senza regole"));
    }

    #[test]
    fn segnala_ogni_espressione_non_compilabile() {
        let errors = check(
            r#"
version: "1"
documents:
  - {match: '[', side: prime, doc_type: pa_plan}
facets:
  nature:
    rules:
      - {id: guarda, value: requirement, any: ['(?=shall)x']}
      - {id: gruppo, value: option, unless: ['(aperta'], when: {heading: ['*']}}
"#,
        )
        .errors;
        assert_eq!(errors.len(), 4, "{errors:?}");
        assert!(errors.iter().any(|e| e.starts_with("documents: espressione non valida '['")));
        assert!(errors.iter().any(|e| e.starts_with("nature/guarda: espressione non valida '(?=shall)x'")));
        assert!(errors.iter().any(|e| e.starts_with("nature/gruppo: espressione non valida '(aperta'")));
        assert!(errors.iter().any(|e| e.starts_with("nature/gruppo: espressione non valida '*'")));
    }

    #[test]
    fn lato_e_tipo_dal_nome_del_documento() {
        let rules = RuleSet::parse(RULES).unwrap();
        let info = rules.document_info("[TD] unita.md").unwrap();
        assert_eq!(info, DocInfo { side: Some("supplier".into()), doc_type: Some("technical_description".into()) });
        assert_eq!(rules.document_info("the rfq.md").unwrap().side.as_deref(), Some("prime"), "senza distinzione maiuscole");
        assert_eq!(rules.document_info("appunti.md"), None);
    }

    #[test]
    fn modalita_sconosciuta() {
        let error = RuleSet::parse("version: \"1\"\nfacets:\n  a:\n    mode: boh\n").unwrap_err();
        assert!(error.contains("modalità sconosciuta"));
    }
}
