use std::collections::BTreeMap;

use plotforge_schema::{Condition, Effect, ResourceDefinition, Rule, WorldDelta, WorldState};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RuleError {
    #[error("resource `{0}` is not defined")]
    UnknownResource(String),
}

#[derive(Clone, Debug)]
pub struct RuleEngine {
    resources: BTreeMap<String, ResourceDefinition>,
    rules: Vec<Rule>,
}

impl RuleEngine {
    pub fn new(resources: Vec<ResourceDefinition>, rules: Vec<Rule>) -> Self {
        let resources = resources
            .into_iter()
            .map(|resource| (resource.key.clone(), resource))
            .collect();
        Self { resources, rules }
    }

    pub fn evaluate(&self, action_type: &str, state: &WorldState) -> Result<WorldDelta, RuleError> {
        let mut next_state = state.clone();
        let mut delta = WorldDelta::default();

        for rule in self
            .rules
            .iter()
            .filter(|rule| rule.action_type == action_type)
        {
            if !rule
                .conditions
                .iter()
                .all(|condition| matches_condition(condition, &next_state))
            {
                continue;
            }

            for effect in &rule.effects {
                apply_effect(effect, &self.resources, &mut next_state, &mut delta)?;
            }
        }

        Ok(delta)
    }

    pub fn apply_delta(
        &self,
        state: &WorldState,
        delta: &WorldDelta,
    ) -> Result<WorldState, RuleError> {
        let mut next = state.clone();

        for (key, amount) in &delta.resource_changes {
            let current = next.resources.get(key).copied().unwrap_or_default();
            next.resources
                .insert(key.clone(), self.clamp_resource(key, current + amount)?);
        }

        for (key, value) in &delta.resource_sets {
            next.resources
                .insert(key.clone(), self.clamp_resource(key, *value)?);
        }

        for (key, value) in &delta.flags {
            next.flags.insert(key.clone(), *value);
        }

        for event in &delta.triggered_events {
            if !next.triggered_events.contains(event) {
                next.triggered_events.push(event.clone());
            }
        }

        Ok(next)
    }

    fn clamp_resource(&self, key: &str, value: i32) -> Result<i32, RuleError> {
        let definition = self
            .resources
            .get(key)
            .ok_or_else(|| RuleError::UnknownResource(key.to_string()))?;
        Ok(value.clamp(definition.min, definition.max))
    }
}

fn matches_condition(condition: &Condition, state: &WorldState) -> bool {
    match condition {
        Condition::ResourceAtLeast { key, value } => {
            state.resources.get(key).copied().unwrap_or_default() >= *value
        }
        Condition::ResourceAtMost { key, value } => {
            state.resources.get(key).copied().unwrap_or_default() <= *value
        }
        Condition::FlagEquals { key, value } => {
            state.flags.get(key).copied().unwrap_or_default() == *value
        }
    }
}

fn apply_effect(
    effect: &Effect,
    resources: &BTreeMap<String, ResourceDefinition>,
    state: &mut WorldState,
    delta: &mut WorldDelta,
) -> Result<(), RuleError> {
    match effect {
        Effect::AddResource { key, amount } => {
            let definition = resources
                .get(key)
                .ok_or_else(|| RuleError::UnknownResource(key.clone()))?;
            let current = state.resources.get(key).copied().unwrap_or_default();
            let next = (current + amount).clamp(definition.min, definition.max);
            state.resources.insert(key.clone(), next);
            *delta.resource_changes.entry(key.clone()).or_default() += next - current;
        }
        Effect::SetResource { key, value } => {
            let definition = resources
                .get(key)
                .ok_or_else(|| RuleError::UnknownResource(key.clone()))?;
            let next = (*value).clamp(definition.min, definition.max);
            state.resources.insert(key.clone(), next);
            delta.resource_sets.insert(key.clone(), next);
        }
        Effect::SetFlag { key, value } => {
            state.flags.insert(key.clone(), *value);
            delta.flags.insert(key.clone(), *value);
        }
        Effect::TriggerEvent { event } => {
            if !state.triggered_events.contains(event) {
                state.triggered_events.push(event.clone());
            }
            if !delta.triggered_events.contains(event) {
                delta.triggered_events.push(event.clone());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use plotforge_schema::{Condition, Effect, ResourceDefinition, Rule, WorldState};

    use super::RuleEngine;

    #[test]
    fn applies_matching_action_effects_and_clamps_resources() {
        let resources = vec![ResourceDefinition {
            key: "treasury".into(),
            label: "Treasury".into(),
            initial: 50,
            min: 0,
            max: 100,
        }];
        let rules = vec![Rule {
            id: "raise-tax".into(),
            action_type: "raise_tax".into(),
            conditions: vec![Condition::ResourceAtLeast {
                key: "treasury".into(),
                value: 40,
            }],
            effects: vec![
                Effect::AddResource {
                    key: "treasury".into(),
                    amount: 80,
                },
                Effect::SetFlag {
                    key: "tax_resistance".into(),
                    value: true,
                },
                Effect::TriggerEvent {
                    event: "local_tax_resistance".into(),
                },
            ],
        }];

        let mut state = WorldState::default();
        state.resources.insert("treasury".into(), 50);
        let engine = RuleEngine::new(resources, rules);

        let delta = engine.evaluate("raise_tax", &state).expect("evaluate");
        let next = engine.apply_delta(&state, &delta).expect("apply");

        assert_eq!(delta.resource_changes["treasury"], 50);
        assert_eq!(next.resources["treasury"], 100);
        assert!(next.flags["tax_resistance"]);
        assert_eq!(next.triggered_events, vec!["local_tax_resistance"]);
    }

    #[test]
    fn ignores_non_matching_actions() {
        let engine = RuleEngine::new(
            vec![ResourceDefinition {
                key: "army_morale".into(),
                label: "Army morale".into(),
                initial: 30,
                min: 0,
                max: 100,
            }],
            vec![Rule {
                id: "pay-army".into(),
                action_type: "pay_army".into(),
                conditions: Vec::new(),
                effects: vec![Effect::AddResource {
                    key: "army_morale".into(),
                    amount: 10,
                }],
            }],
        );

        let state = WorldState::default();
        let delta = engine.evaluate("raise_tax", &state).expect("evaluate");
        assert!(delta.is_empty());
    }

    #[test]
    fn supports_flag_conditions_set_resource_and_event_dedupe() {
        let engine = RuleEngine::new(
            vec![ResourceDefinition {
                key: "treasury".into(),
                label: "Treasury".into(),
                initial: 40,
                min: 0,
                max: 100,
            }],
            vec![Rule {
                id: "stabilize-after-investigation".into(),
                action_type: "stabilize".into(),
                conditions: vec![
                    Condition::FlagEquals {
                        key: "corruption_investigation".into(),
                        value: true,
                    },
                    Condition::ResourceAtMost {
                        key: "treasury".into(),
                        value: 50,
                    },
                ],
                effects: vec![
                    Effect::SetResource {
                        key: "treasury".into(),
                        value: 60,
                    },
                    Effect::TriggerEvent {
                        event: "officials_submit_memorials".into(),
                    },
                    Effect::TriggerEvent {
                        event: "officials_submit_memorials".into(),
                    },
                ],
            }],
        );
        let mut state = WorldState::default();
        state.resources.insert("treasury".into(), 40);
        state.flags.insert("corruption_investigation".into(), true);

        let delta = engine.evaluate("stabilize", &state).expect("evaluate");
        let next = engine.apply_delta(&state, &delta).expect("apply");

        assert_eq!(next.resources["treasury"], 60);
        assert_eq!(next.triggered_events, vec!["officials_submit_memorials"]);
    }

    #[test]
    fn returns_error_for_unknown_resource_effect() {
        let engine = RuleEngine::new(
            Vec::new(),
            vec![Rule {
                id: "bad".into(),
                action_type: "bad_action".into(),
                conditions: Vec::new(),
                effects: vec![Effect::AddResource {
                    key: "missing".into(),
                    amount: 1,
                }],
            }],
        );

        let error = engine
            .evaluate("bad_action", &WorldState::default())
            .expect_err("unknown resource should fail");
        assert_eq!(error, super::RuleError::UnknownResource("missing".into()));
    }
}
