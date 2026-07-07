import { useCallback, useState } from "react";
import { Brain, ChevronDown, Cpu, Loader2, RefreshCw, ShieldCheck } from "lucide-react";
import type {
  ModelOption,
  PermissionLevel,
  RemoteModelInfo,
  ThinkingLevel,
} from "../../../contracts/plotforge";
import type { StudioDataSource } from "./studioDataSource";
import { useStudioI18n } from "./i18n";

// ---------------------------------------------------------------------------
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
  return (
    <label className="inline-flex h-9 items-center gap-1.5 rounded-lg border border-canvas-200/70 bg-canvas-50 pl-2.5 pr-1.5 text-sm text-ink/80">
      <Cpu aria-hidden size={15} className="text-copper-500" />
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
  const labels: Record<ThinkingLevel, string> = {
    high: t("home.thinkingHigh"),
    medium: t("home.thinkingMedium"),
    low: t("home.thinkingLow"),
    off: t("home.thinkingOff"),
  };
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
