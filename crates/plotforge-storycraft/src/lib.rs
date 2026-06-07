use std::collections::BTreeSet;

use plotforge_schema::{
    Character, EmotionalArcPoint, NarrativeIssue, NarrativeIssueKind, NarrativeReview, PlotThread,
    PlotThreadStatus, Scene, Severity, StoryCraftBible, StoryCraftState,
};

pub fn dynasty_embers_story_craft() -> StoryCraftState {
    StoryCraftState {
        bible: StoryCraftBible {
            genre_promise: "Historical crisis simulation with court intrigue".into(),
            central_question:
                "Can the player extend a collapsing dynasty under internal and external pressure?"
                    .into(),
            target_emotions: vec![
                "pressure".into(),
                "lonely authority".into(),
                "short-term relief versus long-term cost".into(),
            ],
            core_foreshadowing: vec![
                "border payroll gap".into(),
                "court insider betrayal".into(),
                "local tax disorder".into(),
                "capital relocation dispute".into(),
            ],
        },
        emotional_arc: vec![
            EmotionalArcPoint {
                scene_key: "court-crisis-001".into(),
                target_emotion: "pressure".into(),
                intensity: 70,
            },
            EmotionalArcPoint {
                scene_key: "court-crisis-002".into(),
                target_emotion: "uncertainty".into(),
                intensity: 78,
            },
            EmotionalArcPoint {
                scene_key: "court-crisis-003".into(),
                target_emotion: "consequence".into(),
                intensity: 86,
            },
        ],
        plot_threads: vec![
            PlotThread {
                id: "border-payroll".into(),
                title: "Border payroll gap".into(),
                promise: "The border army is loyal only while pay keeps flowing.".into(),
                status: PlotThreadStatus::Open,
                last_update: "Rumors mention unpaid soldiers outside the pass.".into(),
            },
            PlotThread {
                id: "court-insider".into(),
                title: "Court insider betrayal".into(),
                promise: "A trusted court channel is leaking decisions before they are issued."
                    .into(),
                status: PlotThreadStatus::Open,
                last_update: "Memorials arrive too quickly after sealed conversations.".into(),
            },
            PlotThread {
                id: "tax-disorder".into(),
                title: "Local tax disorder".into(),
                promise: "Short-term revenue measures can ignite resistance in the provinces."
                    .into(),
                status: PlotThreadStatus::Open,
                last_update: "Provincial reports already omit several granary shortages.".into(),
            },
        ],
    }
}

pub fn review_scene(
    scene: &Scene,
    story_craft: &StoryCraftState,
    characters: &[Character],
) -> NarrativeReview {
    let mut issues = Vec::new();

    if scene.hook.trim().chars().count() < 12 {
        issues.push(issue(
            NarrativeIssueKind::WeakHook,
            Severity::Warning,
            "Scene hook is too weak to create immediate dramatic pressure.",
        ));
    }

    if scene.dramatic_purpose.trim().is_empty() || scene.plot_thread_updates.is_empty() {
        issues.push(issue(
            NarrativeIssueKind::NoProgress,
            Severity::Warning,
            "Scene does not clearly advance a dramatic purpose or plot thread.",
        ));
    }

    for beat in &scene.beats {
        let mut labels = BTreeSet::new();
        for choice in &beat.choices {
            if choice.dramatic_purpose.trim().is_empty() || !labels.insert(choice.label.trim()) {
                issues.push(issue(
                    NarrativeIssueKind::FakeChoice,
                    Severity::Warning,
                    "Choice lacks a distinct dramatic purpose.",
                ));
                break;
            }
        }
    }

    let known_characters: BTreeSet<&str> = characters
        .iter()
        .map(|character| character.id.as_str())
        .collect();
    for character_id in &scene.character_ids {
        if !known_characters.contains(character_id.as_str()) {
            issues.push(issue(
                NarrativeIssueKind::OutOfCharacter,
                Severity::Error,
                "Scene references a character that is not in the project cast.",
            ));
        }
    }

    let known_threads: BTreeSet<&str> = story_craft
        .plot_threads
        .iter()
        .map(|thread| thread.id.as_str())
        .collect();
    for thread_id in scene.plot_thread_updates.keys() {
        if !known_threads.contains(thread_id.as_str()) {
            issues.push(issue(
                NarrativeIssueKind::BrokenPlotThread,
                Severity::Warning,
                "Scene updates a plot thread that is not registered.",
            ));
        }
    }

    let penalty = issues.len().saturating_mul(15) as u8;
    NarrativeReview {
        scene_key: scene.key.clone(),
        score: 100_u8.saturating_sub(penalty),
        issues,
    }
}

fn issue(kind: NarrativeIssueKind, severity: Severity, message: &str) -> NarrativeIssue {
    NarrativeIssue {
        kind,
        severity,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use plotforge_schema::{Beat, Character, Choice, Scene};

    use super::{dynasty_embers_story_craft, review_scene};

    #[test]
    fn story_craft_has_required_plot_threads() {
        let state = dynasty_embers_story_craft();
        assert!(state.emotional_arc.len() >= 3);
        assert!(state.plot_threads.len() >= 3);
    }

    #[test]
    fn review_flags_weak_scene_quality() {
        let scene = Scene {
            key: "bad".into(),
            title: "Bad".into(),
            location: "Court".into(),
            dramatic_purpose: String::new(),
            hook: "flat".into(),
            background_asset: "assets/generated/placeholder.png".into(),
            character_ids: vec!["unknown".into()],
            plot_thread_updates: BTreeMap::from([("missing-thread".into(), "update".into())]),
            beats: vec![Beat {
                id: "beat-1".into(),
                text: "Nothing changes.".into(),
                choices: vec![
                    Choice {
                        id: "a".into(),
                        label: "Wait".into(),
                        action_type: "continue".into(),
                        dramatic_purpose: String::new(),
                        change_scene: false,
                    },
                    Choice {
                        id: "b".into(),
                        label: "Wait".into(),
                        action_type: "continue".into(),
                        dramatic_purpose: String::new(),
                        change_scene: false,
                    },
                ],
            }],
        };
        let review = review_scene(&scene, &dynasty_embers_story_craft(), &[]);

        assert!(
            review
                .issues
                .iter()
                .any(|issue| matches!(issue.kind, plotforge_schema::NarrativeIssueKind::WeakHook))
        );
        assert!(
            review.issues.iter().any(|issue| matches!(
                issue.kind,
                plotforge_schema::NarrativeIssueKind::FakeChoice
            ))
        );
        assert!(!review.passes());
    }

    #[test]
    fn review_accepts_valid_scene() {
        let scene = Scene {
            key: "court-crisis-001".into(),
            title: "Court Crisis".into(),
            location: "Qianqing Palace".into(),
            dramatic_purpose: "Force the emperor to choose between revenue and public order."
                .into(),
            hook: "The border payroll ledger arrives with a fresh red deficit mark.".into(),
            background_asset: "assets/generated/court-crisis-001.png".into(),
            character_ids: vec!["grand-secretary".into()],
            plot_thread_updates: BTreeMap::from([(
                "border-payroll".into(),
                "The army pay gap becomes visible.".into(),
            )]),
            beats: vec![Beat {
                id: "beat-1".into(),
                text: "The court waits.".into(),
                choices: vec![Choice {
                    id: "raise-tax".into(),
                    label: "Raise the Liao levy".into(),
                    action_type: "raise_tax".into(),
                    dramatic_purpose: "Trade public order for treasury relief.".into(),
                    change_scene: true,
                }],
            }],
        };
        let characters = vec![Character {
            id: "grand-secretary".into(),
            name: "Grand Secretary".into(),
            role: "Court administrator".into(),
            traits: vec!["cautious".into()],
            visual_card: "elder official".into(),
            voice_card: "restrained".into(),
        }];

        let review = review_scene(&scene, &dynasty_embers_story_craft(), &characters);
        assert!(review.passes());
    }

    #[test]
    fn review_locks_each_issue_kind() {
        let scene = Scene {
            key: "bad".into(),
            title: "Bad".into(),
            location: "Court".into(),
            dramatic_purpose: String::new(),
            hook: "weak".into(),
            background_asset: "assets/generated/placeholder.png".into(),
            character_ids: vec!["ghost".into()],
            plot_thread_updates: BTreeMap::from([("ghost-thread".into(), "missing".into())]),
            beats: vec![Beat {
                id: "beat-1".into(),
                text: "Flat.".into(),
                choices: vec![
                    Choice {
                        id: "a".into(),
                        label: "Wait".into(),
                        action_type: "continue".into(),
                        dramatic_purpose: String::new(),
                        change_scene: false,
                    },
                    Choice {
                        id: "b".into(),
                        label: "Wait".into(),
                        action_type: "continue".into(),
                        dramatic_purpose: "Duplicate label.".into(),
                        change_scene: false,
                    },
                ],
            }],
        };

        let review = review_scene(&scene, &dynasty_embers_story_craft(), &[]);
        let kinds = review
            .issues
            .iter()
            .map(|issue| issue.kind.clone())
            .collect::<Vec<_>>();

        assert!(kinds.contains(&plotforge_schema::NarrativeIssueKind::WeakHook));
        assert!(kinds.contains(&plotforge_schema::NarrativeIssueKind::FakeChoice));
        assert!(kinds.contains(&plotforge_schema::NarrativeIssueKind::OutOfCharacter));
        assert!(kinds.contains(&plotforge_schema::NarrativeIssueKind::BrokenPlotThread));
        assert!(review.score < 100);
    }
}
