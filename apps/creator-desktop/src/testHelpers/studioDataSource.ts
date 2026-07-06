import { demoPlayOnceReport, demoProjectData, demoReproducibilityMetadata } from "../demoStudioData";
import type {
  AgentSessionConfig,
  McpServerEntry,
  McpServerTestResult,
  McpToolCallRequest,
  McpToolCallResult,
  McpToolManifest,
  PiAgentApplyRequest,
  PiAgentApplyResult,
  ProviderEntry,
  PromptTemplate,
  SkillIndex,
  SkillManifest,
} from "../../../../contracts/plotforge";
import type {
  PlayOnceReport,
  ProviderTestResult,
} from "../tauriBridge";

/**
 * Build a `PlayOnceReport` with an attached runtime snapshot, reused by
 * snapshot-related mock data sources so the snapshot shape stays consistent.
 */
export function playOnceReportWithSnapshot(
  playerInput: string,
  snapshotId: string,
  snapshotPath: string,
): PlayOnceReport {
  const report = demoPlayOnceReport(playerInput);
  return {
    ...report,
    snapshot: {
      id: snapshotId,
      timestamp_ms: report.trace.timestamp_ms,
      reproducibility: {
        ...demoReproducibilityMetadata,
        snapshot_id: snapshotId,
      },
      project_id: demoProjectData.game.id,
      project_version: demoProjectData.game.version,
      story_state: report.trace.story_state_after,
      world_state: report.trace.world_state_after,
      scenes: demoProjectData.scenes,
    },
    snapshot_path: snapshotPath,
  };
}

/**
 * Mock `playOnceProjectWithSave` shared across App tests: returns a report
 * with a snapshot whose id/path derive from the requested save id.
 */
export async function mockPlayOnceProjectWithSave(
  _path: string,
  playerInput: string,
  saveId: string,
): Promise<PlayOnceReport> {
  const report = demoPlayOnceReport(playerInput);
  return {
    ...report,
    snapshot: {
      id: saveId,
      timestamp_ms: report.trace.timestamp_ms,
      reproducibility: {
        ...demoReproducibilityMetadata,
        snapshot_id: saveId,
      },
      project_id: demoProjectData.game.id,
      project_version: demoProjectData.game.version,
      story_state: report.trace.story_state_after,
      world_state: report.trace.world_state_after,
      scenes: demoProjectData.scenes,
    },
    snapshot_path: `/tmp/starter-project/saves/${saveId}.runtime_snapshot.json`,
  };
}

/**
 * Mock `playOnceProjectFromSnapshot`: with a saveId, behave like
 * `mockPlayOnceProjectWithSave`; without one, return the plain report.
 */
export async function mockPlayOnceProjectFromSnapshot(
  path: string,
  playerInput: string,
  _snapshotId: string,
  saveId: string | null = null,
): Promise<PlayOnceReport> {
  return saveId
    ? mockPlayOnceProjectWithSave(path, playerInput, saveId)
    : demoPlayOnceReport(playerInput);
}

/**
 * Mock `playOnceProjectFromLatestSnapshot`: with a saveId, behave like
 * `mockPlayOnceProjectWithSave`; without one, return the plain report.
 */
export async function mockPlayOnceProjectFromLatestSnapshot(
  path: string,
  playerInput: string,
  saveId: string | null = null,
): Promise<PlayOnceReport> {
  return saveId
    ? mockPlayOnceProjectWithSave(path, playerInput, saveId)
    : demoPlayOnceReport(playerInput);
}

/**
 * Mock `piAgentApplyRun`: returns a minimal `PiAgentApplyResult` whose
 * `run`/`scene`/`trace` shape mirrors the demo play-once report so the
 * apply-flow can be exercised in tests without a live provider.
 */
export async function mockPiAgentApplyRun(
  _request: PiAgentApplyRequest,
): Promise<PiAgentApplyResult> {
  const report = demoPlayOnceReport("apply-mock");
  return {
    run: {
      descriptor: {
        agent_id: "scene_planner",
        is_local_pi: true,
        capabilities: [],
      },
      reproducibility: {
        run_seed: 0,
        prompt_version: "",
        model_version: "",
        provider_config_hash: "",
      },
      trace_id: null,
      evidence_summary: "",
    },
    scene_key: report.scene.key,
    scene: report.scene,
    trace: report.trace,
    trace_path: report.trace_path,
    snapshot: report.snapshot,
    snapshot_path: report.snapshot_path,
    delta_summary: report.delta_summary,
  };
}

export async function mockListProviders(): Promise<ProviderEntry[]> {
  return [];
}

export async function mockUpsertProvider(entry: ProviderEntry): Promise<ProviderEntry> {
  return entry;
}

export async function mockDeleteProvider(id: string): Promise<ProviderEntry> {
  return {
    id,
    kind: "openai_compatible",
    label: "",
    endpoint_url: "",
    model: "",
    credential_env_var: "",
    enabled: false,
  };
}

export async function mockTestProviderConnection(_id: string): Promise<ProviderTestResult> {
  return { ok: true, message: "" };
}

export async function mockListUserPromptTemplates(): Promise<PromptTemplate[]> {
  return [];
}

export async function mockListProjectPromptTemplates(_projectPath: string): Promise<PromptTemplate[]> {
  return [];
}

export async function mockUpsertUserPromptTemplate(template: PromptTemplate): Promise<PromptTemplate> {
  return template;
}

export async function mockUpsertProjectPromptTemplate(
  _projectPath: string,
  template: PromptTemplate,
): Promise<PromptTemplate> {
  return template;
}

export async function mockDeleteUserPromptTemplate(_id: string): Promise<void> {
  return;
}

export async function mockDeleteProjectPromptTemplate(
  _projectPath: string,
  _id: string,
): Promise<void> {
  return;
}

export async function mockListSkills(): Promise<SkillManifest[]> {
  return [];
}

export async function mockRefreshSkillIndex(): Promise<SkillIndex> {
  return { version: "1", skills: [], scanned_at: "" };
}

export async function mockImportSkill(skillId: string): Promise<SkillManifest> {
  return {
    id: skillId,
    name: "",
    description: "",
    source: { origin: "plot_forge_user", root_path: "", rel_path: "" },
    interface: null,
    body_path: "",
    scripts: [],
    references: [],
    assets: [],
  };
}

export async function mockReadSkillBody(_skillId: string): Promise<string> {
  return "";
}

export async function mockEnableSkillForProject(
  _projectPath: string,
  _skillId: string,
  _enabled: boolean,
): Promise<AgentSessionConfig> {
  return {
    model_id: "local-pi",
    permission_level: "ask_every_time",
    thinking_level: "medium",
    enabled_skills: [],
    enabled_mcp_servers: [],
  };
}
