export type LocalPreviewConnectionState = "mock-connected" | "local-only";
export type LocalPreviewApprovalState = "pending-local-review" | "ready";
export type LocalPreviewBuildStepState = "complete" | "active" | "pending";
export type LocalPreviewValidationState = "pass" | "pending" | "needs-review";
export type LocalPreviewWorkerKind =
  | "director"
  | "external-worker"
  | "plotforge-subagent";
export type LocalPreviewTraceLevel = "summary" | "detailed" | "full";

export interface LocalPreviewWorker {
  id: string;
  label: string;
  role: string;
  kind: LocalPreviewWorkerKind;
  connectionState: LocalPreviewConnectionState;
  capabilityIds: string[];
}

export interface LocalPreviewCapability {
  id: string;
  label: string;
  description: string;
  providerWorkerId: string;
  plotforgeAgentId: string;
  inputArtifacts: string[];
  outputArtifacts: string[];
  approvalRequired: boolean;
  traceLevel: LocalPreviewTraceLevel;
}

export interface LocalPreviewApproval {
  id: string;
  title: string;
  state: LocalPreviewApprovalState;
  evidenceIds: string[];
}

export interface LocalPreviewBuildStep {
  id: string;
  label: string;
  state: LocalPreviewBuildStepState;
  duration: string;
  detail: string;
  workerId: string;
}

export interface LocalPreviewAgentWorkStatus {
  workerId: string;
  status: "online" | "working" | "generating" | "idle";
  currentTask: string;
}

export interface LocalPreviewArtifactFile {
  path: string;
  additions: number;
  deletions: number;
  status: "created" | "modified" | "reviewed";
}

export interface LocalPreviewProposedChange {
  id: string;
  title: string;
  capabilityId: string;
  summary: string;
  reason: string;
  validation: string;
  playableImpact: string;
  files: LocalPreviewArtifactFile[];
}

export interface LocalPreviewValidationEvidence {
  id: string;
  label: string;
  state: LocalPreviewValidationState;
  detail: string;
  evidenceId: string;
}

export interface LocalPreviewBuildRun {
  id: string;
  status: "in-progress" | "ready-for-review";
  intent: string;
  timeline: LocalPreviewBuildStep[];
  agentStatuses: LocalPreviewAgentWorkStatus[];
  validationSummary: string;
  playtestOutput: string[];
}

export interface LocalPreviewArtifactBundle {
  id: string;
  title: string;
  state: "ready-for-review";
  createdAt: string;
  author: string;
  scope: string;
  summary: string;
  approvalRequirement: string;
  changes: LocalPreviewProposedChange[];
  validationEvidence: LocalPreviewValidationEvidence[];
  playableImpact: string[];
}

export interface LocalPreviewState {
  source: "local-preview-only";
  authoritativeProjectState: false;
  usesStudioDataSourcePort: false;
  networkEnabled: false;
  workers: LocalPreviewWorker[];
  capabilities: LocalPreviewCapability[];
  approvals: LocalPreviewApproval[];
  buildRun: LocalPreviewBuildRun;
  artifactBundle: LocalPreviewArtifactBundle;
  capabilityPolicy: {
    title: string;
    boundaries: string[];
    allowedPaths: string[];
    blockedPaths: string[];
  };
}

export const defaultLocalPreviewState: LocalPreviewState = {
  source: "local-preview-only",
  authoritativeProjectState: false,
  usesStudioDataSourcePort: false,
  networkEnabled: false,
  workers: [
    {
      id: "director-agent",
      label: "Director Agent",
      role: "Frames creator intent and validates playable proof",
      kind: "director",
      connectionState: "local-only",
      capabilityIds: ["intent_frame", "playable_proof"],
    },
    {
      id: "codex-worker",
      label: "Codex Worker",
      role: "Local preview coding worker for proposed patches",
      kind: "external-worker",
      connectionState: "mock-connected",
      capabilityIds: ["StoryCraft.review", "Rules.patch", "Export.package"],
    },
    {
      id: "claude-code-worker",
      label: "Claude Code Worker",
      role: "Local preview reasoning worker for review notes",
      kind: "external-worker",
      connectionState: "mock-connected",
      capabilityIds: ["StoryCraft.review", "Runtime.playtest", "Assets.generate"],
    },
    {
      id: "story-agent",
      label: "Story Agent",
      role: "StoryCraft and narrative review",
      kind: "plotforge-subagent",
      connectionState: "local-only",
      capabilityIds: ["StoryCraft.review"],
    },
    {
      id: "rules-agent",
      label: "Rules Agent",
      role: "Rule logic and constraints",
      kind: "plotforge-subagent",
      connectionState: "local-only",
      capabilityIds: ["Rules.patch"],
    },
    {
      id: "asset-agent",
      label: "Asset Agent",
      role: "Art and media asset review",
      kind: "plotforge-subagent",
      connectionState: "local-only",
      capabilityIds: ["Assets.generate"],
    },
    {
      id: "playtest-agent",
      label: "Playtest Agent",
      role: "Runtime playtest and feedback",
      kind: "plotforge-subagent",
      connectionState: "local-only",
      capabilityIds: ["Runtime.playtest"],
    },
    {
      id: "export-agent",
      label: "Export Agent",
      role: "Local build and delivery evidence",
      kind: "plotforge-subagent",
      connectionState: "local-only",
      capabilityIds: ["Export.package"],
    },
  ],
  capabilities: [
    {
      id: "StoryCraft.review",
      label: "StoryCraft.review",
      description: "Analyze and critique narrative artifacts",
      providerWorkerId: "codex-worker",
      plotforgeAgentId: "story-agent",
      inputArtifacts: ["chapters/*.md", "beats/*.json", "world/*.md"],
      outputArtifacts: ["review.md", "suggestions.json", "risk.json"],
      approvalRequired: false,
      traceLevel: "full",
    },
    {
      id: "Rules.patch",
      label: "Rules.patch",
      description: "Propose rule changes for human review",
      providerWorkerId: "claude-code-worker",
      plotforgeAgentId: "rules-agent",
      inputArtifacts: ["rules/*.yml", "policies/*.md"],
      outputArtifacts: ["patch.diff", "rationale.md"],
      approvalRequired: true,
      traceLevel: "full",
    },
    {
      id: "Runtime.playtest",
      label: "Runtime.playtest",
      description: "Execute local playtest scenarios",
      providerWorkerId: "claude-code-worker",
      plotforgeAgentId: "playtest-agent",
      inputArtifacts: ["scenarios/*.yml", "build/*.zip"],
      outputArtifacts: ["results.json", "metrics.csv"],
      approvalRequired: true,
      traceLevel: "detailed",
    },
    {
      id: "Assets.generate",
      label: "Assets.generate",
      description: "Review or prepare media asset records",
      providerWorkerId: "codex-worker",
      plotforgeAgentId: "asset-agent",
      inputArtifacts: ["art/*", "audio/*", "prompts/*.md"],
      outputArtifacts: ["assets/*", "manifest.json"],
      approvalRequired: true,
      traceLevel: "detailed",
    },
    {
      id: "Export.package",
      label: "Export.package",
      description: "Create local distributable package evidence",
      providerWorkerId: "codex-worker",
      plotforgeAgentId: "export-agent",
      inputArtifacts: ["build/*", "config/export.yml"],
      outputArtifacts: ["package.zip", "checksums.txt"],
      approvalRequired: true,
      traceLevel: "full",
    },
  ],
  approvals: [
    {
      id: "approval-story-thread",
      title: "Review generated story thread bundle",
      state: "pending-local-review",
      evidenceIds: ["trace_id", "files_changed", "playtest_result"],
    },
    {
      id: "approval-export-disclosure",
      title: "Confirm export disclosure evidence",
      state: "ready",
      evidenceIds: ["ai-usage.json", "content-warning.md"],
    },
  ],
  buildRun: {
    id: "run-042",
    status: "ready-for-review",
    intent: "Turn the court crisis into a tense public accusation sequence.",
    timeline: [
      {
        id: "plan",
        label: "Plan",
        state: "complete",
        duration: "1m 12s",
        detail: "Intent parsed and artifact tasks planned.",
        workerId: "director-agent",
      },
      {
        id: "generate",
        label: "Generate",
        state: "complete",
        duration: "3m 45s",
        detail: "Story, rule, and asset proposals prepared.",
        workerId: "story-agent",
      },
      {
        id: "patch",
        label: "Patch",
        state: "complete",
        duration: "1m 05s",
        detail: "Changed files collected as reviewable artifacts.",
        workerId: "codex-worker",
      },
      {
        id: "validate",
        label: "Validate",
        state: "complete",
        duration: "0m 58s",
        detail: "Contracts, references, and safety boundary checked.",
        workerId: "rules-agent",
      },
      {
        id: "playtest",
        label: "Playtest",
        state: "complete",
        duration: "2m 10s",
        detail: "Runtime proof produced visible state deltas.",
        workerId: "playtest-agent",
      },
      {
        id: "review",
        label: "Review",
        state: "active",
        duration: "pending",
        detail: "Awaiting explicit creator approval.",
        workerId: "director-agent",
      },
    ],
    agentStatuses: [
      {
        workerId: "codex-worker",
        status: "online",
        currentTask: "Patch bundle ready for human review",
      },
      {
        workerId: "claude-code-worker",
        status: "online",
        currentTask: "Narrative and rule evidence attached",
      },
      {
        workerId: "story-agent",
        status: "working",
        currentTask: "Story contract change summarized",
      },
      {
        workerId: "rules-agent",
        status: "working",
        currentTask: "Rule patch validation complete",
      },
      {
        workerId: "asset-agent",
        status: "generating",
        currentTask: "Scene asset record reviewed",
      },
      {
        workerId: "export-agent",
        status: "idle",
        currentTask: "Waiting for approved artifact bundle",
      },
    ],
    validationSummary: "All local preview checks passing",
    playtestOutput: [
      "Turn 1: player selected emergency tax pressure.",
      "Turn 2: public_order changed by -8; treasury changed by +12.",
      "Narrative review: 100/100 with no issues.",
    ],
  },
  artifactBundle: {
    id: "bundle-court-crisis-001",
    title: "Court Crisis Pressure Bundle",
    state: "ready-for-review",
    createdAt: "2026-06-09 10:21",
    author: "Codex Worker via local preview",
    scope: "Story, Rules, Assets",
    summary:
      "4 files changed to make the opening crisis more playable and evidence-backed.",
    approvalRequirement:
      "Explicit local approval required before any creator-owned commit step.",
    changes: [
      {
        id: "story-contract",
        title: "Story Contract",
        capabilityId: "StoryCraft.review",
        summary: "Expanded the court accusation beat and player-facing choices.",
        reason: "The opening scene needed a sharper choice consequence.",
        validation: "Story schema valid; no broken scene references.",
        playableImpact:
          "The playable scene now exposes a public order tradeoff after tax pressure.",
        files: [
          {
            path: "chapters/02.md",
            additions: 68,
            deletions: 10,
            status: "modified",
          },
          {
            path: "beats/ambush.json",
            additions: 22,
            deletions: 0,
            status: "created",
          },
        ],
      },
      {
        id: "rule-patch",
        title: "Rule Patch",
        capabilityId: "Rules.patch",
        summary: "Adjusted suspicion and public order effects for the crisis.",
        reason: "The consequence model should match the new public accusation beat.",
        validation: "Rule evaluation passes with deterministic state deltas.",
        playableImpact:
          "Choosing emergency taxes visibly lowers order while increasing treasury.",
        files: [
          {
            path: "rules/suspicion.yml",
            additions: 18,
            deletions: 4,
            status: "modified",
          },
        ],
      },
      {
        id: "scene-asset",
        title: "Scene Asset",
        capabilityId: "Assets.generate",
        summary: "Reviewed the court crisis background asset record.",
        reason: "The proof view needs a concrete scene image reference.",
        validation: "Asset hash, provider metadata, and scene reference are present.",
        playableImpact:
          "The scene canvas can show a stable court image during proof review.",
        files: [
          {
            path: "assets/generated/court-crisis-001.png",
            additions: 1,
            deletions: 0,
            status: "reviewed",
          },
        ],
      },
      {
        id: "risk-notes",
        title: "Risk Notes",
        capabilityId: "Runtime.playtest",
        summary: "Attached playtest and safety notes to the review bundle.",
        reason: "Approval needs validation evidence, not just changed files.",
        validation: "Trace metadata and safety boundary evidence are redaction-safe.",
        playableImpact:
          "Reviewers can replay the proof path before deciding on the bundle.",
        files: [
          {
            path: "reviews/court-crisis-proof.md",
            additions: 24,
            deletions: 0,
            status: "created",
          },
        ],
      },
    ],
    validationEvidence: [
      {
        id: "schema-valid",
        label: "Schema valid",
        state: "pass",
        detail: "All modified files pass generated contract checks.",
        evidenceId: "contracts/check",
      },
      {
        id: "links-valid",
        label: "Links and references",
        state: "pass",
        detail: "No broken scene, rule, or asset references detected.",
        evidenceId: "reference-scan",
      },
      {
        id: "determinism",
        label: "Determinism",
        state: "pass",
        detail: "Proof run is reproducible with the current seed.",
        evidenceId: "seed-872314",
      },
      {
        id: "safety-boundary",
        label: "Safety boundary",
        state: "pass",
        detail: "No writes outside allowed local preview paths.",
        evidenceId: "path-policy",
      },
    ],
    playableImpact: [
      "Public order and treasury deltas are visible after the proof turn.",
      "The current scene remains playable with three player choices.",
      "Trace evidence links the rule result, planner result, and state delta.",
    ],
  },
  capabilityPolicy: {
    title: "Local preview boundary",
    boundaries: [
      "Mock workers are UI preview data only.",
      "Approvals do not persist to folder project files.",
      "No ACP network, provider call, credential, upload, or platform publishing behavior is enabled.",
    ],
    allowedPaths: [
      "/projects/dynasty-embers/",
      "/assets/",
      "/docs/",
      "/build/exports/",
    ],
    blockedPaths: ["/secrets/", "/config/provider-keys/", "~/.ssh/", "/system/"],
  },
};

export function localPreviewSummary(state: LocalPreviewState) {
  const externalWorkers = state.workers.filter(
    (worker) => worker.kind === "external-worker",
  );
  const plotforgeSubagents = state.workers.filter(
    (worker) => worker.kind === "plotforge-subagent",
  );

  return {
    workerCount: state.workers.length,
    externalWorkerCount: externalWorkers.length,
    plotforgeSubagentCount: plotforgeSubagents.length,
    capabilityCount: state.capabilities.length,
    approvalRequiredCapabilityCount: state.capabilities.filter(
      (capability) => capability.approvalRequired,
    ).length,
    pendingApprovalCount: state.approvals.filter(
      (approval) => approval.state === "pending-local-review",
    ).length,
    boundaryCount: state.capabilityPolicy.boundaries.length,
    buildStepCount: state.buildRun.timeline.length,
    artifactChangeCount: state.artifactBundle.changes.length,
    validationCheckCount: state.artifactBundle.validationEvidence.length,
    isAuthoritativeProjectState: state.authoritativeProjectState,
    usesStudioDataSourcePort: state.usesStudioDataSourcePort,
    networkEnabled: state.networkEnabled,
  };
}
