import { describe, expect, it } from "vitest";
import type {
  AiSafetyPolicy,
  AudioBible,
  Character,
  CharacterEditDocument,
  ProjectCreationReport,
  ResourceDefinition,
  Rule,
  RulesEditDocument,
  RuntimeSnapshot,
  StateVariablesEditDocument,
  StoryCraftEditDocument,
  VisualBible,
  WorldEditDocument,
} from "../../../contracts/plotforge";
import {
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

describe("createStudioBridge", () => {
  it("maps typed frontend methods to Tauri command names and arguments", async () => {
    const calls: Array<{ command: string; args?: Record<string, unknown> }> = [];
    const invoke: StudioInvoke = async <T>(command: string, args?: Record<string, unknown>) => {
      calls.push({ command, args });
      if (command === studioCommandNames.createProject) {
        const result = {
          project_path: "/tmp/winter-regency",
          template: "historical_crisis",
          concept: "A frozen court succession crisis.",
          visual_style: "ink wash winter court",
          voice_enabled: true,
          initial_scene_request: "Open with a sealed imperial edict.",
          files_created: ["game.toml"],
          project: demoProjectData,
        } satisfies ProjectCreationReport;
        return result as T;
      }
      if (command === studioCommandNames.checkProject) {
        const result = {
          title: "Dynasty Embers",
          entry_scene: "court-crisis-001",
          scene_count: 1,
          rule_count: 3,
          character_count: 6,
        } satisfies ProjectCheckReport;
        return result as T;
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
      role: "Temporary court authority",
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
      project_id: "dynasty-embers",
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
          prompt: "Sharper winter court ink prompt.",
        },
      ],
    };
    const audioBible: AudioBible = {
      voice_cards: [
        {
          ...demoProjectData.audio_bible.voice_cards[0],
          voice: "dry formal court voice",
        },
      ],
    };

    await bridge.createProject(
      "/tmp/winter-regency",
      {
        template: "historical_crisis",
        concept: "A frozen court succession crisis.",
        visual_style: "ink wash winter court",
        voice_enabled: true,
        initial_scene_request: "Open with a sealed imperial edict.",
      },
      true,
    );
    await bridge.checkProject("/tmp/dynasty-embers");
    await bridge.readWorldEditDocument("/tmp/dynasty-embers");
    await bridge.updateWorldEditDocument("/tmp/dynasty-embers", worldDocument);
    await bridge.readStoryCraftEditDocument("/tmp/dynasty-embers");
    await bridge.updateStoryCraftEditDocument(
      "/tmp/dynasty-embers",
      storyCraftDocument,
    );
    await bridge.readCharacterEditDocument("/tmp/dynasty-embers");
    await bridge.updateCharacterEditDocument(
      "/tmp/dynasty-embers",
      characterDocument,
    );
    await bridge.createCharacter("/tmp/dynasty-embers", character);
    await bridge.readStateVariablesEditDocument("/tmp/dynasty-embers");
    await bridge.updateStateVariablesEditDocument(
      "/tmp/dynasty-embers",
      stateVariablesDocument,
    );
    await bridge.createResource("/tmp/dynasty-embers", resource);
    await bridge.readRulesEditDocument("/tmp/dynasty-embers");
    await bridge.updateRulesEditDocument("/tmp/dynasty-embers", rulesDocument);
    await bridge.createRule("/tmp/dynasty-embers", rule);
    await bridge.generateWorldExpansion(
      "/tmp/dynasty-embers",
      "Expand canon and forbidden facts.",
    );
    await bridge.generateStoryCraft(
      "/tmp/dynasty-embers",
      "Generate the first pressure arc.",
    );
    await bridge.generateCharacter(
      "/tmp/dynasty-embers",
      "Design a grain envoy.",
      "court envoy",
    );
    await bridge.readAiSafetyPolicy("/tmp/dynasty-embers");
    await bridge.updateAiSafetyPolicy("/tmp/dynasty-embers", safetyPolicy);
    await bridge.readVisualBible("/tmp/dynasty-embers");
    await bridge.updateVisualBible("/tmp/dynasty-embers", visualBible);
    await bridge.readAudioBible("/tmp/dynasty-embers");
    await bridge.updateAudioBible("/tmp/dynasty-embers", audioBible);
    await bridge.playOnceProject("/tmp/dynasty-embers", "continue");
    await bridge.playOnceProjectWithSave(
      "/tmp/dynasty-embers",
      "continue",
      runtimeSnapshot.id,
    );
    await bridge.playOnceProjectFromSnapshot(
      "/tmp/dynasty-embers",
      "continue",
      runtimeSnapshot.id,
      "save-002",
    );
    await bridge.playOnceProjectFromLatestSnapshot(
      "/tmp/dynasty-embers",
      "continue",
      "save-003",
    );
    await bridge.exportStaticProject("/tmp/dynasty-embers", "/tmp/export");
    await bridge.exportStaticProjectZip(
      "/tmp/dynasty-embers",
      "/tmp/export",
      "/tmp/export.zip",
    );
    await bridge.listAssetRecords("/tmp/dynasty-embers");
    await bridge.readSourceFile("/tmp/dynasty-embers", "world/world.md");
    await bridge.writeSourceFile(
      "/tmp/dynasty-embers",
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
            concept: "A frozen court succession crisis.",
            visual_style: "ink wash winter court",
            voice_enabled: true,
            initial_scene_request: "Open with a sealed imperial edict.",
          },
          force: true,
        },
      },
      {
        command: "check_project",
        args: { path: "/tmp/dynasty-embers" },
      },
      {
        command: "read_world_edit_document",
        args: { path: "/tmp/dynasty-embers" },
      },
      {
        command: "update_world_edit_document",
        args: { path: "/tmp/dynasty-embers", document: worldDocument },
      },
      {
        command: "read_story_craft_edit_document",
        args: { path: "/tmp/dynasty-embers" },
      },
      {
        command: "update_story_craft_edit_document",
        args: { path: "/tmp/dynasty-embers", document: storyCraftDocument },
      },
      {
        command: "read_character_edit_document",
        args: { path: "/tmp/dynasty-embers" },
      },
      {
        command: "update_character_edit_document",
        args: { path: "/tmp/dynasty-embers", document: characterDocument },
      },
      {
        command: "create_character",
        args: { path: "/tmp/dynasty-embers", character },
      },
      {
        command: "read_state_variables_edit_document",
        args: { path: "/tmp/dynasty-embers" },
      },
      {
        command: "update_state_variables_edit_document",
        args: {
          path: "/tmp/dynasty-embers",
          document: stateVariablesDocument,
        },
      },
      {
        command: "create_resource",
        args: { path: "/tmp/dynasty-embers", resource },
      },
      {
        command: "read_rules_edit_document",
        args: { path: "/tmp/dynasty-embers" },
      },
      {
        command: "update_rules_edit_document",
        args: { path: "/tmp/dynasty-embers", document: rulesDocument },
      },
      {
        command: "create_rule",
        args: { path: "/tmp/dynasty-embers", rule },
      },
      {
        command: "generate_world_expansion",
        args: {
          path: "/tmp/dynasty-embers",
          expansion_goal: "Expand canon and forbidden facts.",
        },
      },
      {
        command: "generate_story_craft",
        args: {
          path: "/tmp/dynasty-embers",
          concept: "Generate the first pressure arc.",
        },
      },
      {
        command: "generate_character",
        args: {
          path: "/tmp/dynasty-embers",
          concept: "Design a grain envoy.",
          role_hint: "court envoy",
        },
      },
      {
        command: "read_ai_safety_policy",
        args: { path: "/tmp/dynasty-embers" },
      },
      {
        command: "update_ai_safety_policy",
        args: { path: "/tmp/dynasty-embers", policy: safetyPolicy },
      },
      {
        command: "read_visual_bible",
        args: { path: "/tmp/dynasty-embers" },
      },
      {
        command: "update_visual_bible",
        args: { path: "/tmp/dynasty-embers", visual_bible: visualBible },
      },
      {
        command: "read_audio_bible",
        args: { path: "/tmp/dynasty-embers" },
      },
      {
        command: "update_audio_bible",
        args: { path: "/tmp/dynasty-embers", audio_bible: audioBible },
      },
      {
        command: "play_once_project",
        args: { path: "/tmp/dynasty-embers", player_input: "continue" },
      },
      {
        command: "play_once_project_with_save",
        args: {
          path: "/tmp/dynasty-embers",
          player_input: "continue",
          save_id: "save-001",
        },
      },
      {
        command: "play_once_project_from_snapshot",
        args: {
          path: "/tmp/dynasty-embers",
          player_input: "continue",
          snapshot_id: "save-001",
          save_id: "save-002",
        },
      },
      {
        command: "play_once_project_from_latest_snapshot",
        args: {
          path: "/tmp/dynasty-embers",
          player_input: "continue",
          save_id: "save-003",
        },
      },
      {
        command: "export_static_project",
        args: { path: "/tmp/dynasty-embers", output_dir: "/tmp/export" },
      },
      {
        command: "export_static_project_zip",
        args: {
          path: "/tmp/dynasty-embers",
          output_dir: "/tmp/export",
          archive_path: "/tmp/export.zip",
        },
      },
      {
        command: "list_asset_records",
        args: { path: "/tmp/dynasty-embers" },
      },
      {
        command: "read_source_file",
        args: {
          path: "/tmp/dynasty-embers",
          relative_path: "world/world.md",
        },
      },
      {
        command: "write_source_file",
        args: {
          path: "/tmp/dynasty-embers",
          relative_path: "world/world.md",
          content: "# World Bible\n",
        },
      },
    ]);
  });
});
