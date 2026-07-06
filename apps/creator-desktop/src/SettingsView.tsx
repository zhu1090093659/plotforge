import { useState } from "react";
import type { AgentSessionConfig } from "../../../contracts/plotforge";
import type { StudioDataSource } from "./studioDataSource";
import { AgentConfigSection } from "./AgentConfigSection";
import { McpSection } from "./McpSection";
import { SkillsSection } from "./SkillsSection";
import { StudioTabs, ViewHeader } from "./studioUi";
import { useStudioI18n } from "./i18n";

// ---------------------------------------------------------------------------
// SettingsView — sidebar item #12, the single entry point for Agent / MCP /
// Skill configuration (replaces the old AgentView).
//
// Owns three internal tabs:
//   - Agent  → Providers / Model / Prompts (AgentConfigSection)
//   - MCP    → MCP server registry management (McpSection): list, add/upsert,
//              delete, test-connection, per-project enable, tool list + invoke.
//   - Skills → user skill library + per-project enablement (SkillsSection)
//
// The per-project model config (AgentSessionConfig) is passed in from
// useAgentConfig so the persisted debounced save path stays single-source.
// ---------------------------------------------------------------------------

export interface SettingsViewProps {
  dataSource: StudioDataSource;
  loadedPath: string;
  agentConfig: AgentSessionConfig;
  onAgentConfigChange: (config: AgentSessionConfig) => void;
  configSaveError: string | null;
}

type SettingsTab = "agent" | "mcp" | "skills";

export function SettingsView({
  dataSource,
  loadedPath,
  agentConfig,
  onAgentConfigChange,
  configSaveError,
}: SettingsViewProps) {
  const { t } = useStudioI18n();
  const [activeTab, setActiveTab] = useState<SettingsTab>("agent");

  const tabs = [
    {
      id: "agent" as const,
      label: t("settings.tab.agent"),
      children: (
        <AgentConfigSection
          dataSource={dataSource}
          loadedPath={loadedPath}
          agentConfig={agentConfig}
          onAgentConfigChange={onAgentConfigChange}
          configSaveError={configSaveError}
        />
      ),
    },
    {
      id: "mcp" as const,
      label: t("settings.tab.mcp"),
      children: (
        <McpSection
          dataSource={dataSource}
          loadedPath={loadedPath}
          agentConfig={agentConfig}
          onAgentConfigChange={onAgentConfigChange}
        />
      ),
    },
    {
      id: "skills" as const,
      label: t("settings.tab.skills"),
      children: (
        <SkillsSection
          dataSource={dataSource}
          loadedPath={loadedPath}
          agentConfig={agentConfig}
          onAgentConfigChange={onAgentConfigChange}
        />
      ),
    },
  ];

  return (
    <div>
      <ViewHeader
        eyebrow="12"
        title={t("nav.settings.label")}
        subtitle={t("nav.settings.description")}
      />
      <StudioTabs
        ariaLabel={t("nav.settings.label")}
        items={tabs}
        activeId={activeTab}
        onActiveChange={(id) => setActiveTab(id as SettingsTab)}
      />
    </div>
  );
}
