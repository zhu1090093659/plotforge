import { Download, Loader2, Save } from "lucide-react";
import type {
  AiSafetyPolicy,
  AiUsageContentKind,
  ExportProfile,
  ProjectData,
} from "../../../contracts/plotforge";
import type { StaticExportReport } from "./tauriBridge";
import type { AssetCatalog } from "./assetCatalog";
import { Collapsible, StudioStatusChip, StudioTabs } from "./studioUi";
import { useStudioI18n } from "./i18n";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type EvidenceStatus = "pass" | "pending" | "review";

export interface ExportViewProps {
  // export state
  exportDir: string;
  setExportDir(value: string): void;
  archivePath: string;
  setArchivePath(value: string): void;
  exportProfiles: ExportProfile[];
  selectedExportProfileId: string | null;
  selectedExportProfile: ExportProfile | null;
  staticExportSelected: boolean;
  exportReport: StaticExportReport | null;
  exporting: boolean;
  exportError: string | null;
  // project data
  assetCatalog: AssetCatalog;
  projectData: ProjectData | null;
  aiSafetyPolicy: AiSafetyPolicy | null;
  formSaving: string | null;
  formStatus: { section: string; tone: "success" | "error"; message: string } | null;
  // actions
  runStaticZipExport(): Promise<void>;
  selectExportProfile(id: string): void;
  updateAiSafetyPolicy(patch: Partial<AiSafetyPolicy>): void;
  saveAiSafetyPolicy(): Promise<void>;
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

function sameStringSet(left: string[], right: string[]) {
  if (left.length !== right.length) {
    return false;
  }
  const rightValues = new Set(right);
  return left.every((value) => rightValues.has(value));
}

function isAbsoluteMachinePath(path: string) {
  return path.startsWith("/") || path.startsWith("~") || /^[A-Za-z]:[\\/]/.test(path);
}

type StudioTranslate = (key: string, params?: Record<string, string | number>) => string;

function evidenceStatusLabel(status: EvidenceStatus, t: StudioTranslate) {
  switch (status) {
    case "pass":
      return t("common.pass");
    case "review":
      return t("common.review");
    case "pending":
      return t("common.pending");
  }
}

function evidenceStatusClassName(status: EvidenceStatus) {
  switch (status) {
    case "pass":
      return "text-health-400";
    case "review":
      return "text-signal";
    case "pending":
      return "text-violet-600";
  }
}

function listToLines(values: string[]) {
  return values.join("\n");
}

function linesToList(value: string) {
  return value
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
}

function optionalText(value: string) {
  const trimmed = value.trim();
  return trimmed ? trimmed : null;
}

function contentKindsFromLines(value: string): AiUsageContentKind[] {
  const allowed: AiUsageContentKind[] = ["text", "image", "audio", "voice", "data"];
  const selected = linesToList(value).filter((kind): kind is AiUsageContentKind =>
    allowed.includes(kind as AiUsageContentKind),
  );
  return selected.length > 0 ? selected : ["text"];
}

// ---------------------------------------------------------------------------
// Sub-components
// ---------------------------------------------------------------------------

function ExportEvidenceCard({
  title,
  badge,
  children,
}: {
  title: string;
  badge: string;
  children: React.ReactNode;
}) {
  return (
    <section className="rounded-md border border-canvas-200 bg-ink/5 px-3 py-3">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <h4 className="text-sm font-semibold text-ink">{title}</h4>
        <span className="rounded-sm border border-health-400/30 bg-health-500/15 px-2 py-1 text-xs font-semibold text-health-400">
          {badge}
        </span>
      </div>
      <div className="mt-3">{children}</div>
    </section>
  );
}

function PackageReadinessRow({
  item,
}: {
  item: {
    label: string;
    detail: string;
    status: string;
    files: number;
    filesLabel: string;
    ready: boolean;
  };
}) {
  const ready = item.ready;
  return (
    <div className="grid gap-3 rounded-md border border-ink/10 bg-canvas-50 px-3 py-2 text-sm md:grid-cols-[minmax(0,1fr)_120px_minmax(160px,0.7fr)_80px]">
      <div className="min-w-0">
        <p className="truncate font-semibold text-ink">{item.label}</p>
        <p className="mt-1 truncate text-xs text-ink/50">{item.detail}</p>
      </div>
      <span
        className={[
          "w-fit rounded-sm border px-2 py-1 text-xs font-semibold",
          ready
            ? "border-sage/30 bg-sage/10 text-sage"
            : "border-plum/30 bg-plum/10 text-plum",
        ].join(" ")}
      >
        {item.status}
      </span>
      <div className="h-2 self-center rounded-full bg-ink/10">
        <div
          className={[
            "h-2 rounded-full",
            ready ? "bg-sage" : "bg-plum",
          ].join(" ")}
          style={{ width: ready ? "100%" : "45%" }}
        />
      </div>
      <p className="text-right text-xs font-semibold text-ink/60">{item.filesLabel}</p>
    </div>
  );
}

function ProfileFlag({
  label,
  value,
  safe,
}: {
  label: string;
  value: string;
  safe: boolean;
}) {
  return (
    <div className="rounded-md border border-ink/10 bg-canvas-50 px-3 py-2">
      <p className="text-xs font-medium uppercase text-ink/45">{label}</p>
      <p
        className={[
          "mt-1 text-sm font-semibold",
          safe ? "text-sage" : "text-signal",
        ].join(" ")}
      >
        {value}
      </p>
    </div>
  );
}

function ProofLikeLine({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex min-w-0 items-center justify-between gap-3">
      <p className="text-xs font-medium uppercase text-graphite-700/55">{label}</p>
      <p className="truncate text-xs font-semibold text-ink">{value}</p>
    </div>
  );
}

function ExportInfo({ label, value }: { label: string; value: string }) {
  return (
    <div className="min-w-0">
      <p className="text-xs font-medium uppercase text-ink/45">{label}</p>
      <p className="mt-1 truncate text-sm font-semibold text-ink">{value}</p>
    </div>
  );
}

function MetricBox({ label, value }: { label: string; value: string | number }) {
  return (
    <div className="rounded-md border border-ink/10 bg-canvas-50 px-3 py-2">
      <p className="text-xs font-medium uppercase text-ink/55">{label}</p>
      <p className="mt-1 truncate font-semibold">{value}</p>
    </div>
  );
}

function TextInput({
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
      <span className="text-xs font-medium uppercase text-ink/55">{label}</span>
      <input
        aria-label={ariaLabel}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        className="h-10 min-w-0 rounded-md border border-canvas-200/70 bg-canvas-50 px-3 text-sm text-ink outline-none transition focus:border-accent-400 focus:ring-1 focus:ring-accent-400/30"
      />
    </label>
  );
}

function TextareaInput({
  label,
  ariaLabel,
  value,
  onChange,
  className = "",
  minHeight = "min-h-40",
}: {
  label: string;
  ariaLabel: string;
  value: string;
  onChange(value: string): void;
  className?: string;
  minHeight?: string;
}) {
  return (
    <label className={`grid gap-1 ${className}`}>
      <span className="text-xs font-medium uppercase text-ink/55">{label}</span>
      <textarea
        aria-label={ariaLabel}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        className={`w-full resize-y rounded-md border border-canvas-200/70 bg-canvas-50 px-3 py-2 text-sm leading-6 text-ink outline-none transition focus:border-accent-400 focus:ring-1 focus:ring-accent-400/30 ${minHeight}`}
      />
    </label>
  );
}

function CheckboxInput({
  label,
  checked,
  onChange,
}: {
  label: string;
  checked: boolean;
  onChange(value: boolean): void;
}) {
  return (
    <label className="inline-flex items-center gap-2">
      <input
        type="checkbox"
        checked={checked}
        onChange={(event) => onChange(event.target.checked)}
        className="h-4 w-4 accent-ink"
      />
      {label}
    </label>
  );
}

function EmptyPanel({ label }: { label: string }) {
  return (
    <div className="mt-4 rounded-md border border-ink/10 bg-canvas-50 px-3 py-3 text-sm text-ink/55">
      {label}
    </div>
  );
}

// ---------------------------------------------------------------------------
// ExportView
// ---------------------------------------------------------------------------

/**
 * Export Package surface — presents local package readiness, manifest evidence,
 * and boundary checks.  Receives all data and callbacks as props; contains no
 * business logic.
 */
export function ExportView({
  exportDir,
  setExportDir,
  archivePath,
  setArchivePath,
  exportProfiles,
  selectedExportProfileId,
  selectedExportProfile,
  staticExportSelected,
  exportReport,
  exporting,
  exportError,
  assetCatalog,
  projectData,
  aiSafetyPolicy,
  formSaving,
  formStatus,
  runStaticZipExport,
  selectExportProfile,
  updateAiSafetyPolicy,
  saveAiSafetyPolicy,
}: ExportViewProps) {
  const { t } = useStudioI18n();
  const exportDisabled = exporting || !staticExportSelected;
  const exportAuditMatched = exportReport
    ? sameStringSet(exportReport.allowed_files, exportReport.files_found)
    : false;
  const packagePaths = exportReport
    ? [
        ...exportReport.allowed_files,
        ...exportReport.files_found,
        ...exportReport.archived_files,
      ]
    : [];
  const hasAbsolutePackagePath = packagePaths.some(isAbsoluteMachinePath);

  const packageItems = [
    {
      label: t("export.playerFiles"),
      detail: t("export.playerFilesDetail"),
      status: exportReport
        ? t("common.ready")
        : staticExportSelected
          ? t("common.pending")
          : t("export.draft"),
      files: exportReport?.files_found.length ?? 0,
      filesLabel: t("common.files", { count: exportReport?.files_found.length ?? 0 }),
      ready: Boolean(exportReport),
    },
    {
      label: t("export.exportManifest"),
      detail: t("export.exportManifestDetail"),
      status: exportReport ? t("common.ready") : t("common.pending"),
      files: 1,
      filesLabel: t("common.files", { count: 1 }),
      ready: Boolean(exportReport),
    },
    {
      label: t("export.reachableAssets"),
      detail: t("export.reachableAssetsDetail"),
      status:
        exportReport && assetCatalog.items.length > 0
          ? t("common.ready")
          : t("common.pending"),
      files: assetCatalog.items.length,
      filesLabel: t("common.files", { count: assetCatalog.items.length }),
      ready: Boolean(exportReport && assetCatalog.items.length > 0),
    },
    {
      label: t("export.storyRulesData"),
      detail: t("export.storyRulesDataDetail"),
      status:
        exportReport && projectData ? t("common.ready") : t("common.pending"),
      files: projectData ? projectData.scenes.length + projectData.rules.length : 0,
      filesLabel: t("common.files", {
        count: projectData ? projectData.scenes.length + projectData.rules.length : 0,
      }),
      ready: Boolean(exportReport && projectData),
    },
    {
      label: t("export.aiUsageDisclosure"),
      detail: aiSafetyPolicy?.policy_source_path ?? t("export.aiUsageDisclosureDetail"),
      status: exportReport && aiSafetyPolicy ? t("common.ready") : t("common.pending"),
      files: aiSafetyPolicy ? 1 : 0,
      filesLabel: t("common.files", { count: aiSafetyPolicy ? 1 : 0 }),
      ready: Boolean(exportReport && aiSafetyPolicy),
    },
    {
      label: t("export.contentWarningDraft"),
      detail: t("export.contentWarningDraftDetail"),
      status: exportReport && aiSafetyPolicy ? t("common.ready") : t("common.pending"),
      files: aiSafetyPolicy?.content_kinds.length ?? 0,
      filesLabel: t("common.files", { count: aiSafetyPolicy?.content_kinds.length ?? 0 }),
      ready: Boolean(exportReport && aiSafetyPolicy),
    },
    {
      label: t("export.archiveManifest"),
      detail: exportReport?.archive_path ?? t("export.archiveManifestDetail"),
      status: exportReport ? t("common.ready") : t("common.pending"),
      files: exportReport?.archived_files.length ?? 0,
      filesLabel: t("common.files", { count: exportReport?.archived_files.length ?? 0 }),
      ready: Boolean(exportReport),
    },
    {
      label: t("export.localSmokeEvidence"),
      detail: t("export.localSmokeEvidenceDetail"),
      status: t("common.pending"),
      files: 0,
      filesLabel: t("common.files", { count: 0 }),
      ready: false,
    },
  ];

  const actionableEvidenceChecks = [
    {
      label: t("export.check.noProviderConfig"),
      status: selectedExportProfile
        ? selectedExportProfile.includes_provider_config
          ? "review"
          : "pass"
        : "pending",
    },
    {
      label: t("export.check.noPrivateTraces"),
      status: selectedExportProfile
        ? selectedExportProfile.includes_private_traces
          ? "review"
          : "pass"
        : "pending",
    },
    {
      label: t("export.check.allAssetsCopied"),
      status: exportReport ? (exportAuditMatched ? "pass" : "review") : "pending",
    },
    {
      label: t("export.check.noAbsolutePaths"),
      status: exportReport
        ? hasAbsolutePackagePath
          ? "review"
          : "pass"
        : "pending",
    },
  ] satisfies Array<{ label: string; status: EvidenceStatus }>;

  const technicalEvidenceChecks = [
    { label: t("export.check.noRawResponses"), status: "pending" as EvidenceStatus },
    { label: t("export.check.noSecretMarkers"), status: "pending" as EvidenceStatus },
    { label: t("export.check.httpSmokePassed"), status: "pending" as EvidenceStatus },
  ];

  const allEvidenceChecks = [...actionableEvidenceChecks, ...technicalEvidenceChecks];
  const passedEvidenceCount = allEvidenceChecks.filter((c) => c.status === "pass").length;
  const reviewEvidenceCount = allEvidenceChecks.filter((c) => c.status === "review").length;
  const selectedProfileReady = allEvidenceChecks.every((c) => c.status === "pass");
  const packageHash = exportReport
    ? t("export.pendingExplicitPackageHash")
    : t("export.pendingExport");

  const panelClassName = "rounded-lg border border-ink/10 bg-canvas-50 p-4 text-ink shadow-studio-panel";

  return (
    <section className={panelClassName}>
      {/* Header */}
      <div className="flex flex-wrap items-start justify-between gap-4">
        <div className="min-w-0">
          <h3 className="text-lg font-semibold">{t("export.exportPackage")}</h3>
          <p className="mt-1 truncate text-sm text-ink/55">
            {t("export.subtitle")}
          </p>
        </div>
        <button
          type="button"
          aria-label={t("export.aria.exportZip")}
          onClick={() => void runStaticZipExport()}
          disabled={exportDisabled}
          className="inline-flex h-10 items-center gap-2 rounded-md bg-ink px-4 text-sm font-semibold text-canvas-50 transition hover:bg-ink/85 disabled:cursor-not-allowed disabled:bg-ink/30"
        >
          {exporting ? (
            <Loader2 aria-hidden size={16} className="animate-spin" />
          ) : (
            <Download aria-hidden size={16} />
          )}
          {t("export.exportZip")}
        </button>
      </div>

      {/* Form status message */}
      {formStatus && formStatus.section === "export-kit" ? (
        <div
          className={`mt-4 rounded-md border px-3 py-2 text-sm ${
            formStatus.tone === "success"
              ? "border-sage/30 bg-sage/10 text-sage"
              : "border-signal/30 bg-signal/10 text-signal"
          }`}
        >
          {formStatus.message}
        </div>
      ) : null}

      {/* Three-tab surface: Package / Profile / Policy */}
      <StudioTabs
        ariaLabel={t("export.aria.exportWorkspace")}
        className="mt-4"
        items={[
          {
            id: "package",
            label: t("export.packageTab"),
            badge: exportReport ? exportReport.files_found.length : undefined,
            children: (
              <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_340px] 2xl:grid-cols-[minmax(0,1fr)_380px]">
                {/* Build Profile + Evidence & Boundaries (left) */}
                <aside className="grid max-h-[620px] content-start gap-3 overflow-y-auto rounded-lg border border-ink/10 bg-graphite-950 p-3 text-ink shadow-studio-panel lg:col-start-2 lg:row-start-1">
                  <div className="flex flex-wrap items-start justify-between gap-3">
                    <div className="min-w-0">
                      <p className="text-xs font-semibold uppercase text-violet-600">
                        {t("export.buildProfile")}
                      </p>
                      <h3 className="mt-1 text-lg font-semibold text-ink">
                        {selectedExportProfile?.id ?? t("export.noProfileSelected")}
                      </h3>
                    </div>
                    <StudioStatusChip tone={staticExportSelected ? "health" : "agent"}>
                      {staticExportSelected
                        ? t("export.executable")
                        : t("export.draft")}
                    </StudioStatusChip>
                  </div>
                  <p className="text-sm leading-6 text-graphite-700/70">
                    {selectedExportProfile?.intent ?? t("export.selectProfilePrompt")}
                  </p>

                  <ExportEvidenceCard
                    title={t("export.aiUsageManifest")}
                    badge={aiSafetyPolicy ? t("export.included") : t("common.pending")}
                  >
                    <p className="text-sm leading-6 text-graphite-700/70">
                      {aiSafetyPolicy?.moderation_policy ?? t("export.aiUsageAfterPolicy")}
                    </p>
                  </ExportEvidenceCard>

                  <ExportEvidenceCard
                    title={t("export.contentWarnings")}
                    badge={aiSafetyPolicy ? t("export.included") : t("common.pending")}
                  >
                    <div className="grid gap-2">
                      {(aiSafetyPolicy?.content_kinds ?? ["text"]).map((kind) => (
                        <div
                          key={kind}
                          className="flex items-center justify-between gap-3 rounded-md border border-canvas-200 bg-ink/5 px-3 py-2 text-sm"
                        >
                          <span className="capitalize text-ink">{kind}</span>
                          <span className="text-graphite-700/60">
                            {t("export.creatorReview")}
                          </span>
                        </div>
                      ))}
                    </div>
                  </ExportEvidenceCard>

                  <ExportEvidenceCard
                    title={t("export.redactionRules")}
                    badge={t("common.on")}
                  >
                    <div className="grid gap-2 text-sm text-graphite-700/70">
                      {[
                        t("export.stripProviderConfig"),
                        t("export.removePrivateTraces"),
                        t("export.removeRawResponses"),
                        t("export.removeSecretMarkers"),
                      ].map((rule) => (
                        <div key={rule} className="flex items-center justify-between gap-3">
                          <span>{rule}</span>
                          <span className="font-semibold text-health-400">
                            {t("common.on")}
                          </span>
                        </div>
                      ))}
                    </div>
                  </ExportEvidenceCard>

                  <ExportEvidenceCard title={t("export.assetWhitelist")} badge="Local">
                    <p className="text-sm leading-6 text-graphite-700/70">
                      {t("export.allowReferencedAssets")}
                    </p>
                    <div className="mt-2 flex flex-wrap gap-2">
                      {[".png", ".jpg", ".webp", ".ogg", ".mp3", ".json", ".md"].map(
                        (extension) => (
                          <span
                            key={extension}
                            className="rounded-sm border border-canvas-200 bg-ink/5 px-2 py-1 text-xs font-semibold text-graphite-700/70"
                          >
                            {extension}
                          </span>
                        ),
                      )}
                    </div>
                  </ExportEvidenceCard>

                  {/* Boundary checks summary (merged from right aside) */}
                  <div className="mt-1">
                    <p className="text-xs font-semibold uppercase text-health-400">
                      {t("export.evidenceBoundaries")}
                    </p>
                    <p className="mt-1 text-sm text-graphite-700/70">
                      {t("export.localExportPackageOnly")}
                    </p>
                  </div>
                  <div className="grid gap-2">
                    {actionableEvidenceChecks.map((check) => (
                      <div
                        key={check.label}
                        className="flex items-center justify-between gap-3 rounded-md border border-canvas-200 bg-ink/5 px-3 py-2 text-sm"
                      >
                        <span>{check.label}</span>
                        <span
                          className={[
                            "font-semibold",
                            evidenceStatusClassName(check.status),
                          ].join(" ")}
                        >
                          {evidenceStatusLabel(check.status, t)}
                        </span>
                      </div>
                    ))}
                  </div>

                  {/* Technical details (permanently pending checks) — collapsed by default */}
                  <Collapsible
                    label={t("export.technicalDetails")}
                    defaultOpen={false}
                    badge={technicalEvidenceChecks.length}
                    className="mt-1"
                  >
                    <div className="mt-2 grid gap-2">
                      {technicalEvidenceChecks.map((check) => (
                        <div
                          key={check.label}
                          className="flex items-center justify-between gap-3 rounded-md border border-canvas-200 bg-ink/5 px-3 py-2 text-sm"
                        >
                          <span>{check.label}</span>
                          <span
                            className={[
                              "font-semibold",
                              evidenceStatusClassName(check.status),
                            ].join(" ")}
                          >
                            {evidenceStatusLabel(check.status, t)}
                          </span>
                        </div>
                      ))}
                      <p className="mt-1 text-xs text-graphite-700/55">
                        {t("export.technicalChecksPending")}
                      </p>
                    </div>
                  </Collapsible>

                  <ExportEvidenceCard title={t("export.packageInformation")} badge="Local">
                    <div className="grid gap-2 text-sm">
                      <ProofLikeLine
                        label={t("export.profile")}
                        value={selectedExportProfile?.id ?? t("common.none")}
                      />
                      <ProofLikeLine
                        label={t("export.packageHash")}
                        value={packageHash}
                      />
                      <ProofLikeLine
                        label={t("export.archive")}
                        value={exportReport?.archive_path ?? t("common.notExported")}
                      />
                    </div>
                  </ExportEvidenceCard>

                  <ExportEvidenceCard title={t("export.validationSummary")} badge="Local">
                    <div className="grid gap-2 text-sm">
                      <ProofLikeLine
                        label={t("export.checksPassed")}
                        value={t("common.checksPassed", {
                          passed: passedEvidenceCount,
                          total: allEvidenceChecks.length,
                        })}
                      />
                      <ProofLikeLine
                        label={t("export.needsReview")}
                        value={String(reviewEvidenceCount + (exportError ? 1 : 0))}
                      />
                    </div>
                  </ExportEvidenceCard>
                </aside>

                {/* Right: Package Readiness + output paths + metrics */}
                <div className="grid content-start gap-4 lg:col-start-1 lg:row-start-1">
                  <section className="rounded-lg border border-ink/10 bg-canvas-50 p-4 shadow-studio-panel">
                    <div className="flex flex-wrap items-start justify-between gap-3">
                      <div>
                        <p className="text-xs font-semibold uppercase text-health-500">
                          {t("export.packageReadiness")}
                        </p>
                        <h3 className="mt-1 text-lg font-semibold">
                          {selectedProfileReady
                            ? t("export.allChecksPassed")
                            : t("export.reviewProfileChecks")}
                        </h3>
                      </div>
                      <div className="grid grid-cols-2 gap-3 text-right text-sm">
                        <ExportInfo
                          label={t("export.files")}
                          value={String(exportReport?.files_found.length ?? 0)}
                        />
                        <ExportInfo
                          label={t("export.assetRecords")}
                          value={String(assetCatalog.items.length)}
                        />
                      </div>
                    </div>

                    {/* Package items — collapsible to reduce visual noise */}
                    <div className="mt-4">
                      <Collapsible
                        label={t("export.packageContents")}
                        defaultOpen={false}
                        badge={packageItems.length}
                      >
                        <div className="mt-3 grid gap-2">
                          {packageItems.map((item) => (
                            <PackageReadinessRow key={item.label} item={item} />
                          ))}
                        </div>
                      </Collapsible>
                    </div>
                  </section>

                  {/* Non-executable profile notice */}
                  {selectedExportProfile && !staticExportSelected ? (
                    <div className="rounded-md border border-ink/10 bg-canvas-50 px-3 py-2 text-sm text-ink/60">
                      {t("export.nonExecutableNotice")}
                    </div>
                  ) : null}

                  {/* Output paths — always visible to avoid layout shift */}
                  <div className="grid gap-3 lg:grid-cols-2">
                    <TextInput
                      label={t("export.outputDirectory")}
                      ariaLabel={t("export.aria.staticExportDir")}
                      value={exportDir}
                      onChange={setExportDir}
                    />
                    <TextInput
                      label={t("export.zipArchive")}
                      ariaLabel={t("export.aria.staticExportZipArchive")}
                      value={archivePath}
                      onChange={setArchivePath}
                    />
                  </div>

                  {/* Export error */}
                  {exportError ? (
                    <div className="rounded-md border border-signal/30 bg-signal/10 px-3 py-2 text-sm text-signal">
                      {exportError}
                    </div>
                  ) : null}

                  {/* Export report metrics */}
                  {exportReport ? (
                    <div className="grid gap-3 text-sm sm:grid-cols-3">
                      <MetricBox
                        label={t("export.archive")}
                        value={exportReport.archive_path ?? t("common.none")}
                      />
                      <MetricBox
                        label={t("export.files")}
                        value={exportReport.archived_files.length}
                      />
                      <MetricBox
                        label={t("export.audit")}
                        value={
                          sameStringSet(
                            exportReport.allowed_files,
                            exportReport.files_found,
                          )
                            ? t("common.matched")
                            : t("common.mismatch")
                        }
                      />
                    </div>
                  ) : null}
                </div>
              </div>
            ),
          },
          {
            id: "profile",
            label: t("export.profileTab"),
            badge: exportProfiles.length > 0 ? exportProfiles.length : undefined,
            children: exportProfiles.length > 0 ? (
              <div className="grid gap-3 lg:grid-cols-[minmax(0,0.9fr)_minmax(0,1.1fr)]">
                <div className="grid content-start gap-2">
                  {exportProfiles.map((profile) => {
                    const selected = profile.id === selectedExportProfileId;
                    return (
                      <button
                        type="button"
                        key={profile.id}
                        aria-label={t("export.aria.selectProfileN", { id: profile.id })}
                        aria-pressed={selected}
                        onClick={() => selectExportProfile(profile.id)}
                        className={[
                          "rounded-md border px-3 py-3 text-left transition",
                          selected
                            ? "border-ink/45 bg-canvas-50"
                            : "border-ink/10 hover:border-ink/30",
                        ].join(" ")}
                      >
                        <div className="flex flex-wrap items-start justify-between gap-2">
                          <div className="min-w-0">
                            <h4 className="truncate text-sm font-semibold">
                              {profile.id}
                            </h4>
                            <p className="mt-1 text-xs font-medium uppercase text-ink/45">
                              {profile.target}
                            </p>
                          </div>
                          <span
                            className={[
                              "rounded-sm px-2 py-1 text-xs font-semibold",
                              profile.target === "static_web"
                                ? "bg-sage/10 text-sage"
                                : "bg-ink/5 text-ink/55",
                            ].join(" ")}
                          >
                            {profile.target === "static_web"
                              ? t("export.executable")
                              : t("export.draft")}
                          </span>
                        </div>
                        <p className="mt-2 text-sm leading-5 text-ink/65">
                          {profile.intent}
                        </p>
                      </button>
                    );
                  })}
                </div>

                {selectedExportProfile ? (
                  <article className="rounded-md border border-ink/10 bg-canvas-50 p-4">
                    <div className="flex flex-wrap items-start justify-between gap-3">
                      <div>
                        <p className="text-xs font-medium uppercase text-ink/45">
                          {t("export.selectedProfile")}
                        </p>
                        <h4 className="mt-1 text-base font-semibold">
                          {selectedExportProfile.id}
                        </h4>
                      </div>
                      <span className="rounded-sm border border-ink/10 bg-canvas-50 px-2 py-1 text-xs font-semibold text-ink/60">
                        {selectedExportProfile.target}
                      </span>
                    </div>

                    <div className="mt-4 grid gap-2 text-sm sm:grid-cols-2">
                      <ProfileFlag
                        label={t("export.runtimeNetwork")}
                        value={
                          selectedExportProfile.requires_network_at_runtime
                            ? t("common.required")
                            : t("common.notRequired")
                        }
                        safe={!selectedExportProfile.requires_network_at_runtime}
                      />
                      <ProfileFlag
                        label={t("export.providerConfig")}
                        value={
                          selectedExportProfile.includes_provider_config
                            ? t("common.included")
                            : t("common.excluded")
                        }
                        safe={!selectedExportProfile.includes_provider_config}
                      />
                      <ProfileFlag
                        label={t("export.privateTraces")}
                        value={
                          selectedExportProfile.includes_private_traces
                            ? t("common.included")
                            : t("common.excluded")
                        }
                        safe={!selectedExportProfile.includes_private_traces}
                      />
                      <ProfileFlag
                        label={t("export.submissionReady")}
                        value={
                          selectedExportProfile.platform_submission_ready
                            ? t("common.claimed")
                            : t("common.notClaimed")
                        }
                        safe={!selectedExportProfile.platform_submission_ready}
                      />
                    </div>

                    <div className="mt-4">
                      <p className="text-xs font-medium uppercase text-ink/45">
                        {t("export.capabilities")}
                      </p>
                      <div className="mt-2 flex flex-wrap gap-2">
                        {selectedExportProfile.capabilities.map((capability) => (
                          <span
                            key={capability}
                            className="rounded-sm border border-ink/10 bg-canvas-50 px-2 py-1 text-xs font-semibold text-ink/65"
                          >
                            {capability}
                          </span>
                        ))}
                      </div>
                    </div>

                    <div className="mt-4 grid gap-2">
                      {selectedExportProfile.notes.map((note) => (
                        <p
                          key={note}
                          className="rounded-md border border-ink/10 bg-canvas-50 px-3 py-2 text-sm text-ink/65"
                        >
                          {note}
                        </p>
                      ))}
                    </div>
                  </article>
                ) : null}
              </div>
            ) : (
              <EmptyPanel label={t("export.noProfiles")} />
            ),
          },
          {
            id: "policy",
            label: t("export.policyTab"),
            badge: aiSafetyPolicy ? undefined : t("common.pending"),
            children: aiSafetyPolicy ? (
              <div className="rounded-md border border-ink/10 bg-canvas-50 p-4">
                <div className="flex flex-wrap items-start justify-between gap-3">
                  <div>
                    <h4 className="text-sm font-semibold uppercase text-ink/55">
                      {t("export.aiSafetyPolicy")}
                    </h4>
                    <p className="mt-1 text-sm text-ink/60">
                      {t("export.exportDisclosureEvidence")}
                    </p>
                  </div>
                  <button
                    type="button"
                    aria-label={t("export.aria.saveAiSafetyPolicy")}
                    disabled={formSaving === "export-kit"}
                    onClick={() => void saveAiSafetyPolicy()}
                    className="inline-flex h-10 items-center gap-2 rounded-md bg-ink px-4 text-sm font-semibold text-canvas-50 transition hover:bg-ink/85 disabled:cursor-not-allowed disabled:bg-ink/30"
                  >
                    {formSaving === "export-kit" ? (
                      <Loader2 aria-hidden size={16} className="animate-spin" />
                    ) : (
                      <Save aria-hidden size={16} />
                    )}
                    {t("export.saveAiSafetyPolicy")}
                  </button>
                </div>
                <div className="mt-4 grid gap-3 lg:grid-cols-2">
                  <CheckboxInput
                    label={t("export.liveGeneratedContent")}
                    checked={aiSafetyPolicy.live_generated_content_enabled}
                    onChange={(value) =>
                      updateAiSafetyPolicy({ live_generated_content_enabled: value })
                    }
                  />
                  <CheckboxInput
                    label={t("export.humanReviewRequired")}
                    checked={aiSafetyPolicy.human_review_required}
                    onChange={(value) =>
                      updateAiSafetyPolicy({ human_review_required: value })
                    }
                  />
                  <CheckboxInput
                    label={t("export.moderationQueueEnabled")}
                    checked={aiSafetyPolicy.moderation_queue_enabled}
                    onChange={(value) =>
                      updateAiSafetyPolicy({ moderation_queue_enabled: value })
                    }
                  />
                  <TextareaInput
                    label={t("export.contentKinds")}
                    ariaLabel={t("export.aria.aiSafetyContentKinds")}
                    value={listToLines(aiSafetyPolicy.content_kinds)}
                    onChange={(value) =>
                      updateAiSafetyPolicy({
                        content_kinds: contentKindsFromLines(value),
                      })
                    }
                    minHeight="min-h-24"
                  />
                  <TextInput
                    label={t("export.reportingPath")}
                    ariaLabel={t("export.aria.aiSafetyReportingPath")}
                    value={aiSafetyPolicy.user_reporting_path}
                    onChange={(value) =>
                      updateAiSafetyPolicy({ user_reporting_path: value })
                    }
                  />
                  <TextareaInput
                    label={t("export.moderationPolicy")}
                    ariaLabel={t("export.aria.aiSafetyModerationPolicy")}
                    value={aiSafetyPolicy.moderation_policy}
                    onChange={(value) =>
                      updateAiSafetyPolicy({ moderation_policy: value })
                    }
                    minHeight="min-h-24"
                  />
                  <TextareaInput
                    label={t("export.safetyGuardrails")}
                    ariaLabel={t("export.aria.aiSafetyGuardrails")}
                    value={listToLines(aiSafetyPolicy.safety_guardrails)}
                    onChange={(value) =>
                      updateAiSafetyPolicy({ safety_guardrails: linesToList(value) })
                    }
                    className="lg:col-span-2"
                    minHeight="min-h-28"
                  />
                </div>
              </div>
            ) : (
              <EmptyPanel label={t("export.aiSafetyNotLoaded")} />
            ),
          },
        ]}
      />
    </section>
  );
}
