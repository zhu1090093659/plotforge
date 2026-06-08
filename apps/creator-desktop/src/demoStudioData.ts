import type { ProjectData } from "../../../contracts/plotforge";
import type { SourceFileContent, SourceFileSummary } from "./tauriBridge";

export const demoProjectPath = "examples/dynasty-embers";

export const demoProjectData: ProjectData = {
  game: {
    id: "dynasty-embers",
    title: "Dynasty Embers",
    version: "0.1.0",
    description: "Historical crisis simulation about a collapsing dynasty.",
    entry_scene: "court-crisis-001",
    run_seed: 7,
  },
  resources: [
    { key: "treasury", label: "Treasury", initial: 40, min: 0, max: 100 },
    { key: "public_order", label: "Public order", initial: 55, min: 0, max: 100 },
    { key: "army_morale", label: "Army morale", initial: 45, min: 0, max: 100 },
  ],
  world_state: {
    resources: { treasury: 40, public_order: 55, army_morale: 45 },
    flags: {},
    triggered_events: [],
  },
  story_state: {
    current_scene_key: "court-crisis-001",
    completed_scene_keys: [],
    turn: 0,
  },
  story_craft: {
    bible: {
      genre_promise: "A desperate emperor weighs survival against legitimacy.",
      central_question: "Can the dynasty survive without becoming what it fears?",
      target_emotions: ["pressure", "suspicion", "resolve"],
      core_foreshadowing: ["Missing silver", "Border arrears"],
      emotional_contract: ["hard tradeoffs", "visible consequences"],
      pacing_profile: {
        escalation_interval_scenes: 2,
        target_tension_curve: [4, 6, 8],
        breather_scene_frequency: 3,
      },
      hook_strategy: {
        primary_hook: "Contradictory court reports expose a hidden bargain.",
        recurring_hook_patterns: ["ledger mismatch", "factional counsel"],
      },
      reversal_strategy: {
        cadence_scenes: 2,
        principle: "Every solution creates a factional cost.",
      },
      banned_cliches: ["empty grandeur"],
      reference_modules: [],
    },
    active_promises: [
      {
        id: "missing-silver",
        text: "The missing silver will return as political leverage.",
        status: "active",
        introduced_at: "court-crisis-001",
      },
    ],
    emotional_arc: [
      {
        scene_key: "court-crisis-001",
        target_emotion: "pressure",
        intensity: 7,
      },
    ],
    plot_threads: [
      {
        id: "famine-ledger",
        title: "The Famine Ledger",
        promise: "The report hides a factional bargain.",
        thread_type: "political",
        status: "escalating",
        introduced_at: "court-crisis-001",
        related_characters: ["censor"],
        related_world_flags: [],
        last_update: "court-crisis-001",
      },
    ],
    character_arcs: [],
    review_notes: [
      {
        id: "opening-stakes",
        scene_key: "court-crisis-001",
        severity: "warning",
        message: "The first choice needs clearer stakes.",
        resolved: false,
      },
    ],
  },
  characters: [
    {
      id: "censor",
      name: "Court Censor",
      role: "Moral and legal critic",
      traits: ["severe", "public-minded"],
      visual_card: "ink portrait",
      voice_card: "precise",
    },
    {
      id: "war-minister",
      name: "Minister of War",
      role: "Military logistics",
      traits: ["urgent", "pragmatic"],
      visual_card: "armored court official",
      voice_card: "terse",
    },
  ],
  rules: [
    {
      id: "raise-tax",
      action_type: "raise_tax",
      conditions: [],
      effects: [{ kind: "add_resource", key: "treasury", amount: 12 }],
    },
  ],
  scenes: [
    {
      key: "court-crisis-001",
      title: "Tax Resistance Memorials",
      location: "Forbidden City",
      dramatic_purpose: "Expose the first crisis.",
      hook: "The treasury report contradicts the war ledger.",
      background_asset: "assets/generated/court-crisis-001.png",
      character_ids: ["censor", "war-minister"],
      plot_thread_updates: {},
      beats: [
        {
          id: "opening-council",
          text: "Memorials arrive before dawn, each asking for silver the treasury cannot admit is missing.",
          choices: [
            {
              id: "raise-tax",
              label: "Raise emergency taxes",
              action_type: "raise_tax",
              dramatic_purpose: "Trade public order for treasury relief.",
              change_scene: false,
            },
          ],
        },
      ],
    },
  ],
};

export const demoSourceFiles: SourceFileSummary[] = [
  { path: "game.toml", kind: "toml", bytes: 164, editable: false },
  { path: "world/world.md", kind: "markdown", bytes: 92, editable: true },
  { path: "story/story_bible.md", kind: "markdown", bytes: 88, editable: true },
  { path: "rules/rules.toml", kind: "toml", bytes: 420, editable: false },
  {
    path: "scenes/court-crisis-001.scene.json",
    kind: "json",
    bytes: 1024,
    editable: false,
  },
];

export const demoSourceContents: Record<string, SourceFileContent> = {
  "game.toml": {
    path: "game.toml",
    kind: "toml",
    editable: false,
    content:
      'id = "dynasty-embers"\ntitle = "Dynasty Embers"\nentry_scene = "court-crisis-001"\n',
  },
  "world/world.md": {
    path: "world/world.md",
    kind: "markdown",
    editable: true,
    content:
      "# World Bible\n\nThe dynasty is still standing, but every resource is under pressure.\n",
  },
  "story/story_bible.md": {
    path: "story/story_bible.md",
    kind: "markdown",
    editable: true,
    content:
      "# Story Bible\n\nThe throne must trade stability, silver, and legitimacy to survive.\n",
  },
  "rules/rules.toml": {
    path: "rules/rules.toml",
    kind: "toml",
    editable: false,
    content: '[[rules]]\nid = "raise-tax"\naction_type = "raise_tax"\n',
  },
  "scenes/court-crisis-001.scene.json": {
    path: "scenes/court-crisis-001.scene.json",
    kind: "json",
    editable: false,
    content: '{\n  "key": "court-crisis-001",\n  "title": "Tax Resistance Memorials"\n}\n',
  },
};
