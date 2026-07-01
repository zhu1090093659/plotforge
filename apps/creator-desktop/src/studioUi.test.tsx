import { fireEvent, render, screen } from "@testing-library/react";
import { Boxes, Gauge } from "lucide-react";
import { describe, expect, it, vi } from "vitest";
import {
  agentNativeDesignTokens,
  Collapsible,
  CollapsibleSection,
  ScenePreviewPlaceholder,
  StudioButton,
  StudioPanel,
  StudioShell,
  StudioStatusChip,
} from "./studioUi";

describe("studioUi", () => {
  it("exposes the agent-native token layer used by the shell", () => {
    expect(agentNativeDesignTokens.shell.graphite).toBe("#1f1a14");
    expect(agentNativeDesignTokens.shell.warmCanvas).toBe("#f4ead4");
    expect(agentNativeDesignTokens.accent.amberAction).toBe("#c98b2f");
    expect(agentNativeDesignTokens.accent.healthGreen).toBe("#34815f");
    expect(agentNativeDesignTokens.accent.accentCopper).toBe("#a85c34");
    expect(agentNativeDesignTokens.accent.agentBrass).toBe("#2d6258");
  });

  it("renders shell landmarks, responsive grid classes, right panel, and command dock", () => {
    const selectWorkflow = vi.fn();
    const selectSurface = vi.fn();
    const openProject = vi.fn();

    render(
      <StudioShell
        projectPath="/tmp/starter-project"
        projectLoading={false}
        onOpenProject={openProject}
        workflowItems={[
          {
            id: "command",
            label: "Command Center",
            sublabel: "Command",
            description: "Director intent",
            icon: Gauge,
            selected: true,
            onSelect: selectWorkflow,
          },
        ]}
        surfaceItems={[
          {
            id: "assets",
            label: "Artifact Review",
            sublabel: "ready",
            description: "Review changed assets",
            icon: Boxes,
            selected: false,
            onSelect: selectSurface,
          },
        ]}
        header={{
          eyebrow: "Command Center / Test runtime",
          title: "Project Launchpad",
          subtitle: "Starter Project - Director intent",
          badges: [{ id: "command-center", label: "Command Center" }],
        }}
        topActions={<StudioButton>Refresh</StudioButton>}
        rightPanel={
          <StudioPanel>
            <StudioStatusChip tone="health">Trace visible</StudioStatusChip>
          </StudioPanel>
        }
        commandDock={<StudioButton variant="primary">Run turn</StudioButton>}
      >
        <StudioPanel>Workspace</StudioPanel>
      </StudioShell>,
    );

    expect(screen.getByLabelText("Studio navigation")).toBeTruthy();
    expect(screen.getByRole("main")).toBeTruthy();
    const evidencePanel = screen.getByLabelText("Evidence panel");
    const commandDock = screen.getByLabelText("Command dock");
    expect(evidencePanel).toBeTruthy();
    expect(commandDock).toBeTruthy();
    const shellGridClass =
      screen.getByTestId("studio-shell-grid").getAttribute("class") ?? "";
    expect(shellGridClass).toContain("grid-cols-[280px_minmax(0,1fr)_320px]");
    expect(shellGridClass).toContain("max-xl:grid-cols-[260px_minmax(0,1fr)]");
    expect(shellGridClass).toContain("max-lg:grid-cols-1");
    expect(evidencePanel.getAttribute("class") ?? "").toContain(
      "max-xl:col-span-2",
    );
    expect(commandDock.parentElement?.getAttribute("class") ?? "").toContain(
      "max-lg:col-span-1",
    );
    expect(screen.getByText("Trace visible")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Command Center" }));
    fireEvent.click(screen.getByRole("button", { name: "Artifact Review" }));
    fireEvent.click(screen.getByTitle("Open project"));

    expect(selectWorkflow).toHaveBeenCalledTimes(1);
    expect(selectSurface).toHaveBeenCalledTimes(1);
    expect(openProject).toHaveBeenCalledTimes(1);
  });
});

describe("ScenePreviewPlaceholder", () => {
  it("renders fallback message when assetPath is null", () => {
    render(<ScenePreviewPlaceholder assetPath={null} />);
    expect(screen.getByText("Scene preview asset unavailable")).toBeTruthy();
    expect(
      screen.getByText("No background asset is declared for this scene."),
    ).toBeTruthy();
  });

  it("renders the provided assetPath when given", () => {
    render(<ScenePreviewPlaceholder assetPath="assets/bg/ruins.webp" />);
    expect(screen.getByText("assets/bg/ruins.webp")).toBeTruthy();
  });
});

describe("Collapsible", () => {
  it("starts collapsed by default (defaultOpen=false)", () => {
    render(
      <Collapsible label="Advanced Options">
        <p>Hidden content</p>
      </Collapsible>,
    );
    const toggle = screen.getByRole("button", { name: /advanced options/i });
    expect(toggle.getAttribute("aria-expanded")).toBe("false");
    expect(screen.queryByText("Hidden content")).toBeNull();
  });

  it("starts open when defaultOpen=true", () => {
    render(
      <Collapsible label="Open Section" defaultOpen>
        <p>Visible content</p>
      </Collapsible>,
    );
    const toggle = screen.getByRole("button", { name: /open section/i });
    expect(toggle.getAttribute("aria-expanded")).toBe("true");
    expect(screen.getByText("Visible content")).toBeTruthy();
  });

  it("toggles open state and aria-expanded on click", () => {
    render(
      <Collapsible label="Toggle Me">
        <p>Toggled content</p>
      </Collapsible>,
    );
    const toggle = screen.getByRole("button", { name: /toggle me/i });

    // Initially closed
    expect(toggle.getAttribute("aria-expanded")).toBe("false");
    expect(screen.queryByText("Toggled content")).toBeNull();

    // Click to open
    fireEvent.click(toggle);
    expect(toggle.getAttribute("aria-expanded")).toBe("true");
    expect(screen.getByText("Toggled content")).toBeTruthy();

    // Click to close
    fireEvent.click(toggle);
    expect(toggle.getAttribute("aria-expanded")).toBe("false");
    expect(screen.queryByText("Toggled content")).toBeNull();
  });

  it("renders badge when provided", () => {
    render(
      <Collapsible label="Items" badge={5}>
        <p>Content</p>
      </Collapsible>,
    );
    expect(screen.getByText("5")).toBeTruthy();
  });

  it("exposes aria-controls pointing to the region id", () => {
    render(
      <Collapsible label="Accessible Section" defaultOpen>
        <p>Region content</p>
      </Collapsible>,
    );
    const toggle = screen.getByRole("button", { name: /accessible section/i });
    const controlsId = toggle.getAttribute("aria-controls");
    expect(controlsId).toBeTruthy();
    const region = document.getElementById(controlsId!);
    expect(region).toBeTruthy();
    expect(region?.getAttribute("role")).toBe("region");
  });

  it("region is labeled by the toggle button", () => {
    render(
      <Collapsible label="Labeled Region" defaultOpen>
        <p>Inner</p>
      </Collapsible>,
    );
    const region = screen.getByRole("region", { name: /labeled region/i });
    expect(region).toBeTruthy();
  });
});

describe("CollapsibleSection", () => {
  it("renders title and wraps children in a StudioPanel", () => {
    render(
      <CollapsibleSection title="Advanced Settings" defaultOpen>
        <p>Section body</p>
      </CollapsibleSection>,
    );
    expect(
      screen.getByRole("button", { name: /advanced settings/i }),
    ).toBeTruthy();
    expect(screen.getByText("Section body")).toBeTruthy();
  });

  it("starts collapsed when defaultOpen is omitted", () => {
    render(
      <CollapsibleSection title="Hidden Section">
        <p>Should not be visible</p>
      </CollapsibleSection>,
    );
    expect(screen.queryByText("Should not be visible")).toBeNull();
    expect(
      screen
        .getByRole("button", { name: /hidden section/i })
        .getAttribute("aria-expanded"),
    ).toBe("false");
  });
});

