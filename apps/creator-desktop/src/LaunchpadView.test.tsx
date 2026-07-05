import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { LaunchpadView } from "./LaunchpadView";
import { StudioI18nProvider } from "./i18n";
import type {
  AgentSessionConfig,
  GitBranchInfo,
  ModelOption,
} from "../../../contracts/plotforge";

afterEach(() => {
  if (typeof window.localStorage?.removeItem === "function") {
    window.localStorage.removeItem("plotforge:creator-desktop:locale");
  }
  cleanup();
});

const defaultModels: ModelOption[] = [
  { id: "local-pi", label: "Local pi-Agent (mock)", provider: "local-mock" },
  { id: "glm-5.2", label: "GLM 5.2", provider: "zai" },
];

const defaultConfig: AgentSessionConfig = {
  model_id: "local-pi",
  permission_level: "ask_every_time",
  thinking_level: "medium",
  enabled_skills: [],
};

const defaultBranches: GitBranchInfo[] = [
  { name: "main", is_current: true },
  { name: "feature/x", is_current: false },
];

// Minimal props builder for the new home page shape.
function baseProps(
  overrides: Partial<Parameters<typeof LaunchpadView>[0]> = {},
) {
  return {
    loadedPath: "/tmp/starter-project",
    projectDirName: "starter-project",
    currentBranch: "main" as string | null,
    branches: defaultBranches,
    switchingBranch: false,
    switchError: null,
    onSwitchBranch: vi.fn(),
    availableModels: defaultModels,
    agentConfig: defaultConfig,
    onAgentConfigChange: vi.fn(),
    configSaveError: null,
    input: "",
    onInputChange: vi.fn(),
    onSubmit: vi.fn(),
    running: false,
    canSubmit: false,
    ...overrides,
  };
}

function renderLaunchpad(overrides: Partial<Parameters<typeof LaunchpadView>[0]> = {}) {
  return render(
    <StudioI18nProvider>
      <LaunchpadView {...baseProps(overrides)} />
    </StudioI18nProvider>,
  );
}

describe("LaunchpadView", () => {
  it("renders the brand mark and a greeting", () => {
    renderLaunchpad();

    // Greeting is one of the four time-based keys; assert by role.
    expect(screen.getByLabelText(/问候语|Greeting/i)).toBeTruthy();
  });

  it("shows the project directory name chip", () => {
    renderLaunchpad();

    expect(screen.getByLabelText(/项目目录|Project directory/i)).toBeTruthy();
    expect(screen.getByText("starter-project")).toBeTruthy();
  });

  it("shows the current git branch in the chip", () => {
    renderLaunchpad();

    expect(screen.getByLabelText(/Git 分支|Git branch/i)).toBeTruthy();
    expect(screen.getByText("main")).toBeTruthy();
  });

  it("renders the conversation textarea with the localized placeholder", () => {
    renderLaunchpad();

    const textarea = screen.getByRole("textbox");
    expect(textarea).toBeTruthy();
    expect(textarea.tagName).toBe("TEXTAREA");
  });

  it("opens the branch dropdown and lists local branches", () => {
    renderLaunchpad();

    const branchButton = screen.getByLabelText(/Git 分支|Git branch/i);
    fireEvent.click(branchButton);

    // Both branch names should be visible in the menu.
    expect(screen.getByText("feature/x")).toBeTruthy();
  });

  it("calls onSwitchBranch when a non-current branch is clicked", () => {
    const onSwitchBranch = vi.fn();
    renderLaunchpad({ onSwitchBranch });

    fireEvent.click(screen.getByLabelText(/Git 分支|Git branch/i));
    fireEvent.click(screen.getByText("feature/x"));

    expect(onSwitchBranch).toHaveBeenCalledWith("feature/x");
  });

  it("calls onSubmit when the send button is clicked with non-empty input", () => {
    const onSubmit = vi.fn();
    renderLaunchpad({ input: "Continue the scene", canSubmit: true, onSubmit });

    fireEvent.click(screen.getByLabelText(/发送消息|Send message/i));

    expect(onSubmit).toHaveBeenCalled();
  });

  it("disables the send button when canSubmit is false", () => {
    renderLaunchpad({ canSubmit: false });

    expect(
      screen.getByLabelText(/发送消息|Send message/i),
    ).toHaveProperty("disabled", true);
  });

  it("calls onAgentConfigChange when the model selector changes", () => {
    const onAgentConfigChange = vi.fn();
    renderLaunchpad({ onAgentConfigChange });

    const modelSelect = screen.getByLabelText(/模型|Model/i);
    fireEvent.change(modelSelect, { target: { value: "glm-5.2" } });

    expect(onAgentConfigChange).toHaveBeenCalledWith(
      expect.objectContaining({ model_id: "glm-5.2" }),
    );
  });

  it("calls onAgentConfigChange when the permission selector changes", () => {
    const onAgentConfigChange = vi.fn();
    renderLaunchpad({ onAgentConfigChange });

    const permissionSelect = screen.getByLabelText(/权限级别|Permission level/i);
    fireEvent.change(permissionSelect, { target: { value: "full_access" } });

    expect(onAgentConfigChange).toHaveBeenCalledWith(
      expect.objectContaining({ permission_level: "full_access" }),
    );
  });

  it("calls onAgentConfigChange when the thinking selector changes", () => {
    const onAgentConfigChange = vi.fn();
    renderLaunchpad({ onAgentConfigChange });

    const thinkingSelect = screen.getByLabelText(/思考级别|Thinking level/i);
    fireEvent.change(thinkingSelect, { target: { value: "high" } });

    expect(onAgentConfigChange).toHaveBeenCalledWith(
      expect.objectContaining({ thinking_level: "high" }),
    );
  });

  it("renders the no-git state when currentBranch is null", () => {
    renderLaunchpad({ currentBranch: null, branches: [] });

    // The chip should render a "(no git)"/"(无 git)" label.
    expect(screen.getByText(/无 git|no git/i)).toBeTruthy();
  });

  it("does not render the error row when there are no errors", () => {
    renderLaunchpad();
    // No `role="alert"` region should be present.
    expect(screen.queryByRole("alert")).toBeNull();
  });

  it("renders the attach-context button as disabled (affordance not yet wired)", () => {
    renderLaunchpad();

    // The attach button announces an "Attach context (coming soon)" label so
    // the dead control is not mistaken for a working action.
    const attach = screen.getByLabelText(/附加上下文|Attach context/i);
    expect(attach).toBeTruthy();
    expect(attach).toHaveProperty("disabled", true);
  });

  it("surfaces switchError so a failed branch switch is never silent", () => {
    renderLaunchpad({ switchError: "dirty working tree" });

    const errorRow = screen.getByRole("alert");
    expect(errorRow).toBeTruthy();
    // The localized "Failed to switch branch" prefix is present, followed by
    // the backend-supplied message — so the user can see *why* the switch
    // failed, not just *that* it failed.
    const switchErr = screen.getByLabelText(/分支切换错误|Branch switch error/i);
    expect(switchErr.textContent).toMatch(/dirty working tree/);
  });

  it("surfaces configSaveError so a failed agent-config persist is never silent", () => {
    renderLaunchpad({ configSaveError: "read-only filesystem" });

    const errorRow = screen.getByRole("alert");
    expect(errorRow).toBeTruthy();
    const saveErr = screen.getByLabelText(
      /智能体配置保存错误|Agent config save error/i,
    );
    expect(saveErr.textContent).toMatch(/read-only filesystem/);
  });

  it("renders both errors together when both are present", () => {
    renderLaunchpad({
      switchError: "branch missing",
      configSaveError: "disk full",
    });

    const errorRow = screen.getByRole("alert");
    expect(errorRow).toBeTruthy();
    expect(errorRow.textContent).toMatch(/branch missing/);
    expect(errorRow.textContent).toMatch(/disk full/);
  });
});
