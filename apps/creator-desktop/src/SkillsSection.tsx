import { useCallback, useEffect, useState } from "react";
import type {
  AgentSessionConfig,
  SkillIndex,
  SkillManifest,
} from "../../../contracts/plotforge";
import type { StudioDataSource } from "./studioDataSource";
import { EmptyState, StudioButton, StudioPanel } from "./studioUi";
import { useStudioI18n } from "./i18n";

// ---------------------------------------------------------------------------
// SkillsSection — the "Skills" tab inside SettingsView.
//
// Covers the user skill library, per-project enablement, and external skill
// import. Split out of the old AgentView and rewritten from inline `style=`
// to Tailwind + `studioUiClassNames` (B4).
// ---------------------------------------------------------------------------

export interface SkillsSectionProps {
  dataSource: StudioDataSource;
  loadedPath: string;
  agentConfig: AgentSessionConfig;
  onAgentConfigChange: (config: AgentSessionConfig) => void;
}

export function SkillsSection({
  dataSource,
  loadedPath,
  agentConfig,
  onAgentConfigChange,
}: SkillsSectionProps) {
  const { t } = useStudioI18n();
  const [skills, setSkills] = useState<SkillManifest[]>([]);
  const [skillsLoading, setSkillsLoading] = useState(false);

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
    void reloadSkills();
  }, [reloadSkills]);

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

  return (
    <StudioPanel>
      <div className="mb-3 flex items-center justify-between gap-2">
        <h4 className="font-display text-lg font-semibold tracking-display-tight text-ink">
          {t("agent.tab.skills")}
        </h4>
        <StudioButton onClick={handleRefreshSkills} disabled={skillsLoading}>
          {t("agent.skill.refresh")}
        </StudioButton>
      </div>
      {skillsLoading ? (
        <EmptyState>…</EmptyState>
      ) : skills.length === 0 ? (
        <EmptyState>{t("agent.skill.refresh")}</EmptyState>
      ) : (
        <ul className="grid gap-2">
          {skills.map((skill) => (
            <SkillRow
              key={`${skill.source.origin}:${skill.id}`}
              skill={skill}
              enabled={agentConfig.enabled_skills.includes(skill.id)}
              onToggle={(enabled) => void handleToggleSkill(skill.id, enabled)}
              onImport={() => void handleImportSkill(skill.id)}
              dataSource={dataSource}
            />
          ))}
        </ul>
      )}
    </StudioPanel>
  );
}

function SkillRow({
  skill,
  enabled,
  onToggle,
  onImport,
  dataSource,
}: {
  skill: SkillManifest;
  enabled: boolean;
  onToggle(enabled: boolean): void;
  onImport(): void;
  dataSource: StudioDataSource;
}) {
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

  // Skills already in the user's own `~/.plotforge/skills/` library
  // (`plot_forge_user` origin) cannot be re-imported (`import_external_skill`
  // errors with "destination already exists"). Hide the Import button for
  // those so the click is never offered as actionable. Finding L2.
  const importable = skill.source.origin !== "plot_forge_user";

  return (
    <li className="rounded-md border border-canvas-200/70 bg-canvas-100/40 p-3">
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <strong className="font-semibold text-ink">{skill.name}</strong>{" "}
          <small className="text-xs text-ink/55">[{originLabel(skill.source.origin, t)}]</small>
          <div className="mt-1 text-sm text-ink/70">{skill.description}</div>
        </div>
        <div className="flex shrink-0 flex-wrap items-center gap-2">
          <label className="inline-flex items-center gap-1.5 text-sm text-ink">
            <input
              type="checkbox"
              checked={enabled}
              onChange={(e) => onToggle(e.target.checked)}
            />
            {t("agent.skill.enable")}
          </label>
          {importable && (
            <StudioButton onClick={onImport}>{t("agent.skill.import")}</StudioButton>
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
          className="mt-2 min-h-24 w-full resize-none rounded-md border border-canvas-200 bg-canvas-50 px-3 py-2 text-sm leading-6 text-ink"
        />
      )}
    </li>
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
