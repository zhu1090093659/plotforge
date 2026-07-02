import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { Boxes, Folder, Gauge } from "lucide-react";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  agentNativeDesignTokens,
  Collapsible,
  CollapsibleSection,
  ScenePreviewPlaceholder,
  StudioButton,
  StudioPanel,
  StudioShell,
  StudioStatusChip,
  StudioTabs,
  type StudioNavItem,
} from "./studioUi";

afterEach(() => {
  cleanup();
});

describe("studioUi", () => {
  it("exposes the agent-native token layer used by the shell", () => {
    expect(agentNativeDesignTokens.shell.graphite).toBe("#faf8ff");
    expect(agentNativeDesignTokens.shell.warmCanvas).toBe("#fdfbff");
    expect(agentNativeDesignTokens.accent.violetAction).toBe("#6f54a3");
    expect(agentNativeDesignTokens.accent.healthGreen).toBe("#5a8a7a");
    expect(agentNativeDesignTokens.accent.accentCopper).toBe("#7a5a95");
    expect(agentNativeDesignTokens.accent.agentSage).toBe("#4d3e7a");
  });

  function treeNavItems(overrides?: {
    selectWorkflow?: () => void;
    selectSurface?: () => void;
  }): StudioNavItem[] {
    return [
      {
        id: "command",
        label: "Command Center",
        sublabel: "Command",
        description: "Director intent",
        icon: Gauge,
        selected: true,
        onSelect: overrides?.selectWorkflow ?? (() => {}),
        children: [
          {
            id: "launchpad",
            label: "Launchpad",
            sublabel: "ready",
            description: "Project launchpad",
            icon: Gauge,
            selected: true,
            onSelect: () => {},
          },
          {
            id: "assets",
            label: "Artifact Review",
            sublabel: "ready",
            description: "Review changed assets",
            icon: Boxes,
            selected: false,
            onSelect: overrides?.selectSurface ?? (() => {}),
          },
        ],
      },
      {
        id: "agents",
        label: "Agent Mesh",
        sublabel: "Agents",
        description: "Agent mesh overview",
        icon: Folder,
        selected: false,
        onSelect: () => {},
        children: [
          {
            id: "agent-mesh",
            label: "Agent Mesh Workspace",
            sublabel: "ready",
            description: "Agent mesh workspace",
            icon: Folder,
            selected: false,
            onSelect: () => {},
          },
        ],
      },
    ];
  }

  function renderShell(props?: {
    expandedIds?: Set<string>;
    onToggleExpand?: (id: string) => void;
    navItems?: StudioNavItem[];
    drawerOpen?: boolean;
    onToggleDrawer?: () => void;
    onCloseDrawer?: () => void;
  }) {
    const toggleExpand = props?.onToggleExpand ?? vi.fn();
    render(
      <StudioShell
        projectPath="/tmp/starter-project"
        projectLoading={false}
        onOpenProject={vi.fn()}
        navItems={props?.navItems ?? treeNavItems()}
        expandedIds={props?.expandedIds ?? new Set(["command"])}
        onToggleExpand={toggleExpand}
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
        drawerOpen={props?.drawerOpen ?? false}
        onToggleDrawer={props?.onToggleDrawer ?? vi.fn()}
        onCloseDrawer={props?.onCloseDrawer ?? vi.fn()}
      >
        <StudioPanel>Workspace</StudioPanel>
      </StudioShell>,
    );
    return { toggleExpand };
  }

  it("renders shell landmarks, evidence panel, and command dock", () => {
    renderShell();

    expect(screen.getByLabelText("Studio navigation")).toBeTruthy();
    expect(screen.getByRole("main")).toBeTruthy();
    expect(screen.getByLabelText("Evidence panel")).toBeTruthy();
    expect(screen.getByLabelText("Command dock")).toBeTruthy();
    expect(screen.getByText("Trace visible")).toBeTruthy();
  });

  it("renders an expanded child section in place under its parent", () => {
    renderShell({ expandedIds: new Set(["command"]) });
    // The child "Artifact Review" is rendered in the tree (parent command is expanded).
    expect(screen.getByRole("button", { name: "Artifact Review" })).toBeTruthy();
  });

  it("hides child sections when the parent is collapsed", () => {
    renderShell({ expandedIds: new Set() });
    expect(screen.queryByRole("button", { name: "Artifact Review" })).toBeNull();
  });

  it("toggles expand/collapse on clicking a workflow row without invoking select", () => {
    const selectWorkflow = vi.fn();
    const onToggleExpand = vi.fn();
    renderShell({
      onToggleExpand,
      navItems: treeNavItems({ selectWorkflow }),
      expandedIds: new Set(),
    });

    // Clicking the workflow row toggles expand; it must not call the workflow
    // select handler (which would jump to the default section).
    fireEvent.click(screen.getByRole("button", { name: "Command Center" }));
    expect(onToggleExpand).toHaveBeenCalledTimes(1);
    expect(selectWorkflow).not.toHaveBeenCalled();
  });

  it("opens the drawer via the header menu button and closes via the overlay", () => {
    const onToggleDrawer = vi.fn();
    const onCloseDrawer = vi.fn();
    const { rerender } = render(
      <StudioShell
        projectPath="/tmp/starter-project"
        projectLoading={false}
        onOpenProject={vi.fn()}
        navItems={treeNavItems()}
        expandedIds={new Set(["command"])}
        onToggleExpand={vi.fn()}
        header={{
          eyebrow: "Command Center / Test runtime",
          title: "Project Launchpad",
          subtitle: "Starter Project - Director intent",
          badges: [{ id: "command-center", label: "Command Center" }],
        }}
        topActions={<StudioButton>Refresh</StudioButton>}
        rightPanel={<StudioPanel>Right</StudioPanel>}
        drawerOpen={false}
        onToggleDrawer={onToggleDrawer}
        onCloseDrawer={onCloseDrawer}
      >
        <StudioPanel>Workspace</StudioPanel>
      </StudioShell>,
    );

    // Drawer is closed initially: header hamburger exists, overlay does not.
    expect(screen.getByLabelText("Open navigation")).toBeTruthy();

    fireEvent.click(screen.getByLabelText("Open navigation"));
    expect(onToggleDrawer).toHaveBeenCalledTimes(1);

    // Re-render with drawer open to assert overlay + drawer nav are visible.
    rerender(
      <StudioShell
        projectPath="/tmp/starter-project"
        projectLoading={false}
        onOpenProject={vi.fn()}
        navItems={treeNavItems()}
        expandedIds={new Set(["command"])}
        onToggleExpand={vi.fn()}
        header={{
          eyebrow: "Command Center / Test runtime",
          title: "Project Launchpad",
          subtitle: "Starter Project - Director intent",
          badges: [{ id: "command-center", label: "Command Center" }],
        }}
        topActions={<StudioButton>Refresh</StudioButton>}
        rightPanel={<StudioPanel>Right</StudioPanel>}
        drawerOpen={true}
        onToggleDrawer={onToggleDrawer}
        onCloseDrawer={onCloseDrawer}
      >
        <StudioPanel>Workspace</StudioPanel>
      </StudioShell>,
    );

    // Two Studio navigation landmarks now: the persistent aside and the drawer aside.
    expect(screen.getAllByLabelText("Studio navigation").length).toBe(2);
    // Clicking the overlay closes the drawer.
    const overlay = screen.getByTestId("drawer-overlay");
    fireEvent.click(overlay);
    expect(onCloseDrawer).toHaveBeenCalledTimes(1);
  });

  it("invokes onOpenProject when the open-project button is clicked", () => {
    const openProject = vi.fn();
    render(
      <StudioShell
        projectPath="/tmp/starter-project"
        projectLoading={false}
        onOpenProject={openProject}
        navItems={treeNavItems()}
        expandedIds={new Set(["command"])}
        onToggleExpand={vi.fn()}
        header={{
          eyebrow: "Command Center / Test runtime",
          title: "Project Launchpad",
          subtitle: "Starter Project - Director intent",
          badges: [{ id: "command-center", label: "Command Center" }],
        }}
        topActions={<StudioButton>Refresh</StudioButton>}
        rightPanel={<StudioPanel>Right</StudioPanel>}
        drawerOpen={false}
        onToggleDrawer={vi.fn()}
        onCloseDrawer={vi.fn()}
      >
        <StudioPanel>Workspace</StudioPanel>
      </StudioShell>,
    );

    fireEvent.click(screen.getByTitle("Open project"));
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

  it("uses the current violet/lavender palette, not the prior forge amber/cream", () => {
    const { container } = render(
      <ScenePreviewPlaceholder assetPath={null} />,
    );
    const className = container.firstChild
      ? String((container.firstChild as HTMLElement).className)
      : "";
    expect(className).not.toContain("rgba(214,160,80");
    expect(className).not.toContain("rgba(245,238,224");
    expect(className).toContain("rgba(138,111,184");
    expect(className).toContain("rgba(253,251,255");
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

describe("StudioTabs", () => {
  it("renders a tablist with the first tab selected by default and only its panel mounted", () => {
    render(
      <StudioTabs
        ariaLabel="Test workspace"
        items={[
          { id: "a", label: "Alpha", children: <p>Alpha panel</p> },
          { id: "b", label: "Beta", children: <p>Beta panel</p> },
        ]}
      />,
    );

    const tablist = screen.getByRole("tablist", { name: "Test workspace" });
    expect(tablist).toBeTruthy();
    const alphaTab = screen.getByRole("tab", { name: /Alpha/i });
    const betaTab = screen.getByRole("tab", { name: /Beta/i });
    expect(alphaTab.getAttribute("aria-selected")).toBe("true");
    expect(betaTab.getAttribute("aria-selected")).toBe("false");
    // Alpha panel is mounted; Beta panel is not.
    expect(screen.getByText("Alpha panel")).toBeTruthy();
    expect(screen.queryByText("Beta panel")).toBeNull();
    // aria-controls points from tab to panel; aria-labelledby points back.
    const alphaPanelId = alphaTab.getAttribute("aria-controls");
    expect(alphaPanelId).toBeTruthy();
    const alphaPanel = document.getElementById(alphaPanelId!);
    expect(alphaPanel?.getAttribute("role")).toBe("tabpanel");
    expect(alphaPanel?.getAttribute("aria-labelledby")).toBe(alphaTab.id);
  });

  it("switches the active panel when a tab is clicked", () => {
    render(
      <StudioTabs
        ariaLabel="Test workspace"
        items={[
          { id: "a", label: "Alpha", children: <p>Alpha panel</p> },
          { id: "b", label: "Beta", children: <p>Beta panel</p> },
        ]}
      />,
    );

    expect(screen.queryByText("Beta panel")).toBeNull();
    fireEvent.click(screen.getByRole("tab", { name: /Beta/i }));
    expect(screen.getByRole("tab", { name: /Beta/i }).getAttribute("aria-selected"))
      .toBe("true");
    expect(screen.getByRole("tab", { name: /Alpha/i }).getAttribute("aria-selected"))
      .toBe("false");
    expect(screen.getByText("Beta panel")).toBeTruthy();
    expect(screen.queryByText("Alpha panel")).toBeNull();
  });

  it("renders a badge count next to the tab label when provided", () => {
    render(
      <StudioTabs
        ariaLabel="Test workspace"
        items={[
          { id: "a", label: "Alpha", badge: 3, children: <p /> },
        ]}
      />,
    );
    expect(screen.getByText("3")).toBeTruthy();
  });

  it("falls back to the first tab when the active tab is removed from items", () => {
    const { rerender } = render(
      <StudioTabs
        ariaLabel="Test workspace"
        items={[
          { id: "a", label: "Alpha", children: <p>Alpha panel</p> },
          { id: "b", label: "Beta", children: <p>Beta panel</p> },
        ]}
      />,
    );
    // Select Beta so it is the active tab.
    fireEvent.click(screen.getByRole("tab", { name: /Beta/i }));
    expect(screen.getByText("Beta panel")).toBeTruthy();

    // Re-render with only Alpha (Beta removed). The stale activeId must be
    // reconciled to the first item so a panel is still rendered.
    rerender(
      <StudioTabs
        ariaLabel="Test workspace"
        items={[
          { id: "a", label: "Alpha", children: <p>Alpha panel</p> },
        ]}
      />,
    );
    expect(screen.getByRole("tab", { name: /Alpha/i }).getAttribute("aria-selected"))
      .toBe("true");
    expect(screen.getByText("Alpha panel")).toBeTruthy();
    expect(screen.queryByText("Beta panel")).toBeNull();
  });

  it("supports keyboard navigation via Arrow Right / Home / End", () => {
    render(
      <StudioTabs
        ariaLabel="Test workspace"
        items={[
          { id: "a", label: "Alpha", children: <p>Alpha panel</p> },
          { id: "b", label: "Beta", children: <p>Beta panel</p> },
          { id: "c", label: "Gamma", children: <p>Gamma panel</p> },
        ]}
      />,
    );

    // Focus the first tab, then Arrow Right should select Beta.
    const alphaTab = screen.getByRole("tab", { name: /Alpha/i });
    alphaTab.focus();
    fireEvent.keyDown(screen.getByRole("tablist"), { key: "ArrowRight" });
    expect(screen.getByRole("tab", { name: /Beta/i }).getAttribute("aria-selected"))
      .toBe("true");
    expect(screen.getByText("Beta panel")).toBeTruthy();

    // End should jump to the last tab (Gamma).
    fireEvent.keyDown(screen.getByRole("tablist"), { key: "End" });
    expect(screen.getByRole("tab", { name: /Gamma/i }).getAttribute("aria-selected"))
      .toBe("true");

    // Home should jump back to the first tab (Alpha).
    fireEvent.keyDown(screen.getByRole("tablist"), { key: "Home" });
    expect(screen.getByRole("tab", { name: /Alpha/i }).getAttribute("aria-selected"))
      .toBe("true");
  });
});

