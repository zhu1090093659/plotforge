import {
  ChevronDown,
  ChevronRight,
  FolderOpen,
  Loader2,
  Menu,
  PanelRight,
  type LucideIcon,
} from "lucide-react";
import {
  type ButtonHTMLAttributes,
  type KeyboardEvent,
  type ReactNode,
  useEffect,
  useId,
  useState,
} from "react";
import { useStudioI18n } from "./i18n";

export const agentNativeDesignTokens = {
  shell: {
    // Light lavender-white chrome — the manuscript page, not the dark forge.
    graphite: "#faf8ff",
    graphitePanel: "#f3f0fb",
    warmCanvas: "#fdfbff",
  },
  accent: {
    violetAction: "#6f54a3",
    healthGreen: "#5a8a7a",
    accentCopper: "#7a5a95",
    agentSage: "#4d3e7a",
  },
} as const;

export const studioUiClassNames = {
  panel:
    "rounded-lg border border-canvas-200/70 bg-canvas-50 p-5 text-ink shadow-studio-panel",
  insetPanel: "rounded-lg border border-canvas-200/70 bg-canvas-100 px-3 py-3 text-ink",
  input:
    "h-10 min-w-0 rounded-md border border-canvas-200 bg-canvas-50 px-3 text-sm text-ink outline-none transition focus:border-accent-400 focus:ring-1 focus:ring-accent-400/30",
  textarea:
    "w-full resize-y rounded-md border border-canvas-200 bg-canvas-50 px-3 py-2 text-sm leading-6 text-ink outline-none transition focus:border-accent-400 focus:ring-1 focus:ring-accent-400/30",
  primaryButton:
    "inline-flex h-10 items-center gap-2 rounded-md bg-violet-500 px-4 text-sm font-semibold text-canvas-50 transition hover:bg-violet-400 disabled:cursor-not-allowed disabled:bg-violet-500/35 disabled:text-canvas-50/45",
  secondaryButton:
    "inline-flex h-9 items-center gap-2 rounded-md border border-canvas-200 bg-canvas-50 px-3 text-sm font-semibold text-ink transition hover:border-canvas-400 disabled:cursor-not-allowed disabled:text-ink/30",
  iconButton:
    "grid h-10 w-10 place-items-center rounded-md border border-canvas-200 bg-canvas-50 text-ink transition hover:border-violet-400 hover:bg-canvas-100",
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
  title: string;
  subtitle: string;
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
  railCollapsed: boolean;
  onToggleRail(): void;
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
  railCollapsed,
  onToggleRail,
  drawerOpen,
  onToggleDrawer,
  onCloseDrawer,
  children,
}: StudioShellProps) {
  const { t } = useStudioI18n();
  const sidebarContent = (
    <>
      <div className="flex items-center gap-3">
        <div className="grid h-10 w-10 place-items-center rounded-md border border-violet-400/55 bg-violet-500 font-display text-base font-black tracking-display text-canvas-50">
          {t("brand.logo")}
        </div>
        <div className="min-w-0">
          <p className="text-xs font-semibold uppercase tracking-tightish text-violet-600">
            {t("brand.studio")}
          </p>
          <h1 className="font-display truncate text-xl font-semibold tracking-display text-ink">
            {t("brand.creatorDesktop")}
          </h1>
        </div>
      </div>

      <div className="mt-6 flex items-center justify-between rounded-lg border border-canvas-200 bg-graphite-850 px-3 py-2">
        <div className="min-w-0">
          <p className="text-xs font-medium uppercase tracking-tightish text-graphite-700/70">
            {t("shell.openProject")}
          </p>
          <p className="max-w-44 truncate text-sm font-semibold text-ink">
            {projectPath}
          </p>
        </div>
        <button
          type="button"
          title={t("shell.openProject")}
          onClick={onOpenProject}
          className="grid h-9 w-9 place-items-center rounded-md border border-canvas-200 bg-canvas-50 text-ink transition hover:border-violet-400"
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
    <div className="min-h-screen bg-graphite-950 text-ink lg:h-screen lg:overflow-hidden">
      <div
        data-testid="studio-shell-grid"
        className={[
          "grid min-h-screen grid-cols-1 lg:h-screen lg:grid-cols-[240px_minmax(0,1fr)] lg:grid-rows-[minmax(0,1fr)_auto]",
          railCollapsed
            ? "xl:grid-cols-[260px_minmax(0,1fr)] 2xl:grid-cols-[280px_minmax(0,1fr)]"
            : "xl:grid-cols-[240px_minmax(0,1fr)_360px] 2xl:grid-cols-[260px_minmax(0,1fr)_400px]",
        ].join(" ")}
      >
        <aside
          aria-label={t("shell.studioNav")}
          className="hidden min-h-0 overflow-y-auto border-r border-canvas-200 bg-graphite-900 px-3 py-4 shadow-shell-inset lg:block"
        >
          {sidebarContent}
        </aside>

        <main className="paper-grain min-w-0 text-ink lg:flex lg:min-h-0 lg:flex-col lg:overflow-hidden">
          <header className="grid gap-3 border-b border-canvas-200 px-6 py-4 lg:grid-cols-[minmax(0,1fr)_minmax(420px,auto)] lg:items-end lg:px-6 lg:py-3 xl:px-8">
            <div className="flex min-w-0 items-center gap-3">
              <button
                type="button"
                aria-label={t("shell.openNav")}
                title={t("shell.openNav")}
                onClick={onToggleDrawer}
                className="grid h-10 w-10 shrink-0 place-items-center rounded-md border border-canvas-200 bg-canvas-50 text-ink transition hover:border-accent-400 lg:hidden"
              >
                <Menu aria-hidden size={18} />
              </button>
              <div className="min-w-0">
                <h2 className="font-display text-xl font-semibold tracking-display text-ink xl:text-2xl">
                  {header.title}
                </h2>
                <p className="mt-1 max-w-3xl truncate text-sm leading-5 text-graphite-700/75">
                  {header.subtitle}
                </p>
              </div>
            </div>
            <div className="flex w-full min-w-0 flex-wrap items-center gap-2 lg:w-auto lg:flex-nowrap lg:justify-end">
              {topActions}
              <button
                type="button"
                aria-label={
                  railCollapsed
                    ? t("shell.expandRail")
                    : t("shell.collapseRail")
                }
                aria-expanded={!railCollapsed}
                title={
                  railCollapsed
                    ? t("shell.expandRail")
                    : t("shell.collapseRail")
                }
                onClick={onToggleRail}
                className="grid h-10 w-10 shrink-0 place-items-center rounded-md border border-canvas-200 bg-canvas-50 text-ink transition hover:border-violet-400"
              >
                <PanelRight aria-hidden size={18} />
              </button>
            </div>
          </header>

          <div className="px-6 py-4 lg:min-h-0 lg:flex-1 lg:overflow-y-auto lg:px-6 xl:px-8">
            {children}
          </div>
        </main>

        {!railCollapsed ? (
          <aside
            aria-label={t("shell.agentRail")}
            className="min-h-0 overflow-hidden border-l border-canvas-200 bg-graphite-900 text-ink shadow-shell-inset lg:col-span-2 lg:max-h-64 lg:border-l-0 lg:border-t xl:col-span-1 xl:max-h-none xl:border-l xl:border-t-0"
          >
            {rightPanel}
          </aside>
        ) : null}
      </div>

      {drawerOpen ? (
        <div className="fixed inset-0 z-50 lg:hidden">
          <div
            aria-hidden
            data-testid="drawer-overlay"
            onClick={onCloseDrawer}
            className="absolute inset-0 bg-ink/30"
          />
          <aside
            aria-label={t("shell.studioNav")}
            className="absolute left-0 top-0 h-full w-80 max-w-[85vw] overflow-y-auto border-r border-canvas-200 bg-graphite-900 px-4 py-5 text-ink shadow-shell-inset"
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
    action: "border-violet-500/35 bg-violet-500/15 text-violet-600",
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
  const { t } = useStudioI18n();
  return (
    <nav aria-label={t("shell.studioNavTree")} className={`${className} grid gap-1`}>
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
  const selectedClass = "border-violet-400/55 bg-violet-500 text-canvas-50";
  return (
    <div className="relative">
      {item.selected ? (
        <span
          aria-hidden
          className="absolute -left-4 top-1 bottom-1 w-1 rounded-full bg-violet-500"
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
            : "border-transparent text-ink/75 hover:border-canvas-200 hover:bg-canvas-100 hover:text-ink",
        ].join(" ")}
      >
        <Icon aria-hidden size={18} className="shrink-0" />
        <span className="min-w-0 flex-1">
          <span className="block truncate text-sm font-medium">{item.label}</span>
          <span
            className={[
              "block truncate text-xs",
              item.selected ? "opacity-70" : "text-graphite-700/55",
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
    "border-accent-400/45 bg-accent-500/15 text-ink";
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
          : "border-transparent text-ink/75 hover:border-canvas-200 hover:bg-canvas-100 hover:text-ink",
      ].join(" ")}
    >
      <Icon aria-hidden size={16} className="shrink-0" />
      <span className="min-w-0 flex-1">
        <span className="block truncate text-sm font-medium">{item.label}</span>
        <span
          className={[
            "block truncate text-xs",
            item.selected ? "opacity-70" : "text-graphite-700/55",
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
  const { t } = useStudioI18n();
  return (
    <div className="absolute inset-0 grid place-items-center bg-[radial-gradient(circle_at_top,_rgba(138,111,184,0.12),_rgba(253,251,255,0.96)_55%)] px-4">
      <div className="max-w-md rounded-md border border-canvas-200 bg-canvas-50 px-4 py-3 text-center">
        <p className="font-display text-sm font-semibold text-ink">
          {t("shell.scenePreviewUnavailable")}
        </p>
        <p className="mt-1 break-words text-xs leading-5 text-graphite-700/65">
          {assetPath ?? t("shell.scenePreviewNoBackground")}
        </p>
      </div>
    </div>
  );
}

/**
 * Renders the scene background image when a resolvable asset URL is
 * available, falling back to {@link ScenePreviewPlaceholder} when the URL is
 * null or the underlying image fails to load (e.g. the asset file is
 * missing or outside the Tauri asset scope). This keeps the preview area
 * honest: it never shows a broken-image icon, only either the real image
 * or an explicit placeholder.
 */
export function ScenePreviewImage({
  src,
  assetPath,
  className,
}: {
  src: string | null;
  assetPath: string | null;
  className?: string;
}) {
  const [failed, setFailed] = useState(false);
  // Reset the failure flag whenever the src changes so a transient load
  // error (e.g. an asset briefly outside the Tauri asset scope) does not
  // permanently pin the placeholder for the rest of the session. Without
  // this, navigating to another scene with a valid asset would keep
  // showing the placeholder because the component instance persists.
  useEffect(() => {
    setFailed(false);
  }, [src]);
  if (src && !failed) {
    return (
      <img
        src={src}
        alt=""
        onError={() => setFailed(true)}
        className={className}
      />
    );
  }
  return <ScenePreviewPlaceholder assetPath={assetPath} />;
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

// ---------------------------------------------------------------------------
// StudioTabs — in-view tab strip for dense surfaces
// ---------------------------------------------------------------------------

export interface StudioTabItem {
  id: string;
  label: string;
  badge?: number | string;
  children: ReactNode;
}

export interface StudioTabsProps {
  /** Stable aria-label for the tablist. */
  ariaLabel: string;
  /** Tab items; the first item is selected by default. */
  items: StudioTabItem[];
  /** Optional initial selected tab id (uncontrolled; defaults to first). */
  defaultId?: string;
  /**
   * Optional controlled active tab id. When provided, the component becomes
   * controlled and `onActiveChange` is required to update it. Use this when
   * the parent needs to programmatically switch tabs (e.g. auto-reveal an
   * editor tab after a file is selected).
   */
  activeId?: string;
  /** Callback invoked when the user requests a tab change (click or keyboard). */
  onActiveChange?(id: string): void;
  /** Optional className on the wrapper. */
  className?: string;
}

/**
 * Accessible in-view tab strip. Uses ARIA tablist / tab / tabpanel roles so
 * screen readers announce the tab relationship. Supports the WAI-ARIA tabs
 * keyboard pattern: Arrow Left/Right move between tabs, Home/End jump to
 * first/last. Only the active panel is mounted; inactive panels unmount to
 * keep the DOM focused and avoid hidden form-label collisions across dense
 * surfaces (e.g. ExportView).
 *
 * If the currently-active tab disappears from `items` (e.g. a conditionally-
 * included tab is removed), the effective active id falls back to the first
 * item so the workspace never renders an empty panel.
 */
export function StudioTabs({
  ariaLabel,
  items,
  defaultId,
  activeId: controlledId,
  onActiveChange,
  className = "",
}: StudioTabsProps) {
  const baseId = useId();
  const [internalId, setInternalId] = useState(
    defaultId ?? items[0]?.id ?? "",
  );
  const isControlled = controlledId !== undefined;
  const requestedId = isControlled ? controlledId : internalId;
  // Reconcile: if the requested id is no longer in items, fall back to first.
  const effectiveId = items.some((item) => item.id === requestedId)
    ? requestedId
    : (items[0]?.id ?? "");

  function selectTab(id: string) {
    if (!isControlled) {
      setInternalId(id);
    }
    onActiveChange?.(id);
  }

  function handleKeyDown(event: KeyboardEvent) {
    const count = items.length;
    if (count === 0) return;
    const currentIndex = items.findIndex((item) => item.id === effectiveId);
    let nextIndex = currentIndex;
    switch (event.key) {
      case "ArrowRight":
      case "ArrowDown":
        nextIndex = (currentIndex + 1) % count;
        break;
      case "ArrowLeft":
      case "ArrowUp":
        nextIndex = (currentIndex - 1 + count) % count;
        break;
      case "Home":
        nextIndex = 0;
        break;
      case "End":
        nextIndex = count - 1;
        break;
      default:
        return;
    }
    event.preventDefault();
    const nextId = items[nextIndex]?.id;
    if (nextId && nextId !== effectiveId) {
      selectTab(nextId);
      // Move focus to the newly-selected tab button.
      const tabId = `${baseId}-tab-${nextId}`;
      document.getElementById(tabId)?.focus();
    }
  }

  const listId = `${baseId}-tablist`;
  return (
    <div className={className}>
      <div
        role="tablist"
        aria-label={ariaLabel}
        id={listId}
        onKeyDown={handleKeyDown}
        className="flex flex-wrap gap-1 border-b border-canvas-200/55"
      >
        {items.map((item) => {
          const selected = item.id === effectiveId;
          const tabId = `${baseId}-tab-${item.id}`;
          const panelId = `${baseId}-panel-${item.id}`;
          return (
            <button
              key={item.id}
              type="button"
              role="tab"
              id={tabId}
              aria-selected={selected}
              aria-controls={panelId}
              tabIndex={selected ? 0 : -1}
              onClick={() => selectTab(item.id)}
              className={[
                "inline-flex h-9 items-center gap-2 border-b-2 px-3 text-sm font-semibold transition",
                selected
                  ? "border-violet-500 text-ink"
                  : "border-transparent text-graphite-700/60 hover:border-canvas-200 hover:text-ink",
              ].join(" ")}
            >
              <span>{item.label}</span>
              {item.badge !== undefined ? (
                <span
                  className={[
                    "rounded-sm border px-1.5 py-0.5 text-xs font-semibold",
                    selected
                      ? "border-violet-500/35 bg-violet-500/15 text-violet-600"
                      : "border-canvas-200/55 bg-canvas-100 text-graphite-700/70",
                  ].join(" ")}
                >
                  {item.badge}
                </span>
              ) : null}
            </button>
          );
        })}
      </div>
      {items.map((item) => {
        const selected = item.id === effectiveId;
        if (!selected) return null;
        const panelId = `${baseId}-panel-${item.id}`;
        const tabId = `${baseId}-tab-${item.id}`;
        return (
          <div
            key={item.id}
            role="tabpanel"
            id={panelId}
            aria-labelledby={tabId}
            className="mt-4"
          >
            {item.children}
          </div>
        );
      })}
    </div>
  );
}
