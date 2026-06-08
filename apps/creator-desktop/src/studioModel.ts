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

export interface StudioSection {
  id: string;
  label: string;
  description: string;
  status: "ready" | "next" | "later";
  icon: LucideIcon;
}

export const studioSections: StudioSection[] = [
  {
    id: "dashboard",
    label: "Dashboard",
    description: "Project health, source files, recent checks",
    status: "ready",
    icon: Gauge,
  },
  {
    id: "world",
    label: "World Bible",
    description: "Canon, forbidden facts, setting notes",
    status: "next",
    icon: Map,
  },
  {
    id: "story",
    label: "Story Craft",
    description: "Promises, hooks, reversals, emotional arc",
    status: "next",
    icon: ScrollText,
  },
  {
    id: "characters",
    label: "Characters",
    description: "Roles, arcs, visual and voice cards",
    status: "ready",
    icon: Users,
  },
  {
    id: "state",
    label: "State",
    description: "Resources, flags, triggered events",
    status: "ready",
    icon: Network,
  },
  {
    id: "rules",
    label: "Rules",
    description: "Declarative conditions and effects",
    status: "ready",
    icon: KeyRound,
  },
  {
    id: "assets",
    label: "Assets",
    description: "Generated and imported files",
    status: "ready",
    icon: Boxes,
  },
  {
    id: "playtest",
    label: "Playtest",
    description: "Local runtime preview",
    status: "ready",
    icon: Play,
  },
  {
    id: "debugger",
    label: "Debugger",
    description: "Trace, diagnostics, review notes",
    status: "ready",
    icon: Bug,
  },
  {
    id: "export",
    label: "Export",
    description: "Static web and package profiles",
    status: "ready",
    icon: ShipWheel,
  },
];

export const sourceFiles = [
  "game.toml",
  "world/world.md",
  "world/canon.md",
  "world/forbidden_facts.json",
  "story/story_craft.toml",
  "characters/*.character.toml",
  "rules/rules.toml",
  "scenes/*.scene.json",
];
