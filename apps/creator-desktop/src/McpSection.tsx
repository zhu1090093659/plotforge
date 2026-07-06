import { useCallback, useEffect, useState } from "react";
import type {
  AgentSessionConfig,
  McpServerEntry,
  McpServerTestResult,
  McpToolCallRequest,
  McpToolCallResult,
  McpToolManifest,
  McpTransportConfig,
  McpTransportKind,
} from "../../../contracts/plotforge";
import type { StudioDataSource } from "./studioDataSource";
import {
  EmptyState,
  StudioButton,
  StudioPanel,
  TextInput,
} from "./studioUi";
import { useStudioI18n } from "./i18n";

// ---------------------------------------------------------------------------
// McpSection — the "MCP" tab inside SettingsView.
//
// Real server management UI for the user-global MCP server registry. Mirrors
// the established AgentConfigSection / SkillsSection patterns: useState/useEffect
// loading, error states, an editing form, test-connection results, a tool list,
// and per-project enablement. The backend (plotforge-studio MCP commands) does
// all transport / lifecycle / redaction work; this component is orchestration +
// rendering only — no Rust business logic is reimplemented here.
//
// Security rule (AGENTS.md): the only credential surface is a plain text input
// bound to the env-var NAME (`credential_env_var`), with a placeholder like
// `e.g. MCP_TOKEN`. The UI must never display or collect secret values, so the
// input is `type="text"` (never a password field) and no credential value ever
// round-trips through the registry.
//
// Copy rule: UI strings must not promise tool outcomes, automatic publishing,
// platform approval, or legal conclusions (no_launch_promise_lint.py enforced).
// ---------------------------------------------------------------------------

const TRANSPORT_KINDS: McpTransportKind[] = ["stdio", "sse", "http"];

/** Build an empty stdio entry draft for the Add button. */
function emptyEntry(): McpServerEntry {
  return {
    id: "",
    kind: "stdio",
    label: "",
    transport_config: { kind: "stdio", command: "", args: [], env: {} },
    credential_env_var: "",
    enabled: true,
  };
}

export interface McpSectionProps {
  dataSource: StudioDataSource;
  loadedPath: string;
  agentConfig: AgentSessionConfig;
  onAgentConfigChange: (config: AgentSessionConfig) => void;
}

export function McpSection({
  dataSource,
  loadedPath,
  agentConfig,
  onAgentConfigChange,
}: McpSectionProps) {
  const { t } = useStudioI18n();

  const [servers, setServers] = useState<McpServerEntry[]>([]);
  const [serversLoading, setServersLoading] = useState(false);
  const [serverError, setServerError] = useState<string | null>(null);
  const [editingEntry, setEditingEntry] = useState<McpServerEntry | null>(null);

  // Per-server test-connection results, keyed by server id.
  const [testResults, setTestResults] = useState<
    Record<string, McpServerTestResult>
  >({});
  const [testingId, setTestingId] = useState<string | null>(null);

  const reloadServers = useCallback(async () => {
    setServersLoading(true);
    setServerError(null);
    try {
      const list = await dataSource.listMcpServers();
      setServers(list);
    } catch (error) {
      setServerError(error instanceof Error ? error.message : String(error));
    } finally {
      setServersLoading(false);
    }
  }, [dataSource]);

  useEffect(() => {
    void reloadServers();
  }, [reloadServers]);

  const handleSaveEntry = async (entry: McpServerEntry) => {
    try {
      await dataSource.upsertMcpServer(entry);
      setEditingEntry(null);
      await reloadServers();
    } catch (error) {
      setServerError(error instanceof Error ? error.message : String(error));
    }
  };

  const handleDeleteEntry = async (id: string) => {
    try {
      await dataSource.deleteMcpServer(id);
      await reloadServers();
    } catch (error) {
      setServerError(error instanceof Error ? error.message : String(error));
    }
  };

  const handleTestConnection = async (id: string) => {
    setTestingId(id);
    setTestResults((prev) => {
      const next = { ...prev };
      delete next[id];
      return next;
    });
    try {
      const result = await dataSource.testMcpServer(id);
      setTestResults((prev) => ({ ...prev, [id]: result }));
    } catch (error) {
      setTestResults((prev) => ({
        ...prev,
        [id]: {
          ok: false,
          message: error instanceof Error ? error.message : String(error),
          tools_count: 0,
        },
      }));
    } finally {
      setTestingId(null);
    }
  };

  const handleToggleServer = async (serverId: string, enabled: boolean) => {
    if (!loadedPath) return;
    try {
      const updated = await dataSource.enableMcpServerForProject(
        loadedPath,
        serverId,
        enabled,
      );
      onAgentConfigChange(updated);
    } catch {
      // Non-fatal; the persisted state is reflected on next load.
    }
  };

  return (
    <StudioPanel>
      <div className="mb-3 flex items-center justify-between gap-2">
        <h4 className="font-display text-lg font-semibold tracking-display-tight text-ink">
          {t("settings.tab.mcp")}
        </h4>
        <StudioButton variant="primary" onClick={() => setEditingEntry(emptyEntry())}>
          {t("settings.mcp.add")}
        </StudioButton>
      </div>
      {serverError && (
        <div
          role="alert"
          className="mt-2 rounded-md border border-signal/30 bg-signal/10 px-3 py-2 text-sm text-signal"
        >
          {serverError}
        </div>
      )}
      {editingEntry ? (
        <McpServerEditor
          entry={editingEntry}
          onCancel={() => setEditingEntry(null)}
          onSave={handleSaveEntry}
        />
      ) : serversLoading ? (
        <EmptyState>…</EmptyState>
      ) : servers.length === 0 ? (
        <EmptyState>{t("settings.mcp.empty")}</EmptyState>
      ) : (
        <div className="mt-2 overflow-x-auto">
          <table className="w-full border-collapse text-sm">
            <thead>
              <tr className="text-left text-xs font-semibold uppercase tracking-eyebrow text-ink/55">
                <th className="px-2 py-1.5">{t("settings.mcp.id")}</th>
                <th className="px-2 py-1.5">{t("settings.mcp.label")}</th>
                <th className="px-2 py-1.5">{t("settings.mcp.kind")}</th>
                <th className="px-2 py-1.5">{t("settings.mcp.enabled")}</th>
                <th className="px-2 py-1.5" aria-label={t("settings.mcp.actions")} />
              </tr>
            </thead>
            <tbody>
              {servers.map((entry) => (
                <McpServerRow
                  key={entry.id}
                  entry={entry}
                  enabledForProject={agentConfig.enabled_mcp_servers.includes(
                    entry.id,
                  )}
                  onToggle={(enabled) => void handleToggleServer(entry.id, enabled)}
                  onEdit={() => setEditingEntry(entry)}
                  onDelete={() => void handleDeleteEntry(entry.id)}
                  onTest={() => void handleTestConnection(entry.id)}
                  testing={testingId === entry.id}
                  testResult={testResults[entry.id] ?? null}
                  dataSource={dataSource}
                  loadedPath={loadedPath}
                />
              ))}
            </tbody>
          </table>
        </div>
      )}
    </StudioPanel>
  );
}

function transportKindLabel(kind: McpTransportKind, t: (key: string) => string): string {
  switch (kind) {
    case "stdio":
      return t("settings.mcp.kind.stdio");
    case "sse":
      return t("settings.mcp.kind.sse");
    case "http":
      return t("settings.mcp.kind.http");
    default:
      return kind;
  }
}

interface McpServerRowProps {
  entry: McpServerEntry;
  enabledForProject: boolean;
  onToggle(enabled: boolean): void;
  onEdit(): void;
  onDelete(): void;
  onTest(): void;
  testing: boolean;
  testResult: McpServerTestResult | null;
  dataSource: StudioDataSource;
  loadedPath: string;
}

function McpServerRow({
  entry,
  enabledForProject,
  onToggle,
  onEdit,
  onDelete,
  onTest,
  testing,
  testResult,
  dataSource,
  loadedPath,
}: McpServerRowProps) {
  const { t } = useStudioI18n();
  return (
    <>
      <tr className="border-t border-canvas-200/55">
        <td className="px-2 py-1.5">{entry.id}</td>
        <td className="px-2 py-1.5">{entry.label}</td>
        <td className="px-2 py-1.5">{transportKindLabel(entry.kind, t)}</td>
        <td className="px-2 py-1.5">{entry.enabled ? "✓" : "—"}</td>
        <td className="px-2 py-1.5">
          <div className="flex flex-wrap items-center gap-1.5">
            <StudioButton onClick={onEdit}>{t("settings.mcp.edit")}</StudioButton>
            <StudioButton onClick={onDelete}>{t("settings.mcp.delete")}</StudioButton>
            <StudioButton onClick={onTest} disabled={testing}>
              {t("settings.mcp.test")}
            </StudioButton>
            <label className="inline-flex items-center gap-1.5 text-sm text-ink">
              <input
                type="checkbox"
                checked={enabledForProject}
                onChange={(e) => onToggle(e.target.checked)}
                disabled={!loadedPath}
                aria-label={t("settings.mcp.enableForProject")}
              />
              {t("settings.mcp.enableForProject")}
            </label>
          </div>
        </td>
      </tr>
      {testResult && (
        <tr>
          <td colSpan={5} className="px-2 py-1.5">
            <div
              role="status"
              className={[
                "rounded-md border px-3 py-2 text-sm",
                testResult.ok
                  ? "border-sage/30 bg-sage/10 text-sage"
                  : "border-signal/30 bg-signal/10 text-signal",
              ].join(" ")}
            >
              {testResult.ok
                ? t("settings.mcp.test.ok")
                : t("settings.mcp.test.failed")}
              {" "}
              {testResult.message}{" "}
              {testResult.ok && testResult.tools_count > 0
                ? t("settings.mcp.test.toolsCount", { count: testResult.tools_count })
                : ""}
            </div>
          </td>
        </tr>
      )}
      {testResult?.ok && (
        <tr>
          <td colSpan={5} className="px-2 py-1.5">
            <McpToolList
              serverId={entry.id}
              dataSource={dataSource}
              loadedPath={loadedPath}
            />
          </td>
        </tr>
      )}
    </>
  );
}

interface McpServerEditorProps {
  entry: McpServerEntry;
  onCancel(): void;
  onSave(entry: McpServerEntry): void;
}

function McpServerEditor({ entry, onCancel, onSave }: McpServerEditorProps) {
  const { t } = useStudioI18n();
  const [draft, setDraft] = useState<McpServerEntry>(entry);
  // Flat editor fields for the transport-config discriminated union. Stdio
  // uses command + args (whitespace-split for ergonomics); SSE/HTTP use a
  // single endpoint_url. The submit path rebuilds the typed transport_config.
  const [command, setCommand] = useState(
    draft.transport_config.kind === "stdio" ? draft.transport_config.command : "",
  );
  const [argsText, setArgsText] = useState(
    draft.transport_config.kind === "stdio"
      ? draft.transport_config.args.join(" ")
      : "",
  );
  const [endpointUrl, setEndpointUrl] = useState(
    draft.transport_config.kind === "stdio" ? "" : draft.transport_config.endpoint_url,
  );

  const update = <K extends keyof McpServerEntry>(key: K, value: McpServerEntry[K]) =>
    setDraft((prev) => ({ ...prev, [key]: value }));

  const handleKindChange = (kind: McpTransportKind) => {
    setDraft((prev) => ({ ...prev, kind }));
    if (kind === "stdio") {
      setEndpointUrl("");
    } else {
      setCommand("");
      setArgsText("");
    }
  };

  const handleSubmit = () => {
    const transportConfig: McpTransportConfig =
      draft.kind === "stdio"
        ? {
            kind: "stdio",
            command: command.trim(),
            args: argsText.trim() ? argsText.trim().split(/\s+/) : [],
            env: {},
          }
        : { kind: draft.kind, endpoint_url: endpointUrl.trim() };
    onSave({ ...draft, kind: draft.kind, transport_config: transportConfig });
  };

  return (
    <StudioPanel className="mt-3">
      <div className="grid max-w-xl gap-3">
        <TextInput
          label={t("settings.mcp.id")}
          ariaLabel={t("settings.mcp.id")}
          value={draft.id}
          onChange={(v) => update("id", v)}
        />
        <TextInput
          label={t("settings.mcp.label")}
          ariaLabel={t("settings.mcp.label")}
          value={draft.label}
          onChange={(v) => update("label", v)}
        />
        <label className="grid gap-1">
          <span className="text-xs font-semibold uppercase tracking-eyebrow text-ink/55">
            {t("settings.mcp.kind")}
          </span>
          <select
            aria-label={t("settings.mcp.kind")}
            value={draft.kind}
            onChange={(e) => handleKindChange(e.target.value as McpTransportKind)}
            className="h-10 min-w-0 rounded-md border border-canvas-200 bg-canvas-50 px-3 text-sm text-ink outline-none transition ease-expo focus:border-accent-400 focus:ring-1 focus:ring-accent-400/30"
          >
            {TRANSPORT_KINDS.map((k) => (
              <option key={k} value={k}>
                {transportKindLabel(k, t)}
              </option>
            ))}
          </select>
        </label>
        {draft.kind === "stdio" ? (
          <>
            <TextInput
              label={t("settings.mcp.command")}
              ariaLabel={t("settings.mcp.command")}
              value={command}
              onChange={setCommand}
              placeholder="e.g. npx"
            />
            <TextInput
              label={t("settings.mcp.args")}
              ariaLabel={t("settings.mcp.args")}
              value={argsText}
              onChange={setArgsText}
              placeholder="e.g. -y @modelcontextprotocol/server-memory"
            />
          </>
        ) : (
          <TextInput
            label={t("settings.mcp.endpoint")}
            ariaLabel={t("settings.mcp.endpoint")}
            value={endpointUrl}
            onChange={setEndpointUrl}
            placeholder="https://example.com/mcp"
          />
        )}
        <div className="grid gap-1">
          <TextInput
            label={t("settings.mcp.credentialEnvVar")}
            ariaLabel={t("settings.mcp.credentialEnvVar")}
            value={draft.credential_env_var}
            onChange={(v) => update("credential_env_var", v)}
            placeholder="e.g. MCP_TOKEN"
          />
          <small className="text-xs text-ink/55">{t("settings.mcp.credentialHint")}</small>
        </div>
        <label className="inline-flex items-center gap-2 text-sm text-ink">
          <input
            type="checkbox"
            checked={draft.enabled}
            onChange={(e) => update("enabled", e.target.checked)}
          />
          {t("settings.mcp.enabled")}
        </label>
        <div className="flex gap-2">
          <StudioButton variant="primary" onClick={handleSubmit}>
            {t("settings.mcp.save")}
          </StudioButton>
          <StudioButton onClick={onCancel}>{t("settings.mcp.cancel")}</StudioButton>
        </div>
      </div>
    </StudioPanel>
  );
}

// ---------------------------------------------------------------------------
// Tool list + invoke — rendered under a server row once its test succeeds.
// Lists `McpToolManifest` (name + description) and offers an invoke button
// whose result is the redacted `McpToolCallResult`.
// ---------------------------------------------------------------------------

interface McpToolListProps {
  serverId: string;
  dataSource: StudioDataSource;
  loadedPath: string;
}

function McpToolList({ serverId, dataSource }: McpToolListProps) {
  const { t } = useStudioI18n();
  const [tools, setTools] = useState<McpToolManifest[] | null>(null);
  const [toolsLoading, setToolsLoading] = useState(false);
  const [toolsError, setToolsError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setToolsLoading(true);
    setToolsError(null);
    dataSource
      .listMcpTools(serverId)
      .then((list) => {
        if (!cancelled) setTools(list);
      })
      .catch((error: unknown) => {
        if (!cancelled) {
          setToolsError(error instanceof Error ? error.message : String(error));
        }
      })
      .finally(() => {
        if (!cancelled) setToolsLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [dataSource, serverId]);

  if (toolsLoading) {
    return (
      <div className="rounded-md border border-canvas-200/70 bg-canvas-100/40 p-3 text-sm text-ink/55">
        …
      </div>
    );
  }
  if (toolsError) {
    return (
      <div
        role="alert"
        className="rounded-md border border-signal/30 bg-signal/10 p-3 text-sm text-signal"
      >
        {toolsError}
      </div>
    );
  }
  if (!tools || tools.length === 0) {
    return (
      <div className="rounded-md border border-canvas-200/70 bg-canvas-100/40 p-3 text-sm text-ink/55">
        {t("settings.mcp.tools.empty")}
      </div>
    );
  }
  return (
    <div className="rounded-md border border-canvas-200/70 bg-canvas-100/40 p-3">
      <div className="mb-2 text-xs font-semibold uppercase tracking-eyebrow text-ink/55">
        {t("settings.mcp.tools.heading")}
      </div>
      <ul className="grid gap-2">
        {tools.map((tool) => (
          <McpToolRow
            key={tool.name}
            serverId={serverId}
            tool={tool}
            dataSource={dataSource}
          />
        ))}
      </ul>
    </div>
  );
}

function McpToolRow({
  serverId,
  tool,
  dataSource,
}: {
  serverId: string;
  tool: McpToolManifest;
  dataSource: StudioDataSource;
}) {
  const { t } = useStudioI18n();
  const [result, setResult] = useState<McpToolCallResult | null>(null);
  const [invoking, setInvoking] = useState(false);
  const [invokeError, setInvokeError] = useState<string | null>(null);

  const handleInvoke = async () => {
    setInvoking(true);
    setInvokeError(null);
    setResult(null);
    const request: McpToolCallRequest = {
      server_id: serverId,
      tool_name: tool.name,
      arguments: {},
    };
    try {
      const res = await dataSource.invokeMcpTool(request);
      setResult(res);
    } catch (error) {
      setInvokeError(error instanceof Error ? error.message : String(error));
    } finally {
      setInvoking(false);
    }
  };

  return (
    <li className="rounded-md border border-canvas-200/70 bg-canvas-50 p-2.5">
      <div className="flex items-start justify-between gap-2">
        <div className="min-w-0">
          <strong className="font-semibold text-ink">{tool.name}</strong>
          <div className="mt-0.5 text-sm text-ink/70">{tool.description}</div>
        </div>
        <StudioButton onClick={() => void handleInvoke()} disabled={invoking}>
          {t("settings.mcp.tools.invoke")}
        </StudioButton>
      </div>
      {invokeError && (
        <div
          role="alert"
          className="mt-2 rounded-md border border-signal/30 bg-signal/10 px-2 py-1.5 text-xs text-signal"
        >
          {invokeError}
        </div>
      )}
      {result && (
        <div
          role="status"
          className={[
            "mt-2 rounded-md border px-2 py-1.5 text-xs",
            result.is_error
              ? "border-signal/30 bg-signal/10 text-signal"
              : "border-sage/30 bg-sage/10 text-sage",
          ].join(" ")}
        >
          <div className="font-semibold">
            {result.is_error
              ? t("settings.mcp.tools.invoke.error")
              : t("settings.mcp.tools.invoke.ok")}
          </div>
          <ul className="mt-1 grid gap-0.5">
            {result.content.map((block, index) => (
              <li key={index}>
                {block.type === "text"
                  ? block.text
                  : block.type === "image"
                    ? t("settings.mcp.tools.content.image")
                    : t("settings.mcp.tools.content.resource")}
              </li>
            ))}
          </ul>
        </div>
      )}
    </li>
  );
}
