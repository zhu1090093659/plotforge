import type { LucideIcon } from "lucide-react";
import {
  Boxes,
  Bug,
  Clapperboard,
  FileText,
  Flag,
  Gauge,
  Image,
  KeyRound,
  Map,
  Network,
  Play,
  ScrollText,
  Settings2,
  ShipWheel,
  Sparkles,
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
    status: "later",
    icon: Users,
  },
  {
    id: "locations",
    label: "Locations",
    description: "Places, factions, scene continuity",
    status: "later",
    icon: Flag,
  },
  {
    id: "state",
    label: "State",
    description: "Resources, flags, triggered events",
    status: "later",
    icon: Network,
  },
  {
    id: "rules",
    label: "Rules",
    description: "Declarative conditions and effects",
    status: "later",
    icon: KeyRound,
  },
  {
    id: "events",
    label: "Events",
    description: "World consequences and story triggers",
    status: "later",
    icon: Clapperboard,
  },
  {
    id: "agents",
    label: "Agent Config",
    description: "Provider adapters and role prompts",
    status: "later",
    icon: Sparkles,
  },
  {
    id: "visual",
    label: "Visual Bible",
    description: "Style rules and reference continuity",
    status: "later",
    icon: Image,
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
    id: "assets",
    label: "Assets",
    description: "Generated and imported files",
    status: "later",
    icon: Boxes,
  },
  {
    id: "export",
    label: "Export",
    description: "Static web and package profiles",
    status: "later",
    icon: ShipWheel,
  },
  {
    id: "source",
    label: "Source Files",
    description: "TOML, JSON, Markdown project files",
    status: "ready",
    icon: FileText,
  },
  {
    id: "settings",
    label: "Settings",
    description: "Local commands and workspace config",
    status: "later",
    icon: Settings2,
  },
];

export const sourceFiles = [
  "game.toml",
  "world/world.md",
  "story/story_craft.toml",
  "characters/*.character.toml",
  "rules/rules.toml",
  "scenes/*.scene.json",
];
