//! Casi di prova di riferimento (appendice I).

use rocket_tag::classifier::Context;
use rocket_tag::rules::RuleSet;
use rocket_tag::schema::Schema;
use serde_json::Value;

const SCHEMA: &str = include_str!("../config/label_schema.json");
const RULES: &str = include_str!("../config/rules.yaml");
const EXAMPLES: &str = include_str!("../config/guide_examples.json");

#[test]
fn i_13_esempi_danno_13_su_13() {
    let schema = Schema::parse(SCHEMA).unwrap();
    let rules = RuleSet::parse(RULES).unwrap();
    let examples: Vec<Value> = serde_json::from_str(EXAMPLES).unwrap();
    assert_eq!(examples.len(), 13);
    let mut wrong = Vec::new();
    for example in &examples {
        let side = example["side"].as_str().unwrap();
        let text = example["text"].as_str().unwrap();
        let context = Context { side: Some(side.into()), ..Context::default() };
        let decision = rules.classify(&schema, text, &context);
        for facet in ["nature", "domain", "destination", "required_action"] {
            if decision.labels.get(facet) != example["labels"].get(facet) {
                wrong.push(format!("{text}: {facet} = {:?}, atteso {}", decision.labels.get(facet), example["labels"][facet]));
            }
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

#[test]
fn x_fixed_when_vale_per_ogni_uscita() {
    let schema = Schema::parse(SCHEMA).unwrap();
    let rules = RuleSet::parse(RULES).unwrap();
    let context = Context { side: Some("supplier".into()), ..Context::default() };
    let decision = rules.classify(&schema, "The supplier shall deliver the Test Report.", &context);
    assert_eq!(decision.labels["required_action"], "none");
    assert_eq!(decision.confidence["required_action"], 1.0);
    assert_eq!(decision.rule_ids["required_action"], "schema x-fixedWhen");
    let names: Vec<_> = decision.labels.keys().map(String::as_str).collect();
    assert_eq!(names, ["nature", "domain", "destination", "required_action", "extraction_quality", "flags"]);
}
