import {
  ChevronDown,
  Folder,
  GitBranch,
  Loader2,
  Send,
} from "lucide-react";
import { useEffect, useRef, useState, type FormEvent } from "react";
import type {
  AgentSessionConfig,
  GitBranchInfo,
  ModelOption,
} from "../../../contracts/plotforge";
import {
  ModelThinkingDeck,
  PermissionSelect,
} from "./agentConfigSelectors";
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
  /** Project picker/load failure shown without hiding a successfully opened project. */
  projectLoadError: string | null;
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
  projectLoadError,
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
    // Home renders inside StudioShell (A1): the shell already paints the
    // `paper-grain` background and the sidebar brand box, so this surface no
    // longer carries its own full-screen wrapper, grain layer, or brand
    // mark. We center the conversation card within the shell's main area
    // instead of the viewport.
    <div className="grid min-h-[60dvh] place-items-center px-2 py-10">
      <div className="grid w-full max-w-3xl place-items-center gap-8">
        {/* Greeting — a time-of-day display line with a quiet copper eyebrow
            above it. The eyebrow anchors the greeting to the brand voice
            ("Studio · Conversation entry") without restating the greeting. */}
        <div className="grid place-items-center gap-1.5 text-center">
          <p className="eyebrow eyebrow--copper">{t("home.eyebrow")}</p>
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
          {(projectLoadError || switchError || configSaveError) && (
            <div
              role="alert"
              aria-label={t("home.aria.errorRow")}
              className="grid gap-1 rounded-xl border border-signal/30 bg-signal/10 px-3 py-2 text-sm text-signal"
            >
              {projectLoadError && (
                <p aria-label={t("home.aria.projectLoadError")}>
                  {t("home.projectLoadError")}: {projectLoadError}
                </p>
              )}
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

          {/* Input — a deliberate compose surface, not a search box. `rows={3}`
              keeps it inviting for a single director-intent turn; `min-h-[5rem]`
              guarantees a touch of breathing room even when the user types
              past the row count, while the tighter vertical padding keeps it
              reading as a focused one-line entry rather than an essay box. */}
          <textarea
            aria-label={t("home.placeholder")}
            value={input}
            onChange={(event) => onInputChange(event.target.value)}
            placeholder={t("home.placeholder")}
            rows={3}
            spellCheck={false}
            className="min-h-[5rem] w-full resize-none rounded-xl border border-canvas-200/70 bg-canvas-50 px-4 py-2.5 text-base leading-6 text-ink outline-none transition ease-expo placeholder:text-ink/35 focus:border-accent-400 focus:ring-1 focus:ring-accent-400/30"
          />

          {/* Bottom toolbar — a hairline separates it from the compose area so
              it reads as a distinct dock (selectors + send), not a continuation
              of the textarea. */}
          <div className="flex flex-wrap items-center justify-between gap-2 border-t border-canvas-200/55 pt-3">
            <div className="flex flex-wrap items-center gap-1.5">
              <PermissionSelect
                value={agentConfig.permission_level}
                onChange={(permission_level) =>
                  onAgentConfigChange({ ...agentConfig, permission_level })
                }
              />
              <ModelThinkingDeck
                models={availableModels}
                modelId={agentConfig.model_id}
                thinkingLevel={agentConfig.thinking_level}
                onChange={({ modelId, thinkingLevel }) =>
                  onAgentConfigChange({
                    ...agentConfig,
                    model_id: modelId,
                    thinking_level: thinkingLevel,
                  })
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
// Selectors: model / permission / thinking — moved to ./agentConfigSelectors
// (shared with AgentConfigSection so the same AgentSessionConfig renders with
// one UI in both the home toolbar and the Settings → Agent tab).
// ---------------------------------------------------------------------------
