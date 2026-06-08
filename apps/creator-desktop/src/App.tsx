import {
  CheckCircle2,
  ChevronRight,
  FolderOpen,
  Loader2,
  Play,
  RefreshCcw,
  Save,
  TerminalSquare,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { summarizeProject, type CreatorProjectSummary } from "./projectSummary";
import {
  createDefaultStudioDataSource,
  defaultProjectPath,
  type StudioDataSource,
} from "./studioDataSource";
import { studioSections } from "./studioModel";
import type {
  ProjectCheckReport,
  SourceFileContent,
  SourceFileSummary,
} from "./tauriBridge";

const boundaryChecks = [
  { label: "Generated contracts", value: "plotforge.d.ts", ok: true },
  { label: "Rust core boundary", value: "UI adapter only", ok: true },
  { label: "Tauri bridge", value: "commands wired", ok: true },
];

export interface AppProps {
  dataSource?: StudioDataSource;
  initialProjectPath?: string;
}

export function App({
  dataSource = createDefaultStudioDataSource(),
  initialProjectPath = defaultProjectPath(),
}: AppProps) {
  const [projectPath, setProjectPath] = useState(initialProjectPath);
  const [loadedPath, setLoadedPath] = useState(initialProjectPath);
  const [projectSummary, setProjectSummary] =
    useState<CreatorProjectSummary | null>(null);
  const [checkReport, setCheckReport] = useState<ProjectCheckReport | null>(
    null,
  );
  const [sourceFiles, setSourceFiles] = useState<SourceFileSummary[]>([]);
  const [selectedFile, setSelectedFile] = useState<SourceFileContent | null>(
    null,
  );
  const [editorContent, setEditorContent] = useState("");
  const [savedContent, setSavedContent] = useState("");
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const dirty = Boolean(selectedFile?.editable && editorContent !== savedContent);
  const metrics = useMemo(
    () => [
      {
        label: "Scenes",
        value: String(checkReport?.scene_count ?? projectSummary?.sceneCount ?? 0),
        tone: "border-jade/50 text-jade",
      },
      {
        label: "Characters",
        value: String(
          checkReport?.character_count ?? projectSummary?.characterCount ?? 0,
        ),
        tone: "border-brass/50 text-brass",
      },
      {
        label: "Rules",
        value: String(checkReport?.rule_count ?? projectSummary?.ruleCount ?? 0),
        tone: "border-signal/50 text-signal",
      },
      {
        label: "Open Threads",
        value: String(projectSummary?.openThreadCount ?? 0),
        tone: "border-ink/30 text-ink",
      },
    ],
    [checkReport, projectSummary],
  );

  useEffect(() => {
    void loadProject(initialProjectPath);
  }, [initialProjectPath]);

  async function loadProject(path: string) {
    setLoading(true);
    setError(null);
    try {
      const [project, report, files] = await Promise.all([
        dataSource.openProject(path),
        dataSource.checkProject(path),
        dataSource.listSourceFiles(path),
      ]);
      const firstEditable = files.find((file) => file.editable) ?? files[0];
      const firstContent = firstEditable
        ? await dataSource.readSourceFile(path, firstEditable.path)
        : null;

      setLoadedPath(path);
      setProjectPath(path);
      setProjectSummary(summarizeProject(project));
      setCheckReport(report);
      setSourceFiles(files);
      setSelectedFile(firstContent);
      setEditorContent(firstContent?.content ?? "");
      setSavedContent(firstContent?.content ?? "");
    } catch (source) {
      setError(source instanceof Error ? source.message : String(source));
    } finally {
      setLoading(false);
    }
  }

  async function selectSourceFile(file: SourceFileSummary) {
    setError(null);
    try {
      const content = await dataSource.readSourceFile(loadedPath, file.path);
      setSelectedFile(content);
      setEditorContent(content.content);
      setSavedContent(content.content);
    } catch (source) {
      setError(source instanceof Error ? source.message : String(source));
    }
  }

  async function saveSelectedFile() {
    if (!selectedFile?.editable) {
      return;
    }
    setSaving(true);
    setError(null);
    try {
      const updated = await dataSource.writeSourceFile(
        loadedPath,
        selectedFile.path,
        editorContent,
      );
      setSelectedFile(updated);
      setEditorContent(updated.content);
      setSavedContent(updated.content);
    } catch (source) {
      setError(source instanceof Error ? source.message : String(source));
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="min-h-screen bg-parchment text-ink">
      <div className="grid min-h-screen grid-cols-[280px_1fr] max-lg:grid-cols-1">
        <aside className="border-r border-ink/10 bg-white/75 px-4 py-5 max-lg:border-b max-lg:border-r-0">
          <div className="flex items-center gap-3">
            <div className="grid h-10 w-10 place-items-center rounded-md bg-ink text-white">
              PF
            </div>
            <div>
              <p className="text-sm font-semibold uppercase text-signal">
                PlotForge Studio
              </p>
              <h1 className="text-xl font-semibold">Creator Desktop</h1>
            </div>
          </div>

          <div className="mt-6 flex items-center justify-between rounded-md border border-ink/10 bg-parchment px-3 py-2">
            <div>
              <p className="text-xs font-medium uppercase text-ink/60">
                Open Project
              </p>
              <p className="max-w-44 truncate text-sm font-semibold">
                {loadedPath}
              </p>
            </div>
            <button
              type="button"
              title="Open project"
              onClick={() => void loadProject(projectPath)}
              className="grid h-9 w-9 place-items-center rounded-md border border-ink/15 bg-white text-ink transition hover:border-ink/40"
            >
              {loading ? (
                <Loader2 aria-hidden size={18} className="animate-spin" />
              ) : (
                <FolderOpen aria-hidden size={18} />
              )}
            </button>
          </div>

          <nav className="mt-5 grid gap-1">
            {studioSections.map((section) => {
              const Icon = section.icon;
              const selected = section.id === "dashboard";
              return (
                <button
                  type="button"
                  key={section.id}
                  title={section.description}
                  className={[
                    "flex min-h-12 items-center gap-3 rounded-md px-3 py-2 text-left transition",
                    selected
                      ? "bg-ink text-white"
                      : "text-ink/75 hover:bg-ink/5 hover:text-ink",
                  ].join(" ")}
                >
                  <Icon aria-hidden size={18} className="shrink-0" />
                  <span className="min-w-0 flex-1">
                    <span className="block truncate text-sm font-medium">
                      {section.label}
                    </span>
                    <span
                      className={[
                        "block truncate text-xs",
                        selected ? "text-white/65" : "text-ink/45",
                      ].join(" ")}
                    >
                      {section.status}
                    </span>
                  </span>
                  {selected ? <ChevronRight aria-hidden size={16} /> : null}
                </button>
              );
            })}
          </nav>
        </aside>

        <main className="min-w-0 px-6 py-5 lg:px-8">
          <header className="flex flex-wrap items-center justify-between gap-4 border-b border-ink/10 pb-5">
            <div>
              <p className="text-sm font-medium uppercase text-ink/55">
                {dataSource.runtimeName}
              </p>
              <h2 className="mt-1 text-2xl font-semibold">
                {projectSummary?.title ?? "PlotForge Dashboard"}
              </h2>
            </div>
            <div className="flex min-w-0 flex-wrap items-center gap-2">
              <input
                aria-label="Project path"
                value={projectPath}
                onChange={(event) => setProjectPath(event.target.value)}
                className="h-10 min-w-0 rounded-md border border-ink/15 bg-white px-3 text-sm text-ink outline-none transition focus:border-ink/45 sm:w-72"
              />
              <button
                type="button"
                title="Refresh project files"
                onClick={() => void loadProject(projectPath)}
                className="grid h-10 w-10 place-items-center rounded-md border border-ink/15 bg-white text-ink transition hover:border-ink/40"
              >
                {loading ? (
                  <Loader2 aria-hidden size={18} className="animate-spin" />
                ) : (
                  <RefreshCcw aria-hidden size={18} />
                )}
              </button>
              <button
                type="button"
                className="inline-flex h-10 items-center gap-2 rounded-md bg-ink px-4 text-sm font-semibold text-white transition hover:bg-black"
              >
                <Play aria-hidden size={16} />
                Playtest
              </button>
            </div>
          </header>

          <section className="grid gap-4 py-5 sm:grid-cols-2 xl:grid-cols-4">
            {metrics.map((metric) => (
              <article
                key={metric.label}
                className={`rounded-md border bg-white p-4 shadow-sm ${metric.tone}`}
              >
                <p className="text-sm font-medium text-ink/55">{metric.label}</p>
                <p className="mt-2 text-3xl font-semibold text-current">
                  {metric.value}
                </p>
              </article>
            ))}
          </section>

          <section className="grid gap-5 xl:grid-cols-[1.2fr_0.8fr]">
            <div className="rounded-md border border-ink/10 bg-white p-5 shadow-sm">
              <div className="flex items-start justify-between gap-4">
                <div>
                  <h3 className="text-lg font-semibold">Source Files</h3>
                  <p className="mt-1 text-sm text-ink/55">
                    {sourceFiles.length} files
                  </p>
                </div>
                <TerminalSquare aria-hidden className="text-signal" size={22} />
              </div>

              <div className="mt-4 grid max-h-80 gap-2 overflow-auto pr-1">
                {sourceFiles.map((file) => (
                  <button
                    type="button"
                    key={file.path}
                    onClick={() => void selectSourceFile(file)}
                    className={[
                      "flex min-h-11 items-center justify-between gap-3 rounded-md border px-3 py-2 text-left transition",
                      selectedFile?.path === file.path
                        ? "border-ink/45 bg-parchment"
                        : "border-ink/10 hover:border-ink/30",
                    ].join(" ")}
                  >
                    <code className="truncate text-sm text-ink/80">
                      {file.path}
                    </code>
                    <span
                      className={[
                        "shrink-0 rounded-sm px-2 py-1 text-xs font-medium",
                        file.editable
                          ? "bg-jade/10 text-jade"
                          : "bg-ink/5 text-ink/55",
                      ].join(" ")}
                    >
                      {file.editable ? "editable" : file.kind}
                    </span>
                  </button>
                ))}
              </div>
            </div>

            <div className="rounded-md border border-ink/10 bg-white p-5 shadow-sm">
              <h3 className="text-lg font-semibold">Boundary Checks</h3>
              <div className="mt-4 grid gap-3">
                {boundaryChecks.map((check) => (
                  <div key={check.label} className="flex items-start gap-3">
                    <CheckCircle2
                      aria-hidden
                      className="mt-0.5 shrink-0 text-jade"
                      size={18}
                    />
                    <div className="min-w-0">
                      <p className="text-sm font-semibold">{check.label}</p>
                      <p className="truncate text-sm text-ink/55">
                        {check.value}
                      </p>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </section>

          <section className="mt-5 rounded-md border border-ink/10 bg-white p-5 shadow-sm">
            <div className="flex flex-wrap items-center justify-between gap-3">
              <div className="min-w-0">
                <h3 className="text-lg font-semibold">Source Editor</h3>
                <p className="truncate text-sm text-ink/55">
                  {selectedFile?.path ?? "No source file selected"}
                </p>
              </div>
              <button
                type="button"
                disabled={!dirty || saving}
                onClick={() => void saveSelectedFile()}
                className="inline-flex h-10 items-center gap-2 rounded-md bg-ink px-4 text-sm font-semibold text-white transition hover:bg-black disabled:cursor-not-allowed disabled:bg-ink/30"
              >
                {saving ? (
                  <Loader2 aria-hidden size={16} className="animate-spin" />
                ) : (
                  <Save aria-hidden size={16} />
                )}
                Save
              </button>
            </div>

            {error ? (
              <div className="mt-4 rounded-md border border-signal/30 bg-signal/8 px-3 py-2 text-sm text-signal">
                {error}
              </div>
            ) : null}

            <div className="mt-4">
              <textarea
                aria-label="Source editor"
                value={editorContent}
                readOnly={!selectedFile?.editable}
                onChange={(event) => setEditorContent(event.target.value)}
                spellCheck={false}
                className="min-h-72 w-full resize-y rounded-md border border-ink/15 bg-parchment px-3 py-3 font-mono text-sm leading-6 text-ink outline-none transition focus:border-ink/45 read-only:bg-ink/5"
              />
              <div className="mt-3 flex flex-wrap items-center gap-2 text-xs font-medium uppercase text-ink/55">
                <span>{selectedFile?.kind ?? "none"}</span>
                <span>{selectedFile?.editable ? "editable" : "read only"}</span>
                {dirty ? <span className="text-brass">modified</span> : null}
              </div>
            </div>
          </section>
        </main>
      </div>
    </div>
  );
}
