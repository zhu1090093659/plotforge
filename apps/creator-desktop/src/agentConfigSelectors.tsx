import { Brain, ChevronDown, Cpu, ShieldCheck } from "lucide-react";
import type {
  ModelOption,
  PermissionLevel,
  ThinkingLevel,
} from "../../../contracts/plotforge";
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
