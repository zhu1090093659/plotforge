import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SourceView } from "./SourceView";
import { StudioI18nProvider } from "./i18n";
import type {
  SourceFileContent,
  SourceFileSummary,
} from "./tauriBridge";

afterEach(() => {
  cleanup();
});

function baseSourceFiles(): SourceFileSummary[] {
  return [
    { path: "game.toml", kind: "toml", bytes: 120, editable: false },
    { path: "world/world.md", kind: "markdown", bytes: 80, editable: true },
  ];
}

function renderSourceView(overrides: Partial<Parameters<typeof SourceView>[0]> = {}) {
  return render(
    <StudioI18nProvider>
      <SourceView
        sourceFiles={baseSourceFiles()}
        selectedFile={null}
        editorContent=""
        setEditorContent={vi.fn()}
        dirty={false}
        saving={false}
        error={null}
        onSelectSourceFile={vi.fn()}
        onSaveSelectedFile={vi.fn()}
        {...overrides}
      />
    </StudioI18nProvider>,
  );
}

describe("SourceView", () => {
  it("shows the source file list directly (no tab switch needed)", () => {
    renderSourceView();

    expect(screen.getByText("game.toml")).toBeTruthy();
    expect(screen.getByText("world/world.md")).toBeTruthy();
  });

  it("calls onSelectSourceFile when a source file is clicked", () => {
    const onSelectSourceFile = vi.fn();
    renderSourceView({ onSelectSourceFile });

    fireEvent.click(screen.getByRole("button", { name: /game\.toml/ }));

    expect(onSelectSourceFile).toHaveBeenCalledWith(
      expect.objectContaining({ path: "game.toml" }),
    );
  });

  it("shows the source editor when a file is selected", () => {
    const selectedFile: SourceFileContent = {
      path: "world/world.md",
      kind: "markdown",
      editable: true,
      content: "# World Bible\n",
    };
    renderSourceView({
      selectedFile,
      editorContent: "# World Bible content",
    });

    const editor = screen.getByLabelText("Source editor");
    expect(editor).toBeTruthy();
    expect((editor as HTMLTextAreaElement).value).toBe("# World Bible content");
  });

  it("calls onSaveSelectedFile when Save is clicked with dirty state", () => {
    const onSaveSelectedFile = vi.fn();
    renderSourceView({
      onSaveSelectedFile,
      selectedFile: {
        path: "world/world.md",
        kind: "markdown",
        editable: true,
        content: "# World Bible\n",
      },
      editorContent: "# Modified content\n",
      dirty: true,
    });

    fireEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(onSaveSelectedFile).toHaveBeenCalledTimes(1);
  });

  it("shows a placeholder prompt when no file is selected", () => {
    renderSourceView({ selectedFile: null });
    expect(screen.getByText("Select a source file to edit.")).toBeTruthy();
  });

  it("renders a read-only textarea and a disabled Save button for a non-editable file", () => {
    renderSourceView({
      selectedFile: {
        path: "game.toml",
        kind: "toml",
        editable: false,
        content: 'title = "Starter"\n',
      },
      editorContent: 'title = "Starter"\n',
      // A non-editable file renders a readOnly textarea, so the editor can
      // never produce a dirty state; with dirty=false the Save button is
      // disabled via the `!dirty || saving` gate.
      dirty: false,
    });
    const editor = screen.getByLabelText("Source editor") as HTMLTextAreaElement;
    expect(editor.hasAttribute("readOnly")).toBe(true);
    expect(
      screen.getByRole("button", { name: "Save" }).hasAttribute("disabled"),
    ).toBe(true);
  });

  it("renders the error banner when the error prop is set", () => {
    renderSourceView({
      selectedFile: {
        path: "world/world.md",
        kind: "markdown",
        editable: true,
        content: "# World Bible\n",
      },
      editorContent: "# World Bible\n",
      error: "write failed: disk full",
    });
    expect(screen.getByText("write failed: disk full")).toBeTruthy();
  });

  it("shows the saving spinner and disables Save while saving", () => {
    renderSourceView({
      selectedFile: {
        path: "world/world.md",
        kind: "markdown",
        editable: true,
        content: "# World Bible\n",
      },
      editorContent: "# World Bible\n",
      dirty: true,
      saving: true,
    });
    expect(
      screen.getByRole("button", { name: "Save" }).hasAttribute("disabled"),
    ).toBe(true);
  });
});
