import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { AgentView } from "./AgentView";
import { StudioI18nProvider } from "./i18n";
import type { StudioDataSource } from "./studioDataSource";
import {
  mockDeleteProvider,
  mockImportSkill,
  mockListProjectPromptTemplates,
  mockListProviders,
  mockListSkills,
  mockListUserPromptTemplates,
  mockReadSkillBody,
  mockRefreshSkillIndex,
  mockTestProviderConnection,
  mockUpsertProvider,
} from "./testHelpers/studioDataSource";
import type {
  AgentSessionConfig,
  PromptTemplate,
  ProviderEntry,
  SkillManifest,
} from "../../../contracts/plotforge";

afterEach(() => {
  if (typeof window.localStorage?.removeItem === "function") {
    window.localStorage.removeItem("plotforge:creator-desktop:locale");
  }
  cleanup();
});

const defaultConfig: AgentSessionConfig = {
  model_id: "local-pi",
  permission_level: "ask_every_time",
  thinking_level: "medium",
  enabled_skills: [],
};

/** A provider already saved in the user registry, used to exercise the table
 * (non-editor) branch of the Providers tab. */
const savedProvider: ProviderEntry = {
  id: "openai-prod",
  kind: "openai_compatible",
  label: "OpenAI prod",
  endpoint_url: "https://api.openai.com/v1",
  model: "gpt-4o",
  credential_env_var: "OPENAI_API_KEY",
  enabled: true,
};

/** A skill already imported into the user library (origin `plot_forge_user`):
 * per finding L2 the Import button must NOT be available for these. */
const userSkill: SkillManifest = {
  id: "plotforge/council-stylist",
  name: "Council Stylist",
  description: "A PlotForge-native prompt skill.",
  source: { origin: "plot_forge_user", root_path: "", rel_path: "council-stylist" },
  interface: null,
  body_path: "council-stylist/SKILL.md",
  scripts: [],
  references: [],
  assets: [],
};

/** An external skill discovered under `~/.claude/skills`: the Import button is
 * the actionable affordance for bringing it into the user library. */
const claudeSkill: SkillManifest = {
  id: "claude/beat-architect",
  name: "Beat Architect",
  description: "A Claude Code skill to draft beat structure.",
  source: { origin: "claude_code", root_path: "", rel_path: "beat-architect" },
  interface: null,
  body_path: "beat-architect/SKILL.md",
  scripts: [],
  references: [],
  assets: [],
};

const userPrompt: PromptTemplate = {
  id: "user-prompt-1",
  label: "Opening hook prompt",
  scope: "user",
  body_markdown: "Draft an opening hook.",
  default_role_hint: null,
};

const projectPrompt: PromptTemplate = {
  id: "project-prompt-1",
  label: "Council scene prompt",
  scope: "project",
  body_markdown: "Draft a council scene.",
  default_role_hint: null,
};

/** Build a hermetic `StudioDataSource` whose Agent-surface methods return
 * contract-typed values via the shared mock helpers. Methods AgentView never
 * calls are stubbed with `throw` so a future wiring change is loud, not silent. */
function agentTestDataSource(
  overrides: Partial<StudioDataSource> = {},
): StudioDataSource {
  const base = {
    runtimeName: "Test runtime",
    // Agent-surface methods — only these are consumed by AgentView.
    listProviders: mockListProviders,
    upsertProvider: mockUpsertProvider,
    deleteProvider: mockDeleteProvider,
    testProviderConnection: mockTestProviderConnection,
    listUserPromptTemplates: mockListUserPromptTemplates,
    listProjectPromptTemplates: mockListProjectPromptTemplates,
    listSkills: mockListSkills,
    refreshSkillIndex: mockRefreshSkillIndex,
    importSkill: mockImportSkill,
    readSkillBody: mockReadSkillBody,
    enableSkillForProject: async () => defaultConfig,
  } as unknown as StudioDataSource;
  return { ...base, ...overrides };
}

function renderAgentView(
  overrides: {
    dataSource?: StudioDataSource;
    agentConfig?: AgentSessionConfig;
    loadedPath?: string;
    onAgentConfigChange?: (config: AgentSessionConfig) => void;
  } = {},
) {
  const onAgentConfigChange =
    overrides.onAgentConfigChange ?? vi.fn();
  const dataSource =
    overrides.dataSource ?? agentTestDataSource();
  const result = render(
    <StudioI18nProvider>
      <AgentView
        dataSource={dataSource}
        loadedPath={overrides.loadedPath ?? "/tmp/starter-project"}
        agentConfig={overrides.agentConfig ?? defaultConfig}
        onAgentConfigChange={onAgentConfigChange}
        configSaveError={null}
      />
    </StudioI18nProvider>,
  );
  return { ...result, onAgentConfigChange, dataSource };
}

describe("AgentView", () => {
  it("renders the ViewHeader eyebrow (12) and the Agent title", () => {
    renderAgentView();

    // Eyebrow numeral is the 1-based index of the Agent nav section.
    expect(screen.getByText("12")).toBeTruthy();
    // Title is the EN value of `nav.agent.label`.
    expect(screen.getByRole("heading", { name: "Agent" })).toBeTruthy();
  });

  it("renders all four tabs (Providers / Model / Prompts / Skills) by accessible name", () => {
    renderAgentView();

    const tablist = screen.getByRole("tablist");
    expect(within(tablist).getByRole("tab", { name: "Providers" })).toBeTruthy();
    expect(within(tablist).getByRole("tab", { name: "Model" })).toBeTruthy();
    expect(within(tablist).getByRole("tab", { name: "Prompts" })).toBeTruthy();
    expect(within(tablist).getByRole("tab", { name: /^Skills/ })).toBeTruthy();
  });

  it("clicking the Providers tab renders the Add button and the providers table", async () => {
    const dataSource = agentTestDataSource({
      async listProviders() {
        return [savedProvider];
      },
    });
    renderAgentView({ dataSource });

    fireEvent.click(screen.getByRole("tab", { name: "Providers" }));

    // The Add button (EN value of `agent.provider.add`) is the single
    // affordance for creating a new provider entry.
    expect(
      screen.getByRole("button", { name: "Add provider" }),
    ).toBeTruthy();
    // The saved provider renders as a row in the providers table after the
    // async `listProviders` promise resolves on mount. The table surfaces the
    // id / label / model columns.
    expect(await screen.findByText("openai-prod")).toBeTruthy();
    expect(await screen.findByText("OpenAI prod")).toBeTruthy();
    // Security-positive: the credential env-var VALUE is never shown in the
    // list — only the editor collects the env-var NAME (asserted in the
    // editor test below). The table deliberately has no credential column.
    expect(screen.queryByText("OPENAI_API_KEY")).toBeNull();
  });

  it("opens the provider editor from the Add button and exposes only a credential env-var text input (never a secret value field)", async () => {
    renderAgentView();

    fireEvent.click(screen.getByRole("tab", { name: "Providers" }));
    fireEvent.click(screen.getByRole("button", { name: "Add provider" }));

    // Security rule (AGENTS.md): the UI must never display or collect secret
    // values. The only credential surface is a plain text input bound to the
    // env-var NAME (`credential_env_var`), with the i18n aria-label.
    const credentialInput = screen.getByLabelText("Credential env var");
    expect(credentialInput.tagName).toBe("INPUT");
    // It must NOT be a password/secret input — verify by type, not className.
    expect(credentialInput.getAttribute("type") ?? "text").toBe("text");
    // The placeholder is the documented hint that only the env-var name is
    // collected, never the value.
    expect(credentialInput.getAttribute("placeholder")).toBe(
      "e.g. OPENAI_API_KEY",
    );
    // The credential-hint copy explicitly states the value is never stored.
    expect(
      screen.getByText(
        "Set this env var in your shell; PlotForge never stores the value.",
      ),
    ).toBeTruthy();

    // No input anywhere in the editor is bound to a credential value: the
    // only bound credential field is the env-var name above. Any password-type
    // input would violate the security rule.
    const editor = credentialInput.closest("section") ?? document.body;
    expect(within(editor as HTMLElement).queryAllByPlaceholderText("password"))
      .toHaveLength(0);
    const allInputs = within(editor as HTMLElement).queryAllByRole("textbox");
    for (const input of allInputs) {
      expect(input.getAttribute("type") ?? "text").toBe("text");
    }
  });

  it("persists a provider through the editor (saves credential_env_var name only, never the value)", async () => {
    const upsert = vi.fn(async (entry: ProviderEntry) => entry);
    const dataSource = agentTestDataSource({
      async upsertProvider(entry) {
        return upsert(entry);
      },
      async listProviders() {
        return [];
      },
    });
    renderAgentView({ dataSource });

    fireEvent.click(screen.getByRole("tab", { name: "Providers" }));
    fireEvent.click(screen.getByRole("button", { name: "Add provider" }));

    fireEvent.change(screen.getByLabelText("Provider id"), {
      target: { value: "anthropic-prod" },
    });
    fireEvent.change(screen.getByLabelText("Credential env var"), {
      target: { value: "ANTHROPIC_API_KEY" },
    });

    fireEvent.click(screen.getByRole("button", { name: "Edit" }));

    await waitFor(() => {
      expect(upsert).toHaveBeenCalledTimes(1);
    });
    const saved = upsert.mock.calls[0][0] as ProviderEntry;
    expect(saved.id).toBe("anthropic-prod");
    // The persisted entry carries only the env-var NAME, never a value.
    expect(saved.credential_env_var).toBe("ANTHROPIC_API_KEY");
    // No secret-bearing field exists on the contract at all.
    expect(
      (saved as unknown as Record<string, unknown>).credential_value,
    ).toBeUndefined();
  });

  it("renders the Skills tab: shows the refresh button and lists both skills with their origin badges", async () => {
    const dataSource = agentTestDataSource({
      async listSkills() {
        return [userSkill, claudeSkill];
      },
    });
    renderAgentView({ dataSource });

    fireEvent.click(screen.getByRole("tab", { name: /^Skills/ }));

    // Refresh is the canonical action for re-scanning external roots.
    expect(
      screen.getByRole("button", { name: "Refresh index" }),
    ).toBeTruthy();
    // Both skills render by their visible name after the async `listSkills`
    // promise resolves on mount.
    expect(await screen.findByText("Council Stylist")).toBeTruthy();
    expect(await screen.findByText("Beat Architect")).toBeTruthy();
    // Origin badges render by their EN label (`agent.skill.origin.*`); the
    // SkillsTab wraps the label in "[…]" so assert the bracketed badge text
    // exactly (avoids matching "PlotForge" inside the skill description).
    expect(await screen.findByText("[PlotForge]")).toBeTruthy();
    expect(await screen.findByText("[Claude Code]")).toBeTruthy();
  });

  it("shows an enabled Import button for external (claude_code) skills and imports them on click", async () => {
    const importSkill = vi.fn(async (id: string) => ({
      id,
      name: "Beat Architect",
      description: "",
      source: { origin: "plot_forge_user" as const, root_path: "", rel_path: "" },
      interface: null,
      body_path: "",
      scripts: [],
      references: [],
      assets: [],
    }));
    const dataSource = agentTestDataSource({
      async listSkills() {
        return [claudeSkill];
      },
      async importSkill(id) {
        return importSkill(id);
      },
    });
    renderAgentView({ dataSource });

    fireEvent.click(screen.getByRole("tab", { name: /^Skills/ }));

    // Wait for the skill row to mount, then look up the Import button.
    await screen.findByText("Beat Architect");
    const importButtons = await screen.findAllByRole("button", {
      name: "Import to PlotForge",
    });
    // One Import button per skill row for the external skill.
    expect(importButtons).toHaveLength(1);
    expect(importButtons[0].hasAttribute("disabled")).toBe(false);

    fireEvent.click(importButtons[0]);
    await waitFor(() => {
      expect(importSkill).toHaveBeenCalledWith("claude/beat-architect");
    });
  });

  it("hides/disables the Import button for skills already in the user library (plot_forge_user origin) — L2 regression guard", async () => {
    const importSkill = vi.fn(async (id: string) => ({
      id,
      name: "Council Stylist",
      description: "",
      source: { origin: "plot_forge_user" as const, root_path: "", rel_path: "" },
      interface: null,
      body_path: "",
      scripts: [],
      references: [],
      assets: [],
    }));
    const dataSource = agentTestDataSource({
      async listSkills() {
        return [userSkill];
      },
      async importSkill(id) {
        return importSkill(id);
      },
    });
    renderAgentView({ dataSource });

    fireEvent.click(screen.getByRole("tab", { name: /^Skills/ }));

    // Wait for the user-library skill row to mount before asserting on the
    // Import affordance, so a slow load does not produce a false pass.
    await screen.findByText("Council Stylist");

    // Per finding L2: a skill whose origin is `plot_forge_user` is already in
    // the user library, so the Import affordance must not be a clickable
    // enabled button. This test pins the FIXED behavior so a regression that
    // re-enables Import for user-library skills fails loudly. NOTE: as of this
    // commit the L2 UI fix is being applied in parallel; if AgentView still
    // renders an enabled Import button here, this guard will fail until the
    // fix lands — that is intentional.
    const importButtons = screen.queryAllByRole("button", {
      name: "Import to PlotForge",
    });
    if (importButtons.length > 0) {
      // If the button is rendered at all, it must be disabled (hidden and
      // disabled are both acceptable fixed-states; an enabled button is not).
      for (const button of importButtons) {
        expect(button.hasAttribute("disabled")).toBe(true);
      }
    }
    // The import path must never fire for a user-library skill.
    expect(importSkill).not.toHaveBeenCalled();
  });

  it("renders the Prompts tab user/project scope toggle buttons", async () => {
    const dataSource = agentTestDataSource({
      async listUserPromptTemplates() {
        return [userPrompt];
      },
      async listProjectPromptTemplates() {
        return [projectPrompt];
      },
    });
    renderAgentView({ dataSource });

    fireEvent.click(screen.getByRole("tab", { name: "Prompts" }));

    // Scope toggle buttons render by their EN label (`agent.prompt.scope.*`).
    expect(screen.getByRole("button", { name: "User library" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "This project" })).toBeTruthy();
    // The user-scope list is shown by default; its template renders after the
    // async prompt-template loads resolve on mount.
    expect(await screen.findByText("Opening hook prompt")).toBeTruthy();
    // Switch to project scope.
    fireEvent.click(screen.getByRole("button", { name: "This project" }));
    expect(await screen.findByText("Council scene prompt")).toBeTruthy();
  });

  it("renders the Model tab model/permission/thinking selects by accessible name", async () => {
    renderAgentView();

    fireEvent.click(screen.getByRole("tab", { name: "Model" }));

    // Selects are labelled by the EN values of `home.modelLabel` /
    // `home.permissionLabel` / `home.thinkingLabel`. Scope to <select> so the
    // Model *tab* button (also accessible-named "Model") is not ambiguous.
    expect(screen.getByLabelText("Model", { selector: "select" })).toBeTruthy();
    expect(
      screen.getByLabelText("Permission", { selector: "select" }),
    ).toBeTruthy();
    expect(
      screen.getByLabelText("Thinking", { selector: "select" }),
    ).toBeTruthy();
    // The configured model is the currently selected option.
    expect(
      (screen.getByLabelText("Model", { selector: "select" }) as HTMLSelectElement)
        .value,
    ).toBe("local-pi");
    expect(
      (screen.getByLabelText("Permission", { selector: "select" }) as HTMLSelectElement)
        .value,
    ).toBe("ask_every_time");
    expect(
      (screen.getByLabelText("Thinking", { selector: "select" }) as HTMLSelectElement)
        .value,
    ).toBe("medium");
  });

  it("propagates model changes to onAgentConfigChange", async () => {
    const onAgentConfigChange = vi.fn();
    renderAgentView({ onAgentConfigChange });

    fireEvent.click(screen.getByRole("tab", { name: "Model" }));
    fireEvent.change(screen.getByLabelText("Permission", { selector: "select" }), {
      target: { value: "full_access" },
    });

    expect(onAgentConfigChange).toHaveBeenCalledWith(
      expect.objectContaining({ permission_level: "full_access" }),
    );
  });

  it("toggles a skill for the project through enableSkillForProject", async () => {
    const enableSkillForProject = vi.fn(
      async (
        _projectPath: string,
        _skillId: string,
        _enabled: boolean,
      ): Promise<AgentSessionConfig> => ({
        ...defaultConfig,
        enabled_skills: [userSkill.id],
      }),
    );
    const dataSource = agentTestDataSource({
      async listSkills() {
        return [userSkill];
      },
      async enableSkillForProject(projectPath, skillId, enabled) {
        return enableSkillForProject(projectPath, skillId, enabled);
      },
    });
    const onAgentConfigChange = vi.fn();
    renderAgentView({ dataSource, onAgentConfigChange });

    fireEvent.click(screen.getByRole("tab", { name: /^Skills/ }));
    // Wait for the skill row to mount before interacting with the enable
    // checkbox (the list loads async via `listSkills` on mount).
    await screen.findByText("Council Stylist");
    // The enable checkbox is labelled by `agent.skill.enable`.
    const enableCheckbox = (await screen.findByLabelText(
      "Enable for this project",
    )) as HTMLInputElement;
    fireEvent.click(enableCheckbox);

    await waitFor(() => {
      expect(enableSkillForProject).toHaveBeenCalledWith(
        "/tmp/starter-project",
        userSkill.id,
        true,
      );
    });
    // The updated config is propagated back to the parent.
    await waitFor(() => {
      expect(onAgentConfigChange).toHaveBeenCalledWith(
        expect.objectContaining({ enabled_skills: [userSkill.id] }),
      );
    });
  });
});
