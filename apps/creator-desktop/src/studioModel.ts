import type { LucideIcon } from "lucide-react";
import {
  Boxes,
  Bug,
  Gauge,
  KeyRound,
  Map,
  Network,
  Play,
  ScrollText,
  ShipWheel,
  Users,
} from "lucide-react";

export const agentNativeScreenIds = [
  "agent-mesh-core",
  "project-launchpad",
  "command-center",
  "director-mode",
  "pi-agent-bridge",
  "live-build-room",
  "artifact-review",
  "playable-proof",
  "trace-debug",
  "export-package",
] as const;

export type AgentNativeScreenId = (typeof agentNativeScreenIds)[number];

export interface AgentNativeScreenReference {
  id: AgentNativeScreenId;
  titleKey: string;
  fileName: string;
}

export const agentNativeScreenReferences: readonly AgentNativeScreenReference[] = [
  {
    id: "agent-mesh-core",
    titleKey: "screen.agentMeshCore.title",
    fileName: "00-agent-mesh-core.png",
  },
  {
    id: "project-launchpad",
    titleKey: "screen.projectLaunchpad.title",
    fileName: "01-project-launchpad.png",
  },
  {
    id: "command-center",
    titleKey: "screen.commandCenter.title",
    fileName: "02-command-center.png",
  },
  {
    id: "director-mode",
    titleKey: "screen.directorMode.title",
    fileName: "03-director-mode.png",
  },
  {
    id: "pi-agent-bridge",
    titleKey: "screen.piAgentBridge.title",
    fileName: "04-pi-agent-bridge-setup.png",
  },
  {
    id: "live-build-room",
    titleKey: "screen.liveBuildRoom.title",
    fileName: "05-live-build-room.png",
  },
  {
    id: "artifact-review",
    titleKey: "screen.artifactReview.title",
    fileName: "06-artifact-review.png",
  },
  {
    id: "playable-proof",
    titleKey: "screen.playableProof.title",
    fileName: "07-playable-proof.png",
  },
  {
    id: "trace-debug",
    titleKey: "screen.traceDebug.title",
    fileName: "08-trace-debug.png",
  },
  {
    id: "export-package",
    titleKey: "screen.exportPackage.title",
    fileName: "09-export-package.png",
  },
];

export const studioSectionIds = [
  "launchpad",
  "agent-mesh",
  "world",
  "story",
  "characters",
  "state",
  "rules",
  "assets",
  "playtest",
  "debugger",
  "export-kit",
] as const;

export type StudioSectionId = (typeof studioSectionIds)[number];

export const agentNativeWorkflowIds = [
  "command",
  "game",
  "agents",
  "artifacts",
  "proof",
  "export",
] as const;

export type AgentNativeWorkflowId = (typeof agentNativeWorkflowIds)[number];

export interface StudioSection {
  id: StudioSectionId;
  labelKey: string;
  descriptionKey: string;
  statusKey: string;
  icon: LucideIcon;
}

export const studioSections: StudioSection[] = [
  {
    id: "launchpad",
    labelKey: "nav.launchpad.label",
    descriptionKey: "nav.launchpad.description",
    statusKey: "status.ready",
    icon: Gauge,
  },
  {
    id: "agent-mesh",
    labelKey: "nav.agentMesh.label",
    descriptionKey: "nav.agentMesh.description",
    statusKey: "status.ready",
    icon: Network,
  },
  {
    id: "world",
    labelKey: "nav.world.label",
    descriptionKey: "nav.world.description",
    statusKey: "status.next",
    icon: Map,
  },
  {
    id: "story",
    labelKey: "nav.story.label",
    descriptionKey: "nav.story.description",
    statusKey: "status.next",
    icon: ScrollText,
  },
  {
    id: "characters",
    labelKey: "nav.characters.label",
    descriptionKey: "nav.characters.description",
    statusKey: "status.ready",
    icon: Users,
  },
  {
    id: "state",
    labelKey: "nav.state.label",
    descriptionKey: "nav.state.description",
    statusKey: "status.ready",
    icon: Network,
  },
  {
    id: "rules",
    labelKey: "nav.rules.label",
    descriptionKey: "nav.rules.description",
    statusKey: "status.ready",
    icon: KeyRound,
  },
  {
    id: "assets",
    labelKey: "nav.assets.label",
    descriptionKey: "nav.assets.description",
    statusKey: "status.ready",
    icon: Boxes,
  },
  {
    id: "playtest",
    labelKey: "nav.playtest.label",
    descriptionKey: "nav.playtest.description",
    statusKey: "status.ready",
    icon: Play,
  },
  {
    id: "debugger",
    labelKey: "nav.debugger.label",
    descriptionKey: "nav.debugger.description",
    statusKey: "status.ready",
    icon: Bug,
  },
  {
    id: "export-kit",
    labelKey: "nav.exportKit.label",
    descriptionKey: "nav.exportKit.description",
    statusKey: "status.ready",
    icon: ShipWheel,
  },
];

export interface AgentNativeWorkflow {
  id: AgentNativeWorkflowId;
  labelKey: string;
  shortLabelKey: string;
  descriptionKey: string;
  statusKey: string;
  icon: LucideIcon;
  defaultSectionId: StudioSectionId;
  sectionIds: readonly StudioSectionId[];
  screenIds: readonly AgentNativeScreenId[];
}

export const agentNativeWorkflows: readonly AgentNativeWorkflow[] = [
  {
    id: "command",
    labelKey: "workflow.command.label",
    shortLabelKey: "workflow.command.shortLabel",
    descriptionKey: "workflow.command.description",
    statusKey: "status.ready",
    icon: Gauge,
    defaultSectionId: "launchpad",
    sectionIds: ["launchpad", "world", "story"],
    screenIds: ["project-launchpad", "command-center"],
  },
  {
    id: "game",
    labelKey: "workflow.game.label",
    shortLabelKey: "workflow.game.shortLabel",
    descriptionKey: "workflow.game.description",
    statusKey: "status.ready",
    icon: Play,
    defaultSectionId: "playtest",
    sectionIds: ["playtest", "world", "story", "characters", "state", "rules"],
    screenIds: ["director-mode"],
  },
  {
    id: "agents",
    labelKey: "workflow.agents.label",
    shortLabelKey: "workflow.agents.shortLabel",
    descriptionKey: "workflow.agents.description",
    statusKey: "status.ready",
    icon: Network,
    defaultSectionId: "agent-mesh",
    sectionIds: ["agent-mesh"],
    screenIds: ["agent-mesh-core", "pi-agent-bridge"],
  },
  {
    id: "artifacts",
    labelKey: "workflow.artifacts.label",
    shortLabelKey: "workflow.artifacts.shortLabel",
    descriptionKey: "workflow.artifacts.description",
    statusKey: "status.ready",
    icon: Boxes,
    defaultSectionId: "assets",
    sectionIds: ["assets", "world", "story", "characters", "rules"],
    screenIds: ["live-build-room", "artifact-review"],
  },
  {
    id: "proof",
    labelKey: "workflow.proof.label",
    shortLabelKey: "workflow.proof.shortLabel",
    descriptionKey: "workflow.proof.description",
    statusKey: "status.ready",
    icon: Bug,
    defaultSectionId: "debugger",
    sectionIds: ["playtest", "debugger"],
    screenIds: ["playable-proof", "trace-debug"],
  },
  {
    id: "export",
    labelKey: "workflow.export.label",
    shortLabelKey: "workflow.export.shortLabel",
    descriptionKey: "workflow.export.description",
    statusKey: "status.ready",
    icon: ShipWheel,
    defaultSectionId: "export-kit",
    sectionIds: ["export-kit", "assets", "debugger"],
    screenIds: ["export-package"],
  },
];

export function getAgentNativeWorkflow(
  id: AgentNativeWorkflowId,
): AgentNativeWorkflow {
  const workflow = agentNativeWorkflows.find((candidate) => candidate.id === id);
  if (!workflow) {
    throw new Error(`Unknown agent-native workflow: ${id}`);
  }
  return workflow;
}

export function getStudioSection(id: StudioSectionId): StudioSection {
  const section = studioSections.find((candidate) => candidate.id === id);
  if (!section) {
    throw new Error(`Unknown Studio section: ${id}`);
  }
  return section;
}

export function getAgentNativeScreenReference(
  id: AgentNativeScreenId,
): AgentNativeScreenReference {
  const reference = agentNativeScreenReferences.find(
    (candidate) => candidate.id === id,
  );
  if (!reference) {
    throw new Error(`Unknown agent-native screen reference: ${id}`);
  }
  return reference;
}

export function screenReferencesForWorkflow(
  workflowId: AgentNativeWorkflowId,
): AgentNativeScreenReference[] {
  return getAgentNativeWorkflow(workflowId).screenIds.map((screenId) =>
    getAgentNativeScreenReference(screenId),
  );
}

export function workflowForSection(
  sectionId: StudioSectionId,
): AgentNativeWorkflow {
  const workflow = agentNativeWorkflows.find((candidate) =>
    candidate.sectionIds.includes(sectionId),
  );
  if (!workflow) {
    throw new Error(`Studio section is not assigned to a workflow: ${sectionId}`);
  }
  return workflow;
}

export function defaultSectionForWorkflow(
  workflowId: AgentNativeWorkflowId,
): StudioSectionId {
  return getAgentNativeWorkflow(workflowId).defaultSectionId;
}

export function isSectionInWorkflow(
  sectionId: StudioSectionId,
  workflowId: AgentNativeWorkflowId,
): boolean {
  return getAgentNativeWorkflow(workflowId).sectionIds.includes(sectionId);
}
