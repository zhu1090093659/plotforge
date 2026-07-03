import type { LucideIcon } from "lucide-react";
import {
  Boxes,
  Bug,
  FileCode,
  Gauge,
  KeyRound,
  Map,
  Play,
  ScrollText,
  ShipWheel,
  Users,
} from "lucide-react";

export const studioSectionIds = [
  "home",
  "play",
  "world",
  "story",
  "characters",
  "state",
  "rules",
  "assets",
  "trace",
  "export-kit",
  "source-files",
] as const;

export type StudioSectionId = (typeof studioSectionIds)[number];

export interface StudioSection {
  id: StudioSectionId;
  labelKey: string;
  descriptionKey: string;
  statusKey: string;
  icon: LucideIcon;
}

export const studioSections: StudioSection[] = [
  {
    id: "home",
    labelKey: "nav.home.label",
    descriptionKey: "nav.home.description",
    statusKey: "status.ready",
    icon: Gauge,
  },
  {
    id: "play",
    labelKey: "nav.play.label",
    descriptionKey: "nav.play.description",
    statusKey: "status.ready",
    icon: Play,
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
    icon: Boxes,
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
    id: "trace",
    labelKey: "nav.trace.label",
    descriptionKey: "nav.trace.description",
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
  {
    id: "source-files",
    labelKey: "nav.source.label",
    descriptionKey: "nav.source.description",
    statusKey: "status.ready",
    icon: FileCode,
  },
];

export function getStudioSection(id: StudioSectionId): StudioSection {
  const section = studioSections.find((candidate) => candidate.id === id);
  if (!section) {
    throw new Error(`Unknown Studio section: ${id}`);
  }
  return section;
}
