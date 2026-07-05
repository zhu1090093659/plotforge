import { useCallback, useEffect, useState } from "react";
import {
  AgentSessionConfig,
  PermissionLevel,
  ProviderEntry,
  ProviderKind,
  PromptScope,
  PromptTemplate,
  SkillIndex,
  SkillManifest,
  ThinkingLevel,
} from "../../../contracts/plotforge";
import type { StudioDataSource } from "./studioDataSource";
import type { ProviderTestResult } from "./tauriBridge";
import {
  StudioButton,
  StudioPanel,
  StudioTabs,
  ViewHeader,
} from "./studioUi";
import { useStudioI18n } from "./i18n";

/**
 * The Agent configuration view: the single entry point for providers, model
 * settings, prompt templates, and skills. Per AGENTS.md, this view owns the
 * Agent configuration surface (LaunchpadView only surfaces a link here).
 *
 * The view keeps its own local state for the user-global lists (providers,
 * skills, prompt templates) and reloads them on mount. The per-project model
 * config (`AgentSessionConfig`) is passed in from `useAgentConfig` so the
 * persisted debounced save path stays single-source.
 */
export interface AgentViewProps {
  dataSource: StudioDataSource;
  loadedPath: string;
  agentConfig: AgentSessionConfig;
  onAgentConfigChange: (config: AgentSessionConfig) => void;
  configSaveError: string | null;
}

type AgentTab = "providers" | "model" | "prompts" | "skills";

const PROVIDER_KINDS: ProviderKind[] = [
  "openai_compatible",
  "openai_responses",
  "anthropic_messages",
];

const PERMISSIONS: PermissionLevel[] = ["full_access", "ask_every_time", "read_only"];
const THINKING: ThinkingLevel[] = ["high", "medium", "low", "off"];

const EMPTY_ENTRY: ProviderEntry = {
  id: "",
  kind: "openai_compatible",
  label: "",
  endpoint_url: "",
  model: "",
  credential_env_var: "",
  enabled: true,
};

export function AgentView({
  dataSource,
  loadedPath,
  agentConfig,
  onAgentConfigChange,
  configSaveError,
}: AgentViewProps) {
  const { t } = useStudioI18n();
  const [activeTab, setActiveTab] = useState<AgentTab>("providers");

  const [providers, setProviders] = useState<ProviderEntry[]>([]);
  const [providersLoading, setProvidersLoading] = useState(false);
  const [providerError, setProviderError] = useState<string | null>(null);
  const [editingEntry, setEditingEntry] = useState<ProviderEntry | null>(null);
  const [testResult, setTestResult] = useState<ProviderTestResult | null>(null);
  const [testing, setTesting] = useState(false);

  const [userPrompts, setUserPrompts] = useState<PromptTemplate[]>([]);
  const [projectPrompts, setProjectPrompts] = useState<PromptTemplate[]>([]);
  const [promptScope, setPromptScope] = useState<PromptScope>("user");

  const [skills, setSkills] = useState<SkillManifest[]>([]);
  const [skillsLoading, setSkillsLoading] = useState(false);

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
        loadedPath ? dataSource.listProjectPromptTemplates(loadedPath) : Promise.resolve([]),
      ]);
      setUserPrompts(user);
      setProjectPrompts(project);
    } catch {
      // Non-fatal: leave lists as-is.
    }
  }, [dataSource, loadedPath]);

  const reloadSkills = useCallback(async () => {
    setSkillsLoading(true);
    try {
      const list = await dataSource.listSkills();
      setSkills(list);
    } finally {
      setSkillsLoading(false);
    }
  }, [dataSource]);

  useEffect(() => {
    void reloadProviders();
    void reloadPrompts();
    void reloadSkills();
  }, [reloadProviders, reloadPrompts, reloadSkills]);

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

  const handleToggleSkill = async (skillId: string, enabled: boolean) => {
    if (!loadedPath) return;
    try {
      const updated = await dataSource.enableSkillForProject(loadedPath, skillId, enabled);
      onAgentConfigChange(updated);
    } catch {
      // Non-fatal; the UI will reflect the persisted state on next load.
    }
  };

  const handleImportSkill = async (skillId: string) => {
    try {
      await dataSource.importSkill(skillId);
      await reloadSkills();
    } catch {
      // Non-fatal.
    }
  };

  const handleRefreshSkills = async () => {
    try {
      const index: SkillIndex = await dataSource.refreshSkillIndex();
      setSkills(index.skills);
    } finally {
      setSkillsLoading(false);
    }
  };

  const tabs = [
    {
      id: "providers" as const,
      label: t("agent.tab.providers"),
      children: (
        <ProvidersTab
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
      ),
    },
    {
      id: "model" as const,
      label: t("agent.tab.model"),
      children: (
        <ModelTab
          agentConfig={agentConfig}
          onAgentConfigChange={onAgentConfigChange}
          saveError={configSaveError}
        />
      ),
    },
    {
      id: "prompts" as const,
      label: t("agent.tab.prompts"),
      children: (
        <PromptsTab
          userPrompts={userPrompts}
          projectPrompts={projectPrompts}
          scope={promptScope}
          onScopeChange={setPromptScope}
        />
      ),
    },
    {
      id: "skills" as const,
      label: t("agent.tab.skills"),
      badge: skills.length,
      children: (
        <SkillsTab
          skills={skills}
          loading={skillsLoading}
          enabledSkillIds={agentConfig.enabled_skills}
          onToggle={handleToggleSkill}
          onImport={handleImportSkill}
          onRefresh={handleRefreshSkills}
          dataSource={dataSource}
        />
      ),
    },
  ];

  return (
    <div>
      <ViewHeader
        eyebrow="12"
        title={t("nav.agent.label")}
        subtitle={t("nav.agent.description")}
      />
      <StudioTabs
        ariaLabel={t("nav.agent.label")}
        items={tabs}
        activeId={activeTab}
        onActiveChange={(id) => setActiveTab(id as AgentTab)}
      />
    </div>
  );
}

// ---------------------------------------------------------------------------
// Providers tab
// ---------------------------------------------------------------------------

interface ProvidersTabProps {
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

function ProvidersTab({
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
}: ProvidersTabProps) {
  const { t } = useStudioI18n();
  if (editingEntry) {
    return (
      <ProviderEditor
        entry={editingEntry}
        onCancel={onCancelEdit}
        onSave={onSave}
      />
    );
  }
  return (
    <StudioPanel>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "0.75rem" }}>
        <strong>{t("agent.provider.id")}</strong>
        <StudioButton variant="primary" onClick={onAdd}>
          {t("agent.provider.add")}
        </StudioButton>
      </div>
      {error && (
        <div role="alert" style={{ marginTop: "0.5rem", padding: "0.5rem", color: "var(--studio-signal, #c0392b)" }}>
          {error}
        </div>
      )}
      {loading ? (
        <div style={{ padding: "1rem", opacity: 0.6 }}>…</div>
      ) : providers.length === 0 ? (
        <div style={{ padding: "1rem", opacity: 0.6 }}>{t("agent.provider.add")}</div>
      ) : (
        <table style={{ width: "100%", borderCollapse: "collapse" }}>
          <thead>
            <tr>
              <th style={{ textAlign: "left", padding: "0.25rem 0.5rem" }}>{t("agent.provider.id")}</th>
              <th style={{ textAlign: "left", padding: "0.25rem 0.5rem" }}>{t("agent.provider.label")}</th>
              <th style={{ textAlign: "left", padding: "0.25rem 0.5rem" }}>{t("agent.provider.kind")}</th>
              <th style={{ textAlign: "left", padding: "0.25rem 0.5rem" }}>{t("agent.provider.model")}</th>
              <th style={{ textAlign: "left", padding: "0.25rem 0.5rem" }}>{t("agent.provider.enabled")}</th>
              <th style={{ padding: "0.25rem 0.5rem" }}></th>
            </tr>
          </thead>
          <tbody>
            {providers.map((entry) => (
              <tr key={entry.id}>
                <td style={{ padding: "0.25rem 0.5rem" }}>{entry.id}</td>
                <td style={{ padding: "0.25rem 0.5rem" }}>{entry.label}</td>
                <td style={{ padding: "0.25rem 0.5rem" }}>{kindLabel(entry.kind, t)}</td>
                <td style={{ padding: "0.25rem 0.5rem" }}>{entry.model}</td>
                <td style={{ padding: "0.25rem 0.5rem" }}>{entry.enabled ? "✓" : "—"}</td>
                <td style={{ padding: "0.25rem 0.5rem", display: "flex", gap: "0.25rem" }}>
                  <StudioButton onClick={() => onEdit(entry)}>{t("agent.provider.edit")}</StudioButton>
                  <StudioButton onClick={() => onDelete(entry.id)}>{t("agent.provider.delete")}</StudioButton>
                  <StudioButton onClick={() => onTest(entry.id)} disabled={testing}>
                    {t("agent.provider.test")}
                  </StudioButton>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
      {testResult && (
        <div
          role="status"
          style={{
            marginTop: "0.5rem",
            padding: "0.5rem",
            color: testResult.ok ? "var(--studio-sage, #2e7d32)" : "var(--studio-signal, #c0392b)",
          }}
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
  entry: ProviderEntry;
  onCancel: () => void;
  onSave: (entry: ProviderEntry) => void;
}

function ProviderEditor({ entry, onCancel, onSave }: ProviderEditorProps) {
  const { t } = useStudioI18n();
  const [draft, setDraft] = useState<ProviderEntry>(entry);
  const update = <K extends keyof ProviderEntry>(key: K, value: ProviderEntry[K]) =>
    setDraft((prev) => ({ ...prev, [key]: value }));
  return (
    <StudioPanel>
      <div style={{ display: "grid", gap: "0.75rem", maxWidth: "32rem" }}>
        <label style={{ display: "grid", gap: "0.25rem" }}>
          {t("agent.provider.id")}
          <input
            value={draft.id}
            onChange={(e) => update("id", e.target.value)}
            aria-label={t("agent.provider.id")}
          />
        </label>
        <label style={{ display: "grid", gap: "0.25rem" }}>
          {t("agent.provider.label")}
          <input
            value={draft.label}
            onChange={(e) => update("label", e.target.value)}
            aria-label={t("agent.provider.label")}
          />
        </label>
        <label style={{ display: "grid", gap: "0.25rem" }}>
          {t("agent.provider.kind")}
          <select
            value={draft.kind}
            onChange={(e) => update("kind", e.target.value as ProviderKind)}
            aria-label={t("agent.provider.kind")}
          >
            {PROVIDER_KINDS.map((k) => (
              <option key={k} value={k}>
                {kindLabel(k, t)}
              </option>
            ))}
          </select>
        </label>
        <label style={{ display: "grid", gap: "0.25rem" }}>
          {t("agent.provider.endpoint")}
          <input
            value={draft.endpoint_url}
            onChange={(e) => update("endpoint_url", e.target.value)}
            aria-label={t("agent.provider.endpoint")}
          />
        </label>
        <label style={{ display: "grid", gap: "0.25rem" }}>
          {t("agent.provider.model")}
          <input
            value={draft.model}
            onChange={(e) => update("model", e.target.value)}
            aria-label={t("agent.provider.model")}
          />
        </label>
        <label style={{ display: "grid", gap: "0.25rem" }}>
          {t("agent.provider.credentialEnvVar")}
          <input
            value={draft.credential_env_var}
            onChange={(e) => update("credential_env_var", e.target.value)}
            aria-label={t("agent.provider.credentialEnvVar")}
            placeholder="e.g. OPENAI_API_KEY"
          />
          <small style={{ opacity: 0.7 }}>
            {t("agent.provider.credentialHint")}
          </small>
        </label>
        <label>
          <input
            type="checkbox"
            checked={draft.enabled}
            onChange={(e) => update("enabled", e.target.checked)}
          />{" "}
          {t("agent.provider.enabled")}
        </label>
        <div style={{ display: "flex", gap: "0.5rem" }}>
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
// Model tab
// ---------------------------------------------------------------------------

interface ModelTabProps {
  agentConfig: AgentSessionConfig;
  onAgentConfigChange: (config: AgentSessionConfig) => void;
  saveError: string | null;
}

function ModelTab({ agentConfig, onAgentConfigChange, saveError }: ModelTabProps) {
  const { t } = useStudioI18n();
  const update = <K extends keyof AgentSessionConfig>(key: K, value: AgentSessionConfig[K]) =>
    onAgentConfigChange({ ...agentConfig, [key]: value });
  return (
    <StudioPanel>
      <div style={{ display: "grid", gap: "0.75rem", maxWidth: "32rem" }}>
        <label style={{ display: "grid", gap: "0.25rem" }}>
          {t("home.modelLabel")}
          <select
            value={agentConfig.model_id}
            onChange={(e) => update("model_id", e.target.value)}
            aria-label={t("home.modelLabel")}
          >
            <option value="local-pi">Local pi-Agent (mock)</option>
          </select>
        </label>
        <label style={{ display: "grid", gap: "0.25rem" }}>
          {t("home.permissionLabel")}
          <select
            value={agentConfig.permission_level}
            onChange={(e) => update("permission_level", e.target.value as PermissionLevel)}
            aria-label={t("home.permissionLabel")}
          >
            {PERMISSIONS.map((p) => (
              <option key={p} value={p}>
                {p}
              </option>
            ))}
          </select>
        </label>
        <label style={{ display: "grid", gap: "0.25rem" }}>
          {t("home.thinkingLabel")}
          <select
            value={agentConfig.thinking_level}
            onChange={(e) => update("thinking_level", e.target.value as ThinkingLevel)}
            aria-label={t("home.thinkingLabel")}
          >
            {THINKING.map((level) => (
              <option key={level} value={level}>
                {level}
              </option>
            ))}
          </select>
        </label>
        {saveError && (
          <div role="alert" style={{ color: "var(--studio-signal, #c0392b)" }}>
            {saveError}
          </div>
        )}
      </div>
    </StudioPanel>
  );
}

// ---------------------------------------------------------------------------
// Prompts tab
// ---------------------------------------------------------------------------

interface PromptsTabProps {
  userPrompts: PromptTemplate[];
  projectPrompts: PromptTemplate[];
  scope: PromptScope;
  onScopeChange: (scope: PromptScope) => void;
}

function PromptsTab({ userPrompts, projectPrompts, scope, onScopeChange }: PromptsTabProps) {
  const { t } = useStudioI18n();
  const list = scope === "user" ? userPrompts : projectPrompts;
  return (
    <StudioPanel>
      <div style={{ display: "flex", gap: "0.5rem", marginBottom: "0.75rem" }}>
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
        <div style={{ padding: "1rem", opacity: 0.6 }}>{t("agent.prompt.add")}</div>
      ) : (
        <ul style={{ listStyle: "none", padding: 0, display: "grid", gap: "0.5rem" }}>
          {list.map((template) => (
            <li key={template.id} style={{ border: "1px solid var(--studio-border)", padding: "0.5rem" }}>
              <strong>{template.label}</strong>
              <textarea
                aria-label={t("agent.prompt.body")}
                value={template.body_markdown}
                readOnly
                style={{ marginTop: "0.25rem", minHeight: "4rem", width: "100%" }}
              />
            </li>
          ))}
        </ul>
      )}
    </StudioPanel>
  );
}

// ---------------------------------------------------------------------------
// Skills tab
// ---------------------------------------------------------------------------

interface SkillsTabProps {
  skills: SkillManifest[];
  loading: boolean;
  enabledSkillIds: string[];
  onToggle: (skillId: string, enabled: boolean) => void;
  onImport: (skillId: string) => void;
  onRefresh: () => void;
  dataSource: StudioDataSource;
}

function SkillsTab({
  skills,
  loading,
  enabledSkillIds,
  onToggle,
  onImport,
  onRefresh,
  dataSource,
}: SkillsTabProps) {
  const { t } = useStudioI18n();
  const [bodySkillId, setBodySkillId] = useState<string | null>(null);
  const [body, setBody] = useState<string>("");
  const [bodyLoading, setBodyLoading] = useState(false);

  const handleViewBody = async (skillId: string) => {
    if (bodySkillId === skillId) {
      setBodySkillId(null);
      return;
    }
    setBodySkillId(skillId);
    setBodyLoading(true);
    try {
      const text = await dataSource.readSkillBody(skillId);
      setBody(text);
    } catch {
      setBody("");
    } finally {
      setBodyLoading(false);
    }
  };

  return (
    <StudioPanel>
      <div style={{ display: "flex", justifyContent: "flex-end", marginBottom: "0.75rem" }}>
        <StudioButton onClick={onRefresh} disabled={loading}>
          {t("agent.skill.refresh")}
        </StudioButton>
      </div>
      {loading ? (
        <div style={{ padding: "1rem", opacity: 0.6 }}>…</div>
      ) : skills.length === 0 ? (
        <div style={{ padding: "1rem", opacity: 0.6 }}>{t("agent.skill.refresh")}</div>
      ) : (
        <ul style={{ listStyle: "none", padding: 0, display: "grid", gap: "0.5rem" }}>
          {skills.map((skill) => {
            const enabled = enabledSkillIds.includes(skill.id);
            // Skills already in the user's own `~/.plotforge/skills/`
            // library (`plot_forge_user` origin) cannot be re-imported
            // (`import_external_skill` errors with "destination already
            // exists"). Hide the Import button for those so the click is
            // never offered as actionable. Finding L2.
            const importable = skill.source.origin !== "plot_forge_user";
            return (
              <li
                key={`${skill.source.origin}:${skill.id}`}
                style={{ border: "1px solid var(--studio-border)", padding: "0.5rem" }}
              >
                <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
                  <div>
                    <strong>{skill.name}</strong>{" "}
                    <small style={{ opacity: 0.7 }}>[{originLabel(skill.source.origin, t)}]</small>
                    <div style={{ opacity: 0.8 }}>{skill.description}</div>
                  </div>
                  <div style={{ display: "flex", gap: "0.25rem" }}>
                    <label>
                      <input
                        type="checkbox"
                        checked={enabled}
                        onChange={(e) => onToggle(skill.id, e.target.checked)}
                      />{" "}
                      {t("agent.skill.enable")}
                    </label>
                    {importable && (
                      <StudioButton onClick={() => onImport(skill.id)}>
                        {t("agent.skill.import")}
                      </StudioButton>
                    )}
                    <StudioButton onClick={() => void handleViewBody(skill.id)}>
                      {t("agent.skill.viewBody")}
                    </StudioButton>
                  </div>
                </div>
                {bodySkillId === skill.id && (
                  <textarea
                    aria-label={t("agent.skill.viewBody")}
                    value={bodyLoading ? "…" : body}
                    readOnly
                    style={{ marginTop: "0.25rem", minHeight: "6rem", width: "100%" }}
                  />
                )}
              </li>
            );
          })}
        </ul>
      )}
    </StudioPanel>
  );
}

function originLabel(origin: SkillManifest["source"]["origin"], t: (key: string) => string): string {
  switch (origin) {
    case "plot_forge_user":
      return t("agent.skill.origin.plotForgeUser");
    case "claude_code":
      return t("agent.skill.origin.claudeCode");
    case "codex":
      return t("agent.skill.origin.codex");
    case "z_code":
      return t("agent.skill.origin.zCode");
    default:
      return t("agent.skill.origin.other");
  }
}
