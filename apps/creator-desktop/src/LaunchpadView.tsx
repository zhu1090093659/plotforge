import {
  CheckCircle2,
  Loader2,
  PlusCircle,
  Save,
  TerminalSquare,
  XCircle,
} from "lucide-react";
import type { FormEvent } from "react";
import type {
  ProjectCreationReport,
  ProjectCreationRequest,
  ProjectTemplateId,
} from "../../../contracts/plotforge";
import { CommandCenterView } from "./CommandCenterView";
import { useStudioI18n } from "./i18n";
import type { CreatorProjectSummary } from "./projectSummary";
import type { StudioDataSource } from "./studioDataSource";
import type { StudioMetric } from "./useStudioWorkspace";
import type { StudioSectionId } from "./studioModel";
import type {
  ExportProfile,
  ProjectData,
} from "../../../contracts/plotforge";
import type {
  PlayOnceReport,
  ProjectCheckReport,
  SourceFileSummary,
  SourceFileContent,
  StaticExportReport,
} from "./tauriBridge";
import type { AssetCatalog } from "./assetCatalog";
import {
  StudioTabs,
  type StudioTabItem,
} from "./studioUi";

// ---------------------------------------------------------------------------
// LaunchpadViewProps
// ---------------------------------------------------------------------------

export interface LaunchpadViewProps {
  // --- project data ---
  projectSummary: CreatorProjectSummary | null;
  projectData: ProjectData | null;
  loadedPath: string;
  checkReport: ProjectCheckReport | null;
  metrics: StudioMetric[];
  // --- source files ---
  sourceFiles: SourceFileSummary[];
  selectedFile: SourceFileContent | null;
  // --- source editor ---
  editorContent: string;
  setEditorContent(value: string): void;
  dirty: boolean;
  saving: boolean;
  error: string | null;
  // --- playtest ---
  playtestInput: string;
  setPlaytestInput(value: string): void;
  playtesting: boolean;
  playtestReport: PlayOnceReport | null;
  playtestError: string | null;
  // --- export ---
  exportProfiles: ExportProfile[];
  exportReport: StaticExportReport | null;
  // --- asset catalog ---
  assetCatalog: AssetCatalog;
  // --- project creation wizard state ---
  createProjectPath: string;
  setCreateProjectPath(value: string): void;
  createTemplate: ProjectTemplateId;
  setCreateTemplate(value: ProjectTemplateId): void;
  createConcept: string;
  setCreateConcept(value: string): void;
  createVisualStyle: string;
  setCreateVisualStyle(value: string): void;
  createVoiceEnabled: boolean;
  setCreateVoiceEnabled(value: boolean): void;
  createInitialSceneRequest: string;
  setCreateInitialSceneRequest(value: string): void;
  createForce: boolean;
  setCreateForce(value: boolean): void;
  createReport: ProjectCreationReport | null;
  creating: boolean;
  createError: string | null;
  // --- callbacks ---
  onRunPlayableProof(): void;
  onOpenSection(section: StudioSectionId): void;
  onOpenExportProfile(profileId: string): void;
  onSelectSourceFile(file: SourceFileSummary): void;
  onSaveSelectedFile(): void;
  onCreateProject(path: string, request: ProjectCreationRequest, force: boolean): void;
  // --- data source for CommandCenterView ---
  dataSource: StudioDataSource;
}

// ---------------------------------------------------------------------------
// LaunchpadView
// ---------------------------------------------------------------------------

export function LaunchpadView({
  projectSummary,
  projectData,
  loadedPath,
  checkReport,
  metrics,
  sourceFiles,
  selectedFile,
  editorContent,
  setEditorContent,
  dirty,
  saving,
  error,
  playtestInput,
  setPlaytestInput,
  playtesting,
  playtestReport,
  playtestError,
  exportProfiles,
  exportReport,
  assetCatalog,
  createProjectPath,
  setCreateProjectPath,
  createTemplate,
  setCreateTemplate,
  createConcept,
  setCreateConcept,
  createVisualStyle,
  setCreateVisualStyle,
  createVoiceEnabled,
  setCreateVoiceEnabled,
  createInitialSceneRequest,
  setCreateInitialSceneRequest,
  createForce,
  setCreateForce,
  createReport,
  creating,
  createError,
  onRunPlayableProof,
  onOpenSection,
  onOpenExportProfile,
  onSelectSourceFile,
  onSaveSelectedFile,
  onCreateProject,
}: LaunchpadViewProps) {
  const { t } = useStudioI18n();

  return (
    <div className="grid gap-5">
      {/* Core CTA: Command Center */}
      <CommandCenterView
        projectSummary={projectSummary}
        projectData={projectData}
        loadedPath={loadedPath}
        metrics={metrics}
        sourceFiles={sourceFiles}
        selectedFile={selectedFile}
        playtestInput={playtestInput}
        playtesting={playtesting}
        playtestReport={playtestReport}
        playtestError={playtestError}
        exportProfiles={exportProfiles}
        dirty={dirty}
        onIntentChange={setPlaytestInput}
        onRunPlayableProof={onRunPlayableProof}
        onOpenSection={onOpenSection}
        onOpenExportProfile={onOpenExportProfile}
      />

      {/* Secondary surfaces share one tab strip so they never stack past the
          fold. The Editor tab appears when a file is selected; the user clicks
          it to edit (StudioTabs reconciles if the active tab disappears). */}
      <StudioTabs
        ariaLabel={t("launchpad.secondarySurfaces")}
        className="mt-1"
        items={[
          {
            id: "new-project",
            label: t("launchpad.newProject"),
            children: (
              <NewProjectForm
                createProjectPath={createProjectPath}
                setCreateProjectPath={setCreateProjectPath}
                createTemplate={createTemplate}
                setCreateTemplate={setCreateTemplate}
                createConcept={createConcept}
                setCreateConcept={setCreateConcept}
                createVisualStyle={createVisualStyle}
                setCreateVisualStyle={setCreateVisualStyle}
                createVoiceEnabled={createVoiceEnabled}
                setCreateVoiceEnabled={setCreateVoiceEnabled}
                createInitialSceneRequest={createInitialSceneRequest}
                setCreateInitialSceneRequest={setCreateInitialSceneRequest}
                createForce={createForce}
                setCreateForce={setCreateForce}
                createReport={createReport}
                creating={creating}
                createError={createError}
                onCreateProject={onCreateProject}
              />
            ),
          },
          {
            id: "source-artifacts",
            label: t("launchpad.sourceArtifacts"),
            badge: sourceFiles.length,
            children: (
              <div className="grid gap-3 lg:grid-cols-[1.2fr_0.8fr]">
                <SourceFileList
                  sourceFiles={sourceFiles}
                  selectedFile={selectedFile}
                  onSelectSourceFile={onSelectSourceFile}
                />
                <BoundaryChecks
                  checkReport={checkReport}
                  loadedPath={loadedPath}
                  projectData={projectData}
                />
              </div>
            ),
          },
          selectedFile
            ? {
                id: "editor",
                label: t("launchpad.editor"),
                badge: dirty ? t("common.modified") : undefined,
                children: (
                  <SourceEditor
                    selectedFile={selectedFile}
                    editorContent={editorContent}
                    setEditorContent={setEditorContent}
                    dirty={dirty}
                    saving={saving}
                    error={error}
                    onSave={onSaveSelectedFile}
                  />
                ),
              }
            : null,
        ].filter(Boolean) as StudioTabItem[]}
      />
    </div>
  );
}

// ---------------------------------------------------------------------------
// NewProjectForm — extracted sub-component
// ---------------------------------------------------------------------------

interface NewProjectFormProps {
  createProjectPath: string;
  setCreateProjectPath(value: string): void;
  createTemplate: ProjectTemplateId;
  setCreateTemplate(value: ProjectTemplateId): void;
  createConcept: string;
  setCreateConcept(value: string): void;
  createVisualStyle: string;
  setCreateVisualStyle(value: string): void;
  createVoiceEnabled: boolean;
  setCreateVoiceEnabled(value: boolean): void;
  createInitialSceneRequest: string;
  setCreateInitialSceneRequest(value: string): void;
  createForce: boolean;
  setCreateForce(value: boolean): void;
  createReport: ProjectCreationReport | null;
  creating: boolean;
  createError: string | null;
  onCreateProject(path: string, request: ProjectCreationRequest, force: boolean): void;
}

function NewProjectForm({
  createProjectPath,
  setCreateProjectPath,
  createTemplate,
  setCreateTemplate,
  createConcept,
  setCreateConcept,
  createVisualStyle,
  setCreateVisualStyle,
  createVoiceEnabled,
  setCreateVoiceEnabled,
  createInitialSceneRequest,
  setCreateInitialSceneRequest,
  createForce,
  setCreateForce,
  createReport,
  creating,
  createError,
  onCreateProject,
}: NewProjectFormProps) {
  const { t } = useStudioI18n();

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const path = createProjectPath.trim();
    const request: ProjectCreationRequest = {
      template: createTemplate,
      concept: createConcept.trim(),
      visual_style: createVisualStyle.trim(),
      voice_enabled: createVoiceEnabled,
      initial_scene_request: createInitialSceneRequest.trim(),
    };
    onCreateProject(path, request, createForce);
  }

  return (
    <div className="grid gap-4 xl:grid-cols-[1.1fr_0.9fr]">
      <form onSubmit={handleSubmit} className="grid gap-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <p className="text-sm text-ink/55">
            {t("launchpad.scaffold")}
          </p>
          <button
            type="submit"
            disabled={creating}
            className="inline-flex h-10 items-center gap-2 rounded-md bg-ink px-4 text-sm font-semibold text-canvas-50 transition hover:bg-ink/85 disabled:cursor-not-allowed disabled:bg-ink/30"
          >
            {creating ? (
              <Loader2 aria-hidden size={16} className="animate-spin" />
            ) : (
              <PlusCircle aria-hidden size={16} />
            )}
            {t("launchpad.create")}
          </button>
        </div>

        <div className="grid gap-3 lg:grid-cols-2">
          <LaunchpadTextInput
            label={t("launchpad.projectLabel")}
            ariaLabel={t("launchpad.aria.newProjectPath")}
            value={createProjectPath}
            onChange={setCreateProjectPath}
            className="lg:col-span-2"
          />
          <label className="grid gap-1">
            <span className="text-xs font-semibold uppercase text-ink/55">
              {t("launchpad.template")}
            </span>
            <select
              aria-label={t("launchpad.aria.template")}
              value={createTemplate}
              onChange={(event) =>
                setCreateTemplate(event.target.value as ProjectTemplateId)
              }
              className="h-10 min-w-0 rounded-md border border-canvas-200/70 bg-canvas-50 px-3 text-sm text-ink outline-none transition focus:border-accent-400 focus:ring-1 focus:ring-accent-400/30"
            >
              <option value="historical_crisis">{t("launchpad.customStoryProject")}</option>
            </select>
          </label>
          <LaunchpadTextInput
            label={t("launchpad.visualStyle")}
            ariaLabel={t("launchpad.aria.visualStyle")}
            value={createVisualStyle}
            onChange={setCreateVisualStyle}
          />
          <LaunchpadTextareaInput
            label={t("launchpad.concept")}
            ariaLabel={t("launchpad.aria.concept")}
            value={createConcept}
            onChange={setCreateConcept}
            className="lg:col-span-2"
          />
          <LaunchpadTextareaInput
            label={t("launchpad.initialScene")}
            ariaLabel={t("launchpad.aria.initialSceneRequest")}
            value={createInitialSceneRequest}
            onChange={setCreateInitialSceneRequest}
            className="lg:col-span-2"
          />
        </div>

        <div className="flex flex-wrap items-center gap-4 text-sm font-medium text-ink/70">
          <LaunchpadCheckbox
            label={t("launchpad.voiceEnabled")}
            checked={createVoiceEnabled}
            onChange={setCreateVoiceEnabled}
          />
          <LaunchpadCheckbox
            label={t("launchpad.overwrite")}
            checked={createForce}
            onChange={setCreateForce}
          />
        </div>

        {createError ? (
          <p className="rounded-md border border-signal/20 bg-signal/10 px-3 py-2 text-sm text-signal">
            {createError}
          </p>
        ) : null}
      </form>

      {/* Creation Report */}
      <section className="rounded-md border border-canvas-200/55 bg-canvas-50 p-4 shadow-studio-panel">
        <h4 className="text-xs font-semibold uppercase tracking-tightish text-ink/55">
          {t("launchpad.creationReport")}
        </h4>
        {createReport ? (
          <div className="mt-3 grid gap-3 text-sm">
            <div className="rounded-md border border-health-500/25 bg-health-500/10 px-3 py-2">
              <p className="text-xs font-medium uppercase text-health-500">
                {t("launchpad.projectLabel")}
              </p>
              <p className="mt-1 truncate font-semibold text-ink">
                {createReport.project.game.title}
              </p>
              <p className="mt-1 truncate text-ink/60">
                {createReport.project_path}
              </p>
            </div>
            <div className="grid gap-3 sm:grid-cols-2">
              <MetricBox label={t("launchpad.files")} value={createReport.files_created.length} />
              <MetricBox label={t("launchpad.template")} value={createReport.template} />
            </div>
          </div>
        ) : (
          <p className="mt-3 text-sm text-ink/55">
            {t("launchpad.noProjectCreated")}
          </p>
        )}
      </section>
    </div>
  );
}

// ---------------------------------------------------------------------------
// SourceFileList
// ---------------------------------------------------------------------------

function SourceFileList({
  sourceFiles,
  selectedFile,
  onSelectSourceFile,
}: {
  sourceFiles: SourceFileSummary[];
  selectedFile: SourceFileContent | null;
  onSelectSourceFile(file: SourceFileSummary): void;
}) {
  const { t } = useStudioI18n();

  return (
    <section className="rounded-md border border-canvas-200/55 bg-canvas-50 p-5 shadow-studio-panel">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h3 className="font-display text-lg font-semibold tracking-display">{t("launchpad.sourceArtifacts")}</h3>
          <p className="mt-1 text-sm text-ink/55">
            {t("common.files", { count: sourceFiles.length })}
          </p>
        </div>
        <TerminalSquare aria-hidden className="text-signal" size={22} />
      </div>

      <div className="mt-4 grid max-h-80 gap-2 overflow-auto pr-1">
        {sourceFiles.map((file) => (
          <button
            type="button"
            key={file.path}
            onClick={() => onSelectSourceFile(file)}
            className={[
              "flex min-h-11 items-center justify-between gap-3 rounded-md border px-3 py-2 text-left transition",
              selectedFile?.path === file.path
                ? "border-ink/45 bg-canvas-50"
                : "border-ink/10 hover:border-ink/30",
            ].join(" ")}
          >
            <code className="truncate text-sm text-ink/80">{file.path}</code>
            <span
              className={[
                "shrink-0 rounded-sm px-2 py-1 text-xs font-medium",
                file.editable
                  ? "bg-sage/10 text-sage"
                  : "bg-ink/5 text-ink/55",
              ].join(" ")}
            >
              {file.editable ? t("common.editable") : file.kind}
            </span>
          </button>
        ))}
      </div>
    </section>
  );
}

// ---------------------------------------------------------------------------
// BoundaryChecks — reads from real checkReport instead of hardcoded array
// ---------------------------------------------------------------------------

interface BoundaryCheckItem {
  label: string;
  value: string;
  ok: boolean;
}

type StudioTranslate = (key: string, params?: Record<string, string | number>) => string;

function buildBoundaryChecks(
  checkReport: ProjectCheckReport | null,
  loadedPath: string,
  t: StudioTranslate,
): BoundaryCheckItem[] {
  if (!checkReport) {
    return [];
  }
  return [
    {
      label: t("launchpad.check.project"),
      value: checkReport.title,
      ok: Boolean(checkReport.title),
    },
    {
      label: t("launchpad.check.entryScene"),
      value: checkReport.entry_scene,
      ok: Boolean(checkReport.entry_scene),
    },
    {
      label: t("launchpad.check.scenes"),
      value: String(checkReport.scene_count),
      ok: checkReport.scene_count > 0,
    },
    {
      label: t("launchpad.check.rules"),
      value: String(checkReport.rule_count),
      ok: checkReport.rule_count >= 0,
    },
    {
      label: t("launchpad.check.characters"),
      value: String(checkReport.character_count),
      ok: checkReport.character_count >= 0,
    },
    {
      label: t("launchpad.check.projectPath"),
      value: loadedPath,
      ok: Boolean(loadedPath),
    },
  ];
}

function BoundaryChecks({
  checkReport,
  loadedPath,
  projectData,
}: {
  checkReport: ProjectCheckReport | null;
  loadedPath: string;
  projectData: ProjectData | null;
}) {
  const { t } = useStudioI18n();
  const checks = buildBoundaryChecks(checkReport, loadedPath, t);
  const projectLoaded = Boolean(projectData) || Boolean(loadedPath);
  const emptyKey = projectLoaded
    ? "launchpad.boundaryChecksPending"
    : "launchpad.noBoundaryChecks";

  return (
    <section className="rounded-md border border-canvas-200/55 bg-canvas-50 p-5 shadow-studio-panel">
      <h3 className="font-display text-lg font-semibold tracking-display">{t("launchpad.boundaryChecks")}</h3>
      <div className="mt-4 grid gap-3">
        {checks.length > 0 ? (
          checks.map((check) => (
            <div key={check.label} className="flex items-start gap-3">
              {check.ok ? (
                <CheckCircle2
                  aria-hidden
                  className="mt-0.5 shrink-0 text-sage"
                  size={18}
                />
              ) : (
                <XCircle
                  aria-hidden
                  className="mt-0.5 shrink-0 text-signal"
                  size={18}
                />
              )}
              <div className="min-w-0">
                <p className="text-sm font-semibold">{check.label}</p>
                <p className="truncate text-sm text-ink/55">{check.value}</p>
              </div>
            </div>
          ))
        ) : (
          <p className="text-sm text-ink/55">{t(emptyKey)}</p>
        )}
      </div>
    </section>
  );
}

// ---------------------------------------------------------------------------
// SourceEditor
// ---------------------------------------------------------------------------

function SourceEditor({
  selectedFile,
  editorContent,
  setEditorContent,
  dirty,
  saving,
  error,
  onSave,
}: {
  selectedFile: SourceFileContent;
  editorContent: string;
  setEditorContent(value: string): void;
  dirty: boolean;
  saving: boolean;
  error: string | null;
  onSave(): void;
}) {
  const { t } = useStudioI18n();

  return (
    <div>
      <div className="flex flex-wrap items-center justify-between gap-3">
        <p className="truncate text-sm text-ink/55">
          {selectedFile.path}
        </p>
        <button
          type="button"
          disabled={!dirty || saving}
          onClick={onSave}
          className="inline-flex h-10 items-center gap-2 rounded-md bg-ink px-4 text-sm font-semibold text-canvas-50 transition hover:bg-ink/85 disabled:cursor-not-allowed disabled:bg-ink/30"
        >
          {saving ? (
            <Loader2 aria-hidden size={16} className="animate-spin" />
          ) : (
            <Save aria-hidden size={16} />
          )}
          {t("common.save")}
        </button>
      </div>

      {error ? (
        <p className="mt-4 rounded-md border border-signal/20 bg-signal/10 px-3 py-2 text-sm text-signal">
          {error}
        </p>
      ) : null}

      <div className="mt-4">
        <textarea
          aria-label={t("launchpad.aria.sourceEditor")}
          value={editorContent}
          readOnly={!selectedFile.editable}
          onChange={(event) => setEditorContent(event.target.value)}
          spellCheck={false}
          className="min-h-72 w-full resize-y rounded-md border border-canvas-200/70 bg-canvas-50 px-3 py-3 font-mono text-sm leading-6 text-ink outline-none transition focus:border-accent-400 focus:ring-1 focus:ring-accent-400/30 read-only:bg-ink/5"
        />
        <div className="mt-3 flex flex-wrap items-center gap-2 text-xs font-medium uppercase text-ink/55">
          <span>{selectedFile.kind}</span>
          <span>{selectedFile.editable ? t("common.editable") : t("common.readOnly")}</span>
          {dirty ? <span className="text-plum">{t("common.modified")}</span> : null}
        </div>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Small internal primitive components
// ---------------------------------------------------------------------------

function LaunchpadTextInput({
  label,
  ariaLabel,
  value,
  onChange,
  className = "",
}: {
  label: string;
  ariaLabel: string;
  value: string;
  onChange(value: string): void;
  className?: string;
}) {
  return (
    <label className={`grid gap-1 ${className}`}>
      <span className="text-xs font-semibold uppercase text-ink/55">{label}</span>
      <input
        type="text"
        aria-label={ariaLabel}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        className="h-10 min-w-0 rounded-md border border-canvas-200/70 bg-canvas-50 px-3 text-sm text-ink outline-none transition focus:border-accent-400 focus:ring-1 focus:ring-accent-400/30"
      />
    </label>
  );
}

function LaunchpadTextareaInput({
  label,
  ariaLabel,
  value,
  onChange,
  className = "",
}: {
  label: string;
  ariaLabel: string;
  value: string;
  onChange(value: string): void;
  className?: string;
}) {
  return (
    <label className={`grid gap-1 ${className}`}>
      <span className="text-xs font-semibold uppercase text-ink/55">{label}</span>
      <textarea
        aria-label={ariaLabel}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        className="min-h-20 w-full resize-y rounded-md border border-canvas-200/70 bg-canvas-50 px-3 py-2 text-sm leading-6 text-ink outline-none transition focus:border-accent-400 focus:ring-1 focus:ring-accent-400/30"
      />
    </label>
  );
}

function LaunchpadCheckbox({
  label,
  checked,
  onChange,
}: {
  label: string;
  checked: boolean;
  onChange(value: boolean): void;
}) {
  return (
    <label className="flex items-center gap-2 cursor-pointer">
      <input
        type="checkbox"
        checked={checked}
        onChange={(event) => onChange(event.target.checked)}
        className="h-4 w-4 rounded border border-canvas-200/70 accent-violet-500"
      />
      {label}
    </label>
  );
}

function MetricBox({
  label,
  value,
}: {
  label: string;
  value: string | number;
}) {
  return (
    <div className="rounded-md border border-ink/10 bg-canvas-100 px-3 py-2">
      <p className="text-xs font-medium uppercase text-ink/45">{label}</p>
      <p className="mt-1 truncate text-sm font-semibold text-ink">{String(value)}</p>
    </div>
  );
}
