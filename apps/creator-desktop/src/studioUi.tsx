import {
  ChevronDown,
  ChevronRight,
  FolderOpen,
  Loader2,
  Menu,
  type LucideIcon,
} from "lucide-react";
import { type ButtonHTMLAttributes, type ReactNode, useState } from "react";

export const agentNativeDesignTokens = {
  shell: {
    graphite: "#1f1a14",
    graphitePanel: "#271f18",
    warmCanvas: "#f4ead4",
  },
  accent: {
    amberAction: "#c98b2f",
    healthGreen: "#34815f",
    accentCopper: "#a85c34",
    agentBrass: "#2d6258",
  },
} as const;

export const studioUiClassNames = {
  panel:
    "rounded-lg border border-canvas-200/60 bg-canvas-50 p-5 text-ink shadow-studio-panel",
  insetPanel: "rounded-lg border border-canvas-200/55 bg-canvas-100 px-3 py-3 text-ink",
  input:
    "h-10 min-w-0 rounded-md border border-canvas-200/70 bg-canvas-50 px-3 text-sm text-ink outline-none transition focus:border-accent-400 focus:ring-1 focus:ring-accent-400/30",
  textarea:
    "w-full resize-y rounded-md border border-canvas-200/70 bg-canvas-50 px-3 py-2 text-sm leading-6 text-ink outline-none transition focus:border-accent-400 focus:ring-1 focus:ring-accent-400/30",
  primaryButton:
    "inline-flex h-10 items-center gap-2 rounded-md bg-amber-500 px-4 text-sm font-semibold text-graphite-950 transition hover:bg-amber-400 disabled:cursor-not-allowed disabled:bg-amber-500/35 disabled:text-graphite-950/45",
  secondaryButton:
    "inline-flex h-9 items-center gap-2 rounded-md border border-canvas-200/70 bg-canvas-50 px-3 text-sm font-semibold text-ink transition hover:border-canvas-200 disabled:cursor-not-allowed disabled:text-ink/30",
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
  /** Nested surface items rendered in-place when this item is expanded. */
  children?: StudioNavItem[];
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
  navItems: StudioNavItem[];
  expandedIds: Set<string>;
  onToggleExpand(id: string): void;
  header: StudioShellHeader;
  topActions: ReactNode;
  rightPanel: ReactNode;
  commandDock?: ReactNode;
  drawerOpen: boolean;
  onToggleDrawer(): void;
  onCloseDrawer(): void;
  children: ReactNode;
}

export function StudioShell({
  projectPath,
  projectLoading,
  onOpenProject,
  navItems,
  expandedIds,
  onToggleExpand,
  header,
  topActions,
  rightPanel,
  commandDock,
  drawerOpen,
  onToggleDrawer,
  onCloseDrawer,
  children,
}: StudioShellProps) {
  const sidebarContent = (
    <>
      <div className="flex items-center gap-3">
        <div className="grid h-10 w-10 place-items-center rounded-md border border-amber-400/55 bg-amber-500 font-display text-base font-black tracking-display text-graphite-950">
          PF
        </div>
        <div className="min-w-0">
          <p className="text-xs font-semibold uppercase tracking-tightish text-amber-400">
            PlotForge Studio
          </p>
          <h1 className="font-display truncate text-xl font-semibold tracking-display text-canvas-50">
            Creator Desktop
          </h1>
        </div>
      </div>

      <div className="mt-6 flex items-center justify-between rounded-lg border border-canvas-200/12 bg-graphite-850 px-3 py-2">
        <div className="min-w-0">
          <p className="text-xs font-medium uppercase tracking-tightish text-canvas-200/60">
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
          className="grid h-9 w-9 place-items-center rounded-md border border-canvas-200/18 bg-canvas-50/10 text-canvas-50 transition hover:border-amber-400/70"
        >
          {projectLoading ? (
            <Loader2 aria-hidden size={18} className="animate-spin" />
          ) : (
            <FolderOpen aria-hidden size={18} />
          )}
        </button>
      </div>

      <StudioNavTree
        items={navItems}
        expandedIds={expandedIds}
        onToggleExpand={onToggleExpand}
        className="mt-5"
      />
    </>
  );

  return (
    <div className="min-h-screen bg-graphite-950 text-canvas-50">
      <div
        data-testid="studio-shell-grid"
        className="grid min-h-screen grid-cols-1 lg:grid-cols-[260px_minmax(0,1fr)] xl:grid-cols-[280px_minmax(0,1fr)_320px]"
      >
        <aside
          aria-label="Studio navigation"
          className="hidden border-r border-canvas-200/12 bg-graphite-900 px-4 py-5 shadow-shell-inset lg:block"
        >
          {sidebarContent}
        </aside>

        <main className="paper-grain min-w-0 text-ink">
          <header className="flex flex-wrap items-center justify-between gap-4 border-b border-canvas-200/55 px-6 py-5 lg:px-8">
            <div className="flex min-w-0 items-center gap-3">
              <button
                type="button"
                aria-label="Open navigation"
                title="Open navigation"
                onClick={onToggleDrawer}
                className="grid h-10 w-10 shrink-0 place-items-center rounded-md border border-canvas-200/70 bg-canvas-50 text-ink transition hover:border-accent-400 lg:hidden"
              >
                <Menu aria-hidden size={18} />
              </button>
              <div className="min-w-0">
                <p className="text-xs font-semibold uppercase tracking-tightish text-graphite-700/65">
                  {header.eyebrow}
                </p>
                <h2 className="font-display mt-1 text-2xl font-semibold tracking-display text-ink">
                  {header.title}
                </h2>
                <p className="mt-1 max-w-3xl text-sm leading-6 text-graphite-700/75">
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
            </div>
            <div className="flex min-w-0 flex-wrap items-center gap-2">
              {topActions}
            </div>
          </header>

          <div className="px-6 py-5 lg:px-8">{children}</div>
        </main>

        <aside
          aria-label="Evidence panel"
          className="border-l border-canvas-200/12 bg-graphite-900 px-4 py-5 text-canvas-50 shadow-shell-inset lg:col-span-2 lg:border-l-0 lg:border-t xl:col-span-1 xl:border-l xl:border-t-0"
        >
          {rightPanel}
        </aside>

        {commandDock ? (
          <div className="border-t border-canvas-200/12 bg-graphite-950 px-4 py-3 shadow-studio-dock lg:col-span-2 xl:col-span-3">
            <div
              aria-label="Command dock"
              className="mx-auto flex max-w-7xl flex-wrap items-center justify-between gap-3"
            >
              {commandDock}
            </div>
          </div>
        ) : null}
      </div>

      {drawerOpen ? (
        <div className="fixed inset-0 z-50 lg:hidden">
          <div
            aria-hidden
            data-testid="drawer-overlay"
            onClick={onCloseDrawer}
            className="absolute inset-0 bg-black/50"
          />
          <aside
            aria-label="Studio navigation"
            className="absolute left-0 top-0 h-full w-80 max-w-[85vw] overflow-y-auto border-r border-canvas-200/12 bg-graphite-900 px-4 py-5 text-canvas-50 shadow-shell-inset"
          >
            {sidebarContent}
          </aside>
        </div>
      ) : null}
    </div>
  );
}


export function EmptyPanel({ label }: { label: string }) {
  return (
    <div className="flex min-h-[8rem] items-center justify-center rounded-md border border-dashed border-ink/20 bg-ink/[0.025] p-6 text-center text-sm text-ink/55">
      {label}
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
  tone?: "neutral" | "action" | "health" | "accent" | "agent" | "danger";
  title?: string;
  children: ReactNode;
}) {
  const toneClass = {
    neutral: "border-canvas-200/40 bg-canvas-100/60 text-graphite-700/80",
    action: "border-amber-500/35 bg-amber-500/15 text-amber-600",
    health: "border-health-500/35 bg-health-500/15 text-health-500",
    accent: "border-accent-500/35 bg-accent-500/15 text-accent-500",
    agent: "border-agent-500/35 bg-agent-500/15 text-agent-400",
    danger: "border-signal/35 bg-signal/12 text-signal",
  }[tone];
  return (
    <span title={title} className={`${studioUiClassNames.chip} ${toneClass}`}>
      {children}
    </span>
  );
}

function StudioNavTree({
  items,
  expandedIds,
  onToggleExpand,
  className,
}: {
  items: StudioNavItem[];
  expandedIds: Set<string>;
  onToggleExpand(id: string): void;
  className: string;
}) {
  return (
    <nav aria-label="Studio navigation tree" className={`${className} grid gap-1`}>
      {items.map((item) => (
        <StudioNavTreeNode
          key={item.id}
          item={item}
          expanded={expandedIds.has(item.id)}
          onToggleExpand={onToggleExpand}
        />
      ))}
    </nav>
  );
}

function StudioNavTreeNode({
  item,
  expanded,
  onToggleExpand,
}: {
  item: StudioNavItem;
  expanded: boolean;
  onToggleExpand(id: string): void;
}) {
  const Icon = item.icon;
  const hasChildren = (item.children?.length ?? 0) > 0;
  const selectedClass = "border-amber-400/55 bg-amber-500 text-graphite-950";
  return (
    <div className="relative">
      {item.selected ? (
        <span
          aria-hidden
          className="absolute -left-4 top-1 bottom-1 w-1 rounded-full bg-amber-400"
        />
      ) : null}
      <button
        type="button"
        aria-label={item.label}
        aria-pressed={item.selected}
        aria-expanded={hasChildren ? expanded : undefined}
        aria-controls={hasChildren ? `nav-children-${item.id}` : undefined}
        title={item.description}
        onClick={() => {
          if (hasChildren) {
            onToggleExpand(item.id);
          } else {
            item.onSelect();
          }
        }}
        className={[
          "flex min-h-12 w-full items-center gap-3 rounded-md border px-3 py-2 text-left transition",
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
        {hasChildren ? (
          expanded ? (
            <ChevronDown aria-hidden size={16} className="shrink-0 opacity-60" />
          ) : (
            <ChevronRight aria-hidden size={16} className="shrink-0 opacity-60" />
          )
        ) : null}
      </button>
      {hasChildren && expanded ? (
        <ul
          id={`nav-children-${item.id}`}
          className="mt-1 grid gap-1 pl-9"
        >
          {item.children!.map((child) => (
            <li key={child.id}>
              <StudioNavSurfaceButton item={child} />
            </li>
          ))}
        </ul>
      ) : null}
    </div>
  );
}

function StudioNavSurfaceButton({ item }: { item: StudioNavItem }) {
  const Icon = item.icon;
  const selectedClass =
    "border-accent-400/45 bg-accent-500/20 text-canvas-50";
  return (
    <button
      type="button"
      aria-label={item.label}
      aria-pressed={item.selected}
      title={item.description}
      onClick={item.onSelect}
      className={[
        "flex min-h-9 w-full items-center gap-3 rounded-md border px-3 py-2 text-left transition",
        item.selected
          ? selectedClass
          : "border-transparent text-canvas-50/75 hover:border-canvas-200/15 hover:bg-canvas-50/10 hover:text-canvas-50",
      ].join(" ")}
    >
      <Icon aria-hidden size={16} className="shrink-0" />
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
    <div className="absolute inset-0 grid place-items-center bg-[radial-gradient(circle_at_top,_rgba(214,160,80,0.18),_rgba(31,26,20,0.92)_55%)] px-4">
      <div className="max-w-md rounded-md border border-canvas-200/15 bg-graphite-950/70 px-4 py-3 text-center">
        <p className="font-display text-sm font-semibold text-canvas-50">
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
          <ChevronDown aria-hidden size={15} className="shrink-0 text-graphite-700/60" />
        ) : (
          <ChevronRight aria-hidden size={15} className="shrink-0 text-graphite-700/60" />
        )}
        <span className="flex-1">{label}</span>
        {badge !== undefined ? (
          <span className="rounded-sm border border-canvas-200/55 bg-canvas-100 px-1.5 py-0.5 text-xs font-semibold text-graphite-700/75">
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
