import type { ProviderTestResult } from "./tauriBridge";
import { useStudioI18n } from "./i18n";

export function QuotaFields({
  maxConcurrency,
  requestsPerMinute,
  dailyTokenBudget,
  onMaxConcurrencyChange,
  onRequestsPerMinuteChange,
  onDailyTokenBudgetChange,
  supportsDailyTokenBudget,
  unsupportedDailyTokenBudgetHint,
}: {
  maxConcurrency: number | null;
  requestsPerMinute: number | null;
  dailyTokenBudget: number | null;
  onMaxConcurrencyChange: (value: number | null) => void;
  onRequestsPerMinuteChange: (value: number | null) => void;
  onDailyTokenBudgetChange: (value: number | null) => void;
  supportsDailyTokenBudget: boolean;
  unsupportedDailyTokenBudgetHint?: string;
}) {
  const { t } = useStudioI18n();
  const optionalNumber = (value: string) =>
    value === "" ? null : Number(value);
  return (
    <fieldset className="grid gap-3 rounded-md border border-canvas-200/70 p-3">
      <legend className="px-1 text-xs font-semibold uppercase tracking-eyebrow text-ink/55">
        {t("agent.provider.quota.title")}
      </legend>
      <label className="grid gap-1">
        <span className="text-xs font-semibold uppercase tracking-eyebrow text-ink/55">
          {t("agent.provider.quota.maxConcurrency")}
        </span>
        <input
          type="number"
          min={1}
          aria-label={t("agent.provider.quota.maxConcurrency")}
          value={maxConcurrency ?? ""}
          onChange={(event) =>
            onMaxConcurrencyChange(optionalNumber(event.target.value))
          }
          className="h-10 min-w-0 rounded-md border border-canvas-200 bg-canvas-50 px-3 text-sm text-ink outline-none transition ease-expo focus:border-accent-400 focus:ring-1 focus:ring-accent-400/30"
        />
      </label>
      <label className="grid gap-1">
        <span className="text-xs font-semibold uppercase tracking-eyebrow text-ink/55">
          {t("agent.provider.quota.requestsPerMinute")}
        </span>
        <input
          type="number"
          min={1}
          aria-label={t("agent.provider.quota.requestsPerMinute")}
          value={requestsPerMinute ?? ""}
          onChange={(event) =>
            onRequestsPerMinuteChange(optionalNumber(event.target.value))
          }
          className="h-10 min-w-0 rounded-md border border-canvas-200 bg-canvas-50 px-3 text-sm text-ink outline-none transition ease-expo focus:border-accent-400 focus:ring-1 focus:ring-accent-400/30"
        />
      </label>
      <label className="grid gap-1">
        <span className="text-xs font-semibold uppercase tracking-eyebrow text-ink/55">
          {t("agent.provider.quota.dailyTokenBudget")}
        </span>
        <input
          type="number"
          min={1}
          disabled={!supportsDailyTokenBudget}
          aria-label={t("agent.provider.quota.dailyTokenBudget")}
          value={supportsDailyTokenBudget ? dailyTokenBudget ?? "" : ""}
          onChange={(event) =>
            onDailyTokenBudgetChange(optionalNumber(event.target.value))
          }
          className="h-10 min-w-0 rounded-md border border-canvas-200 bg-canvas-50 px-3 text-sm text-ink outline-none transition ease-expo focus:border-accent-400 focus:ring-1 focus:ring-accent-400/30 disabled:cursor-not-allowed disabled:opacity-55"
        />
        {!supportsDailyTokenBudget && (
          <small className="text-xs text-ink/55">
            {unsupportedDailyTokenBudgetHint ??
              t("agent.provider.quota.dailyTokenBudgetTextOnly")}
          </small>
        )}
      </label>
    </fieldset>
  );
}

export function ProviderTestStatus({
  result,
}: {
  result: ProviderTestResult | null;
}) {
  const { t } = useStudioI18n();
  if (!result) return null;
  return (
    <div
      role="status"
      className={[
        "mt-3 rounded-md border px-3 py-2 text-sm",
        result.ok
          ? "border-sage/30 bg-sage/10 text-sage"
          : "border-signal/30 bg-signal/10 text-signal",
      ].join(" ")}
    >
      {result.ok
        ? t("agent.provider.test.ok")
        : t("agent.provider.test.failed")}{" "}
      {result.message}
    </div>
  );
}
