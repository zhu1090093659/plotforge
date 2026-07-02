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

function evidenceStatusLabel(status: EvidenceStatus) {
  switch (status) {
    case "pass":
      return "Pass";
    case "review":
      return "Review";
    case "pending":
      return "Pending";
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
  };
}) {
  const ready = item.status === "Ready" || item.status === "Passed";
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
      <p className="text-right text-xs font-semibold text-ink/60">
        {item.files} files
      </p>
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
      label: "Player files",
      detail: "HTML, CSS, JS, fonts",
      status: exportReport ? "Ready" : staticExportSelected ? "Pending" : "Draft",
      files: exportReport?.files_found.length ?? 0,
    },
    {
      label: "ExportManifest",
      detail: "export-manifest.json",
      status: exportReport ? "Ready" : "Pending",
      files: 1,
    },
    {
      label: "Reachable assets",
      detail: "Images, audio, fonts",
      status: exportReport && assetCatalog.items.length > 0 ? "Ready" : "Pending",
      files: assetCatalog.items.length,
    },
    {
      label: "Story and rules data",
      detail: "Scenes, rules, contracts",
      status: exportReport && projectData ? "Ready" : "Pending",
      files: projectData ? projectData.scenes.length + projectData.rules.length : 0,
    },
    {
      label: "AI usage disclosure",
      detail: aiSafetyPolicy?.policy_source_path ?? "ai-usage manifest",
      status: exportReport && aiSafetyPolicy ? "Ready" : "Pending",
      files: aiSafetyPolicy ? 1 : 0,
    },
    {
      label: "Content warning draft",
      detail: "local creator review",
      status: exportReport && aiSafetyPolicy ? "Ready" : "Pending",
      files: aiSafetyPolicy?.content_kinds.length ?? 0,
    },
    {
      label: "Archive manifest",
      detail: exportReport?.archive_path ?? "created after export",
      status: exportReport ? "Ready" : "Pending",
      files: exportReport?.archived_files.length ?? 0,
    },
    {
      label: "Local smoke evidence",
      detail: "static export smoke not run by this command",
      status: "Pending",
      files: 0,
    },
  ];

  // Checks that can actually resolve (user-actionable or export-dependent)
  const actionableEvidenceChecks = [
    {
      label: "No provider configuration",
      status: selectedExportProfile
        ? selectedExportProfile.includes_provider_config
          ? "review"
          : "pass"
        : "pending",
    },
    {
      label: "No private traces",
      status: selectedExportProfile
        ? selectedExportProfile.includes_private_traces
          ? "review"
          : "pass"
        : "pending",
    },
    {
      label: "All referenced assets copied",
      status: exportReport ? (exportAuditMatched ? "pass" : "review") : "pending",
    },
    {
      label: "No absolute machine paths",
      status: exportReport
        ? hasAbsolutePackagePath
          ? "review"
          : "pass"
        : "pending",
    },
  ] satisfies Array<{ label: string; status: EvidenceStatus }>;

  // Permanently-pending checks (system-level, user cannot act on them here)
  const technicalEvidenceChecks = [
    { label: "No raw responses", status: "pending" as EvidenceStatus },
    { label: "No secret markers", status: "pending" as EvidenceStatus },
    { label: "HTTP smoke test passed", status: "pending" as EvidenceStatus },
  ];

  const allEvidenceChecks = [...actionableEvidenceChecks, ...technicalEvidenceChecks];
  const passedEvidenceCount = allEvidenceChecks.filter((c) => c.status === "pass").length;
  const reviewEvidenceCount = allEvidenceChecks.filter((c) => c.status === "review").length;
  const selectedProfileReady = allEvidenceChecks.every((c) => c.status === "pass");
  const packageHash = exportReport ? "pending explicit package hash" : "pending export";

  const panelClassName = "rounded-lg border border-ink/10 bg-canvas-50 p-4 text-ink shadow-studio-panel";

  return (
    <section className={panelClassName}>
      {/* Header */}
      <div className="flex flex-wrap items-start justify-between gap-4">
        <div className="min-w-0">
          <h3 className="text-lg font-semibold">Export Package</h3>
          <p className="mt-1 truncate text-sm text-ink/55">
            Local package readiness, manifest evidence, and boundary checks
          </p>
        </div>
        <button
          type="button"
          aria-label="Export zip"
          onClick={() => void runStaticZipExport()}
          disabled={exportDisabled}
          className="inline-flex h-10 items-center gap-2 rounded-md bg-ink px-4 text-sm font-semibold text-canvas-50 transition hover:bg-ink/85 disabled:cursor-not-allowed disabled:bg-ink/30"
        >
          {exporting ? (
            <Loader2 aria-hidden size={16} className="animate-spin" />
          ) : (
            <Download aria-hidden size={16} />
          )}
          Export zip
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
        ariaLabel="Export workspace"
        className="mt-4"
        items={[
          {
            id: "package",
            label: "Package",
            badge: exportReport ? exportReport.files_found.length : undefined,
            children: (
              <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_340px] 2xl:grid-cols-[minmax(0,1fr)_380px]">
                {/* Build Profile + Evidence & Boundaries (left) */}
                <aside className="grid max-h-[620px] content-start gap-3 overflow-y-auto rounded-lg border border-ink/10 bg-graphite-950 p-3 text-ink shadow-studio-panel lg:col-start-2 lg:row-start-1">
                  <div className="flex flex-wrap items-start justify-between gap-3">
                    <div className="min-w-0">
                      <p className="text-xs font-semibold uppercase text-violet-600">
                        Build Profile
                      </p>
                      <h3 className="mt-1 text-lg font-semibold text-ink">
                        {selectedExportProfile?.id ?? "No profile selected"}
                      </h3>
                    </div>
                    <StudioStatusChip tone={staticExportSelected ? "health" : "agent"}>
                      {staticExportSelected ? "Executable" : "Draft"}
                    </StudioStatusChip>
                  </div>
                  <p className="text-sm leading-6 text-graphite-700/70">
                    {selectedExportProfile?.intent ??
                      "Select a local package profile to inspect export readiness."}
                  </p>

                  <ExportEvidenceCard
                    title="AI Usage Manifest"
                    badge={aiSafetyPolicy ? "Included" : "Pending"}
                  >
                    <p className="text-sm leading-6 text-graphite-700/70">
                      {aiSafetyPolicy?.moderation_policy ??
                        "AI usage evidence appears after policy load."}
                    </p>
                  </ExportEvidenceCard>

                  <ExportEvidenceCard
                    title="Content Warnings"
                    badge={aiSafetyPolicy ? "Included" : "Pending"}
                  >
                    <div className="grid gap-2">
                      {(aiSafetyPolicy?.content_kinds ?? ["text"]).map((kind) => (
                        <div
                          key={kind}
                          className="flex items-center justify-between gap-3 rounded-md border border-canvas-200 bg-ink/5 px-3 py-2 text-sm"
                        >
                          <span className="capitalize text-ink">{kind}</span>
                          <span className="text-graphite-700/60">creator review</span>
                        </div>
                      ))}
                    </div>
                  </ExportEvidenceCard>

                  <ExportEvidenceCard title="Redaction Rules" badge="On">
                    <div className="grid gap-2 text-sm text-graphite-700/70">
                      {[
                        "Strip provider configuration",
                        "Remove private traces",
                        "Remove raw responses",
                        "Remove secret markers",
                      ].map((rule) => (
                        <div key={rule} className="flex items-center justify-between gap-3">
                          <span>{rule}</span>
                          <span className="font-semibold text-health-400">On</span>
                        </div>
                      ))}
                    </div>
                  </ExportEvidenceCard>

                  <ExportEvidenceCard title="Asset Whitelist" badge="Local">
                    <p className="text-sm leading-6 text-graphite-700/70">
                      Allow only referenced assets under project asset paths.
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
                      Evidence &amp; Boundaries
                    </p>
                    <p className="mt-1 text-sm text-graphite-700/70">
                      Local export package only
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
                          {evidenceStatusLabel(check.status)}
                        </span>
                      </div>
                    ))}
                  </div>

                  {/* Technical details (permanently pending checks) — collapsed by default */}
                  <Collapsible
                    label="Technical Details"
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
                            {evidenceStatusLabel(check.status)}
                          </span>
                        </div>
                      ))}
                      <p className="mt-1 text-xs text-graphite-700/55">
                        These checks remain pending until a separate smoke test is run
                        after export.
                      </p>
                    </div>
                  </Collapsible>

                  <ExportEvidenceCard title="Package Information" badge="Local">
                    <div className="grid gap-2 text-sm">
                      <ProofLikeLine label="Profile" value={selectedExportProfile?.id ?? "none"} />
                      <ProofLikeLine label="Package hash" value={packageHash} />
                      <ProofLikeLine
                        label="Archive"
                        value={exportReport?.archive_path ?? "not exported"}
                      />
                    </div>
                  </ExportEvidenceCard>

                  <ExportEvidenceCard title="Validation Summary" badge="Local">
                    <div className="grid gap-2 text-sm">
                      <ProofLikeLine
                        label="Checks passed"
                        value={`${passedEvidenceCount} / ${allEvidenceChecks.length}`}
                      />
                      <ProofLikeLine
                        label="Needs review"
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
                          Package Readiness
                        </p>
                        <h3 className="mt-1 text-lg font-semibold">
                          {selectedProfileReady
                            ? "All local boundary checks passed"
                            : "Review profile boundary checks"}
                        </h3>
                      </div>
                      <div className="grid grid-cols-2 gap-3 text-right text-sm">
                        <ExportInfo
                          label="Files"
                          value={String(exportReport?.files_found.length ?? 0)}
                        />
                        <ExportInfo
                          label="Asset records"
                          value={String(assetCatalog.items.length)}
                        />
                      </div>
                    </div>

                    {/* Package items — collapsible to reduce visual noise */}
                    <div className="mt-4">
                      <Collapsible
                        label="Package Contents"
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
                      This profile is available as contract metadata only; no Studio export
                      command is wired for this target.
                    </div>
                  ) : null}

                  {/* Output paths — always visible to avoid layout shift */}
                  <div className="grid gap-3 lg:grid-cols-2">
                    <TextInput
                      label="Output directory"
                      ariaLabel="Static export output directory"
                      value={exportDir}
                      onChange={setExportDir}
                    />
                    <TextInput
                      label="Zip archive"
                      ariaLabel="Static export zip archive"
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
                      <MetricBox label="Archive" value={exportReport.archive_path ?? "none"} />
                      <MetricBox label="Files" value={exportReport.archived_files.length} />
                      <MetricBox
                        label="Audit"
                        value={
                          sameStringSet(exportReport.allowed_files, exportReport.files_found)
                            ? "matched"
                            : "mismatch"
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
            label: "Profile",
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
                        aria-label={`Select export profile ${profile.id}`}
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
                            {profile.target === "static_web" ? "Executable" : "Draft"}
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
                          Selected Profile
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
                        label="Runtime network"
                        value={
                          selectedExportProfile.requires_network_at_runtime
                            ? "required"
                            : "not required"
                        }
                        safe={!selectedExportProfile.requires_network_at_runtime}
                      />
                      <ProfileFlag
                        label="Provider config"
                        value={
                          selectedExportProfile.includes_provider_config
                            ? "included"
                            : "excluded"
                        }
                        safe={!selectedExportProfile.includes_provider_config}
                      />
                      <ProfileFlag
                        label="Private traces"
                        value={
                          selectedExportProfile.includes_private_traces
                            ? "included"
                            : "excluded"
                        }
                        safe={!selectedExportProfile.includes_private_traces}
                      />
                      <ProfileFlag
                        label="Submission ready"
                        value={
                          selectedExportProfile.platform_submission_ready
                            ? "claimed"
                            : "not claimed"
                        }
                        safe={!selectedExportProfile.platform_submission_ready}
                      />
                    </div>

                    <div className="mt-4">
                      <p className="text-xs font-medium uppercase text-ink/45">
                        Capabilities
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
              <EmptyPanel label="No export profiles returned by the Studio adapter." />
            ),
          },
          {
            id: "policy",
            label: "Policy",
            badge: aiSafetyPolicy ? undefined : "pending",
            children: aiSafetyPolicy ? (
              <div className="rounded-md border border-ink/10 bg-canvas-50 p-4">
                <div className="flex flex-wrap items-start justify-between gap-3">
                  <div>
                    <h4 className="text-sm font-semibold uppercase text-ink/55">
                      AI Safety Policy
                    </h4>
                    <p className="mt-1 text-sm text-ink/60">
                      Export disclosure evidence
                    </p>
                  </div>
                  <button
                    type="button"
                    aria-label="Save AI Safety Policy"
                    disabled={formSaving === "export-kit"}
                    onClick={() => void saveAiSafetyPolicy()}
                    className="inline-flex h-10 items-center gap-2 rounded-md bg-ink px-4 text-sm font-semibold text-canvas-50 transition hover:bg-ink/85 disabled:cursor-not-allowed disabled:bg-ink/30"
                  >
                    {formSaving === "export-kit" ? (
                      <Loader2 aria-hidden size={16} className="animate-spin" />
                    ) : (
                      <Save aria-hidden size={16} />
                    )}
                    Save AI Safety Policy
                  </button>
                </div>
                <div className="mt-4 grid gap-3 lg:grid-cols-2">
                  <CheckboxInput
                    label="Live generated content enabled"
                    checked={aiSafetyPolicy.live_generated_content_enabled}
                    onChange={(value) =>
                      updateAiSafetyPolicy({ live_generated_content_enabled: value })
                    }
                  />
                  <CheckboxInput
                    label="Human review required"
                    checked={aiSafetyPolicy.human_review_required}
                    onChange={(value) =>
                      updateAiSafetyPolicy({ human_review_required: value })
                    }
                  />
                  <CheckboxInput
                    label="Moderation queue enabled"
                    checked={aiSafetyPolicy.moderation_queue_enabled}
                    onChange={(value) =>
                      updateAiSafetyPolicy({ moderation_queue_enabled: value })
                    }
                  />
                  <TextareaInput
                    label="Content kinds"
                    ariaLabel="AI safety content kinds"
                    value={listToLines(aiSafetyPolicy.content_kinds)}
                    onChange={(value) =>
                      updateAiSafetyPolicy({
                        content_kinds: contentKindsFromLines(value),
                      })
                    }
                    minHeight="min-h-24"
                  />
                  <TextInput
                    label="Reporting path"
                    ariaLabel="AI safety reporting path"
                    value={aiSafetyPolicy.user_reporting_path}
                    onChange={(value) =>
                      updateAiSafetyPolicy({ user_reporting_path: value })
                    }
                  />
                  <TextareaInput
                    label="Moderation policy"
                    ariaLabel="AI safety moderation policy"
                    value={aiSafetyPolicy.moderation_policy}
                    onChange={(value) =>
                      updateAiSafetyPolicy({ moderation_policy: value })
                    }
                    minHeight="min-h-24"
                  />
                  <TextareaInput
                    label="Safety guardrails"
                    ariaLabel="AI safety guardrails"
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
              <EmptyPanel label="AI Safety Policy not loaded for this project." />
            ),
          },
        ]}
      />
    </section>
  );
}
