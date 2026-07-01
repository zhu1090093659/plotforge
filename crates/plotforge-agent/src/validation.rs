//! Agent output proposal validation.
//!
//! Pure validation of `AgentOutputProposal` payloads and scene assembly from
//! validated proposals. This module depends only on the schema and the shared
//! `payload_kind` helper, so it can be reused by the pipeline, scene planner,
//! and pi-Agent facade without introducing cycles.

use std::collections::BTreeMap;

use plotforge_schema::{
    AgentOutputProposal, AgentProposalPayload, AgentRole, Beat, BeatDraftProposal,
    BeatDraftsProposal, BeatNext, CharacterProposal, ReviewProposal, Scene, ScenePlanProposal,
    StoryCraftPlanProposal, WorldExpansionProposal,
};

use crate::shared::payload_kind;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentProposalValidationError {
    MissingBeatDrafts,
    EmptyField(&'static str),
    AgentPayloadMismatch {
        agent: AgentRole,
        payload_kind: &'static str,
    },
    DuplicateBeatId(String),
    EntryBeatMissing(String),
    BeatDraftsSceneMismatch {
        expected_scene_key: String,
        actual_scene_key: String,
    },
    BeatSceneMismatch {
        beat_id: String,
        expected_scene_key: String,
        actual_scene_key: String,
    },
    ReviewSceneMismatch {
        expected_scene_key: String,
        actual_scene_key: String,
    },
}

pub fn validate_agent_output_proposal(
    proposal: &AgentOutputProposal,
) -> Result<(), AgentProposalValidationError> {
    require_non_empty(&proposal.id, "proposal.id")?;

    match (&proposal.agent, &proposal.output) {
        (AgentRole::StoryArchitect, AgentProposalPayload::WorldExpansion(world_expansion)) => {
            validate_world_expansion_proposal(world_expansion)
        }
        (AgentRole::StoryCraftPlanner, AgentProposalPayload::StoryCraftPlan(story_craft_plan)) => {
            validate_story_craft_plan_proposal(story_craft_plan)
        }
        (AgentRole::CharacterDesigner, AgentProposalPayload::CharacterProfile(character)) => {
            validate_character_proposal(character)
        }
        (AgentRole::ScenePlanner, AgentProposalPayload::ScenePlan(scene_plan)) => {
            validate_scene_plan_proposal(scene_plan)
        }
        (AgentRole::BeatWriter, AgentProposalPayload::BeatDrafts(beat_drafts)) => {
            validate_beat_drafts_proposal(beat_drafts)
        }
        (
            AgentRole::PlotDoctor | AgentRole::ConsistencyChecker,
            AgentProposalPayload::Review(review),
        ) => validate_review_proposal(review),
        (agent, output) => Err(AgentProposalValidationError::AgentPayloadMismatch {
            agent: agent.clone(),
            payload_kind: payload_kind(output),
        }),
    }
}

pub fn validate_world_expansion_proposal(
    world_expansion: &WorldExpansionProposal,
) -> Result<(), AgentProposalValidationError> {
    require_non_empty(
        &world_expansion.world_bible_markdown,
        "world_expansion.world_bible_markdown",
    )?;
    require_non_empty(
        &world_expansion.canon_markdown,
        "world_expansion.canon_markdown",
    )?;

    Ok(())
}

pub fn validate_story_craft_plan_proposal(
    story_craft_plan: &StoryCraftPlanProposal,
) -> Result<(), AgentProposalValidationError> {
    require_non_empty(
        &story_craft_plan.story_bible_markdown,
        "story_craft_plan.story_bible_markdown",
    )?;
    require_non_empty(
        &story_craft_plan.style_guide_markdown,
        "story_craft_plan.style_guide_markdown",
    )?;
    require_non_empty(
        &story_craft_plan.story_craft.bible.genre_promise,
        "story_craft_plan.story_craft.bible.genre_promise",
    )?;
    require_non_empty(
        &story_craft_plan.story_craft.bible.central_question,
        "story_craft_plan.story_craft.bible.central_question",
    )?;

    Ok(())
}

pub fn validate_character_proposal(
    character: &CharacterProposal,
) -> Result<(), AgentProposalValidationError> {
    require_non_empty(&character.character.id, "character.id")?;
    require_non_empty(&character.character.name, "character.name")?;
    require_non_empty(&character.character.role, "character.role")?;
    require_non_empty(&character.character.visual_card, "character.visual_card")?;
    require_non_empty(&character.character.voice_card, "character.voice_card")?;

    Ok(())
}

pub fn validate_scene_plan_proposal(
    scene_plan: &ScenePlanProposal,
) -> Result<(), AgentProposalValidationError> {
    require_non_empty(&scene_plan.scene_key, "scene_plan.scene_key")?;
    require_non_empty(&scene_plan.title, "scene_plan.title")?;
    require_non_empty(&scene_plan.location, "scene_plan.location")?;
    require_non_empty(&scene_plan.scene_summary, "scene_plan.scene_summary")?;
    require_non_empty(&scene_plan.dramatic_purpose, "scene_plan.dramatic_purpose")?;
    require_non_empty(&scene_plan.hook, "scene_plan.hook")?;
    require_non_empty(&scene_plan.entry_beat_id, "scene_plan.entry_beat_id")?;

    Ok(())
}

pub fn validate_beat_drafts_proposal(
    beat_drafts: &BeatDraftsProposal,
) -> Result<(), AgentProposalValidationError> {
    require_non_empty(&beat_drafts.scene_key, "beat_drafts.scene_key")?;
    if beat_drafts.beats.is_empty() {
        return Err(AgentProposalValidationError::MissingBeatDrafts);
    }

    let mut beat_ids = std::collections::BTreeSet::new();
    for beat in &beat_drafts.beats {
        validate_beat_draft_proposal(beat, &beat_drafts.scene_key)?;
        if !beat_ids.insert(beat.id.as_str()) {
            return Err(AgentProposalValidationError::DuplicateBeatId(
                beat.id.clone(),
            ));
        }
    }

    Ok(())
}

pub fn validate_review_proposal(
    review: &ReviewProposal,
) -> Result<(), AgentProposalValidationError> {
    require_non_empty(&review.scene_key, "review.scene_key")?;
    require_non_empty(&review.review.scene_key, "review.review.scene_key")?;
    if review.review.scene_key != review.scene_key {
        return Err(AgentProposalValidationError::ReviewSceneMismatch {
            expected_scene_key: review.scene_key.clone(),
            actual_scene_key: review.review.scene_key.clone(),
        });
    }

    Ok(())
}

pub fn validate_scene_proposal_parts(
    scene_plan: &ScenePlanProposal,
    beat_drafts: &BeatDraftsProposal,
    review: Option<&ReviewProposal>,
) -> Result<(), AgentProposalValidationError> {
    validate_scene_plan_proposal(scene_plan)?;
    validate_beat_drafts_proposal(beat_drafts)?;

    if beat_drafts.scene_key != scene_plan.scene_key {
        return Err(AgentProposalValidationError::BeatDraftsSceneMismatch {
            expected_scene_key: scene_plan.scene_key.clone(),
            actual_scene_key: beat_drafts.scene_key.clone(),
        });
    }

    let beat_ids = beat_drafts
        .beats
        .iter()
        .map(|beat| beat.id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    if !beat_ids.contains(scene_plan.entry_beat_id.as_str()) {
        return Err(AgentProposalValidationError::EntryBeatMissing(
            scene_plan.entry_beat_id.clone(),
        ));
    }

    if let Some(review) = review {
        validate_review_proposal(review)?;
        if review.scene_key != scene_plan.scene_key {
            return Err(AgentProposalValidationError::ReviewSceneMismatch {
                expected_scene_key: scene_plan.scene_key.clone(),
                actual_scene_key: review.scene_key.clone(),
            });
        }
    }

    Ok(())
}

pub fn scene_from_proposals(
    scene_plan: &ScenePlanProposal,
    beat_drafts: &BeatDraftsProposal,
    review: Option<&ReviewProposal>,
) -> Result<Scene, AgentProposalValidationError> {
    validate_scene_proposal_parts(scene_plan, beat_drafts, review)?;
    Ok(Scene {
        key: scene_plan.scene_key.clone(),
        title: scene_plan.title.clone(),
        location: scene_plan.location.clone(),
        dramatic_purpose: scene_plan.dramatic_purpose.clone(),
        hook: scene_plan.hook.clone(),
        background_asset: scene_plan
            .background_asset
            .clone()
            .unwrap_or_else(|| format!("assets/generated/{}.png", scene_plan.scene_key)),
        audio_refs: Vec::new(),
        character_ids: scene_plan.cast.clone(),
        plot_thread_updates: BTreeMap::new(),
        entry_beat_id: Some(scene_plan.entry_beat_id.clone()),
        beats: beat_drafts
            .beats
            .iter()
            .enumerate()
            .map(|(index, beat)| Beat {
                id: beat.id.clone(),
                text: beat.text.clone(),
                speaker: None,
                line_delivery: None,
                audio_refs: Vec::new(),
                choices: beat.choices.clone(),
                next: beat_drafts
                    .beats
                    .get(index + 1)
                    .map(|next| BeatNext::Beat(next.id.clone()))
                    .unwrap_or(BeatNext::Scene),
            })
            .collect(),
    })
}

fn validate_beat_draft_proposal(
    beat: &BeatDraftProposal,
    expected_scene_key: &str,
) -> Result<(), AgentProposalValidationError> {
    require_non_empty(&beat.id, "beat_drafts.id")?;
    require_non_empty(&beat.scene_key, "beat_drafts.scene_key")?;
    require_non_empty(&beat.text, "beat_drafts.text")?;
    if beat.scene_key != expected_scene_key {
        return Err(AgentProposalValidationError::BeatSceneMismatch {
            beat_id: beat.id.clone(),
            expected_scene_key: expected_scene_key.to_string(),
            actual_scene_key: beat.scene_key.clone(),
        });
    }
    if beat.choices.is_empty() {
        return Err(AgentProposalValidationError::EmptyField(
            "beat_drafts.choices",
        ));
    }
    for choice in &beat.choices {
        require_non_empty(&choice.id, "choice.id")?;
        require_non_empty(&choice.label, "choice.label")?;
        require_non_empty(&choice.action_type, "choice.action_type")?;
        require_non_empty(&choice.dramatic_purpose, "choice.dramatic_purpose")?;
    }

    Ok(())
}

fn require_non_empty(value: &str, field: &'static str) -> Result<(), AgentProposalValidationError> {
    if value.trim().is_empty() {
        Err(AgentProposalValidationError::EmptyField(field))
    } else {
        Ok(())
    }
}
