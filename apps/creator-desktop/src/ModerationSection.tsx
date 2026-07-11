import { useCallback, useEffect, useState } from "react";
import type { ModerationProviderEntry } from "../../../contracts/plotforge";
import type { StudioDataSource } from "./studioDataSource";
import type { ProviderTestResult } from "./tauriBridge";
import { useStudioI18n } from "./i18n";
import {
  ProviderTestStatus,
  QuotaFields,
} from "./ProviderConfigFields";
import { StudioButton, StudioPanel, TextInput } from "./studioUi";

const EMPTY_MODERATION_ENTRY: ModerationProviderEntry = {
  id: "",
  endpoint_url: "",
  model: "",
  credential_env_var: "",
  enabled: true,
  max_concurrency: null,
  requests_per_minute: null,
  daily_token_budget: null,
};

interface ModerationEditorState {
  entry: ModerationProviderEntry;
  existingId: string | null;
}

export function ModerationSection({
  dataSource,
}: {
  dataSource: StudioDataSource;
}) {
  const { t } = useStudioI18n();
  const [providers, setProviders] = useState<ModerationProviderEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [editorState, setEditorState] =
    useState<ModerationEditorState | null>(null);
  const [testResult, setTestResult] =
    useState<ProviderTestResult | null>(null);
  const [testing, setTesting] = useState(false);

  const reloadProviders = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      setProviders(await dataSource.listModerationProviders());
    } catch (loadError) {
      setError(
        loadError instanceof Error ? loadError.message : String(loadError),
      );
    } finally {
      setLoading(false);
    }
  }, [dataSource]);

  useEffect(() => {
    void reloadProviders();
  }, [reloadProviders]);

  const saveProvider = async (
    entry: ModerationProviderEntry,
    existingId: string | null,
  ) => {
    setError(null);
    try {
      await dataSource.upsertModerationProvider({
        ...entry,
        id: existingId ?? entry.id,
        daily_token_budget: null,
      });
      setEditorState(null);
      await reloadProviders();
    } catch (saveError) {
      setError(
        saveError instanceof Error ? saveError.message : String(saveError),
      );
    }
  };

  const deleteProvider = async (id: string) => {
    setError(null);
    try {
      await dataSource.deleteModerationProvider(id);
      await reloadProviders();
    } catch (deleteError) {
      setError(
        deleteError instanceof Error
          ? deleteError.message
          : String(deleteError),
      );
    }
  };

  const testProvider = async (id: string) => {
    setTesting(true);
    setTestResult(null);
    try {
      setTestResult(await dataSource.testModerationProvider(id));
    } catch (testError) {
      setTestResult({
        ok: false,
        message:
          testError instanceof Error ? testError.message : String(testError),
      });
    } finally {
      setTesting(false);
    }
  };

  const firstEnabled = providers.find((provider) => provider.enabled);

  return (
    <StudioPanel>
      <div className="mb-3 flex items-center justify-between gap-2">
        <h4 className="font-display text-lg font-semibold tracking-display-tight text-ink">
          {t("agent.moderationProvider.title")}
        </h4>
        <StudioButton
          variant="primary"
          onClick={() =>
            setEditorState({
              entry: { ...EMPTY_MODERATION_ENTRY },
              existingId: null,
            })
          }
        >
          {t("agent.moderationProvider.add")}
        </StudioButton>
      </div>
      <p className="mb-3 text-sm text-ink-faint">
        {firstEnabled
          ? t("agent.moderationProvider.firstEnabled", {
              id: firstEnabled.id,
            })
          : t("agent.moderationProvider.noneEnabled")}
      </p>
      {error && (
        <div role="alert" className="mb-3 text-sm text-danger">
          {error}
        </div>
      )}
      {editorState ? (
        <ModerationProviderEditor
          entry={editorState.entry}
          existingId={editorState.existingId}
          onCancel={() => setEditorState(null)}
          onSave={saveProvider}
        />
      ) : loading ? (
        <p className="text-sm text-ink-faint">
          {t("agent.provider.loading")}
        </p>
      ) : providers.length === 0 ? (
        <p className="text-sm text-ink-faint">
          {t("agent.moderationProvider.empty")}
        </p>
      ) : (
        <ul className="grid gap-2">
          {providers.map((provider) => (
            <li
              key={provider.id}
              className="rounded-md border border-canvas-200/70 bg-canvas-100/40 p-3"
            >
              <div className="flex items-center justify-between gap-2">
                <strong className="font-semibold text-ink">
                  {provider.id}
                </strong>
                <span
                  className={
                    provider.enabled
                      ? "text-xs font-medium text-success"
                      : "text-xs font-medium text-ink-faint"
                  }
                >
                  {provider.enabled
                    ? t("agent.provider.enabled")
                    : t("agent.provider.disabled")}
                </span>
              </div>
              <dl className="mt-2 grid grid-cols-1 gap-1 text-xs text-ink-faint sm:grid-cols-2">
                <div>
                  <dt className="inline font-medium text-ink">
                    {t("agent.moderationProvider.model")}
                  </dt>
                  <dd className="inline"> {provider.model}</dd>
                </div>
                <div>
                  <dt className="inline font-medium text-ink">
                    {t("agent.moderationProvider.endpoint")}
                  </dt>
                  <dd className="inline break-all">
                    {" "}
                    {provider.endpoint_url}
                  </dd>
                </div>
              </dl>
              <div className="mt-3 flex gap-1.5">
                <StudioButton
                  onClick={() =>
                    setEditorState({
                      entry: provider,
                      existingId: provider.id,
                    })
                  }
                >
                  {t("agent.provider.edit")}
                </StudioButton>
                <StudioButton onClick={() => void deleteProvider(provider.id)}>
                  {t("agent.provider.delete")}
                </StudioButton>
                <StudioButton
                  onClick={() => void testProvider(provider.id)}
                  disabled={testing}
                >
                  {t("agent.provider.test")}
                </StudioButton>
              </div>
            </li>
          ))}
        </ul>
      )}
      <ProviderTestStatus result={testResult} />
    </StudioPanel>
  );
}

function ModerationProviderEditor({
  entry,
  existingId,
  onCancel,
  onSave,
}: {
  entry: ModerationProviderEntry;
  existingId: string | null;
  onCancel: () => void;
  onSave: (
    entry: ModerationProviderEntry,
    existingId: string | null,
  ) => void;
}) {
  const { t } = useStudioI18n();
  const [draft, setDraft] = useState({
    ...entry,
    daily_token_budget: null,
  });
  const update = <K extends keyof ModerationProviderEntry>(
    key: K,
    value: ModerationProviderEntry[K],
  ) =>
    setDraft((previous) => ({
      ...previous,
      [key]: value,
    }));

  return (
    <div className="grid max-w-xl gap-3 rounded-md border border-canvas-200/70 bg-canvas-100/40 p-4">
      <TextInput
        label={t("agent.provider.id")}
        ariaLabel={t("agent.provider.id")}
        value={draft.id}
        onChange={(value) => {
          if (existingId === null) update("id", value);
        }}
        readOnly={existingId !== null}
      />
      <TextInput
        label={t("agent.provider.endpoint")}
        ariaLabel={t("agent.provider.endpoint")}
        value={draft.endpoint_url}
        onChange={(value) => update("endpoint_url", value)}
      />
      <TextInput
        label={t("agent.provider.model")}
        ariaLabel={t("agent.provider.model")}
        value={draft.model}
        onChange={(value) => update("model", value)}
      />
      <div className="grid gap-1">
        <TextInput
          label={t("agent.provider.credentialEnvVar")}
          ariaLabel={t("agent.provider.credentialEnvVar")}
          value={draft.credential_env_var}
          onChange={(value) => update("credential_env_var", value)}
          placeholder={t("agent.provider.credentialPlaceholder")}
        />
        <small className="text-xs text-ink/55">
          {t("agent.provider.credentialHint")}
        </small>
      </div>
      <QuotaFields
        maxConcurrency={draft.max_concurrency ?? null}
        requestsPerMinute={draft.requests_per_minute ?? null}
        dailyTokenBudget={null}
        onMaxConcurrencyChange={(value) =>
          update("max_concurrency", value)
        }
        onRequestsPerMinuteChange={(value) =>
          update("requests_per_minute", value)
        }
        onDailyTokenBudgetChange={() =>
          update("daily_token_budget", null)
        }
        supportsDailyTokenBudget={false}
        unsupportedDailyTokenBudgetHint={t(
          "agent.moderationProvider.dailyTokenBudgetTextOnly",
        )}
      />
      <label className="inline-flex items-center gap-2 text-sm text-ink">
        <input
          type="checkbox"
          checked={draft.enabled}
          onChange={(event) => update("enabled", event.target.checked)}
        />
        {t("agent.provider.enabled")}
      </label>
      <div className="flex gap-2">
        <StudioButton
          variant="primary"
          onClick={() => onSave(draft, existingId)}
        >
          {t("agent.provider.save")}
        </StudioButton>
        <StudioButton onClick={onCancel}>
          {t("agent.provider.cancel")}
        </StudioButton>
      </div>
    </div>
  );
}
