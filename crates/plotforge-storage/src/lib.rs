use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use plotforge_schema::{
    Beat, Character, Choice, Condition, Effect, GameProject, PlotThread, ProjectData,
    ResourceDefinition, Rule, Scene, StoryState, WorldState,
};
use plotforge_storycraft::dynasty_embers_story_craft;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("project path already exists and is not empty: {0}")]
    ProjectExists(PathBuf),
    #[error("missing required project file: {0}")]
    MissingFile(PathBuf),
    #[error("io error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("toml serialization error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),
    #[error("toml parse error at {path}: {source}")]
    TomlParse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
    #[error("json error at {path}: {source}")]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

#[derive(Debug, Serialize, Deserialize)]
struct ResourceFile {
    resources: Vec<ResourceDefinition>,
}

#[derive(Debug, Serialize, Deserialize)]
struct StoryCraftFile {
    story_craft: plotforge_schema::StoryCraftState,
}

#[derive(Debug, Serialize, Deserialize)]
struct RulesFile {
    rules: Vec<Rule>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PlotThreadsFile {
    plot_threads: Vec<PlotThread>,
}

pub fn create_demo_project(
    path: impl AsRef<Path>,
    force: bool,
) -> Result<ProjectData, StorageError> {
    let path = path.as_ref();
    if path.exists() && !force && path.read_dir().map_io(path)?.next().is_some() {
        return Err(StorageError::ProjectExists(path.to_path_buf()));
    }

    let project = dynasty_embers_project();
    fs::create_dir_all(path).map_io(path)?;
    create_project_dirs(path)?;
    write_project(path, &project)?;
    Ok(project)
}

pub fn load_project(path: impl AsRef<Path>) -> Result<ProjectData, StorageError> {
    let path = path.as_ref();
    let game = read_toml::<GameProject>(&path.join("game.toml"))?;
    let resources = read_toml::<ResourceFile>(&path.join("world/resources.toml"))?.resources;
    let world_state = read_json::<WorldState>(&path.join("world/initial_state.json"))?;
    let story_craft =
        read_toml::<StoryCraftFile>(&path.join("story/story_craft.toml"))?.story_craft;
    let story_state = read_json::<StoryState>(&path.join("saves/initial_story_state.json"))?;
    let characters = read_collection(path.join("characters"), ".character.toml")?;
    let rules = read_toml::<RulesFile>(&path.join("rules/rules.toml"))?.rules;
    let scenes = read_collection(path.join("scenes"), ".scene.json")?;

    Ok(ProjectData {
        game,
        resources,
        world_state,
        story_state,
        story_craft,
        characters,
        rules,
        scenes,
    })
}

pub fn validate_project(path: impl AsRef<Path>) -> Result<ProjectData, StorageError> {
    let project = load_project(path)?;
    let entry_scene = project.game.entry_scene.as_str();
    if project.scene(entry_scene).is_none() {
        return Err(StorageError::MissingFile(PathBuf::from(format!(
            "scenes/{entry_scene}.scene.json"
        ))));
    }
    Ok(project)
}

pub fn write_trace(
    project_path: impl AsRef<Path>,
    trace: &plotforge_schema::RuntimeTrace,
) -> Result<PathBuf, StorageError> {
    let traces_dir = project_path.as_ref().join("traces");
    fs::create_dir_all(&traces_dir).map_io(&traces_dir)?;
    let trace_path = traces_dir.join(format!("{}.json", trace.id));
    write_json(&trace_path, trace)?;
    let latest_path = traces_dir.join("latest.json");
    write_json(&latest_path, trace)?;
    Ok(trace_path)
}

pub fn dynasty_embers_project() -> ProjectData {
    let game = GameProject {
        id: "dynasty-embers".into(),
        title: "Dynasty Embers".into(),
        version: "0.1.0".into(),
        description: "Historical crisis simulation about a collapsing dynasty.".into(),
        entry_scene: "court-crisis-001".into(),
        run_seed: 7,
    };
    let resources = vec![
        resource("treasury", "Treasury", 40, 0, 100),
        resource("public_order", "Public order", 55, 0, 100),
        resource("army_morale", "Army morale", 45, 0, 100),
        resource("court_stability", "Court stability", 50, 0, 100),
        resource("local_control", "Local control", 48, 0, 100),
        resource("enemy_pressure", "Enemy pressure", 70, 0, 100),
    ];
    let world_state = WorldState {
        resources: resources
            .iter()
            .map(|definition| (definition.key.clone(), definition.initial))
            .collect(),
        flags: BTreeMap::new(),
        triggered_events: Vec::new(),
    };
    let story_state = StoryState {
        current_scene_key: game.entry_scene.clone(),
        completed_scene_keys: Vec::new(),
        turn: 0,
    };

    ProjectData {
        game,
        resources,
        world_state,
        story_state,
        story_craft: dynasty_embers_story_craft(),
        characters: dynasty_characters(),
        rules: dynasty_rules(),
        scenes: vec![initial_scene()],
    }
}

fn create_project_dirs(path: &Path) -> Result<(), StorageError> {
    for dir in [
        "world",
        "story",
        "characters",
        "locations",
        "rules",
        "events",
        "agents",
        "references/methods",
        "references/user_imports",
        "references/analyses",
        "reviews/narrative_reviews",
        "assets/images",
        "assets/voices",
        "assets/generated",
        "saves",
        "traces",
        "exports",
        "scenes",
    ] {
        fs::create_dir_all(path.join(dir)).map_io(path.join(dir))?;
    }
    Ok(())
}

fn write_project(path: &Path, project: &ProjectData) -> Result<(), StorageError> {
    write_toml(&path.join("game.toml"), &project.game)?;
    write_toml(
        &path.join("world/resources.toml"),
        &ResourceFile {
            resources: project.resources.clone(),
        },
    )?;
    write_json(&path.join("world/initial_state.json"), &project.world_state)?;
    write_text(
        &path.join("world/world.md"),
        "# World Bible\n\nThe dynasty is still standing, but every resource is under pressure.\n",
    )?;
    write_text(
        &path.join("world/canon.md"),
        "# Canon Rules\n\n- The player is the final authority.\n- Every order has visible state consequences.\n- Court factions respond to short-term tradeoffs.\n",
    )?;
    write_toml(
        &path.join("story/story_craft.toml"),
        &StoryCraftFile {
            story_craft: project.story_craft.clone(),
        },
    )?;
    write_json(
        &path.join("story/emotional_arc.json"),
        &project.story_craft.emotional_arc,
    )?;
    write_toml(
        &path.join("story/plot_threads.toml"),
        &PlotThreadsFile {
            plot_threads: project.story_craft.plot_threads.clone(),
        },
    )?;
    write_text(
        &path.join("story/story_bible.md"),
        "# Story Bible\n\nThe throne must trade stability, silver, and legitimacy to survive.\n",
    )?;
    write_text(
        &path.join("story/style_guide.md"),
        "# Style Guide\n\nTense, concrete, political, and consequence-driven. Avoid empty grandeur.\n",
    )?;
    for character in &project.characters {
        write_toml(
            &path.join(format!("characters/{}.character.toml", character.id)),
            character,
        )?;
    }
    write_toml(
        &path.join("rules/rules.toml"),
        &RulesFile {
            rules: project.rules.clone(),
        },
    )?;
    write_json(
        &path.join("saves/initial_story_state.json"),
        &project.story_state,
    )?;
    for scene in &project.scenes {
        write_json(
            &path.join(format!("scenes/{}.scene.json", scene.key)),
            scene,
        )?;
    }
    write_text(
        &path.join("agents/scene_planner.prompt.md"),
        "Plan one scene from validated state.\n",
    )?;
    write_text(
        &path.join("agents/beat_writer.prompt.md"),
        "Write beats from a scene plan.\n",
    )?;
    write_text(
        &path.join("agents/plot_doctor.prompt.md"),
        "Review hook, progress, choice quality, character fit, and threads.\n",
    )?;
    write_text(&path.join("AGENTS.md"), project_agents_md())?;
    write_placeholder_png(&path.join("assets/generated/placeholder.png"))?;
    write_placeholder_png(&path.join("assets/generated/court-crisis-001.png"))?;
    Ok(())
}

fn write_toml<T: Serialize>(path: &Path, value: &T) -> Result<(), StorageError> {
    let encoded = toml::to_string_pretty(value)?;
    write_text(path, &encoded)
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), StorageError> {
    let encoded = serde_json::to_string_pretty(value).map_err(|source| StorageError::Json {
        path: path.to_path_buf(),
        source,
    })?;
    write_text(path, &(encoded + "\n"))
}

fn write_text(path: &Path, text: &str) -> Result<(), StorageError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_io(parent)?;
    }
    fs::write(path, text).map_io(path)
}

pub fn write_placeholder_png(path: &Path) -> Result<(), StorageError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_io(parent)?;
    }
    fs::write(path, PLACEHOLDER_PNG).map_io(path)
}

fn read_toml<T: DeserializeOwned>(path: &Path) -> Result<T, StorageError> {
    let text = fs::read_to_string(path).map_io(path)?;
    toml::from_str(&text).map_err(|source| StorageError::TomlParse {
        path: path.to_path_buf(),
        source,
    })
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, StorageError> {
    let text = fs::read_to_string(path).map_io(path)?;
    serde_json::from_str(&text).map_err(|source| StorageError::Json {
        path: path.to_path_buf(),
        source,
    })
}

fn read_collection<T: DeserializeOwned>(
    dir: PathBuf,
    suffix: &str,
) -> Result<Vec<T>, StorageError> {
    let mut paths = fs::read_dir(&dir)
        .map_io(&dir)?
        .map(|entry| entry.map(|entry| entry.path()).map_io(&dir))
        .collect::<Result<Vec<_>, _>>()?;
    paths.sort();

    paths
        .into_iter()
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(suffix))
        })
        .map(|path| {
            if suffix.ends_with(".json") {
                read_json(&path)
            } else {
                read_toml(&path)
            }
        })
        .collect()
}

fn resource(key: &str, label: &str, initial: i32, min: i32, max: i32) -> ResourceDefinition {
    ResourceDefinition {
        key: key.into(),
        label: label.into(),
        initial,
        min,
        max,
    }
}

fn dynasty_characters() -> Vec<Character> {
    vec![
        character(
            "grand-secretary",
            "Grand Secretary",
            "Court administrator",
            &["cautious", "faction-aware"],
        ),
        character(
            "war-minister",
            "Minister of War",
            "Military logistics",
            &["urgent", "pragmatic"],
        ),
        character(
            "eunuch-director",
            "Eunuch Director",
            "Palace intelligence channel",
            &["watchful", "ambiguous"],
        ),
        character(
            "border-general",
            "Border General",
            "Frontier commander",
            &["loyal-if-paid", "blunt"],
        ),
        character(
            "censor",
            "Court Censor",
            "Moral and legal critic",
            &["severe", "public-minded"],
        ),
        character(
            "provincial-governor",
            "Provincial Governor",
            "Local implementation",
            &["strained", "risk-averse"],
        ),
    ]
}

fn character(id: &str, name: &str, role: &str, traits: &[&str]) -> Character {
    Character {
        id: id.into(),
        name: name.into(),
        role: role.into(),
        traits: traits
            .iter()
            .map(|trait_name| (*trait_name).into())
            .collect(),
        visual_card: format!("{name}, restrained historical portrait"),
        voice_card: format!("{name}, concise court speech"),
    }
}

fn dynasty_rules() -> Vec<Rule> {
    vec![
        Rule {
            id: "raise-tax-emergency".into(),
            action_type: "raise_tax".into(),
            conditions: vec![Condition::ResourceAtMost {
                key: "treasury".into(),
                value: 80,
            }],
            effects: vec![
                Effect::AddResource {
                    key: "treasury".into(),
                    amount: 12,
                },
                Effect::AddResource {
                    key: "public_order".into(),
                    amount: -8,
                },
                Effect::AddResource {
                    key: "court_stability".into(),
                    amount: -5,
                },
                Effect::AddResource {
                    key: "army_morale".into(),
                    amount: 4,
                },
                Effect::TriggerEvent {
                    event: "local_tax_resistance".into(),
                },
            ],
        },
        Rule {
            id: "inspect-corruption-ledger".into(),
            action_type: "inspect_corruption".into(),
            conditions: Vec::new(),
            effects: vec![
                Effect::AddResource {
                    key: "treasury".into(),
                    amount: 4,
                },
                Effect::AddResource {
                    key: "court_stability".into(),
                    amount: -8,
                },
                Effect::SetFlag {
                    key: "corruption_investigation".into(),
                    value: true,
                },
                Effect::TriggerEvent {
                    event: "officials_submit_memorials".into(),
                },
            ],
        },
        Rule {
            id: "pay-border-army".into(),
            action_type: "pay_army".into(),
            conditions: vec![Condition::ResourceAtLeast {
                key: "treasury".into(),
                value: 10,
            }],
            effects: vec![
                Effect::AddResource {
                    key: "treasury".into(),
                    amount: -10,
                },
                Effect::AddResource {
                    key: "army_morale".into(),
                    amount: 12,
                },
                Effect::AddResource {
                    key: "enemy_pressure".into(),
                    amount: -3,
                },
                Effect::TriggerEvent {
                    event: "border_army_paid".into(),
                },
            ],
        },
    ]
}

fn initial_scene() -> Scene {
    Scene {
        key: "court-crisis-001".into(),
        title: "The Red Deficit Ledger".into(),
        location: "Qianqing Palace".into(),
        dramatic_purpose: "Force the player to choose between revenue, order, and military loyalty.".into(),
        hook: "The border payroll ledger arrives with a fresh red deficit mark beside the army columns.".into(),
        background_asset: "assets/generated/court-crisis-001.png".into(),
        character_ids: vec![
            "grand-secretary".into(),
            "war-minister".into(),
            "eunuch-director".into(),
        ],
        plot_thread_updates: BTreeMap::from([(
            "border-payroll".into(),
            "The unpaid army becomes the first visible crisis.".into(),
        )]),
        beats: vec![Beat {
            id: "court-crisis-001-beat-001".into(),
            text: "The court kneels around a ledger that says the border army is two months from mutiny.".into(),
            choices: vec![
                Choice {
                    id: "raise-tax".into(),
                    label: "加征辽饷，立刻补军饷".into(),
                    action_type: "raise_tax".into(),
                    dramatic_purpose: "Trade public order for immediate treasury relief.".into(),
                    change_scene: true,
                },
                Choice {
                    id: "inspect-corruption".into(),
                    label: "严查军饷贪墨".into(),
                    action_type: "inspect_corruption".into(),
                    dramatic_purpose: "Seek stolen funds while angering court factions.".into(),
                    change_scene: true,
                },
                Choice {
                    id: "pay-army".into(),
                    label: "先拨内帑稳住边军".into(),
                    action_type: "pay_army".into(),
                    dramatic_purpose: "Spend scarce treasury to buy military time.".into(),
                    change_scene: true,
                },
            ],
        }],
    }
}

fn project_agents_md() -> &'static str {
    "# AGENTS.md\n\n## Project goal\n\nBuild a playable PlotForge story project from local files.\n\n## Commands\n\n- `plotforge check .`\n- `plotforge play . --once`\n- `plotforge export static . --out exports/static`\n\n## Rules\n\n- Files are source of truth.\n- Do not put API keys in project files or exports.\n- AI output proposes content; engine rules commit state.\n"
}

trait IoContext<T> {
    fn map_io(self, path: impl AsRef<Path>) -> Result<T, StorageError>;
}

impl<T> IoContext<T> for Result<T, std::io::Error> {
    fn map_io(self, path: impl AsRef<Path>) -> Result<T, StorageError> {
        self.map_err(|source| StorageError::Io {
            path: path.as_ref().to_path_buf(),
            source,
        })
    }
}

const PLACEHOLDER_PNG: &[u8] = &[
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0,
    0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 120, 156, 99, 100, 96, 96, 248, 15, 0, 1,
    5, 1, 2, 161, 13, 197, 111, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
];

#[cfg(test)]
mod tests {
    use super::{create_demo_project, load_project, validate_project};

    #[test]
    fn creates_and_loads_demo_project() {
        let temp = tempfile::tempdir().expect("tempdir");
        let project_path = temp.path().join("dynasty-embers");

        create_demo_project(&project_path, false).expect("create demo");
        let loaded = load_project(&project_path).expect("load project");
        validate_project(&project_path).expect("validate project");

        assert_eq!(loaded.game.id, "dynasty-embers");
        assert!(project_path.join("story/story_craft.toml").exists());
        assert!(project_path.join("AGENTS.md").exists());
        assert!(
            loaded
                .rules
                .iter()
                .any(|rule| rule.action_type == "raise_tax")
        );
    }
}
