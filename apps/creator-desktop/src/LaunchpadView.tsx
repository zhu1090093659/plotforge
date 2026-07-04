import {
  ChevronDown,
  Folder,
  GitBranch,
  Loader2,
  Plus,
  Send,
  ShieldCheck,
  Sparkles,
} from "lucide-react";
import { useEffect, useRef, useState, type FormEvent } from "react";
import type {
  AgentSessionConfig,
  GitBranchInfo,
  ModelOption,
  PermissionLevel,
  ThinkingLevel,
} from "../../../contracts/plotforge";
import { useStudioI18n } from "./i18n";

// ---------------------------------------------------------------------------
// LaunchpadView — the full-screen, centered "conversation entry" home page.
//
// This replaces the old project-overview + new-project-form + boundary-checks
// triptych. The home page is now a single agent-native conversation entry:
// a brand mark, a time-based localized greeting, and a centered chat card
// whose top row shows the project directory name + git branch and whose
// bottom toolbar carries model / permission / thinking selectors + send.
// All selectors are functional — they drive persisted AgentSessionConfig —
// but real provider routing enforcement stays in the Rust backend
// (plotforge-studio / plotforge-agent).
// ---------------------------------------------------------------------------

export interface LaunchpadViewProps {
  /** Loaded project filesystem path (empty when no project is loaded). */
  loadedPath: string;
  /** Directory basename of the loaded project, or empty when none. */
  projectDirName: string;
  /** Current git branch name, or null when the path is not a git repo. */
  currentBranch: string | null;
  /** Local branch list for the dropdown. */
  branches: GitBranchInfo[];
  /** True while a branch switch is in flight. */
  switchingBranch: boolean;
  /** Last branch-switch error, or null when the last switch succeeded. */
  switchError: string | null;
  /** Switch to a local branch; backend performs `git checkout`. */
  onSwitchBranch(branch: string): void;
  /** Available model options for the selector. */
  availableModels: ModelOption[];
  /** Persisted agent session config (model / permission / thinking). */
  agentConfig: AgentSessionConfig;
  /** Persist a new agent session config (called on every selector change). */
  onAgentConfigChange(config: AgentSessionConfig): void;
  /** Last agent-config persist error, or null when the last save succeeded. */
  configSaveError: string | null;
  /** Conversation input state (shared with the playtest/agent pipeline). */
  input: string;
  onInputChange(value: string): void;
  /** Submit the current input as a director-intent turn. */
  onSubmit(): void;
  /** Whether a turn is currently running. */
  running: boolean;
  canSubmit: boolean;
}

export function LaunchpadView({
  loadedPath,
  projectDirName,
  currentBranch,
  branches,
  switchingBranch,
  switchError,
  onSwitchBranch,
  availableModels,
  agentConfig,
  onAgentConfigChange,
  configSaveError,
  input,
  onInputChange,
  onSubmit,
  running,
  canSubmit,
}: LaunchpadViewProps) {
  const { t } = useStudioI18n();
  const greetingKey = pickGreetingKey(new Date());

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!canSubmit) return;
    onSubmit();
  }

  return (
    <div className="relative flex min-h-100dvh flex-col items-center justify-center bg-parchment px-4 py-10">
      <div className="paper-grain pointer-events-none absolute inset-0 opacity-60" aria-hidden />

      <div className="relative grid w-full max-w-3xl place-items-center gap-8">
        {/* Brand mark */}
        <PlotForgeBrandMark />

        {/* Greeting */}
        <div className="grid place-items-center gap-1.5 text-center">
          <p
            aria-label={t("home.aria.greeting")}
            className="font-display text-2xl font-semibold tracking-display-tight text-ink sm:text-3xl"
          >
            {t(greetingKey)}
          </p>
        </div>

        {/* Conversation entry card */}
        <form
          onSubmit={handleSubmit}
          className="relative grid w-full gap-3 rounded-2xl border border-canvas-200/70 bg-canvas-50/90 p-4 shadow-studio-panel backdrop-blur-sm"
        >
          {/* Top row: project dir + git branch */}
          <div className="flex flex-wrap items-center gap-2">
            <ProjectDirChip
              dirName={projectDirName}
              loadedPath={loadedPath}
            />
            <GitBranchChip
              currentBranch={currentBranch}
              branches={branches}
              switching={switchingBranch}
              onSwitch={onSwitchBranch}
            />
          </div>

          {/* Visible error surface for git switch + agent-config persist failures.
              These are never silent — a failure to switch branches or persist
              the agent config leaves the user on the old state, so the error
              must be shown until the next successful operation. */}
          {(switchError || configSaveError) && (
            <div
              role="alert"
              aria-label={t("home.aria.errorRow")}
              className="grid gap-1 rounded-xl border border-signal/30 bg-signal/10 px-3 py-2 text-sm text-signal"
            >
              {switchError && (
                <p aria-label={t("home.aria.switchError")}>
                  {t("home.branchSwitchError")}: {switchError}
                </p>
              )}
              {configSaveError && (
                <p aria-label={t("home.aria.configSaveError")}>
                  {t("home.configSaveError")}: {configSaveError}
                </p>
              )}
            </div>
          )}

          {/* Input */}
          <textarea
            aria-label={t("home.placeholder")}
            value={input}
            onChange={(event) => onInputChange(event.target.value)}
            placeholder={t("home.placeholder")}
            rows={4}
            spellCheck={false}
            className="w-full resize-none rounded-xl border border-canvas-200/70 bg-canvas-50 px-4 py-3 text-base leading-7 text-ink outline-none transition ease-expo focus:border-accent-400 focus:ring-1 focus:ring-accent-400/30"
          />

          {/* Bottom toolbar */}
          <div className="flex flex-wrap items-center justify-between gap-2">
            <div className="flex flex-wrap items-center gap-1.5">
              <button
                type="button"
                aria-label={t("home.placeholder")}
                className="grid h-9 w-9 place-items-center rounded-lg border border-canvas-200/70 bg-canvas-50 text-ink/70 transition ease-expo hover:border-copper-500 hover:bg-canvas-100 active:translate-y-px"
              >
                <Plus aria-hidden size={18} />
              </button>
              <PermissionSelect
                value={agentConfig.permission_level}
                onChange={(permission_level) =>
                  onAgentConfigChange({ ...agentConfig, permission_level })
                }
              />
              <ModelSelect
                models={availableModels}
                value={agentConfig.model_id}
                onChange={(model_id) =>
                  onAgentConfigChange({ ...agentConfig, model_id })
                }
              />
              <ThinkingSelect
                value={agentConfig.thinking_level}
                onChange={(thinking_level) =>
                  onAgentConfigChange({ ...agentConfig, thinking_level })
                }
              />
            </div>
            <button
              type="submit"
              disabled={!canSubmit}
              aria-label={t("home.aria.send")}
              className="grid h-10 w-10 place-items-center rounded-full bg-ink text-canvas-50 shadow-[inset_0_-1px_0_rgba(138,100,80,0.35)] transition ease-expo hover:bg-ink/90 active:translate-y-px disabled:cursor-not-allowed disabled:bg-ink/30"
            >
              {running ? (
                <Loader2 aria-hidden size={18} className="animate-spin" />
              ) : (
                <Send aria-hidden size={18} />
              )}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Greeting time-of-day picker
// ---------------------------------------------------------------------------

function pickGreetingKey(date: Date): string {
  const hour = date.getHours();
  if (hour >= 5 && hour < 12) return "home.greeting.morning";
  if (hour >= 12 && hour < 18) return "home.greeting.afternoon";
  if (hour >= 18 && hour < 23) return "home.greeting.evening";
  return "home.greeting.night";
}

// ---------------------------------------------------------------------------
// Brand mark — the abstract PlotForge "Z" shape
// ---------------------------------------------------------------------------

function PlotForgeBrandMark() {
  return (
    <div
      className="grid h-16 w-16 place-items-center rounded-2xl bg-ink text-canvas-50 shadow-studio-panel"
      aria-hidden
    >
      <Sparkles size={32} strokeWidth={1.5} />
    </div>
  );
}

// ---------------------------------------------------------------------------
// Project directory chip
// ---------------------------------------------------------------------------

function ProjectDirChip({
  dirName,
  loadedPath,
}: {
  dirName: string;
  loadedPath: string;
}) {
  const { t } = useStudioI18n();
  const label = dirName || (loadedPath ? t("home.projectDirFallback") : t("home.noProject"));
  return (
    <span
      aria-label={t("home.aria.projectDir")}
      className="inline-flex h-9 items-center gap-1.5 rounded-lg border border-canvas-200/70 bg-canvas-50 px-2.5 text-sm font-medium text-ink/80"
    >
      <Folder aria-hidden size={15} className="text-copper-500" />
      <span className="max-w-[14rem] truncate">{label}</span>
    </span>
  );
}

// ---------------------------------------------------------------------------
// Git branch chip with dropdown
// ---------------------------------------------------------------------------

function GitBranchChip({
  currentBranch,
  branches,
  switching,
  onSwitch,
}: {
  currentBranch: string | null;
  branches: GitBranchInfo[];
  switching: boolean;
  onSwitch(branch: string): void;
}) {
  const { t } = useStudioI18n();
  const [open, setOpen] = useState(false);
  const toggleRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        event.preventDefault();
        setOpen(false);
      }
    }
    function handlePointerDown(event: MouseEvent) {
      const target = event.target as Node | null;
      if (!target) return;
      if (menuRef.current?.contains(target)) return;
      if (toggleRef.current?.contains(target)) return;
      setOpen(false);
    }
    window.addEventListener("keydown", handleKeyDown);
    window.addEventListener("mousedown", handlePointerDown);
    return () => {
      window.removeEventListener("keydown", handleKeyDown);
      window.removeEventListener("mousedown", handlePointerDown);
    };
  }, [open]);

  const display =
    currentBranch === null
      ? t("home.branchUnknown")
      : (switching ? t("home.branchLoading") : currentBranch);

  return (
    <div className="relative">
      <button
        ref={toggleRef}
        type="button"
        aria-label={t("home.aria.gitBranch")}
        aria-expanded={open}
        aria-haspopup="menu"
        disabled={switching || branches.length === 0}
        onClick={() => setOpen((prev) => !prev)}
        className="inline-flex h-9 items-center gap-1.5 rounded-lg border border-canvas-200/70 bg-canvas-50 px-2.5 text-sm font-medium text-ink/80 transition ease-expo hover:border-copper-500 hover:bg-canvas-100 active:translate-y-px disabled:cursor-not-allowed disabled:opacity-50"
      >
        <GitBranch aria-hidden size={15} className="text-copper-500" />
        {switching ? (
          <Loader2 aria-hidden size={13} className="animate-spin" />
        ) : null}
        <span className="max-w-[12rem] truncate">{display}</span>
        <ChevronDown aria-hidden size={14} className="text-ink/45" />
      </button>
      {open && branches.length > 0 ? (
        <div
          ref={menuRef}
          role="menu"
          className="absolute left-0 top-full z-50 mt-1 max-h-64 w-56 overflow-y-auto rounded-lg border border-canvas-200 bg-canvas-50 py-1 shadow-studio-panel"
        >
          {branches.map((branch) => (
            <button
              key={branch.name}
              type="button"
              role="menuitemradio"
              aria-checked={branch.is_current}
              onClick={() => {
                if (!branch.is_current) onSwitch(branch.name);
                setOpen(false);
              }}
              className="flex w-full items-center justify-between gap-2 px-3 py-1.5 text-left text-sm text-ink transition ease-expo hover:bg-canvas-100"
            >
              <span className="truncate">{branch.name}</span>
              {branch.is_current ? (
                <span className="text-xs font-semibold text-sage">✓</span>
              ) : null}
            </button>
          ))}
        </div>
      ) : null}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Selectors: model / permission / thinking
// ---------------------------------------------------------------------------

function ModelSelect({
  models,
  value,
  onChange,
}: {
  models: ModelOption[];
  value: string;
  onChange(value: string): void;
}) {
  const { t } = useStudioI18n();
  return (
    <label className="inline-flex h-9 items-center gap-1.5 rounded-lg border border-canvas-200/70 bg-canvas-50 pl-2.5 pr-1.5 text-sm text-ink/80">
      <span className="sr-only">{t("home.aria.modelSelect")}</span>
      <select
        aria-label={t("home.aria.modelSelect")}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        className="cursor-pointer bg-transparent text-sm font-medium outline-none"
      >
        {models.map((model) => (
          <option key={model.id} value={model.id}>
            {model.label}
          </option>
        ))}
      </select>
      <ChevronDown aria-hidden size={14} className="text-ink/45" />
    </label>
  );
}

function PermissionSelect({
  value,
  onChange,
}: {
  value: PermissionLevel;
  onChange(value: PermissionLevel): void;
}) {
  const { t } = useStudioI18n();
  const labels: Record<PermissionLevel, string> = {
    full_access: t("home.permissionFull"),
    ask_every_time: t("home.permissionAsk"),
    read_only: t("home.permissionReadOnly"),
  };
  return (
    <label className="inline-flex h-9 items-center gap-1.5 rounded-lg border border-canvas-200/70 bg-canvas-50 pl-2.5 pr-1.5 text-sm text-ink/80">
      <ShieldCheck aria-hidden size={15} className="text-copper-500" />
      <span className="sr-only">{t("home.aria.permissionSelect")}</span>
      <select
        aria-label={t("home.aria.permissionSelect")}
        value={value}
        onChange={(event) => onChange(event.target.value as PermissionLevel)}
        className="cursor-pointer bg-transparent text-sm font-medium outline-none"
      >
        {(Object.keys(labels) as PermissionLevel[]).map((level) => (
          <option key={level} value={level}>
            {labels[level]}
          </option>
        ))}
      </select>
      <ChevronDown aria-hidden size={14} className="text-ink/45" />
    </label>
  );
}

function ThinkingSelect({
  value,
  onChange,
}: {
  value: ThinkingLevel;
  onChange(value: ThinkingLevel): void;
}) {
  const { t } = useStudioI18n();
  const labels: Record<ThinkingLevel, string> = {
    high: t("home.thinkingHigh"),
    medium: t("home.thinkingMedium"),
    low: t("home.thinkingLow"),
    off: t("home.thinkingOff"),
  };
  return (
    <label className="inline-flex h-9 items-center gap-1.5 rounded-lg border border-canvas-200/70 bg-canvas-50 pl-2.5 pr-1.5 text-sm text-ink/80">
      <span className="sr-only">{t("home.aria.thinkingSelect")}</span>
      <select
        aria-label={t("home.aria.thinkingSelect")}
        value={value}
        onChange={(event) => onChange(event.target.value as ThinkingLevel)}
        className="cursor-pointer bg-transparent text-sm font-medium outline-none"
      >
        {(Object.keys(labels) as ThinkingLevel[]).map((level) => (
          <option key={level} value={level}>
            {labels[level]}
          </option>
        ))}
      </select>
      <ChevronDown aria-hidden size={14} className="text-ink/45" />
    </label>
  );
}
