import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type CSSProperties,
  type KeyboardEvent as ReactKeyboardEvent,
} from "react";
import { createPortal } from "react-dom";
import {
  Brain,
  ChevronDown,
  Cpu,
  Loader2,
  RefreshCw,
  ShieldCheck,
  X,
  Zap,
} from "lucide-react";
import type {
  ModelOption,
  PermissionLevel,
  RemoteModelInfo,
  ThinkingLevel,
} from "../../../contracts/plotforge";
import type { StudioDataSource } from "./studioDataSource";
import { useStudioI18n } from "./i18n";

// ---------------------------------------------------------------------------

const thinkingLevels: ThinkingLevel[] = ["off", "low", "medium", "high"];

function useThinkingLabels(): Record<ThinkingLevel, string> {
  const { t } = useStudioI18n();
  return {
    high: t("home.thinkingHigh"),
    medium: t("home.thinkingMedium"),
    low: t("home.thinkingLow"),
    off: t("home.thinkingOff"),
  };
}

function modelDisplayLabel(model: ModelOption): string {
  return model.provider
    ? `${model.label} — ${model.provider}`
    : model.label;
}
// Shared agent-config selectors.
//
// A single source of truth for the model / permission / thinking selectors so
// the two editing surfaces (the home `LaunchpadView` bottom toolbar and the
// `AgentConfigSection` Model area) render the same `AgentSessionConfig` with
// the same UI. Previously `LaunchpadView` carried Tailwind + lucide-styled
// selectors while `AgentView` used bare `<select>` + inline `style=`, which
// read as two different applications editing the same config (B3).
// ---------------------------------------------------------------------------

export function ModelSelect({
  models,
  value,
  onChange,
}: {
  models: ModelOption[];
  value: string;
  onChange(value: string): void;
}) {
  const { t } = useStudioI18n();
  const hasSelectedModel = models.some((model) => model.id === value);
  const selectedValue = hasSelectedModel ? value : "";
  return (
    <label className="inline-flex h-9 items-center gap-1.5 rounded-lg border border-canvas-200/70 bg-canvas-50 pl-2.5 pr-1.5 text-sm text-ink/80">
      <Cpu aria-hidden size={15} className="text-copper-500" />
      <span className="sr-only">{t("home.aria.modelSelect")}</span>
      <select
        aria-label={t("home.aria.modelSelect")}
        value={selectedValue}
        onChange={(event) => onChange(event.target.value)}
        disabled={models.length === 0}
        className="cursor-pointer bg-transparent text-sm font-medium outline-none"
      >
        {!hasSelectedModel && (
          <option value="">{t("home.noModels")}</option>
        )}
        {models.map((model) => (
          <option key={model.id} value={model.id}>
            {modelDisplayLabel(model)}
          </option>
        ))}
      </select>
      <ChevronDown aria-hidden size={14} className="text-ink/45" />
    </label>
  );
}

export function PermissionSelect({
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

export function ThinkingSelect({
  value,
  onChange,
}: {
  value: ThinkingLevel;
  onChange(value: ThinkingLevel): void;
}) {
  const { t } = useStudioI18n();
  const labels = useThinkingLabels();
  return (
    <label className="inline-flex h-9 items-center gap-1.5 rounded-lg border border-canvas-200/70 bg-canvas-50 pl-2.5 pr-1.5 text-sm text-ink/80">
      <Brain aria-hidden size={15} className="text-copper-500" />
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

// ---------------------------------------------------------------------------
// ModelThinkingDeck — the compose-surface model + thinking picker.
//
// This is intentionally one control: a model row and thinking column form a
// single persisted run profile. The matrix is keyboard navigable and remains
// only a UI adapter over the AgentSessionConfig owned by useAgentConfig.
// ---------------------------------------------------------------------------

export interface ModelThinkingSelection {
  modelId: string;
  thinkingLevel: ThinkingLevel;
}

export function ModelThinkingDeck({
  models,
  modelId,
  thinkingLevel,
  onChange,
}: {
  models: ModelOption[];
  modelId: string;
  thinkingLevel: ThinkingLevel;
  onChange(selection: ModelThinkingSelection): void;
}) {
  const { t } = useStudioI18n();
  const labels = useThinkingLabels();
  const [open, setOpen] = useState(false);
  const [popoverStyle, setPopoverStyle] = useState<CSSProperties>();
  const triggerRef = useRef<HTMLButtonElement>(null);
  const popoverRef = useRef<HTMLDivElement>(null);
  const cellRefs = useRef(new Map<string, HTMLButtonElement>());
  const selectedModelIndex = models.findIndex((model) => model.id === modelId);
  const activeModelIndex = selectedModelIndex >= 0 ? selectedModelIndex : 0;
  const activeThinkingIndex = Math.max(0, thinkingLevels.indexOf(thinkingLevel));
  const selectedModel = selectedModelIndex >= 0 ? models[selectedModelIndex] : undefined;
  const selectedSummary = selectedModel
    ? `${selectedModel.label} · ${labels[thinkingLevel]}`
    : t("home.noModels");

  const close = useCallback(() => {
    setOpen(false);
    triggerRef.current?.focus();
  }, []);

  const positionPopover = useCallback(() => {
    const trigger = triggerRef.current;
    if (!trigger) return;
    const rect = trigger.getBoundingClientRect();
    const viewportWidth = Math.max(window.innerWidth, 320);
    const width = Math.min(610, viewportWidth - 16);
    const left = Math.min(
      Math.max(8, rect.right - width),
      Math.max(8, viewportWidth - width - 8),
    );
    const above = rect.top >= 330;
    setPopoverStyle(
      above
        ? { left, bottom: window.innerHeight - rect.top + 10, width }
        : { left, top: rect.bottom + 10, width },
    );
  }, []);

  useLayoutEffect(() => {
    if (!open) return;
    positionPopover();
  }, [open, positionPopover]);

  useEffect(() => {
    if (!open) return;
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        event.preventDefault();
        close();
      }
    }
    function handlePointerDown(event: MouseEvent) {
      const target = event.target as Node | null;
      if (!target) return;
      if (popoverRef.current?.contains(target)) return;
      if (triggerRef.current?.contains(target)) return;
      close();
    }
    window.addEventListener("keydown", handleKeyDown);
    window.addEventListener("mousedown", handlePointerDown);
    window.addEventListener("resize", positionPopover);
    return () => {
      window.removeEventListener("keydown", handleKeyDown);
      window.removeEventListener("mousedown", handlePointerDown);
      window.removeEventListener("resize", positionPopover);
    };
  }, [close, open, positionPopover]);

  function select(modelIndex: number, thinkingIndex: number) {
    const model = models[modelIndex];
    const level = thinkingLevels[thinkingIndex];
    if (!model || !level) return;
    onChange({ modelId: model.id, thinkingLevel: level });
    cellRefs.current.get(`${model.id}:${level}`)?.focus();
  }

  function handleCellKeyDown(
    event: ReactKeyboardEvent<HTMLButtonElement>,
    modelIndex: number,
    thinkingIndex: number,
  ) {
    const next = { modelIndex, thinkingIndex };
    if (event.key === "ArrowLeft") next.thinkingIndex -= 1;
    else if (event.key === "ArrowRight") next.thinkingIndex += 1;
    else if (event.key === "ArrowUp") next.modelIndex -= 1;
    else if (event.key === "ArrowDown") next.modelIndex += 1;
    else if (event.key === "Home") next.thinkingIndex = 0;
    else if (event.key === "End") next.thinkingIndex = thinkingLevels.length - 1;
    else return;
    event.preventDefault();
    select(
      Math.min(Math.max(next.modelIndex, 0), models.length - 1),
      Math.min(Math.max(next.thinkingIndex, 0), thinkingLevels.length - 1),
    );
  }

  const popover = open && popoverStyle
    ? createPortal(
        <div
          ref={popoverRef}
          role="dialog"
          aria-label={t("home.modelDeck.title")}
          style={popoverStyle}
          className="model-deck fixed z-[70] overflow-hidden rounded-[1.35rem] border border-canvas-200 bg-canvas-50 shadow-panel-lift"
        >
          <div className="flex items-start justify-between gap-4 border-b border-canvas-200/70 px-4 py-3.5">
            <div className="min-w-0">
              <p className="eyebrow eyebrow--copper">{t("home.modelDeck.eyebrow")}</p>
              <p className="mt-1 truncate font-display text-lg font-semibold text-ink">
                {selectedSummary}
              </p>
            </div>
            <button
              type="button"
              aria-label={t("home.modelDeck.close")}
              onClick={close}
              className="grid h-8 w-8 shrink-0 place-items-center rounded-full border border-canvas-200 bg-canvas-50 text-ink/60 transition ease-expo hover:border-copper-500 hover:text-ink"
            >
              <X aria-hidden size={15} />
            </button>
          </div>

          <div className="grid grid-cols-[minmax(0,1fr)_4.25rem] gap-3 p-3 sm:p-4">
            <div className="min-w-0">
              <div
                className="grid items-end gap-1 px-1 pb-2"
                style={{ gridTemplateColumns: "minmax(5.75rem,1.35fr) repeat(4,minmax(2.75rem,1fr))" }}
              >
                <span className="text-[0.65rem] font-bold uppercase tracking-eyebrow text-ink/35">
                  {t("home.modelLabel")}
                </span>
                {thinkingLevels.map((level) => (
                  <span
                    key={level}
                    className="text-center text-[0.65rem] font-bold uppercase tracking-[0.1em] text-ink/45"
                  >
                    {labels[level]}
                  </span>
                ))}
              </div>

              <div role="grid" aria-label={t("home.modelDeck.gridLabel")} className="model-deck__matrix max-h-[17rem] overflow-y-auto rounded-xl border border-canvas-200/80 bg-canvas-100/80 p-1.5">
                {models.map((model, modelIndex) => {
                  const activeRow = model.id === modelId;
                  return (
                    <div
                      key={model.id}
                      role="row"
                      className="grid items-center gap-1 rounded-lg px-1 py-0.5"
                      style={{ gridTemplateColumns: "minmax(5.75rem,1.35fr) repeat(4,minmax(2.75rem,1fr))" }}
                    >
                      <div
                        role="rowheader"
                        title={modelDisplayLabel(model)}
                        className={[
                          "min-w-0 truncate px-2 text-sm font-semibold transition-colors",
                          activeRow ? "text-sage" : "text-ink/65",
                        ].join(" ")}
                      >
                        {model.label}
                      </div>
                      {thinkingLevels.map((level, thinkingIndex) => {
                        const active = activeRow && level === thinkingLevel;
                        const lit = activeRow && thinkingIndex <= activeThinkingIndex;
                        return (
                          <button
                            key={level}
                            ref={(node) => {
                              const key = `${model.id}:${level}`;
                              if (node) cellRefs.current.set(key, node);
                              else cellRefs.current.delete(key);
                            }}
                            type="button"
                            role="gridcell"
                            aria-selected={active}
                            aria-label={`${modelDisplayLabel(model)}, ${labels[level]}`}
                            tabIndex={
                              active ||
                              (selectedModelIndex < 0 &&
                                modelIndex === activeModelIndex &&
                                thinkingIndex === activeThinkingIndex)
                                ? 0
                                : -1
                            }
                            onClick={() => select(modelIndex, thinkingIndex)}
                            onKeyDown={(event) =>
                              handleCellKeyDown(event, modelIndex, thinkingIndex)
                            }
                            className={[
                              "group relative grid h-11 place-items-center overflow-hidden transition ease-expo focus-visible:z-10",
                              lit ? "bg-sage/20" : "hover:bg-canvas-50/80",
                              thinkingIndex === 0 ? "rounded-l-full" : "",
                              thinkingIndex === thinkingLevels.length - 1 ? "rounded-r-full" : "",
                            ].join(" ")}
                          >
                            <span
                              aria-hidden
                              className={[
                                "relative z-10 block rounded-full transition-[transform,background-color,box-shadow] duration-300 ease-expo",
                                active
                                  ? "h-8 w-8 border border-copper-500/45 bg-canvas-50 shadow-[0_5px_14px_rgba(90,70,130,0.2),0_0_0_6px_rgba(90,138,122,0.14)]"
                                  : lit
                                    ? "h-2.5 w-2.5 bg-sage"
                                    : "h-2 w-2 bg-ink/20 group-hover:bg-ink/35",
                              ].join(" ")}
                            />
                          </button>
                        );
                      })}
                    </div>
                  );
                })}
              </div>
            </div>

            <div aria-hidden className="model-deck__lever grid min-h-48 place-items-center rounded-xl border border-canvas-200/80 px-2 py-2">
              <span className="text-[0.6rem] font-black uppercase tracking-[0.16em] text-copper-600">
                {t("home.thinkingLabel")}
              </span>
              <div className="relative h-36 w-8 rounded-full border border-ink/15 bg-ink/90 shadow-inner">
                <span className="absolute left-1/2 top-3 h-[calc(100%-1.5rem)] w-1 -translate-x-1/2 rounded-full bg-canvas-200" />
                <span
                  className="model-deck__lever-knob absolute left-1/2 h-9 w-9 -translate-x-1/2 -translate-y-1/2 rounded-full border border-copper-600/40 bg-copper-400 shadow-[0_6px_12px_rgba(43,34,56,0.24),inset_0_2px_2px_rgba(255,255,255,0.35)] transition-[top] duration-300 ease-expo"
                  style={{ top: `${16 + activeThinkingIndex * 22.5}%` }}
                />
              </div>
            </div>
          </div>

          <p className="border-t border-canvas-200/60 px-4 py-2.5 text-xs text-ink/45">
            {t("home.modelDeck.hint")}
          </p>
        </div>,
        document.body,
      )
    : null;

  return (
    <>
      <button
        ref={triggerRef}
        type="button"
        aria-label={t("home.aria.modelThinkingSelect")}
        aria-expanded={open}
        aria-haspopup="dialog"
        title={selectedSummary}
        disabled={models.length === 0}
        onClick={() => setOpen((current) => !current)}
        className="inline-flex h-9 min-w-0 max-w-full items-center gap-2 rounded-full border border-canvas-200/80 bg-canvas-50 pl-2.5 pr-2 text-sm text-ink/80 shadow-[inset_0_-1px_0_rgba(138,100,80,0.08)] transition ease-expo hover:border-copper-500 hover:bg-canvas-100 disabled:cursor-not-allowed disabled:opacity-55"
      >
        <Zap aria-hidden size={14} className="shrink-0 fill-copper-400 text-copper-500" />
        <span className="min-w-0 truncate font-semibold">{selectedModel?.label ?? t("home.noModels")}</span>
        <span className="shrink-0 text-ink/45">{selectedModel ? labels[thinkingLevel] : ""}</span>
        <ChevronDown
          aria-hidden
          size={14}
          className={`shrink-0 text-ink/40 transition-transform duration-300 ease-expo ${open ? "rotate-180" : ""}`}
        />
      </button>
      {popover}
    </>
  );
}

// ---------------------------------------------------------------------------
// ModelCombobox — a combobox (text input + suggestions) used by the provider
// editor in Settings → Agent. Unlike the compact `ModelSelect` (a forced
// `<select>` for the home toolbar), this lets the user TYPE a custom model id
// while a "Fetch models" call to `dataSource.listRemoteModels(providerId)`
// populates a `<datalist>` of suggestions from the provider's upstream
// `/models` endpoint. The user is never forced to pick a fetched suggestion.
//
// Loading state disables the Fetch button + shows a spinner; a fetch failure
// surfaces a redaction-safe error message (e.g. "Missing credential",
// "HTTP error") without leaking provider response bodies. The fetched model
// ids are the only upstream content kept — `RemoteModelInfo` carries no
// endpoint URL or credential.
// ---------------------------------------------------------------------------

export interface ModelComboboxFetchProps {
  dataSource: StudioDataSource;
  /** The provider registry id whose upstream `/models` endpoint is queried. */
  providerId: string;
}

export function ModelCombobox({
  value,
  onChange,
  fetch,
  ariaLabel,
  placeholder,
}: {
  value: string;
  onChange(value: string): void;
  fetch: ModelComboboxFetchProps;
  ariaLabel: string;
  placeholder?: string;
}) {
  const { t } = useStudioI18n();
  const [fetched, setFetched] = useState<RemoteModelInfo[] | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleFetch = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const models = await fetch.dataSource.listRemoteModels(fetch.providerId);
      setFetched(models);
    } catch (err) {
      setFetched(null);
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, [fetch]);

  const listId = `model-combobox-${fetch.providerId}`;
  const suggestions = fetched ?? [];

  return (
    <div className="grid gap-1.5">
      <div className="flex flex-wrap items-center gap-2">
        <input
          aria-label={ariaLabel}
          list={listId}
          value={value}
          onChange={(event) => onChange(event.target.value)}
          placeholder={placeholder}
          autoComplete="off"
          className="h-10 min-w-0 flex-1 rounded-md border border-canvas-200 bg-canvas-50 px-3 text-sm text-ink outline-none transition ease-expo focus:border-accent-400 focus:ring-1 focus:ring-accent-400/30"
        />
        <button
          type="button"
          onClick={handleFetch}
          disabled={loading}
          aria-label={t("agent.provider.fetchModels")}
          className="inline-flex h-10 items-center gap-1.5 rounded-md border border-canvas-200 bg-canvas-50 px-3 text-sm font-medium text-ink transition ease-expo hover:border-copper-500 hover:bg-canvas-100 active:translate-y-px disabled:cursor-not-allowed disabled:opacity-60"
        >
          {loading ? (
            <Loader2 aria-hidden size={15} className="animate-spin" />
          ) : (
            <RefreshCw aria-hidden size={15} className="text-copper-500" />
          )}
          {loading
            ? t("agent.provider.fetching")
            : t("agent.provider.fetchModels")}
        </button>
      </div>
      <datalist id={listId}>
        {suggestions.map((model) => (
          <option key={model.id} value={model.id}>
            {model.owned_by ? `${model.id} — ${model.owned_by}` : model.id}
          </option>
        ))}
      </datalist>
      {error && (
        <div
          role="alert"
          className="rounded-md border border-signal/30 bg-signal/10 px-2.5 py-1.5 text-xs text-signal"
        >
          {error}
        </div>
      )}
    </div>
  );
}
