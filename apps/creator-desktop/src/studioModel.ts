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
  Settings,
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
  "settings",
] as const;

export type StudioSectionId = (typeof studioSectionIds)[number];

export interface StudioSection {
  id: StudioSectionId;
  labelKey: string;
  descriptionKey: string;
  icon: LucideIcon;
  /** 1-based position in the flat nav list, rendered as the copper eyebrow
   * numeral ("01", "02", …) in the sidebar and the view header. Pure data,
   * never localized. */
  index: number;
}

export const studioSections: StudioSection[] = [
  {
    id: "home",
    labelKey: "nav.home.label",
    descriptionKey: "nav.home.description",
    icon: Gauge,
    index: 1,
  },
  {
    id: "play",
    labelKey: "nav.play.label",
    descriptionKey: "nav.play.description",
    icon: Play,
    index: 2,
  },
  {
    id: "world",
    labelKey: "nav.world.label",
    descriptionKey: "nav.world.description",
    icon: Map,
    index: 3,
  },
  {
    id: "story",
    labelKey: "nav.story.label",
    descriptionKey: "nav.story.description",
    icon: ScrollText,
    index: 4,
  },
  {
    id: "characters",
    labelKey: "nav.characters.label",
    descriptionKey: "nav.characters.description",
    icon: Users,
    index: 5,
  },
  {
    id: "state",
    labelKey: "nav.state.label",
    descriptionKey: "nav.state.description",
    icon: Boxes,
    index: 6,
  },
  {
    id: "rules",
    labelKey: "nav.rules.label",
    descriptionKey: "nav.rules.description",
    icon: KeyRound,
    index: 7,
  },
  {
    id: "assets",
    labelKey: "nav.assets.label",
    descriptionKey: "nav.assets.description",
    icon: Boxes,
    index: 8,
  },
  {
    id: "trace",
    labelKey: "nav.trace.label",
    descriptionKey: "nav.trace.description",
    icon: Bug,
    index: 9,
  },
  {
    id: "export-kit",
    labelKey: "nav.exportKit.label",
    descriptionKey: "nav.exportKit.description",
    icon: ShipWheel,
    index: 10,
  },
  {
    id: "source-files",
    labelKey: "nav.source.label",
    descriptionKey: "nav.source.description",
    icon: FileCode,
    index: 11,
  },
  {
    id: "settings",
    labelKey: "nav.settings.label",
    descriptionKey: "nav.settings.description",
    icon: Settings,
    index: 12,
  },
];

/** Zero-padded 2-digit numeral for the copper eyebrow ("01"…). */
export function studioSectionNumeral(index: number): string {
  return String(index).padStart(2, "0");
}

export function getStudioSection(id: StudioSectionId): StudioSection {
  const section = studioSections.find((candidate) => candidate.id === id);
  if (!section) {
    throw new Error(`Unknown Studio section: ${id}`);
  }
  return section;
}
