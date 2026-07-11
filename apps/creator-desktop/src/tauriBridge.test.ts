import { describe, expect, it } from "vitest";
import type {
  AgentSessionConfig,
  AiSafetyPolicy,
  AudioBible,
  Character,
  CharacterEditDocument,
  ImageProviderEntry,
  McpServerEntry,
  McpServerTestResult,
  McpToolCallRequest,
  McpToolCallResult,
  McpToolManifest,
  ProjectCreationReport,
  ResourceDefinition,
  Rule,
  RulesEditDocument,
  RuntimeSnapshot,
  StateVariablesEditDocument,
  StoryCraftEditDocument,
  TtsProviderEntry,
  VisualBible,
  WorldEditDocument,
} from "../../../contracts/plotforge";
import {
  demoExportProfiles,
  demoProjectData,
  demoReproducibilityMetadata,
} from "./demoStudioData";
import {
  createStudioBridge,
  studioCommandNames,
  type ProjectCheckReport,
  type StaticExportReport,
  type StudioInvoke,
} from "./tauriBridge";

const demoMcpServerEntry: McpServerEntry = {
  id: "weather",
  kind: "stdio",
  label: "Weather MCP",
  transport_config: {
    kind: "stdio",
    command: "weather-mcp",
    args: ["--stdio"],
    env: {},
  },
  credential_env_var: "",
  enabled: true,
};

const demoMcpServerTestResult: McpServerTestResult = {
  ok: true,
  message: "MCP server `weather` responded; 1 tool(s) available",
  tools_count: 1,
};

const demoMcpToolManifest: McpToolManifest = {
  name: "get_forecast",
  description: "Returns the forecast for a city.",
  input_schema: { type: "object", properties: { city: { type: "string" } } },
};

const demoMcpToolCallResult: McpToolCallResult = {
  ok: true,
  content: [{ type: "text", text: "Rain expected in Reik." }],
  is_error: false,
};

const demoAgentSessionConfig: AgentSessionConfig = {
  model_id: "demo-model",
  permission_level: "ask_every_time",
  thinking_level: "medium",
  enabled_skills: [],
  enabled_mcp_servers: ["weather"],
};

const demoImageProviderEntry: ImageProviderEntry = {
  id: "image-local",
  endpoint_url: "http://localhost:11434/v1",
  model: "image-model",
  credential_env_var: "",
  enabled: true,
  default_size: "1024x1024",
  default_quality: "medium",
  max_concurrency: null,
  requests_per_minute: null,
  daily_token_budget: null,
};

const demoTtsProviderEntry: TtsProviderEntry = {
  id: "tts-local",
  endpoint_url: "http://localhost:11434/v1",
  model: "tts-model",
  credential_env_var: "",
  enabled: true,
  voice: "coral",
  format: "mp3",
  max_concurrency: null,
  requests_per_minute: null,
  daily_token_budget: null,
};

describe("createStudioBridge", () => {
  it("maps typed frontend methods to Tauri command names and arguments", async () => {
    const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
    const invoke: StudioInvoke = async <T>(command: string, args?: Record<string, unknown>) => {
      calls.push({ command, args });
      if (command === studioCommandNames.createProject) {
        const result = {
          project_path: "/tmp/winter-regency",
          template: "historical_crisis",
          concept: "A frozen council succession crisis.",
          visual_style: "ink wash winter council",
          voice_enabled: true,
          initial_scene_request: "Open with a sealed imperial edict.",
          files_created: ["game.toml"],
          project: demoProjectData,
        } satisfies ProjectCreationReport;
        return result as T;
      }
      if (command === studioCommandNames.checkProject) {
        const result = {
          title: "Starter Project",
          entry_scene: "opening-scene",
          scene_count: 1,
          rule_count: 3,
          character_count: 6,
        } satisfies ProjectCheckReport;
        return result as T;
      }
      if (command === studioCommandNames.listExportProfiles) {
        return demoExportProfiles as T;
      }
      if (command === studioCommandNames.exportStaticProject) {
        const result = {
          output_dir: "/tmp/export",
          archive_path: null,
          files_written: ["/tmp/export/index.html"],
          archived_files: [],
          allowed_files: ["index.html"],
          files_found: ["index.html"],
        } satisfies StaticExportReport;
        return result as T;
      }
      if (command === studioCommandNames.exportStaticProjectZip) {
        const result = {
          output_dir: "/tmp/export",
          archive_path: "/tmp/export.zip",
          files_written: ["/tmp/export/index.html"],
          archived_files: ["index.html"],
          allowed_files: ["index.html"],
          files_found: ["index.html"],
        } satisfies StaticExportReport;
        return result as T;
      }
      if (command === studioCommandNames.listAssetRecords) {
        return demoProjectData.asset_records as T;
      }
      if (command === studioCommandNames.readVisualBible) {
        return demoProjectData.visual_bible as T;
      }
      if (command === studioCommandNames.updateVisualBible) {
        return (args?.visual_bible ?? demoProjectData.visual_bible) as T;
      }
      if (command === studioCommandNames.readAudioBible) {
        return demoProjectData.audio_bible as T;
      }
      if (command === studioCommandNames.updateAudioBible) {
        return (args?.audio_bible ?? demoProjectData.audio_bible) as T;
      }
      return {} as never;
    };

    const bridge = createStudioBridge(invoke);
    const worldDocument: WorldEditDocument = {
      world_bible_markdown: "# World Bible\n",
      canon_markdown: "# Canon\n",
      forbidden_facts: ["No hidden immortality."],
    };
    const storyCraftDocument: StoryCraftEditDocument = {
      story_bible_markdown: "# Story Bible\n",
      style_guide_markdown: "# Style Guide\n",
      story_craft: demoProjectData.story_craft,
    };
    const character: Character = {
      id: "regent",
      name: "Regent",
      role: "Temporary council authority",
      traits: ["cautious"],
      visual_card: "ink portrait",
      voice_card: "measured",
    };
    const characterDocument: CharacterEditDocument = {
      characters: [character],
    };
    const resource: ResourceDefinition = {
      key: "grain",
      label: "Grain",
      initial: 30,
      min: 0,
      max: 100,
    };
    const stateVariablesDocument: StateVariablesEditDocument = {
      resources: [resource],
      initial_world_state: {
        resources: { grain: 30 },
        flags: {},
        triggered_events: [],
      },
      initial_story_state: demoProjectData.story_state,
    };
    const rule: Rule = {
      id: "spend-grain",
      action_type: "spend_grain",
      conditions: [],
      effects: [{ kind: "add_resource", key: "grain", amount: -3 }],
    };
    const rulesDocument: RulesEditDocument = {
      rules: [rule],
    };
    const runtimeSnapshot: RuntimeSnapshot = {
      id: "save-001",
      timestamp_ms: 1,
      reproducibility: {
        ...demoReproducibilityMetadata,
        snapshot_id: "save-001",
      },
      project_id: "starter-project",
      project_version: "0.1.0",
      story_state: demoProjectData.story_state,
      world_state: demoProjectData.world_state,
      scenes: demoProjectData.scenes,
    };
    const safetyPolicy: AiSafetyPolicy = {
      ...demoProjectData.ai_safety_policy,
      live_generated_content_enabled: true,
    };
    const visualBible: VisualBible = {
      style_cards: [
        {
          ...demoProjectData.visual_bible.style_cards[0],
          prompt: "Sharper winter council ink prompt.",
        },
      ],
    };
    const audioBible: AudioBible = {
      voice_cards: [
        {
          ...demoProjectData.audio_bible.voice_cards[0],
          voice: "dry formal council voice",
        },
      ],
    };

    await bridge.createProject(
      "/tmp/winter-regency",
      {
        template: "historical_crisis",
        concept: "A frozen council succession crisis.",
        visual_style: "ink wash winter council",
        voice_enabled: true,
        initial_scene_request: "Open with a sealed imperial edict.",
      },
      true,
    );
    await bridge.checkProject("/tmp/starter-project");
    await bridge.listExportProfiles();
    await bridge.readWorldEditDocument("/tmp/starter-project");
    await bridge.updateWorldEditDocument("/tmp/starter-project", worldDocument);
    await bridge.readStoryCraftEditDocument("/tmp/starter-project");
    await bridge.updateStoryCraftEditDocument(
      "/tmp/starter-project",
      storyCraftDocument,
    );
    await bridge.readCharacterEditDocument("/tmp/starter-project");
    await bridge.updateCharacterEditDocument(
      "/tmp/starter-project",
      characterDocument,
    );
    await bridge.createCharacter("/tmp/starter-project", character);
    await bridge.readStateVariablesEditDocument("/tmp/starter-project");
    await bridge.updateStateVariablesEditDocument(
      "/tmp/starter-project",
      stateVariablesDocument,
    );
    await bridge.createResource("/tmp/starter-project", resource);
    await bridge.readRulesEditDocument("/tmp/starter-project");
    await bridge.updateRulesEditDocument("/tmp/starter-project", rulesDocument);
    await bridge.createRule("/tmp/starter-project", rule);
    await bridge.generateWorldExpansion(
      "/tmp/starter-project",
      "Expand canon and forbidden facts.",
    );
    await bridge.generateStoryCraft(
      "/tmp/starter-project",
      "Generate the first pressure arc.",
    );
    await bridge.generateCharacter(
      "/tmp/starter-project",
      "Design a grain envoy.",
      "council envoy",
    );
    await bridge.readAiSafetyPolicy("/tmp/starter-project");
    await bridge.updateAiSafetyPolicy("/tmp/starter-project", safetyPolicy);
    await bridge.readVisualBible("/tmp/starter-project");
    await bridge.updateVisualBible("/tmp/starter-project", visualBible);
    await bridge.readAudioBible("/tmp/starter-project");
    await bridge.updateAudioBible("/tmp/starter-project", audioBible);
    await bridge.playOnceProject("/tmp/starter-project", "continue");
    await bridge.playOnceProjectWithSave(
      "/tmp/starter-project",
      "continue",
      runtimeSnapshot.id,
    );
    await bridge.playOnceProjectFromSnapshot(
      "/tmp/starter-project",
      "continue",
      runtimeSnapshot.id,
      "save-002",
    );
    await bridge.playOnceProjectFromLatestSnapshot(
      "/tmp/starter-project",
      "continue",
      "save-003",
    );
    await bridge.exportStaticProject("/tmp/starter-project", "/tmp/export");
    await bridge.exportStaticProjectZip(
      "/tmp/starter-project",
      "/tmp/export",
      "/tmp/export.zip",
    );
    await bridge.listAssetRecords("/tmp/starter-project");
    await bridge.readSourceFile("/tmp/starter-project", "world/world.md");
    await bridge.writeSourceFile(
      "/tmp/starter-project",
      "world/world.md",
      "# World Bible\n",
    );

    expect(calls).toEqual([
      {
        command: "create_project",
        args: {
          path: "/tmp/winter-regency",
          request: {
            template: "historical_crisis",
            concept: "A frozen council succession crisis.",
            visual_style: "ink wash winter council",
            voice_enabled: true,
            initial_scene_request: "Open with a sealed imperial edict.",
          },
          force: true,
        },
      },
      {
        command: "check_project",
        args: { path: "/tmp/starter-project" },
      },
      {
        command: "list_export_profiles",
        args: undefined,
      },
      {
        command: "read_world_edit_document",
        args: { path: "/tmp/starter-project" },
      },
      {
        command: "update_world_edit_document",
        args: { path: "/tmp/starter-project", document: worldDocument },
      },
      {
        command: "read_story_craft_edit_document",
        args: { path: "/tmp/starter-project" },
      },
      {
        command: "update_story_craft_edit_document",
        args: { path: "/tmp/starter-project", document: storyCraftDocument },
      },
      {
        command: "read_character_edit_document",
        args: { path: "/tmp/starter-project" },
      },
      {
        command: "update_character_edit_document",
        args: { path: "/tmp/starter-project", document: characterDocument },
      },
      {
        command: "create_character",
        args: { path: "/tmp/starter-project", character },
      },
      {
        command: "read_state_variables_edit_document",
        args: { path: "/tmp/starter-project" },
      },
      {
        command: "update_state_variables_edit_document",
        args: {
          path: "/tmp/starter-project",
          document: stateVariablesDocument,
        },
      },
      {
        command: "create_resource",
        args: { path: "/tmp/starter-project", resource },
      },
      {
        command: "read_rules_edit_document",
        args: { path: "/tmp/starter-project" },
      },
      {
        command: "update_rules_edit_document",
        args: { path: "/tmp/starter-project", document: rulesDocument },
      },
      {
        command: "create_rule",
        args: { path: "/tmp/starter-project", rule },
      },
      {
        command: "generate_world_expansion",
        args: {
          path: "/tmp/starter-project",
          expansion_goal: "Expand canon and forbidden facts.",
        },
      },
      {
        command: "generate_story_craft",
        args: {
          path: "/tmp/starter-project",
          concept: "Generate the first pressure arc.",
        },
      },
      {
        command: "generate_character",
        args: {
          path: "/tmp/starter-project",
          concept: "Design a grain envoy.",
          role_hint: "council envoy",
        },
      },
      {
        command: "read_ai_safety_policy",
        args: { path: "/tmp/starter-project" },
      },
      {
        command: "update_ai_safety_policy",
        args: { path: "/tmp/starter-project", policy: safetyPolicy },
      },
      {
        command: "read_visual_bible",
        args: { path: "/tmp/starter-project" },
      },
      {
        command: "update_visual_bible",
        args: { path: "/tmp/starter-project", visual_bible: visualBible },
      },
      {
        command: "read_audio_bible",
        args: { path: "/tmp/starter-project" },
      },
      {
        command: "update_audio_bible",
        args: { path: "/tmp/starter-project", audio_bible: audioBible },
      },
      {
        command: "play_once_project",
        args: { path: "/tmp/starter-project", player_input: "continue" },
      },
      {
        command: "play_once_project_with_save",
        args: {
          path: "/tmp/starter-project",
          player_input: "continue",
          save_id: "save-001",
        },
      },
      {
        command: "play_once_project_from_snapshot",
        args: {
          path: "/tmp/starter-project",
          player_input: "continue",
          snapshot_id: "save-001",
          save_id: "save-002",
        },
      },
      {
        command: "play_once_project_from_latest_snapshot",
        args: {
          path: "/tmp/starter-project",
          player_input: "continue",
          save_id: "save-003",
        },
      },
      {
        command: "export_static_project",
        args: { path: "/tmp/starter-project", output_dir: "/tmp/export" },
      },
      {
        command: "export_static_project_zip",
        args: {
          path: "/tmp/starter-project",
          output_dir: "/tmp/export",
          archive_path: "/tmp/export.zip",
        },
      },
      {
        command: "list_asset_records",
        args: { path: "/tmp/starter-project" },
      },
      {
        command: "read_source_file",
        args: {
          path: "/tmp/starter-project",
          relative_path: "world/world.md",
        },
      },
      {
        command: "write_source_file",
        args: {
          path: "/tmp/starter-project",
          relative_path: "world/world.md",
          content: "# World Bible\n",
        },
      },
    ]);
  });
});

describe("createStudioBridge MCP commands", () => {
  it("maps each MCP method to the correct snake_case command and arguments", async () => {
    const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
    const invoke: StudioInvoke = async <T>(command: string, args?: Record<string, unknown>) => {
      calls.push({ command, args });
      if (command === studioCommandNames.listMcpServers) {
        return [demoMcpServerEntry] as T;
      }
      if (command === studioCommandNames.upsertMcpServer) {
        return (args?.entry ?? demoMcpServerEntry) as T;
      }
      if (command === studioCommandNames.deleteMcpServer) {
        return demoMcpServerEntry as T;
      }
      if (command === studioCommandNames.testMcpServer) {
        return demoMcpServerTestResult as T;
      }
      if (command === studioCommandNames.listMcpTools) {
        return [demoMcpToolManifest] as T;
      }
      if (command === studioCommandNames.invokeMcpTool) {
        return demoMcpToolCallResult as T;
      }
      if (command === studioCommandNames.enableMcpServerForProject) {
        return demoAgentSessionConfig as T;
      }
      return {} as never;
    };

    const bridge = createStudioBridge(invoke);
    const entry: McpServerEntry = demoMcpServerEntry;
    const request: McpToolCallRequest = {
      server_id: "weather",
      tool_name: "get_forecast",
      arguments: { city: "Reik" },
    };

    await bridge.listMcpServers();
    await bridge.upsertMcpServer(entry);
    await bridge.deleteMcpServer("weather");
    await bridge.testMcpServer("weather");
    await bridge.listMcpTools("weather");
    await bridge.invokeMcpTool(request);
    await bridge.enableMcpServerForProject("/tmp/starter-project", "weather", true);

    expect(calls).toEqual([
      { command: "list_mcp_servers", args: undefined },
      { command: "upsert_mcp_server", args: { entry } },
      { command: "delete_mcp_server", args: { id: "weather" } },
      { command: "test_mcp_server", args: { id: "weather" } },
      { command: "list_mcp_tools", args: { server_id: "weather" } },
      { command: "invoke_mcp_tool", args: { request } },
      {
        command: "enable_mcp_server_for_project",
        args: {
          project_path: "/tmp/starter-project",
          server_id: "weather",
          enabled: true,
        },
      },
    ]);
  });
});

describe("createStudioBridge image and TTS provider commands", () => {
  it("maps both four-command provider families to snake_case commands", async () => {
    const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
    const invoke: StudioInvoke = async <T>(
      command: string,
      args?: Record<string, unknown>,
    ) => {
      calls.push({ command, args });
      return {} as T;
    };
    const bridge = createStudioBridge(invoke);

    await bridge.listImageProviders();
    await bridge.upsertImageProvider(demoImageProviderEntry);
    await bridge.deleteImageProvider("image-local");
    await bridge.testImageProvider("image-local");
    await bridge.listTtsProviders();
    await bridge.upsertTtsProvider(demoTtsProviderEntry);
    await bridge.deleteTtsProvider("tts-local");
    await bridge.testTtsProvider("tts-local");

    expect(calls).toEqual([
      { command: "list_image_providers", args: undefined },
      {
        command: "upsert_image_provider",
        args: { entry: demoImageProviderEntry },
      },
      { command: "delete_image_provider", args: { id: "image-local" } },
      { command: "test_image_provider", args: { id: "image-local" } },
      { command: "list_tts_providers", args: undefined },
      {
        command: "upsert_tts_provider",
        args: { entry: demoTtsProviderEntry },
      },
      { command: "delete_tts_provider", args: { id: "tts-local" } },
      { command: "test_tts_provider", args: { id: "tts-local" } },
    ]);
  });
});

describe("createStudioBridge usage commands", () => {
  it("maps usage rollups to their snake_case commands", async () => {
    const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
    const invoke: StudioInvoke = async <T>(
      command: string,
      args?: Record<string, unknown>,
    ) => {
      calls.push({ command, args });
      return {} as T;
    };
    const bridge = createStudioBridge(invoke);

    await bridge.getUsageSummary();
    await bridge.getProviderCostReport("openai-prod");

    expect(calls).toEqual([
      { command: "get_usage_summary", args: undefined },
      {
        command: "get_provider_cost_report",
        args: { provider_id: "openai-prod" },
      },
    ]);
  });
});
