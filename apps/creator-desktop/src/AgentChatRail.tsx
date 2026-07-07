import { useEffect, useRef, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { Loader2, Send, ShieldQuestion, TerminalSquare } from "lucide-react";
import { useStudioI18n } from "./i18n";
import { StudioButton, StudioStatusChip } from "./studioUi";
import type { AgentTurn } from "./useAgentConversation";

// ---------------------------------------------------------------------------
// AgentChatRail — the Cursor-style right panel.
//
// Replaces the old static "evidence panel". This is the PRIMARY interaction
// verb: the creator directs the pi-Agent (director intent) and each run
// becomes a chat turn (intent bubble + agent result with trace/scene/error).
// The no-fake honesty surface (boundary evidence) is preserved as an
// on-demand "Evidence" popover in the header, not always-painted.
// ---------------------------------------------------------------------------

export interface AgentChatRailProps {
  turns: AgentTurn[];
  input: string;
  onInputChange(value: string): void;
  running: boolean;
  canSubmit: boolean;
  onSubmit(): void;
  onOpenTrace(): void;
  /** Honesty-surface evidence (boundary block); shown in the Evidence popover. */
  evidence: ReactNode;
}

export function AgentChatRail({
  turns,
  input,
  onInputChange,
  running,
  canSubmit,
  onSubmit,
  onOpenTrace,
  evidence,
}: AgentChatRailProps) {
  const { t } = useStudioI18n();
  const [evidenceOpen, setEvidenceOpen] = useState(false);
  const toggleRef = useRef<HTMLButtonElement>(null);
  const popoverRef = useRef<HTMLDivElement>(null);

  function handleSubmit(event: React.FormEvent) {
    event.preventDefault();
    if (!canSubmit) return;
    onSubmit();
  }

  // Close the Evidence popover on Escape and on outside click (WAI-ARIA dialog
  // contract). Rendered via a portal so it escapes the rail <aside>'s
  // overflow-hidden + max-h clipping.
  useEffect(() => {
    if (!evidenceOpen) return;
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        event.preventDefault();
        setEvidenceOpen(false);
      }
    }
    function handlePointerDown(event: MouseEvent) {
      const target = event.target as Node | null;
      if (!target) return;
      if (popoverRef.current?.contains(target)) return;
      if (toggleRef.current?.contains(target)) return;
      setEvidenceOpen(false);
    }
    window.addEventListener("keydown", handleKeyDown);
    window.addEventListener("mousedown", handlePointerDown);
    return () => {
      window.removeEventListener("keydown", handleKeyDown);
      window.removeEventListener("mousedown", handlePointerDown);
    };
  }, [evidenceOpen]);

  // Anchor the popover under the toggle button. Computed once when open so it
  // stays put while open (repositioning on scroll is out of scope for this
  // on-demand disclosure).
  const popoverStyle = (() => {
    if (!evidenceOpen || !toggleRef.current) return undefined;
    const rect = toggleRef.current.getBoundingClientRect();
    return {
      left: Math.max(8, rect.right - 288),
      top: rect.bottom + 6,
    };
  })();

  return (
    <div className="flex h-full min-h-0 flex-col">
      {/* Header: pi-Agent identity + on-demand evidence popover */}
      <div className="relative flex items-center justify-between gap-2 px-3 py-2.5">
        <div className="flex min-w-0 items-center gap-2">
          <StudioStatusChip tone="agent">{t("agent.title")}</StudioStatusChip>
        </div>
        <button
          ref={toggleRef}
          type="button"
          aria-label={t("agent.evidence")}
          aria-expanded={evidenceOpen}
          aria-haspopup="dialog"
          title={t("agent.evidence")}
          onClick={() => setEvidenceOpen((prev) => !prev)}
          className="grid h-8 w-8 shrink-0 place-items-center rounded-md border border-canvas-200 bg-canvas-50 text-ink transition ease-expo hover:border-copper-500 hover:bg-canvas-100 active:translate-y-px"
        >
          <ShieldQuestion aria-hidden size={16} />
        </button>
        {evidenceOpen && popoverStyle
          ? createPortal(
              <div
                ref={popoverRef}
                role="dialog"
                aria-label={t("agent.evidence")}
                style={popoverStyle}
                className="fixed z-50 max-h-80 w-72 overflow-y-auto rounded-lg border border-canvas-200 bg-canvas-50 p-3 shadow-studio-panel"
              >
                {evidence}
              </div>,
              document.body,
            )
          : null}
      </div>
      <div className="hairline mx-3" />

      {/* Conversation turns */}
      <div className="min-h-0 flex-1 overflow-y-auto px-3 py-3">
        {turns.length === 0 ? (
          <div className="mt-6 grid place-items-center gap-1.5 px-4 py-8 text-center">
            <p className="font-display text-sm font-semibold tracking-tightish text-ink">
              {t("agent.you")} → {t("agent.agent")}
            </p>
            <p className="max-w-xs text-sm text-ink/45">
              {t("agent.empty")}
            </p>
          </div>
        ) : (
          <ol className="grid gap-3">
            {turns.map((turn) => (
              <li key={turn.id} className="grid gap-2">
                <TurnBubble
                  role={t("agent.you")}
                  tone="user"
                  text={turn.intent}
                />
                <TurnResult
                  role={t("agent.agent")}
                  turn={turn}
                  onOpenTrace={onOpenTrace}
                  openTraceLabel={t("agent.openTrace")}
                  traceLabel={t("agent.trace")}
                  sceneLabel={t("agent.scene")}
                  errorLabel={t("agent.error")}
                  // Friendly, code-specific i18n messages. When `errorCode`
                  // is null the rail falls back to the generic `agent.error`
                  // label + the raw redacted message (R1).
                  missingCredentialLabel={t("agent.apply.missingCredential")}
                  providerTimeoutLabel={t("agent.apply.providerTimeout")}
                  rateLimitLabel={t("agent.apply.rateLimit")}
                  contentFilteredLabel={t("agent.apply.contentFiltered")}
                  outputTruncatedLabel={t("agent.apply.outputTruncated")}
                  failedLabel={t("agent.apply.failed")}
                />
              </li>
            ))}
          </ol>
        )}
      </div>

      {/* Input dock */}
      <div className="hairline mx-3" />
      <form
        onSubmit={handleSubmit}
        className="grid gap-2 px-3 py-3"
      >
        <textarea
          aria-label={t("agent.directPrompt")}
          value={input}
          onChange={(event) => onInputChange(event.target.value)}
          placeholder={t("agent.directPrompt")}
          rows={2}
          spellCheck={false}
          className="w-full resize-none rounded-md border border-canvas-200 bg-canvas-50 px-3 py-2 text-sm leading-6 text-ink outline-none transition ease-expo focus:border-accent-400 focus:ring-1 focus:ring-accent-400/30"
        />
        <div className="flex items-center justify-end gap-2">
          <StudioButton
            variant="primary"
            type="submit"
            disabled={!canSubmit}
          >
            {running ? (
              <Loader2 aria-hidden size={16} className="animate-spin" />
            ) : (
              <Send aria-hidden size={16} />
            )}
            {t("agent.send")}
          </StudioButton>
        </div>
      </form>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Turn bubbles
// ---------------------------------------------------------------------------

function TurnBubble({
  role,
  tone,
  text,
}: {
  role: string;
  tone: "user" | "agent";
  text: string;
}) {
  const ruleClass =
    tone === "user"
      ? "border-l-2 border-l-violet-500"
      : "border-l-2 border-l-copper-500";
  return (
    <div
      className={[
        "rounded-md border border-canvas-200 bg-canvas-100 px-3 py-2 pl-3.5 text-sm leading-6",
        ruleClass,
      ].join(" ")}
    >
      <p className="text-xs font-semibold uppercase tracking-eyebrow text-ink/55">
        {role}
      </p>
      <p className="mt-1 whitespace-pre-wrap break-words">{text}</p>
    </div>
  );
}

function TurnResult({
  role,
  turn,
  onOpenTrace,
  openTraceLabel,
  traceLabel,
  sceneLabel,
  errorLabel,
  missingCredentialLabel,
  providerTimeoutLabel,
  rateLimitLabel,
  contentFilteredLabel,
  outputTruncatedLabel,
  failedLabel,
}: {
  role: string;
  turn: AgentTurn;
  onOpenTrace(): void;
  openTraceLabel: string;
  traceLabel: string;
  sceneLabel: string;
  errorLabel: string;
  missingCredentialLabel: string;
  providerTimeoutLabel: string;
  rateLimitLabel: string;
  contentFilteredLabel: string;
  outputTruncatedLabel: string;
  failedLabel: string;
}) {
  const { report, error, errorCode, errorEnvVar } = turn;
  // Render a friendly, code-specific message when `errorCode` is set. The
  // raw redacted `error` is shown as a detail line so the user can still
  // see the provider's redacted message (R1 — `errorCode` is no longer dead
  // UI state).
  const friendlyMessage = (() => {
    if (!errorCode) return null;
    if (errorCode === "pi_agent_missing_credential") {
      return errorEnvVar
        ? missingCredentialLabel.replace("{envVar}", errorEnvVar)
        : missingCredentialLabel.replace("{envVar}", "");
    }
    if (errorCode === "pi_agent_provider_timeout") return providerTimeoutLabel;
    if (errorCode === "text_provider_rate_limit") return rateLimitLabel;
    if (errorCode === "text_provider_content_filtered")
      return contentFilteredLabel;
    if (errorCode === "text_provider_output_truncated")
      return outputTruncatedLabel;
    return failedLabel;
  })();
  return (
    <div className="rounded-md border border-canvas-200 border-l-2 border-l-copper-500 bg-canvas-50 px-3 py-2 pl-3.5 text-sm">
      <div className="flex items-center justify-between gap-2">
        <p className="text-xs font-semibold uppercase tracking-eyebrow text-ink/55">
          {role}
        </p>
        {report ? (
          <button
            type="button"
            onClick={onOpenTrace}
            className="inline-flex items-center gap-1 text-xs font-semibold text-violet-600 transition ease-expo hover:text-violet-500 hover:underline"
          >
            <TerminalSquare aria-hidden size={12} />
            {openTraceLabel}
          </button>
        ) : null}
      </div>
      {error ? (
        <div className="mt-2 rounded-md border border-signal/20 bg-signal/10 px-2 py-1 text-xs text-signal">
          <p>
            <span className="font-semibold">{errorLabel}:</span>{" "}
            {friendlyMessage ?? error}
          </p>
          {friendlyMessage ? (
            <p className="mt-0.5 opacity-80">{error}</p>
          ) : null}
        </div>
      ) : report ? (
        <dl className="mt-2 grid gap-1 text-xs text-ink/70">
          <div className="flex gap-2">
            <dt className="font-semibold text-ink/55">{traceLabel}:</dt>
            <dd className="truncate font-mono">{report.trace.id}</dd>
          </div>
          <div className="flex gap-2">
            <dt className="font-semibold text-ink/55">{sceneLabel}:</dt>
            <dd className="truncate">{report.scene.title}</dd>
          </div>
        </dl>
      ) : null}
    </div>
  );
}
