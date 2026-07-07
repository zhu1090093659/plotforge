import { useCallback, useEffect, useState } from "react";
import type {
  AgentSessionConfig,
  ProviderEntry,
  ProviderKind,
  PromptScope,
  PromptTemplate,
} from "../../../contracts/plotforge";
import type { StudioDataSource } from "./studioDataSource";
import type { ProviderTestResult } from "./tauriBridge";
import {
  ModelCombobox,
  PermissionSelect,
  ThinkingSelect,
} from "./agentConfigSelectors";
import {
  EmptyState,
  SaveButton,
  StudioButton,
  StudioPanel,
  TextInput,
} from "./studioUi";
import { useStudioI18n } from "./i18n";

// ---------------------------------------------------------------------------
// AgentConfigSection — the "Agent" tab inside SettingsView.
//
// Covers Providers / Model / Prompts (the three Agent-config surfaces). This
// was split out of the old AgentView and rewritten from inline `style=` to
// Tailwind + `studioUiClassNames` (B4) so the Settings view reads as the same
// application as the rest of the shell. The Model area uses the shared
// selectors from ./agentConfigSelectors so it matches the home toolbar (B3).
// ---------------------------------------------------------------------------

const PROVIDER_KINDS: ProviderKind[] = [
  "openai_compatible",
  "openai_responses",
  "anthropic_messages",
];

const EMPTY_ENTRY: ProviderEntry = {
  id: "",
  kind: "openai_compatible",
  label: "",
  endpoint_url: "",
  model: "",
  credential_env_var: "",
  enabled: true,
};

export interface AgentConfigSectionProps {
  dataSource: StudioDataSource;
  loadedPath: string;
  agentConfig: AgentSessionConfig;
  onAgentConfigChange: (config: AgentSessionConfig) => void;
  configSaveError: string | null;
}

export function AgentConfigSection({
  dataSource,
  loadedPath,
  agentConfig,
  onAgentConfigChange,
  configSaveError,
}: AgentConfigSectionProps) {
  const { t } = useStudioI18n();

  const [providers, setProviders] = useState<ProviderEntry[]>([]);
  const [providersLoading, setProvidersLoading] = useState(false);
  const [providerError, setProviderError] = useState<string | null>(null);
  const [editingEntry, setEditingEntry] = useState<ProviderEntry | null>(null);
  const [testResult, setTestResult] = useState<ProviderTestResult | null>(null);
  const [testing, setTesting] = useState(false);

  const [userPrompts, setUserPrompts] = useState<PromptTemplate[]>([]);
  const [projectPrompts, setProjectPrompts] = useState<PromptTemplate[]>([]);
  const [promptScope, setPromptScope] = useState<PromptScope>("user");

  const reloadProviders = useCallback(async () => {
    setProvidersLoading(true);
    setProviderError(null);
    try {
      const list = await dataSource.listProviders();
      setProviders(list);
    } catch (error) {
      setProviderError(error instanceof Error ? error.message : String(error));
    } finally {
      setProvidersLoading(false);
    }
  }, [dataSource]);

  const reloadPrompts = useCallback(async () => {
    try {
      const [user, project] = await Promise.all([
        dataSource.listUserPromptTemplates(),
        loadedPath
          ? dataSource.listProjectPromptTemplates(loadedPath)
          : Promise.resolve([]),
      ]);
      setUserPrompts(user);
      setProjectPrompts(project);
    } catch {
      // Non-fatal: leave lists as-is.
    }
  }, [dataSource, loadedPath]);

  useEffect(() => {
    void reloadProviders();
    void reloadPrompts();
  }, [reloadProviders, reloadPrompts]);

  const handleSaveEntry = async (entry: ProviderEntry) => {
    try {
      await dataSource.upsertProvider(entry);
      setEditingEntry(null);
      await reloadProviders();
    } catch (error) {
      setProviderError(error instanceof Error ? error.message : String(error));
    }
  };

  const handleDeleteEntry = async (id: string) => {
    try {
      await dataSource.deleteProvider(id);
      await reloadProviders();
    } catch (error) {
      setProviderError(error instanceof Error ? error.message : String(error));
    }
  };

  const handleTestConnection = async (id: string) => {
    setTesting(true);
    setTestResult(null);
    try {
      const result = await dataSource.testProviderConnection(id);
      setTestResult(result);
    } catch (error) {
      setTestResult({
        ok: false,
        message: error instanceof Error ? error.message : String(error),
      });
    } finally {
      setTesting(false);
    }
  };

  return (
    <div className="grid gap-6">
      <ProvidersArea
        dataSource={dataSource}
        providers={providers}
        loading={providersLoading}
        error={providerError}
        editingEntry={editingEntry}
        onEdit={setEditingEntry}
        onCancelEdit={() => setEditingEntry(null)}
        onSave={handleSaveEntry}
        onDelete={handleDeleteEntry}
        onTest={handleTestConnection}
        testing={testing}
        testResult={testResult}
        onAdd={() => setEditingEntry({ ...EMPTY_ENTRY })}
      />
      <ModelArea
        agentConfig={agentConfig}
        onAgentConfigChange={onAgentConfigChange}
        saveError={configSaveError}
      />
      <PromptsArea
        userPrompts={userPrompts}
        projectPrompts={projectPrompts}
        scope={promptScope}
        onScopeChange={setPromptScope}
      />
    </div>
  );
}

// ---------------------------------------------------------------------------
// Providers area
// ---------------------------------------------------------------------------

interface ProvidersAreaProps {
  dataSource: StudioDataSource;
  providers: ProviderEntry[];
  loading: boolean;
  error: string | null;
  editingEntry: ProviderEntry | null;
  onEdit: (entry: ProviderEntry) => void;
  onCancelEdit: () => void;
  onSave: (entry: ProviderEntry) => void;
  onDelete: (id: string) => void;
  onTest: (id: string) => void;
  testing: boolean;
  testResult: ProviderTestResult | null;
  onAdd: () => void;
}

function ProvidersArea({
  dataSource,
  providers,
  loading,
  error,
  editingEntry,
  onEdit,
  onCancelEdit,
  onSave,
  onDelete,
  onTest,
  testing,
  testResult,
  onAdd,
}: ProvidersAreaProps) {
  const { t } = useStudioI18n();
  return (
    <StudioPanel>
      <div className="mb-3 flex items-center justify-between gap-2">
        <h4 className="font-display text-lg font-semibold tracking-display-tight text-ink">
          {t("agent.provider.id")}
        </h4>
        <StudioButton variant="primary" onClick={onAdd}>
          {t("agent.provider.add")}
        </StudioButton>
      </div>
      {error && (
        <div
          role="alert"
          className="mt-2 rounded-md border border-signal/30 bg-signal/10 px-3 py-2 text-sm text-signal"
        >
          {error}
        </div>
      )}
      {editingEntry ? (
        <ProviderEditor
          dataSource={dataSource}
          entry={editingEntry}
          onCancel={onCancelEdit}
          onSave={onSave}
        />
      ) : loading ? (
        <EmptyState>…</EmptyState>
      ) : providers.length === 0 ? (
        <EmptyState>{t("agent.provider.add")}</EmptyState>
      ) : (
        <div className="mt-2 overflow-x-auto">
          <table className="w-full border-collapse text-sm">
            <thead>
              <tr className="text-left text-xs font-semibold uppercase tracking-eyebrow text-ink/55">
                <th className="px-2 py-1.5">{t("agent.provider.id")}</th>
                <th className="px-2 py-1.5">{t("agent.provider.label")}</th>
                <th className="px-2 py-1.5">{t("agent.provider.kind")}</th>
                <th className="px-2 py-1.5">{t("agent.provider.model")}</th>
                <th className="px-2 py-1.5">{t("agent.provider.enabled")}</th>
                <th className="px-2 py-1.5" aria-label={t("agent.provider.edit")} />
              </tr>
            </thead>
            <tbody>
              {providers.map((entry) => (
                <tr key={entry.id} className="border-t border-canvas-200/55">
                  <td className="px-2 py-1.5">{entry.id}</td>
                  <td className="px-2 py-1.5">{entry.label}</td>
                  <td className="px-2 py-1.5">{kindLabel(entry.kind, t)}</td>
                  <td className="px-2 py-1.5">{entry.model}</td>
                  <td className="px-2 py-1.5">{entry.enabled ? "✓" : "—"}</td>
                  <td className="px-2 py-1.5">
                    <div className="flex gap-1.5">
                      <StudioButton onClick={() => onEdit(entry)}>
                        {t("agent.provider.edit")}
                      </StudioButton>
                      <StudioButton onClick={() => onDelete(entry.id)}>
                        {t("agent.provider.delete")}
                      </StudioButton>
                      <StudioButton onClick={() => onTest(entry.id)} disabled={testing}>
                        {t("agent.provider.test")}
                      </StudioButton>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      {testResult && (
        <div
          role="status"
          className={[
            "mt-3 rounded-md border px-3 py-2 text-sm",
            testResult.ok
              ? "border-sage/30 bg-sage/10 text-sage"
              : "border-signal/30 bg-signal/10 text-signal",
          ].join(" ")}
        >
          {testResult.ok ? t("agent.provider.test.ok") : t("agent.provider.test.failed")}
          {" "}
          {testResult.message}
        </div>
      )}
    </StudioPanel>
  );
}

function kindLabel(kind: ProviderKind, t: (key: string) => string): string {
  switch (kind) {
    case "openai_compatible":
      return t("agent.provider.kind.openaiCompatible");
    case "openai_responses":
      return t("agent.provider.kind.openaiResponses");
    case "anthropic_messages":
      return t("agent.provider.kind.anthropicMessages");
    default:
      return kind;
  }
}

interface ProviderEditorProps {
  dataSource: StudioDataSource;
  entry: ProviderEntry;
  onCancel: () => void;
  onSave: (entry: ProviderEntry) => void;
}

function ProviderEditor({ dataSource, entry, onCancel, onSave }: ProviderEditorProps) {
  const { t } = useStudioI18n();
  const [draft, setDraft] = useState<ProviderEntry>(entry);
  const update = <K extends keyof ProviderEntry>(key: K, value: ProviderEntry[K]) =>
    setDraft((prev) => ({ ...prev, [key]: value }));
  // The provider id must be set before its upstream models can be fetched.
  // The combobox's Fetch button is only enabled once an id is present; an
  // empty id would surface a `provider_not_found` error otherwise.
  const canFetch = draft.id.trim().length > 0;
  return (
    <StudioPanel className="mt-3">
      <div className="grid max-w-xl gap-3">
        <TextInput
          label={t("agent.provider.id")}
          ariaLabel={t("agent.provider.id")}
          value={draft.id}
          onChange={(v) => update("id", v)}
        />
        <TextInput
          label={t("agent.provider.label")}
          ariaLabel={t("agent.provider.label")}
          value={draft.label}
          onChange={(v) => update("label", v)}
        />
        <label className="grid gap-1">
          <span className="text-xs font-semibold uppercase tracking-eyebrow text-ink/55">
            {t("agent.provider.kind")}
          </span>
          <select
            aria-label={t("agent.provider.kind")}
            value={draft.kind}
            onChange={(e) => update("kind", e.target.value as ProviderKind)}
            className="h-10 min-w-0 rounded-md border border-canvas-200 bg-canvas-50 px-3 text-sm text-ink outline-none transition ease-expo focus:border-accent-400 focus:ring-1 focus:ring-accent-400/30"
          >
            {PROVIDER_KINDS.map((k) => (
              <option key={k} value={k}>
                {kindLabel(k, t)}
              </option>
            ))}
          </select>
        </label>
        <TextInput
          label={t("agent.provider.endpoint")}
          ariaLabel={t("agent.provider.endpoint")}
          value={draft.endpoint_url}
          onChange={(v) => update("endpoint_url", v)}
        />
        <div className="grid gap-1">
          <span className="text-xs font-semibold uppercase tracking-eyebrow text-ink/55">
            {t("agent.provider.model")}
          </span>
          {canFetch ? (
            <ModelCombobox
              ariaLabel={t("agent.provider.model")}
              value={draft.model}
              onChange={(v) => update("model", v)}
              placeholder={t("agent.provider.modelPlaceholder")}
              fetch={{ dataSource, providerId: draft.id }}
            />
          ) : (
            <input
              aria-label={t("agent.provider.model")}
              value={draft.model}
              onChange={(e) => update("model", e.target.value)}
              placeholder={t("agent.provider.modelPlaceholder")}
              autoComplete="off"
              className="h-10 min-w-0 rounded-md border border-canvas-200 bg-canvas-50 px-3 text-sm text-ink outline-none transition ease-expo focus:border-accent-400 focus:ring-1 focus:ring-accent-400/30"
            />
          )}
          <small className="text-xs text-ink/55">{t("agent.provider.modelHint")}</small>
        </div>
        <div className="grid gap-1">
          <TextInput
            label={t("agent.provider.credentialEnvVar")}
            ariaLabel={t("agent.provider.credentialEnvVar")}
            value={draft.credential_env_var}
            onChange={(v) => update("credential_env_var", v)}
            placeholder="e.g. OPENAI_API_KEY"
          />
          <small className="text-xs text-ink/55">{t("agent.provider.credentialHint")}</small>
        </div>
        <label className="grid gap-1">
          <span className="text-xs font-semibold uppercase tracking-eyebrow text-ink/55">
            {t("agent.provider.maxOutputTokens")}
          </span>
          <input
            type="number"
            min={1}
            aria-label={t("agent.provider.maxOutputTokens")}
            value={draft.max_output_tokens ?? ""}
            onChange={(e) =>
              update(
                "max_output_tokens",
                e.target.value === "" ? null : Number(e.target.value),
              )
            }
            placeholder={t("agent.provider.maxOutputTokensPlaceholder")}
            className="h-10 min-w-0 rounded-md border border-canvas-200 bg-canvas-50 px-3 text-sm text-ink outline-none transition ease-expo focus:border-accent-400 focus:ring-1 focus:ring-accent-400/30"
          />
          <small className="text-xs text-ink/55">{t("agent.provider.maxOutputTokensHint")}</small>
        </label>
        <label className="inline-flex items-center gap-2 text-sm text-ink">
          <input
            type="checkbox"
            checked={draft.enabled}
            onChange={(e) => update("enabled", e.target.checked)}
          />
          {t("agent.provider.enabled")}
        </label>
        <div className="flex gap-2">
          <StudioButton variant="primary" onClick={() => onSave(draft)}>
            {t("agent.provider.edit")}
          </StudioButton>
          <StudioButton onClick={onCancel}>Cancel</StudioButton>
        </div>
      </div>
    </StudioPanel>
  );
}

// ---------------------------------------------------------------------------
// Model area — uses the shared selectors so the Settings → Agent tab and the
// home toolbar render the same controls for the same AgentSessionConfig.
// ---------------------------------------------------------------------------

interface ModelAreaProps {
  agentConfig: AgentSessionConfig;
  onAgentConfigChange: (config: AgentSessionConfig) => void;
  saveError: string | null;
}

function ModelArea({ agentConfig, onAgentConfigChange, saveError }: ModelAreaProps) {
  const { t } = useStudioI18n();
  const update = <K extends keyof AgentSessionConfig>(key: K, value: AgentSessionConfig[K]) =>
    onAgentConfigChange({ ...agentConfig, [key]: value });
  return (
    <StudioPanel>
      <h4 className="mb-3 font-display text-lg font-semibold tracking-display-tight text-ink">
        {t("home.modelLabel")}
      </h4>
      <div className="grid max-w-xl gap-3">
        <label className="grid gap-1">
          <span className="text-xs font-semibold uppercase tracking-eyebrow text-ink/55">
            {t("home.modelLabel")}
          </span>
          <select
            aria-label={t("home.modelLabel")}
            value={agentConfig.model_id}
            onChange={(e) => update("model_id", e.target.value)}
            className="h-10 min-w-0 rounded-md border border-canvas-200 bg-canvas-50 px-3 text-sm text-ink outline-none transition ease-expo focus:border-accent-400 focus:ring-1 focus:ring-accent-400/30"
          >
            <option value="local-pi">Local pi-Agent (mock)</option>
          </select>
        </label>
        <div className="grid gap-1">
          <span className="text-xs font-semibold uppercase tracking-eyebrow text-ink/55">
            {t("home.permissionLabel")}
          </span>
          <PermissionSelect
            value={agentConfig.permission_level}
            onChange={(permission_level) =>
              onAgentConfigChange({ ...agentConfig, permission_level })
            }
          />
        </div>
        <div className="grid gap-1">
          <span className="text-xs font-semibold uppercase tracking-eyebrow text-ink/55">
            {t("home.thinkingLabel")}
          </span>
          <ThinkingSelect
            value={agentConfig.thinking_level}
            onChange={(thinking_level) =>
              onAgentConfigChange({ ...agentConfig, thinking_level })
            }
          />
        </div>
        {saveError && (
          <div role="alert" className="rounded-md border border-signal/30 bg-signal/10 px-3 py-2 text-sm text-signal">
            {saveError}
          </div>
        )}
      </div>
    </StudioPanel>
  );
}

// ---------------------------------------------------------------------------
// Prompts area
// ---------------------------------------------------------------------------

interface PromptsAreaProps {
  userPrompts: PromptTemplate[];
  projectPrompts: PromptTemplate[];
  scope: PromptScope;
  onScopeChange: (scope: PromptScope) => void;
}

function PromptsArea({ userPrompts, projectPrompts, scope, onScopeChange }: PromptsAreaProps) {
  const { t } = useStudioI18n();
  const list = scope === "user" ? userPrompts : projectPrompts;
  return (
    <StudioPanel>
      <h4 className="mb-3 font-display text-lg font-semibold tracking-display-tight text-ink">
        {t("agent.tab.prompts")}
      </h4>
      <div className="mb-3 flex gap-2">
        <StudioButton
          variant={scope === "user" ? "primary" : "secondary"}
          onClick={() => onScopeChange("user")}
        >
          {t("agent.prompt.scope.user")}
        </StudioButton>
        <StudioButton
          variant={scope === "project" ? "primary" : "secondary"}
          onClick={() => onScopeChange("project")}
        >
          {t("agent.prompt.scope.project")}
        </StudioButton>
      </div>
      {list.length === 0 ? (
        <EmptyState>{t("agent.prompt.add")}</EmptyState>
      ) : (
        <ul className="grid gap-2">
          {list.map((template) => (
            <li
              key={template.id}
              className="rounded-md border border-canvas-200/70 bg-canvas-100/40 p-3"
            >
              <strong className="font-semibold text-ink">{template.label}</strong>
              <textarea
                aria-label={t("agent.prompt.body")}
                value={template.body_markdown}
                readOnly
                className="mt-2 min-h-16 w-full resize-none rounded-md border border-canvas-200 bg-canvas-50 px-3 py-2 text-sm leading-6 text-ink"
              />
            </li>
          ))}
        </ul>
      )}
    </StudioPanel>
  );
}

// Re-export SaveButton for callers that want a primary save affordance; kept
// here so the Settings → Agent tab can re-use the studio-wide save style
// without re-importing from studioUi in the future.
export { SaveButton };
