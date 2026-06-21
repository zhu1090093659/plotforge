import {
  ChevronDown,
  ChevronRight,
  FolderOpen,
  Loader2,
  type LucideIcon,
} from "lucide-react";
import { type ButtonHTMLAttributes, type ReactNode, useState } from "react";

export const agentNativeDesignTokens = {
  shell: {
    graphite: "#111419",
    graphitePanel: "#171b21",
    warmCanvas: "#f7f0df",
  },
  accent: {
    amberAction: "#c98b2f",
    healthGreen: "#34815f",
    acpCyan: "#2e8ca0",
    agentPurple: "#7559a8",
  },
} as const;

export const studioUiClassNames = {
  panel:
    "rounded-lg border border-ink/10 bg-canvas-50 p-5 text-ink shadow-studio-panel",
  insetPanel: "rounded-lg border border-ink/10 bg-canvas-100 px-3 py-3 text-ink",
  input:
    "h-10 min-w-0 rounded-md border border-graphite-700/20 bg-canvas-50 px-3 text-sm text-ink outline-none transition focus:border-acp-400",
  textarea:
    "w-full resize-y rounded-md border border-graphite-700/20 bg-canvas-50 px-3 py-2 text-sm leading-6 text-ink outline-none transition focus:border-acp-400",
  primaryButton:
    "inline-flex h-10 items-center gap-2 rounded-md bg-amber-500 px-4 text-sm font-semibold text-graphite-950 transition hover:bg-amber-400 disabled:cursor-not-allowed disabled:bg-amber-500/35 disabled:text-graphite-950/45",
  secondaryButton:
    "inline-flex h-9 items-center gap-2 rounded-md border border-graphite-700/20 bg-canvas-50 px-3 text-sm font-semibold text-ink transition hover:border-graphite-700/45 disabled:cursor-not-allowed disabled:text-ink/30",
  iconButton:
    "grid h-10 w-10 place-items-center rounded-md border border-canvas-200/20 bg-canvas-50/10 text-canvas-50 transition hover:border-amber-400/70 hover:bg-canvas-50/15",
  chip:
    "rounded-sm border px-2 py-1 text-xs font-semibold",
} as const;

export interface StudioNavItem {
  id: string;
  label: string;
  sublabel: string;
  description: string;
  icon: LucideIcon;
  selected: boolean;
  onSelect(): void;
}

interface StudioShellHeader {
  eyebrow: string;
  title: string;
  subtitle: string;
  badges: Array<{
    id: string;
    label: string;
    title?: string;
  }>;
}

interface StudioShellProps {
  projectPath: string;
  projectLoading: boolean;
  onOpenProject(): void;
  workflowItems: StudioNavItem[];
  surfaceItems: StudioNavItem[];
  header: StudioShellHeader;
  topActions: ReactNode;
  rightPanel: ReactNode;
  commandDock?: ReactNode;
  children: ReactNode;
}

export function StudioShell({
  projectPath,
  projectLoading,
  onOpenProject,
  workflowItems,
  surfaceItems,
  header,
  topActions,
  rightPanel,
  commandDock,
  children,
}: StudioShellProps) {
  return (
    <div className="min-h-screen bg-graphite-950 text-canvas-50">
      <div
        data-testid="studio-shell-grid"
        className="grid min-h-screen grid-cols-[280px_minmax(0,1fr)_320px] max-xl:grid-cols-[260px_minmax(0,1fr)] max-lg:grid-cols-1"
      >
        <aside
          aria-label="Studio navigation"
          className="border-r border-canvas-200/10 bg-graphite-900 px-4 py-5 max-lg:border-b max-lg:border-r-0"
        >
          <div className="flex items-center gap-3">
            <div className="grid h-10 w-10 place-items-center rounded-md border border-amber-400/45 bg-amber-500 text-sm font-black text-graphite-950">
              PF
            </div>
            <div className="min-w-0">
              <p className="text-xs font-semibold uppercase text-amber-400">
                PlotForge Studio
              </p>
              <h1 className="truncate text-xl font-semibold text-canvas-50">
                Creator Desktop
              </h1>
            </div>
          </div>

          <div className="mt-6 flex items-center justify-between rounded-lg border border-canvas-200/10 bg-graphite-850 px-3 py-2">
            <div className="min-w-0">
              <p className="text-xs font-medium uppercase text-canvas-200/60">
                Open Project
              </p>
              <p className="max-w-44 truncate text-sm font-semibold text-canvas-50">
                {projectPath}
              </p>
            </div>
            <button
              type="button"
              title="Open project"
              onClick={onOpenProject}
              className="grid h-9 w-9 place-items-center rounded-md border border-canvas-200/15 bg-canvas-50/10 text-canvas-50 transition hover:border-amber-400/70"
            >
              {projectLoading ? (
                <Loader2 aria-hidden size={18} className="animate-spin" />
              ) : (
                <FolderOpen aria-hidden size={18} />
              )}
            </button>
          </div>

          <StudioNavList
            label="Agent-native workflows"
            items={workflowItems}
            className="mt-5"
            selectedTone="workflow"
          />
          <StudioNavList
            label="Workflow surfaces"
            items={surfaceItems}
            className="mt-5"
            selectedTone="surface"
          />
        </aside>

        <main className="min-w-0 bg-canvas-100 text-ink">
          <header className="flex flex-wrap items-center justify-between gap-4 border-b border-graphite-700/10 px-6 py-5 lg:px-8">
            <div className="min-w-0">
              <p className="text-xs font-semibold uppercase text-graphite-700/55">
                {header.eyebrow}
              </p>
              <h2 className="mt-1 text-2xl font-semibold text-ink">
                {header.title}
              </h2>
              <p className="mt-1 max-w-3xl text-sm text-graphite-700/70">
                {header.subtitle}
              </p>
              <div className="mt-2 flex flex-wrap gap-2">
                {header.badges.map((badge) => (
                  <StudioStatusChip key={badge.id} title={badge.title}>
                    {badge.label}
                  </StudioStatusChip>
                ))}
              </div>
            </div>
            <div className="flex min-w-0 flex-wrap items-center gap-2">
              {topActions}
            </div>
          </header>

          <div className="px-6 py-5 lg:px-8">{children}</div>
        </main>

        <aside
          aria-label="Evidence panel"
          className="border-l border-canvas-200/10 bg-graphite-900 px-4 py-5 text-canvas-50 max-xl:col-span-2 max-xl:border-l-0 max-xl:border-t max-lg:col-span-1"
        >
          {rightPanel}
        </aside>

        {commandDock ? (
          <div className="col-span-3 border-t border-canvas-200/10 bg-graphite-950 px-4 py-3 shadow-studio-dock max-xl:col-span-2 max-lg:col-span-1">
            <div
              aria-label="Command dock"
              className="mx-auto flex max-w-7xl flex-wrap items-center justify-between gap-3"
            >
              {commandDock}
            </div>
          </div>
        ) : null}
      </div>
    </div>
  );
}

export function StudioPanel({
  children,
  className = "",
}: {
  children: ReactNode;
  className?: string;
}) {
  return (
    <section className={`${studioUiClassNames.panel} ${className}`}>
      {children}
    </section>
  );
}

export function StudioButton({
  variant = "secondary",
  className = "",
  children,
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: "primary" | "secondary" | "icon";
}) {
  const variantClass =
    variant === "primary"
      ? studioUiClassNames.primaryButton
      : variant === "icon"
        ? studioUiClassNames.iconButton
        : studioUiClassNames.secondaryButton;
  return (
    <button type="button" className={`${variantClass} ${className}`} {...props}>
      {children}
    </button>
  );
}

export function StudioStatusChip({
  tone = "neutral",
  title,
  children,
}: {
  tone?: "neutral" | "action" | "health" | "acp" | "agent" | "danger";
  title?: string;
  children: ReactNode;
}) {
  const toneClass = {
    neutral: "border-graphite-700/15 bg-canvas-50 text-graphite-700/70",
    action: "border-amber-500/30 bg-amber-500/15 text-amber-600",
    health: "border-health-500/30 bg-health-500/15 text-health-500",
    acp: "border-acp-500/30 bg-acp-500/15 text-acp-500",
    agent: "border-agent-500/30 bg-agent-500/15 text-agent-500",
    danger: "border-signal/30 bg-signal/10 text-signal",
  }[tone];
  return (
    <span title={title} className={`${studioUiClassNames.chip} ${toneClass}`}>
      {children}
    </span>
  );
}

function StudioNavList({
  label,
  items,
  className,
  selectedTone,
}: {
  label: string;
  items: StudioNavItem[];
  className: string;
  selectedTone: "workflow" | "surface";
}) {
  return (
    <nav aria-label={label} className={`${className} grid gap-1`}>
      {items.map((item) => (
        <StudioNavButton
          key={item.id}
          item={item}
          selectedTone={selectedTone}
        />
      ))}
    </nav>
  );
}

function StudioNavButton({
  item,
  selectedTone,
}: {
  item: StudioNavItem;
  selectedTone: "workflow" | "surface";
}) {
  const Icon = item.icon;
  const selectedClass =
    selectedTone === "workflow"
      ? "border-amber-400/45 bg-amber-500 text-graphite-950"
      : "border-acp-400/35 bg-acp-500/20 text-canvas-50";
  return (
    <button
      type="button"
      aria-label={item.label}
      aria-pressed={item.selected}
      title={item.description}
      onClick={item.onSelect}
      className={[
        "flex min-h-12 items-center gap-3 rounded-md border px-3 py-2 text-left transition",
        item.selected
          ? selectedClass
          : "border-transparent text-canvas-50/75 hover:border-canvas-200/15 hover:bg-canvas-50/10 hover:text-canvas-50",
      ].join(" ")}
    >
      <Icon aria-hidden size={18} className="shrink-0" />
      <span className="min-w-0 flex-1">
        <span className="block truncate text-sm font-medium">{item.label}</span>
        <span
          className={[
            "block truncate text-xs",
            item.selected ? "opacity-70" : "text-canvas-200/45",
          ].join(" ")}
        >
          {item.sublabel}
        </span>
      </span>
      {item.selected ? <ChevronRight aria-hidden size={16} /> : null}
    </button>
  );
}

// ---------------------------------------------------------------------------
// ScenePreviewPlaceholder
// ---------------------------------------------------------------------------

/** Shown when no background asset is available for a scene preview area. */
export function ScenePreviewPlaceholder({
  assetPath,
}: {
  assetPath: string | null;
}) {
  return (
    <div className="absolute inset-0 grid place-items-center bg-[radial-gradient(circle_at_top,_rgba(229,181,95,0.20),_rgba(17,20,25,0.92)_55%)] px-4">
      <div className="max-w-md rounded-md border border-canvas-200/15 bg-graphite-950/70 px-4 py-3 text-center">
        <p className="text-sm font-semibold text-canvas-50">
          Scene preview asset unavailable
        </p>
        <p className="mt-1 break-words text-xs leading-5 text-canvas-200/55">
          {assetPath ?? "No background asset is declared for this scene."}
        </p>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Collapsible / CollapsibleSection
// ---------------------------------------------------------------------------

export interface CollapsibleProps {
  /** Header label shown in the toggle button. */
  label: string;
  /**
   * Optional stable identifier used to generate `aria-controls` / `id`
   * attributes.  When omitted, the label text is used (slugified).  Provide
   * this when multiple collapsibles in the same document share the same label
   * text (e.g. per-item character cards) to avoid duplicate DOM ids.
   */
  id?: string;
  /** Whether the section is open by default (uncontrolled). */
  defaultOpen?: boolean;
  /** Optional badge count shown to the right of the label. */
  badge?: number | string;
  children: ReactNode;
  className?: string;
}

/**
 * Lightweight collapsible container.  Can be used uncontrolled (via
 * `defaultOpen`) or wrapped in a parent that manages open state externally.
 * Uses a <button> with `aria-expanded` for full keyboard and screen-reader
 * accessibility.
 */
export function Collapsible({
  label,
  id: idProp,
  defaultOpen = false,
  badge,
  children,
  className = "",
}: CollapsibleProps) {
  const [open, setOpen] = useState(defaultOpen);
  const slug = (idProp ?? label).toLowerCase().replace(/\s+/g, "-");
  const headingId = `collapsible-heading-${slug}`;
  const regionId = `collapsible-region-${slug}`;
  return (
    <div className={className}>
      <button
        type="button"
        id={headingId}
        aria-expanded={open}
        aria-controls={regionId}
        onClick={() => setOpen((prev) => !prev)}
        className="flex w-full items-center gap-2 rounded-md px-1 py-1 text-left text-sm font-semibold text-ink transition hover:bg-canvas-100"
      >
        {open ? (
          <ChevronDown aria-hidden size={15} className="shrink-0 text-graphite-700/55" />
        ) : (
          <ChevronRight aria-hidden size={15} className="shrink-0 text-graphite-700/55" />
        )}
        <span className="flex-1">{label}</span>
        {badge !== undefined ? (
          <span className="rounded-sm border border-graphite-700/15 bg-canvas-100 px-1.5 py-0.5 text-xs font-semibold text-graphite-700/70">
            {badge}
          </span>
        ) : null}
      </button>
      {open ? (
        <div
          id={regionId}
          role="region"
          aria-labelledby={headingId}
        >
          {children}
        </div>
      ) : null}
    </div>
  );
}

export interface CollapsibleSectionProps {
  /** Section heading shown in the toggle button. */
  title: string;
  /** Whether the section is open by default (uncontrolled). */
  defaultOpen?: boolean;
  /** Optional badge count shown to the right of the title. */
  badge?: number | string;
  children: ReactNode;
  className?: string;
}

/**
 * A `StudioPanel`-wrapped collapsible section.  Use for secondary/advanced
 * areas that should be hidden by default to reduce visual complexity.
 */
export function CollapsibleSection({
  title,
  defaultOpen = false,
  badge,
  children,
  className = "",
}: CollapsibleSectionProps) {
  return (
    <StudioPanel className={className}>
      <Collapsible label={title} defaultOpen={defaultOpen} badge={badge}>
        <div className="mt-3">{children}</div>
      </Collapsible>
    </StudioPanel>
  );
}
