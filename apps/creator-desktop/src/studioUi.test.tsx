import { fireEvent, render, screen } from "@testing-library/react";
import { Boxes, Gauge } from "lucide-react";
import { describe, expect, it, vi } from "vitest";
import {
  agentNativeDesignTokens,
  StudioButton,
  StudioPanel,
  StudioShell,
  StudioStatusChip,
} from "./studioUi";

describe("studioUi", () => {
  it("exposes the agent-native token layer used by the shell", () => {
    expect(agentNativeDesignTokens.shell.graphite).toBe("#111419");
    expect(agentNativeDesignTokens.shell.warmCanvas).toBe("#f7f0df");
    expect(agentNativeDesignTokens.accent.amberAction).toBe("#c98b2f");
    expect(agentNativeDesignTokens.accent.healthGreen).toBe("#34815f");
    expect(agentNativeDesignTokens.accent.acpCyan).toBe("#2e8ca0");
    expect(agentNativeDesignTokens.accent.agentPurple).toBe("#7559a8");
  });

  it("renders shell landmarks, responsive grid classes, right panel, and command dock", () => {
    const selectWorkflow = vi.fn();
    const selectSurface = vi.fn();
    const openProject = vi.fn();

    render(
      <StudioShell
        projectPath="/tmp/dynasty-embers"
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
          subtitle: "Dynasty Embers - Director intent",
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
