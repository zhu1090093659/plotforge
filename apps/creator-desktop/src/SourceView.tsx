import { Loader2, Save, TerminalSquare } from "lucide-react";
import { useStudioI18n } from "./i18n";
import { StudioPanel } from "./studioUi";
import type {
  SourceFileContent,
  SourceFileSummary,
} from "./tauriBridge";

// ---------------------------------------------------------------------------
// SourceView — the source-artifact browser + text editor surface.
//
// Phase B: extracted out of LaunchpadView so the launchpad stops being a
// kitchen-sink. This view owns the source file list and the inline text
// editor; Launchpad keeps only project creation + health. The boundary
// checks panel stays on the Launchpad (it is project health, not source
// editing), so this view is intentionally source-only.
// ---------------------------------------------------------------------------

export interface SourceViewProps {
  sourceFiles: SourceFileSummary[];
  selectedFile: SourceFileContent | null;
  editorContent: string;
  setEditorContent(value: string): void;
  dirty: boolean;
  saving: boolean;
  error: string | null;
  onSelectSourceFile(file: SourceFileSummary): void;
  onSaveSelectedFile(): void;
}

export function SourceView({
  sourceFiles,
  selectedFile,
  editorContent,
  setEditorContent,
  dirty,
  saving,
  error,
  onSelectSourceFile,
  onSaveSelectedFile,
}: SourceViewProps) {
  const { t } = useStudioI18n();

  return (
    <StudioPanel>
      <div className="grid gap-4 lg:grid-cols-[1.2fr_0.8fr]">
        <SourceFileList
          sourceFiles={sourceFiles}
          selectedFile={selectedFile}
          onSelectSourceFile={onSelectSourceFile}
        />
        <div className="grid content-start gap-4">
          {selectedFile ? (
            <SourceEditor
              selectedFile={selectedFile}
              editorContent={editorContent}
              setEditorContent={setEditorContent}
              dirty={dirty}
              saving={saving}
              error={error}
              onSave={onSaveSelectedFile}
            />
          ) : (
            <p className="text-sm text-ink/55">
              {t("source.selectFileToEdit")}
            </p>
          )}
        </div>
      </div>
    </StudioPanel>
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
          <h3 className="font-display text-lg font-semibold tracking-display">
            {t("launchpad.sourceArtifacts")}
          </h3>
          <p className="mt-1 text-sm text-ink/55">
            {t("common.files", { count: sourceFiles.length })}
          </p>
        </div>
        <TerminalSquare aria-hidden className="text-signal" size={22} />
      </div>

      <div className="mt-4 grid gap-2 pr-1">
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
        <p className="truncate text-sm text-ink/55">{selectedFile.path}</p>
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
          className="min-h-[40vh] max-h-[60vh] w-full resize-none rounded-md border border-canvas-200/70 bg-canvas-50 px-3 py-3 font-mono text-sm leading-6 text-ink outline-none transition focus:border-accent-400 focus:ring-1 focus:ring-accent-400/30 read-only:bg-ink/5"
        />
        <div className="mt-3 flex flex-wrap items-center gap-2 text-xs font-medium uppercase text-ink/55">
          <span>{selectedFile.kind}</span>
          <span>
            {selectedFile.editable
              ? t("common.editable")
              : t("common.readOnly")}
          </span>
          {dirty ? (
            <span className="text-plum">{t("common.modified")}</span>
          ) : null}
        </div>
      </div>
    </div>
  );
}
