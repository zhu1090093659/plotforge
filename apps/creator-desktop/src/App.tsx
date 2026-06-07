import {
  AlertTriangle,
  CheckCircle2,
  ChevronRight,
  FolderOpen,
  Play,
  RefreshCcw,
  TerminalSquare,
} from "lucide-react";
import { sourceFiles, studioSections } from "./studioModel";

const metrics = [
  { label: "Scenes", value: "1", tone: "border-jade/50 text-jade" },
  { label: "Characters", value: "6", tone: "border-brass/50 text-brass" },
  { label: "Rules", value: "3", tone: "border-signal/50 text-signal" },
  { label: "Open Threads", value: "4", tone: "border-ink/30 text-ink" },
];

const checks = [
  { label: "Generated contracts", value: "plotforge.d.ts", ok: true },
  { label: "Rust core boundary", value: "UI adapter only", ok: true },
  { label: "Tauri bridge", value: "Phase 3.2", ok: false },
];

export function App() {
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
              <p className="text-sm font-semibold">Dynasty Embers</p>
            </div>
            <button
              type="button"
              title="Open project folder"
              className="grid h-9 w-9 place-items-center rounded-md border border-ink/15 bg-white text-ink transition hover:border-ink/40"
            >
              <FolderOpen aria-hidden size={18} />
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
                Workspace
              </p>
              <h2 className="mt-1 text-2xl font-semibold">
                Dynasty Embers Dashboard
              </h2>
            </div>
            <div className="flex items-center gap-2">
              <button
                type="button"
                title="Refresh project files"
                className="grid h-10 w-10 place-items-center rounded-md border border-ink/15 bg-white text-ink transition hover:border-ink/40"
              >
                <RefreshCcw aria-hidden size={18} />
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
                </div>
                <TerminalSquare aria-hidden className="text-signal" size={22} />
              </div>

              <div className="mt-4 grid gap-2">
                {sourceFiles.map((file) => (
                  <div
                    key={file}
                    className="flex items-center justify-between gap-3 rounded-md border border-ink/10 px-3 py-2"
                  >
                    <code className="truncate text-sm text-ink/80">{file}</code>
                    <span className="rounded-sm bg-jade/10 px-2 py-1 text-xs font-medium text-jade">
                      source
                    </span>
                  </div>
                ))}
              </div>
            </div>

            <div className="rounded-md border border-ink/10 bg-white p-5 shadow-sm">
              <h3 className="text-lg font-semibold">Boundary Checks</h3>
              <div className="mt-4 grid gap-3">
                {checks.map((check) => (
                  <div key={check.label} className="flex items-start gap-3">
                    {check.ok ? (
                      <CheckCircle2
                        aria-hidden
                        className="mt-0.5 shrink-0 text-jade"
                        size={18}
                      />
                    ) : (
                      <AlertTriangle
                        aria-hidden
                        className="mt-0.5 shrink-0 text-brass"
                        size={18}
                      />
                    )}
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
            <div className="grid gap-4 lg:grid-cols-[0.8fr_1.2fr]">
              <div>
                <h3 className="text-lg font-semibold">Story Craft Queue</h3>
                <p className="mt-1 text-sm text-ink/60">
                  Hooks, promises, arcs, and review notes
                </p>
              </div>
              <div className="grid gap-2 sm:grid-cols-3">
                {["Open hook", "Foreshadowing", "Narrative review"].map(
                  (label) => (
                    <div
                      key={label}
                      className="rounded-md border border-ink/10 px-3 py-3"
                    >
                      <p className="text-sm font-semibold">{label}</p>
                      <p className="mt-1 text-xs text-ink/55">pending adapter</p>
                    </div>
                  ),
                )}
              </div>
            </div>
          </section>
        </main>
      </div>
    </div>
  );
}
