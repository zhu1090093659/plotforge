import type {
  AiSafetyPolicy,
  ProjectData,
  ReproducibilityMetadata,
} from "../../../contracts/plotforge";
import type {
  PlayOnceReport,
  SourceFileContent,
  SourceFileSummary,
} from "./tauriBridge";

export const demoProjectPath = "examples/dynasty-embers";

export const demoReproducibilityMetadata: ReproducibilityMetadata = {
  run_seed: 7,
  prompt_version: "plotforge-local-mock-prompt-v1",
  model_version: "plotforge-local-mock-model-v1",
  provider_config_hash: "sha256:plotforge-local-mock-provider-config-v1",
  trace_id: null,
  snapshot_id: null,
};

export const demoAiSafetyPolicy: AiSafetyPolicy = {
  live_generated_content_enabled: false,
  content_kinds: ["text", "image", "voice"],
  safety_guardrails: [
    "Provider credentials stay outside project source, traces, and exports.",
    "Raw provider responses are not stored in project files.",
    "Generated content requires creator review before distribution.",
  ],
  user_reporting_path: "local-creator-review",
  moderation_policy:
    "Live provider-backed generation is disabled by default; generated output must be reviewed before export.",
  human_review_required: true,
  moderation_queue_enabled: false,
  policy_source_path: "safety/ai_safety_policy.toml",
  evidence_ids: [],
  policy_hash: null,
  notices: [
    "This local policy is descriptive evidence only and is not platform approval.",
  ],
};

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
    current_beat_id: "court-crisis-001-beat-001",
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
      portrait_request: null,
    },
    {
      id: "war-minister",
      name: "Minister of War",
      role: "Military logistics",
      traits: ["urgent", "pragmatic"],
      visual_card: "armored court official",
      voice_card: "terse",
      portrait_request: null,
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
      entry_beat_id: "court-crisis-001-beat-001",
      beats: [
        {
          id: "court-crisis-001-beat-001",
          text: "Memorials arrive before dawn, each asking for silver the treasury cannot admit is missing.",
          choices: [
            {
              id: "continue-council",
              label: "Hear one more minister",
              action_type: "continue",
              input_terms: ["continue", "hear", "minister", "听", "继续", "陈情"],
              dramatic_purpose: "Stay in the council scene before committing an order.",
              change_scene: false,
            },
            {
              id: "raise-tax",
              label: "Raise emergency taxes",
              action_type: "raise_tax",
              input_terms: ["raise", "tax", "levy", "加征", "辽饷"],
              dramatic_purpose: "Trade public order for treasury relief.",
              change_scene: true,
            },
          ],
          next: { kind: "beat", payload: "court-crisis-001-beat-002" },
        },
        {
          id: "court-crisis-001-beat-002",
          text: "The war minister points at the unpaid garrison columns and waits for an order.",
          choices: [
            {
              id: "raise-tax",
              label: "Raise emergency taxes",
              action_type: "raise_tax",
              input_terms: ["raise", "tax", "levy", "加征", "辽饷"],
              dramatic_purpose: "Trade public order for treasury relief.",
              change_scene: true,
            },
            {
              id: "pay-army",
              label: "Pay the army",
              action_type: "pay_army",
              input_terms: ["pay", "army", "军饷", "拨", "内帑", "边军"],
              dramatic_purpose: "Spend scarce treasury to buy military time.",
              change_scene: true,
            },
          ],
          next: { kind: "scene" },
        },
      ],
    },
  ],
  ai_safety_policy: demoAiSafetyPolicy,
};

export const demoSourceFiles: SourceFileSummary[] = [
  { path: "game.toml", kind: "toml", bytes: 164, editable: false },
  { path: "world/forbidden_facts.json", kind: "json", bytes: 28, editable: false },
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
  "world/forbidden_facts.json": {
    path: "world/forbidden_facts.json",
    kind: "json",
    editable: false,
    content: '{\n  "forbidden_facts": []\n}\n',
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

export function demoPlayOnceReport(playerInput: string): PlayOnceReport {
  return {
    scene: demoProjectData.scenes[0],
    trace_path: `${demoProjectPath}/traces/trace-001.json`,
    snapshot: null,
    snapshot_path: null,
    delta_summary: [
      "public_order: -8",
      "treasury: +12",
      "event: local_tax_resistance",
    ],
    trace: {
      id: "trace-001",
      timestamp_ms: 1,
      reproducibility: {
        ...demoReproducibilityMetadata,
        trace_id: "trace-001",
      },
      player_input: playerInput,
      selected_choice: "raise-tax",
      action_intent: {
        status: "supported",
        choice_id: "raise-tax",
        action_type: "raise_tax",
        matched_terms: ["tax"],
        reason: null,
      },
      rule_result: {
        action_type: "raise_tax",
        delta_empty: false,
        state_committed: true,
        error: null,
      },
      planner_result: {
        requested_action_type: "raise_tax",
        scene_key: "court-crisis-001",
        fallback_used: false,
        error: null,
      },
      diagnostics: [
        {
          stage: "interpret_action",
          status: "completed",
          message: "action `raise_tax` matched 1 term(s)",
        },
        {
          stage: "select_choice",
          status: "completed",
          message: "selected choice `raise-tax`",
        },
        {
          stage: "evaluate_rules",
          status: "completed",
          message: "rules evaluated for `raise_tax` and produced a world delta",
        },
        {
          stage: "plan_scene",
          status: "completed",
          message: "planner returned scene `court-crisis-001`",
        },
        {
          stage: "commit_state",
          status: "completed",
          message: "committed story turn 1",
        },
      ],
      world_state_before: demoProjectData.world_state,
      world_state_delta: {
        resource_changes: { public_order: -8, treasury: 12 },
        resource_sets: {},
        flags: {},
        triggered_events: ["local_tax_resistance"],
      },
      world_state_after: {
        resources: { treasury: 52, public_order: 47, army_morale: 45 },
        flags: {},
        triggered_events: ["local_tax_resistance"],
      },
      story_state_before: demoProjectData.story_state,
      story_state_after: {
        current_scene_key: "court-crisis-001",
        current_beat_id: "court-crisis-001-beat-001",
        completed_scene_keys: ["court-crisis-001"],
        turn: 1,
      },
      narrative_review: {
        scene_key: "court-crisis-001",
        score: 100,
        hook_score: 100,
        pacing_score: 100,
        character_consistency_score: 100,
        payoff_score: 100,
        choice_meaningfulness_score: 100,
        ai_slop_risk: 0,
        issues: [],
      },
      media_references: [
        {
          reference: {
            reference_kind: "scene",
            reference_id: "court-crisis-001",
            slot: "background_asset",
          },
          project_path: "assets/generated/court-crisis-001.png",
        },
      ],
      errors: [],
      fallback_used: false,
    },
  };
}
