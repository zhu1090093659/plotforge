import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { App } from "./App";
import { demoPlayOnceReport, demoProjectData } from "./demoStudioData";
import type { StudioDataSource } from "./studioDataSource";
import type {
  PlayOnceReport,
  SourceFileContent,
  SourceFileSummary,
} from "./tauriBridge";

afterEach(cleanup);

describe("App", () => {
  it("loads project data, lists source files, and saves editable text", async () => {
    const writes: Array<{ relativePath: string; content: string }> = [];
    const files: SourceFileSummary[] = [
      { path: "game.toml", kind: "toml", bytes: 120, editable: false },
      { path: "world/world.md", kind: "markdown", bytes: 80, editable: true },
    ];
    const contents: Record<string, SourceFileContent> = {
      "game.toml": {
        path: "game.toml",
        kind: "toml",
        editable: false,
        content: 'title = "Dynasty Embers"\n',
      },
      "world/world.md": {
        path: "world/world.md",
        kind: "markdown",
        editable: true,
        content: "# World Bible\n\nThe dynasty is under pressure.\n",
      },
    };
    const dataSource: StudioDataSource = {
      runtimeName: "Test runtime",
      async openProject() {
        return demoProjectData;
      },
      async checkProject() {
        return {
          title: "Dynasty Embers",
          entry_scene: "court-crisis-001",
          scene_count: 1,
          rule_count: 1,
          character_count: 2,
        };
      },
      async playOnceProject() {
        return demoPlayOnceReport("continue");
      },
      async listSourceFiles() {
        return files;
      },
      async readSourceFile(_path, relativePath) {
        const file = contents[relativePath];
        if (!file) {
          throw new Error(`missing source fixture: ${relativePath}`);
        }
        return file;
      },
      async writeSourceFile(_path, relativePath, content) {
        const file = contents[relativePath];
        if (!file) {
          throw new Error(`missing source fixture: ${relativePath}`);
        }
        writes.push({ relativePath, content });
        const updated = { ...file, content };
        contents[relativePath] = updated;
        return updated;
      },
    };

    render(
      <App dataSource={dataSource} initialProjectPath="/tmp/dynasty-embers" />,
    );

    expect(await screen.findByText("Dynasty Embers")).toBeTruthy();
    expect(screen.getAllByText("world/world.md").length).toBeGreaterThan(0);
    expect(screen.getByDisplayValue(/The dynasty is under pressure/)).toBeTruthy();

    fireEvent.change(screen.getByLabelText("Source editor"), {
      target: { value: "# World Bible\n\nThe court has changed.\n" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => {
      expect(writes).toEqual([
        {
          relativePath: "world/world.md",
          content: "# World Bible\n\nThe court has changed.\n",
        },
      ]);
    });
  });

  it("runs playtest and renders fallback trace, errors, review, and diagnostics without raw keys", async () => {
    const fallbackReport = fallbackPlayOnceReport();
    const dataSource = appTestDataSource({
      async playOnceProject() {
        return fallbackReport;
      },
    });

    render(
      <App dataSource={dataSource} initialProjectPath="/tmp/dynasty-embers" />,
    );

    expect(await screen.findByText("Dynasty Embers")).toBeTruthy();

    fireEvent.change(screen.getByLabelText("Playtest input"), {
      target: { value: "continue" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Run turn" }));

    await screen.findByText("Fallback Council");
    expect(screen.getAllByText("trace-fallback").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Fallback").length).toBeGreaterThan(0);
    expect(screen.getByText("provider_timeout")).toBeTruthy();
    expect(screen.getByText("weak_hook")).toBeTruthy();
    expect(
      screen.getByText("planner returned fallback scene `[REDACTED_SECRET]`"),
    ).toBeTruthy();
    expect(
      screen.getByText(
        "Provider pipeline used fallback scene [REDACTED_SECRET]",
      ),
    ).toBeTruthy();
    expect(screen.queryByText(/sk-test-secret/)).toBeNull();
  });
});

function appTestDataSource(
  overrides: Partial<StudioDataSource> = {},
): StudioDataSource {
  const files: SourceFileSummary[] = [
    { path: "game.toml", kind: "toml", bytes: 120, editable: false },
    { path: "world/world.md", kind: "markdown", bytes: 80, editable: true },
  ];
  const contents: Record<string, SourceFileContent> = {
    "game.toml": {
      path: "game.toml",
      kind: "toml",
      editable: false,
      content: 'title = "Dynasty Embers"\n',
    },
    "world/world.md": {
      path: "world/world.md",
      kind: "markdown",
      editable: true,
      content: "# World Bible\n\nThe dynasty is under pressure.\n",
    },
  };

  const base: StudioDataSource = {
    runtimeName: "Test runtime",
    async openProject() {
      return demoProjectData;
    },
    async checkProject() {
      return {
        title: "Dynasty Embers",
        entry_scene: "court-crisis-001",
        scene_count: 1,
        rule_count: 1,
        character_count: 2,
      };
    },
    async playOnceProject(_path, playerInput) {
      return demoPlayOnceReport(playerInput);
    },
    async listSourceFiles() {
      return files;
    },
    async readSourceFile(_path, relativePath) {
      const file = contents[relativePath];
      if (!file) {
        throw new Error(`missing source fixture: ${relativePath}`);
      }
      return file;
    },
    async writeSourceFile(_path, relativePath, content) {
      const file = contents[relativePath];
      if (!file) {
        throw new Error(`missing source fixture: ${relativePath}`);
      }
      const updated = { ...file, content };
      contents[relativePath] = updated;
      return updated;
    },
  };

  return { ...base, ...overrides };
}

function fallbackPlayOnceReport(): PlayOnceReport {
  const report = demoPlayOnceReport("sk-test-secret should not render");
  const review = report.trace.narrative_review;
  if (!review) {
    throw new Error("demo report should include narrative review");
  }

  return {
    ...report,
    scene: {
      ...report.scene,
      key: "fallback-001",
      title: "Fallback Council",
      hook: "A fallback council forms because the requested scene was missing.",
    },
    trace_path: "/tmp/dynasty-embers/traces/trace-fallback.json",
    delta_summary: [],
    trace: {
      ...report.trace,
      id: "trace-fallback",
      player_input: "sk-test-secret should not render",
      selected_choice: "continue",
      action_intent: {
        status: "supported",
        action_type: "continue",
        matched_terms: ["continue"],
        reason: null,
      },
      rule_result: {
        action_type: "continue",
        delta_empty: true,
        state_committed: true,
        error: null,
      },
      planner_result: {
        requested_action_type: "continue",
        scene_key: "fallback-001",
        fallback_used: true,
        error: {
          code: "provider_timeout",
          message: "Provider pipeline used fallback scene [REDACTED_SECRET]",
        },
      },
      diagnostics: [
        {
          stage: "interpret_action",
          status: "completed",
          message: "action `continue` matched 1 term(s)",
        },
        {
          stage: "plan_scene",
          status: "fallback",
          message: "planner returned fallback scene `[REDACTED_SECRET]`",
        },
      ],
      narrative_review: {
        ...review,
        scene_key: "fallback-001",
        score: 42,
        hook_score: 35,
        pacing_score: 45,
        character_consistency_score: 70,
        payoff_score: 30,
        choice_meaningfulness_score: 25,
        ai_slop_risk: 68,
        issues: [
          {
            kind: "weak_hook",
            severity: "warning",
            message: "Fallback scene needs a sharper hook.",
          },
        ],
      },
      errors: [
        {
          code: "provider_timeout",
          message: "Provider pipeline used fallback scene [REDACTED_SECRET]",
        },
      ],
      fallback_used: true,
    },
  };
}
