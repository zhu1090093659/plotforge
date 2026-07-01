use std::collections::BTreeSet;

use plotforge_schema::{
    Character, CharacterArc, CharacterArcStatus, EmotionalArcPoint, HookStrategy, NarrativeIssue,
    NarrativeIssueKind, NarrativeReview, NarrativeReviewNote, PacingProfile, PlotThread,
    PlotThreadStatus, PlotThreadType, ReferenceModule, ReversalStrategy, Scene, Severity,
    StoryCraftBible, StoryCraftState, StoryPromise, StoryPromiseStatus,
};

pub fn sample_story_craft_state() -> StoryCraftState {
    StoryCraftState {
        bible: StoryCraftBible {
            genre_promise: "Civic crisis simulation with council intrigue".into(),
            central_question: "Can the player stabilize a city under internal and external pressure?"
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
            target_audience: Some("Players who enjoy political crisis fiction and visible systemic tradeoffs.".into()),
            emotional_contract: vec![
                "power under pressure".into(),
                "lonely decisions".into(),
                "short-term relief with long-term cost".into(),
            ],
            pacing_profile: PacingProfile {
                escalation_interval_scenes: 2,
                target_tension_curve: vec![70, 78, 86, 92],
                breather_scene_frequency: Some(4),
            },
            hook_strategy: HookStrategy {
                primary_hook: "Open each scene with a concrete court pressure that demands a tradeoff.".into(),
                recurring_hook_patterns: vec![
                    "ledger reveals hidden cost".into(),
                    "sealed memorial arrives too early".into(),
                    "messenger interrupts with provincial unrest".into(),
                ],
            },
            reversal_strategy: Some(ReversalStrategy {
                cadence_scenes: 3,
                principle: "Every relief action should expose a new political cost.".into(),
            }),
            prose_style_guide: Some("Tense, concrete, political, and consequence-driven.".into()),
            banned_cliches: vec![
                "empty civic grandeur".into(),
                "prophecy without systems consequence".into(),
            ],
            reference_modules: vec![ReferenceModule {
                id: "civic-crisis-escalation".into(),
                title: "Civic crisis escalation".into(),
                summary: "Use resources, factions, and visible tradeoffs to escalate pressure without copying source text.".into(),
            }],
        },
        active_promises: vec![
            StoryPromise {
                id: "city-stability".into(),
                text: "The player can stabilize the city only by accepting visible costs.".into(),
                status: StoryPromiseStatus::Active,
                introduced_at: "civic-crisis-001".into(),
                payoff_hint: Some("A later scene should force a choice between legitimacy and survival.".into()),
            },
            StoryPromise {
                id: "sealed-decisions-leak".into(),
                text: "Sealed court decisions may be leaking before orders are issued.".into(),
                status: StoryPromiseStatus::Active,
                introduced_at: "civic-crisis-001".into(),
                payoff_hint: Some("Expose or exploit the insider channel.".into()),
            },
        ],
        emotional_arc: vec![
            EmotionalArcPoint {
                scene_key: "civic-crisis-001".into(),
                target_emotion: "pressure".into(),
                intensity: 70,
            },
            EmotionalArcPoint {
                scene_key: "civic-crisis-002".into(),
                target_emotion: "uncertainty".into(),
                intensity: 78,
            },
            EmotionalArcPoint {
                scene_key: "civic-crisis-003".into(),
                target_emotion: "consequence".into(),
                intensity: 86,
            },
        ],
        plot_threads: vec![
            PlotThread {
                id: "border-payroll".into(),
                title: "Border payroll gap".into(),
                promise: "The border army is loyal only while pay keeps flowing.".into(),
                thread_type: PlotThreadType::Survival,
                status: PlotThreadStatus::Open,
                introduced_at: "civic-crisis-001".into(),
                expected_payoff: Some("The army either gets paid, defects, or becomes a coup risk.".into()),
                related_characters: vec!["war-minister".into(), "border-general".into()],
                related_world_flags: vec!["border_army_paid".into()],
                last_update: "Rumors mention unpaid soldiers outside the pass.".into(),
            },
            PlotThread {
                id: "council-insider".into(),
                title: "Council insider betrayal".into(),
                promise: "A trusted council channel is leaking decisions before they are issued."
                    .into(),
                thread_type: PlotThreadType::Mystery,
                status: PlotThreadStatus::Open,
                introduced_at: "civic-crisis-001".into(),
                expected_payoff: Some("A future review should identify who benefits from the leak.".into()),
                related_characters: vec!["guild-liaison".into(), "city-treasurer".into()],
                related_world_flags: vec!["corruption_investigation".into()],
                last_update: "Memorials arrive too quickly after sealed conversations.".into(),
            },
            PlotThread {
                id: "tax-disorder".into(),
                title: "Local tax disorder".into(),
                promise: "Short-term revenue measures can ignite resistance in the provinces."
                    .into(),
                thread_type: PlotThreadType::Political,
                status: PlotThreadStatus::Open,
                introduced_at: "civic-crisis-001".into(),
                expected_payoff: Some("The provinces either comply, revolt, or bargain for autonomy.".into()),
                related_characters: vec!["provincial-governor".into(), "censor".into()],
                related_world_flags: vec!["local_tax_resistance".into()],
                last_update: "Provincial reports already omit several granary shortages.".into(),
            },
        ],
        character_arcs: vec![
            CharacterArc {
                id: "city-treasurer-loyalty".into(),
                character_id: "city-treasurer".into(),
                desire: "Keep the city functioning without becoming the scapegoat.".into(),
                pressure: "Every treasury order creates a factional enemy.".into(),
                current_state: "Cautious administrator".into(),
                target_state: "Openly chooses stability or self-preservation.".into(),
                status: CharacterArcStatus::Setup,
            },
            CharacterArc {
                id: "guild-liaison-leverage".into(),
                character_id: "guild-liaison".into(),
                desire: "Preserve guild leverage over official channels.".into(),
                pressure: "A corruption inquiry may expose broker networks.".into(),
                current_state: "Ambiguous information broker".into(),
                target_state: "Reveals whether loyalty is personal or institutional.".into(),
                status: CharacterArcStatus::Setup,
            },
        ],
        pacing_score: Some(78),
        tension_score: Some(82),
        ai_slop_risk: Some(12),
        review_notes: vec![NarrativeReviewNote {
            id: "opening-pressure-clear".into(),
            scene_key: Some("civic-crisis-001".into()),
            severity: Severity::Info,
            message: "Opening scene has a clear pressure hook and three systemic tradeoffs.".into(),
            resolved: true,
        }],
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

    let has_weak_hook = issues
        .iter()
        .any(|issue| issue.kind == NarrativeIssueKind::WeakHook);
    let has_no_progress = issues
        .iter()
        .any(|issue| issue.kind == NarrativeIssueKind::NoProgress);
    let has_fake_choice = issues
        .iter()
        .any(|issue| issue.kind == NarrativeIssueKind::FakeChoice);
    let has_ooc = issues
        .iter()
        .any(|issue| issue.kind == NarrativeIssueKind::OutOfCharacter);
    let has_broken_thread = issues
        .iter()
        .any(|issue| issue.kind == NarrativeIssueKind::BrokenPlotThread);
    let penalty = issues.len().saturating_mul(15) as u8;
    NarrativeReview {
        scene_key: scene.key.clone(),
        score: 100_u8.saturating_sub(penalty),
        hook_score: if has_weak_hook { 55 } else { 100 },
        pacing_score: if has_no_progress { 60 } else { 100 },
        character_consistency_score: if has_ooc { 35 } else { 100 },
        payoff_score: if has_broken_thread { 60 } else { 100 },
        choice_meaningfulness_score: if has_fake_choice { 45 } else { 100 },
        ai_slop_risk: if has_weak_hook || has_no_progress {
            35
        } else {
            5
        },
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

    use plotforge_schema::{Beat, BeatNext, Character, Choice, Scene};

    use super::{review_scene, sample_story_craft_state};

    #[test]
    fn story_craft_has_required_plot_threads() {
        let state = sample_story_craft_state();
        assert!(state.active_promises.len() >= 2);
        assert!(state.emotional_arc.len() >= 3);
        assert!(state.plot_threads.len() >= 3);
        assert!(state.character_arcs.len() >= 2);
        assert!(state.pacing_score.is_some());
        assert!(state.tension_score.is_some());
        assert!(state.ai_slop_risk.is_some());
        assert!(!state.review_notes.is_empty());
        assert_eq!(state.bible.pacing_profile.escalation_interval_scenes, 2);
        assert!(!state.bible.emotional_contract.is_empty());
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
            audio_refs: Vec::new(),
            character_ids: vec!["unknown".into()],
            plot_thread_updates: BTreeMap::from([("missing-thread".into(), "update".into())]),
            entry_beat_id: Some("beat-1".into()),
            beats: vec![Beat {
                id: "beat-1".into(),
                text: "Nothing changes.".into(),
                speaker: None,
                line_delivery: None,
                audio_refs: Vec::new(),
                choices: vec![
                    Choice {
                        id: "a".into(),
                        label: "Wait".into(),
                        action_type: "continue".into(),
                        input_terms: vec!["wait".into()],
                        dramatic_purpose: String::new(),
                        change_scene: false,
                    },
                    Choice {
                        id: "b".into(),
                        label: "Wait".into(),
                        action_type: "continue".into(),
                        input_terms: vec!["wait".into()],
                        dramatic_purpose: String::new(),
                        change_scene: false,
                    },
                ],
                next: BeatNext::None,
            }],
        };
        let review = review_scene(&scene, &sample_story_craft_state(), &[]);

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
        assert!(review.hook_score < 100);
        assert!(review.pacing_score < 100);
        assert!(review.character_consistency_score < 100);
        assert!(review.choice_meaningfulness_score < 100);
        assert!(review.ai_slop_risk > 0);
    }

    #[test]
    fn review_accepts_valid_scene() {
        let scene = Scene {
            key: "civic-crisis-001".into(),
            title: "Civic Crisis".into(),
            location: "Council Chamber".into(),
            dramatic_purpose: "Force the emperor to choose between revenue and public order."
                .into(),
            hook: "The border payroll ledger arrives with a fresh red deficit mark.".into(),
            background_asset: "assets/generated/civic-crisis-001.png".into(),
            audio_refs: Vec::new(),
            character_ids: vec!["city-treasurer".into()],
            plot_thread_updates: BTreeMap::from([(
                "border-payroll".into(),
                "The army pay gap becomes visible.".into(),
            )]),
            entry_beat_id: Some("beat-1".into()),
            beats: vec![Beat {
                id: "beat-1".into(),
                text: "The court waits.".into(),
                speaker: None,
                line_delivery: None,
                audio_refs: Vec::new(),
                choices: vec![Choice {
                    id: "raise-levy".into(),
                    label: "Raise the harbor levy".into(),
                    action_type: "raise_tax".into(),
                    input_terms: vec!["raise".into(), "levy".into()],
                    dramatic_purpose: "Trade public order for treasury relief.".into(),
                    change_scene: true,
                }],
                next: BeatNext::Scene,
            }],
        };
        let characters = vec![Character {
            id: "city-treasurer".into(),
            name: "City Treasurer".into(),
            role: "Civic administrator".into(),
            traits: vec!["cautious".into()],
            visual_card: "elder official".into(),
            voice_card: "restrained".into(),
            portrait_request: None,
        }];

        let review = review_scene(&scene, &sample_story_craft_state(), &characters);
        assert!(review.passes());
        assert_eq!(review.hook_score, 100);
        assert_eq!(review.pacing_score, 100);
        assert_eq!(review.character_consistency_score, 100);
        assert_eq!(review.payoff_score, 100);
        assert_eq!(review.choice_meaningfulness_score, 100);
        assert!(review.ai_slop_risk <= 5);
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
            audio_refs: Vec::new(),
            character_ids: vec!["ghost".into()],
            plot_thread_updates: BTreeMap::from([("ghost-thread".into(), "missing".into())]),
            entry_beat_id: Some("beat-1".into()),
            beats: vec![Beat {
                id: "beat-1".into(),
                text: "Flat.".into(),
                speaker: None,
                line_delivery: None,
                audio_refs: Vec::new(),
                choices: vec![
                    Choice {
                        id: "a".into(),
                        label: "Wait".into(),
                        action_type: "continue".into(),
                        input_terms: vec!["wait".into()],
                        dramatic_purpose: String::new(),
                        change_scene: false,
                    },
                    Choice {
                        id: "b".into(),
                        label: "Wait".into(),
                        action_type: "continue".into(),
                        input_terms: vec!["wait".into()],
                        dramatic_purpose: "Duplicate label.".into(),
                        change_scene: false,
                    },
                ],
                next: BeatNext::None,
            }],
        };

        let review = review_scene(&scene, &sample_story_craft_state(), &[]);
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
