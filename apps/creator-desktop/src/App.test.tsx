import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { App } from "./App";
import { demoProjectData } from "./demoStudioData";
import type { StudioDataSource } from "./studioDataSource";
import type { SourceFileContent, SourceFileSummary } from "./tauriBridge";

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
});
