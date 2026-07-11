import { useCallback, useEffect, useMemo, useState } from "react";
import type { UsageSummary } from "../../../contracts/plotforge";
import type { StudioDataSource } from "./studioDataSource";
import { EmptyState, StudioButton, StudioPanel } from "./studioUi";
import { useStudioI18n } from "./i18n";

export function UsageSection({ dataSource }: { dataSource: StudioDataSource }) {
  const { locale, t } = useStudioI18n();
  const [summary, setSummary] = useState<UsageSummary | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      setSummary(await dataSource.getUsageSummary());
    } catch (cause) {
      setSummary(null);
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setLoading(false);
    }
  }, [dataSource]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const providers = useMemo(
    () => Object.values(summary?.by_provider ?? {}),
    [summary],
  );
  const number = useMemo(
    () => new Intl.NumberFormat(locale === "zh" ? "zh-CN" : "en-US"),
    [locale],
  );

  return (
    <StudioPanel>
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h4 className="font-display text-lg font-semibold tracking-display-tight text-ink">
            {t("agent.usage.title")}
          </h4>
          <p className="mt-1 text-sm leading-5 text-ink/55">
            {t("agent.usage.subtitle")}
          </p>
        </div>
        <StudioButton onClick={() => void refresh()} disabled={loading}>
          {loading ? t("agent.usage.refreshing") : t("agent.usage.refresh")}
        </StudioButton>
      </div>

      {error ? (
        <div
          role="alert"
          className="mt-4 rounded-md border border-signal/30 bg-signal/10 px-3 py-2 text-sm text-signal"
        >
          {t("agent.usage.loadError")} {error}
        </div>
      ) : null}

      {summary ? (
        <>
          <dl className="mt-4 grid overflow-hidden rounded-md border border-canvas-200/70 bg-canvas-100/45 sm:grid-cols-3 sm:divide-x sm:divide-canvas-200/70">
            <UsageMetric
              label={t("agent.usage.inputTokens")}
              value={number.format(summary.total_input_tokens)}
            />
            <UsageMetric
              label={t("agent.usage.outputTokens")}
              value={number.format(summary.total_output_tokens)}
            />
            <UsageMetric
              label={t("agent.usage.costUnits")}
              value={number.format(summary.total_spent_cost_units)}
            />
          </dl>

          {providers.length === 0 ? (
            <EmptyState>{t("agent.usage.empty")}</EmptyState>
          ) : (
            <div className="mt-4 overflow-x-auto">
              <table className="w-full min-w-[44rem] border-collapse text-left text-sm">
                <caption className="sr-only">
                  {t("agent.usage.providerTable")}
                </caption>
                <thead>
                  <tr className="border-b border-canvas-200/70 text-xs font-semibold uppercase tracking-eyebrow text-ink/50">
                    <th className="px-2 py-2">{t("agent.usage.provider")}</th>
                    <th className="px-2 py-2 text-right">{t("agent.usage.input")}</th>
                    <th className="px-2 py-2 text-right">{t("agent.usage.output")}</th>
                    <th className="px-2 py-2 text-right">{t("agent.usage.textCalls")}</th>
                    <th className="px-2 py-2 text-right">{t("agent.usage.imageCalls")}</th>
                    <th className="px-2 py-2 text-right">{t("agent.usage.ttsCalls")}</th>
                    <th className="px-2 py-2 text-right">{t("agent.usage.cost")}</th>
                  </tr>
                </thead>
                <tbody>
                  {providers.map((provider) => (
                    <tr
                      key={provider.provider_id}
                      className="border-b border-canvas-200/45 last:border-b-0"
                    >
                      <th className="px-2 py-3 font-semibold text-ink">
                        {provider.provider_id}
                      </th>
                      <td className="px-2 py-3 text-right tabular-nums text-ink/70">
                        {number.format(provider.input_tokens)}
                      </td>
                      <td className="px-2 py-3 text-right tabular-nums text-ink/70">
                        {number.format(provider.output_tokens)}
                      </td>
                      <td className="px-2 py-3 text-right tabular-nums text-ink/70">
                        {number.format(provider.text_calls)}
                      </td>
                      <td className="px-2 py-3 text-right tabular-nums text-ink/70">
                        {number.format(provider.image_calls)}
                      </td>
                      <td className="px-2 py-3 text-right tabular-nums text-ink/70">
                        {number.format(provider.tts_calls)}
                      </td>
                      <td className="px-2 py-3 text-right font-semibold tabular-nums text-ink">
                        {number.format(provider.spent_cost_units)}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </>
      ) : null}
    </StudioPanel>
  );
}

function UsageMetric({ label, value }: { label: string; value: string }) {
  return (
    <div className="px-4 py-3">
      <dt className="text-xs font-semibold uppercase tracking-eyebrow text-ink/50">
        {label}
      </dt>
      <dd className="mt-1 font-display text-2xl font-semibold tabular-nums text-ink">
        {value}
      </dd>
    </div>
  );
}
