import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SettingsView } from "./SettingsView";
import { StudioI18nProvider, type StudioLocale } from "./i18n";
import type { StudioDataSource } from "./studioDataSource";
import {
  mockDeleteProvider,
  mockDeleteImageProvider,
  mockDeleteTtsProvider,
  mockImportSkill,
  mockGetUsageSummary,
  mockListImageProviders,
  mockListTtsProviders,
  mockListProjectPromptTemplates,
  mockListProviders,
  mockListRemoteModels,
  mockListSkills,
  mockListUserPromptTemplates,
  mockReadSkillBody,
  mockRefreshSkillIndex,
  mockTestProviderConnection,
  mockTestImageProvider,
  mockTestTtsProvider,
  mockUpsertProvider,
  mockUpsertImageProvider,
  mockUpsertTtsProvider,
} from "./testHelpers/studioDataSource";
import type {
  AgentSessionConfig,
  ImageProviderEntry,
  McpServerEntry,
  PromptTemplate,
  ProviderEntry,
  SkillManifest,
  TtsProviderEntry,
  UsageSummary,
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
  enabled_mcp_servers: [],
};

/** A provider already saved in the user registry, used to exercise the table
 * (non-editor) branch of the Providers area inside the Agent tab. */
const savedProvider: ProviderEntry = {
  id: "openai-prod",
  kind: "openai_compatible",
  label: "OpenAI prod",
  endpoint_url: "https://api.openai.com/v1",
  model: "gpt-4o",
  credential_env_var: "OPENAI_API_KEY",
  enabled: true,
};

const savedImageProvider: ImageProviderEntry = {
  id: "image-prod",
  endpoint_url: "https://api.openai.com/v1",
  model: "gpt-image-1",
  credential_env_var: "OPENAI_API_KEY",
  enabled: true,
  default_size: "1024x1024",
  default_quality: "medium",
};

const savedTtsProvider: TtsProviderEntry = {
  id: "tts-prod",
  endpoint_url: "https://api.openai.com/v1",
  model: "gpt-4o-mini-tts",
  credential_env_var: "OPENAI_API_KEY",
  enabled: true,
  voice: "coral",
  format: "mp3",
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
 * contract-typed values via the shared mock helpers. Methods SettingsView never
 * calls are stubbed with `throw` so a future wiring change is loud, not silent. */
function settingsTestDataSource(
  overrides: Partial<StudioDataSource> = {},
): StudioDataSource {
  const base = {
    runtimeName: "Test runtime",
    // Agent-surface methods — only these are consumed by SettingsView.
    listProviders: mockListProviders,
    getUsageSummary: mockGetUsageSummary,
    getProviderCostReport: async (providerId: string) => ({
      provider_id: providerId,
      text_calls: 0,
      image_calls: 0,
      tts_calls: 0,
      input_tokens: 0,
      output_tokens: 0,
      spent_cost_units: 0,
    }),
    upsertProvider: mockUpsertProvider,
    deleteProvider: mockDeleteProvider,
    testProviderConnection: mockTestProviderConnection,
    listRemoteModels: mockListRemoteModels,
    listImageProviders: mockListImageProviders,
    upsertImageProvider: mockUpsertImageProvider,
    deleteImageProvider: mockDeleteImageProvider,
    testImageProvider: mockTestImageProvider,
    listTtsProviders: mockListTtsProviders,
    upsertTtsProvider: mockUpsertTtsProvider,
    deleteTtsProvider: mockDeleteTtsProvider,
    testTtsProvider: mockTestTtsProvider,
    listUserPromptTemplates: mockListUserPromptTemplates,
    listProjectPromptTemplates: mockListProjectPromptTemplates,
    listSkills: mockListSkills,
    refreshSkillIndex: mockRefreshSkillIndex,
    importSkill: mockImportSkill,
    readSkillBody: mockReadSkillBody,
    enableSkillForProject: async () => defaultConfig,
    // MCP-surface methods (Phase 6): default to empty/ok so the MCP tab
    // renders its real management surface without a live server.
    listMcpServers: async () => [],
    upsertMcpServer: async (entry: McpServerEntry) => entry,
    deleteMcpServer: async () => undefined,
    testMcpServer: async () => ({ ok: true, message: "ok", tools_count: 0 }),
    listMcpTools: async () => [],
    invokeMcpTool: async () => ({ ok: true, content: [], is_error: false }),
    enableMcpServerForProject: async () => defaultConfig,
  } as unknown as StudioDataSource;
  return { ...base, ...overrides };
}

function renderSettingsView(
  overrides: {
    dataSource?: StudioDataSource;
    agentConfig?: AgentSessionConfig;
    loadedPath?: string;
    locale?: StudioLocale;
    onAgentConfigChange?: (config: AgentSessionConfig) => void;
  } = {},
) {
  const onAgentConfigChange =
    overrides.onAgentConfigChange ?? vi.fn();
  const dataSource =
    overrides.dataSource ?? settingsTestDataSource();
  const result = render(
    <StudioI18nProvider defaultLocale={overrides.locale}>
      <SettingsView
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

describe("SettingsView", () => {
  it("renders the ViewHeader eyebrow (12) and the Settings title", () => {
    renderSettingsView();

    // Eyebrow numeral is the 1-based index of the Settings nav section.
    expect(screen.getByText("12")).toBeTruthy();
    // Title is the EN value of `nav.settings.label`.
    expect(screen.getByRole("heading", { name: "Settings" })).toBeTruthy();
  });

  it("renders all three tabs (Agent / MCP / Skills) by accessible name", () => {
    renderSettingsView();

    const tablist = screen.getByRole("tablist");
    expect(within(tablist).getByRole("tab", { name: "Agent" })).toBeTruthy();
    expect(within(tablist).getByRole("tab", { name: "MCP" })).toBeTruthy();
    expect(within(tablist).getByRole("tab", { name: "Skills" })).toBeTruthy();
  });

  it("renders typed usage totals and provider rows, then refreshes on demand", async () => {
    const summary: UsageSummary = {
      total_input_tokens: 1500,
      total_output_tokens: 375,
      total_spent_cost_units: 42,
      by_provider: {
        "openai-prod": {
          provider_id: "openai-prod",
          text_calls: 3,
          image_calls: 1,
          tts_calls: 2,
          input_tokens: 1200,
          output_tokens: 300,
          spent_cost_units: 36,
        },
      },
    };
    const getUsageSummary = vi.fn().mockResolvedValue(summary);
    renderSettingsView({
      dataSource: settingsTestDataSource({ getUsageSummary }),
    });

    expect(await screen.findByRole("heading", { name: "Usage" })).toBeTruthy();
    expect(screen.getByText("1,500")).toBeTruthy();
    expect(screen.getByText("375")).toBeTruthy();
    expect(screen.getByText("42")).toBeTruthy();
    const table = screen.getByRole("table", {
      name: "Provider usage breakdown",
    });
    expect(within(table).getByText("openai-prod")).toBeTruthy();
    expect(within(table).getByText("1,200")).toBeTruthy();
    expect(getUsageSummary).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByRole("button", { name: "Refresh" }));
    await waitFor(() => expect(getUsageSummary).toHaveBeenCalledTimes(2));
  });

  it("renders the usage empty state and Chinese copy without adding a fourth tab", async () => {
    renderSettingsView({ locale: "zh" });

    expect(await screen.findByRole("heading", { name: "用量" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "刷新" })).toBeTruthy();
    expect(screen.getByText("尚未记录任何提供商用量。")).toBeTruthy();
    expect(screen.getAllByRole("tab")).toHaveLength(3);
  });

  it("clicking the Agent tab surfaces the Add provider button and the providers table", async () => {
    const dataSource = settingsTestDataSource({
      async listProviders() {
        return [savedProvider];
      },
    });
    renderSettingsView({ dataSource });

    // Agent tab is the default, but click explicitly so the test does not rely
    // on default-tab behavior.
    fireEvent.click(screen.getByRole("tab", { name: "Agent" }));

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
    renderSettingsView();

    fireEvent.click(screen.getByRole("tab", { name: "Agent" }));
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
    const dataSource = settingsTestDataSource({
      async upsertProvider(entry) {
        return upsert(entry);
      },
      async listProviders() {
        return [];
      },
    });
    renderSettingsView({ dataSource });

    fireEvent.click(screen.getByRole("tab", { name: "Agent" }));
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

  it("fetches remote models into the provider editor combobox on success", async () => {
    const listRemoteModels = vi.fn(async (_providerId: string) => [
      {
        id: "gpt-4o",
        owned_by: "openai",
        created: 1715367600,
        max_input_tokens: 128000,
        max_output_tokens: 16384,
      },
      {
        id: "gpt-4o-mini",
        owned_by: "openai",
        created: 1715367600,
        max_input_tokens: 128000,
        max_output_tokens: 16384,
      },
    ]);
    const dataSource = settingsTestDataSource({
      async listRemoteModels(providerId) {
        return listRemoteModels(providerId);
      },
    });
    renderSettingsView({ dataSource });

    fireEvent.click(screen.getByRole("tab", { name: "Agent" }));
    fireEvent.click(screen.getByRole("button", { name: "Add provider" }));

    // A provider id is required before the Fetch button is enabled, so set it.
    fireEvent.change(screen.getByLabelText("Provider id"), {
      target: { value: "openai-prod" },
    });

    // The Fetch models button calls `listRemoteModels(providerId)`.
    fireEvent.click(screen.getByRole("button", { name: "Fetch models" }));
    await waitFor(() => {
      expect(listRemoteModels).toHaveBeenCalledWith("openai-prod");
    });

    // The fetched model ids appear as <datalist> suggestions. A combobox keeps
    // the text input editable (the user is never forced to pick a suggestion).
    const modelInput = screen.getByLabelText("Model", { selector: "input" });
    const listId = modelInput.getAttribute("list") ?? "";
    const datalist = document.getElementById(listId) as HTMLDataListElement | null;
    expect(datalist).not.toBeNull();
    const optionValues = Array.from(datalist!.options).map((o) => o.value);
    expect(optionValues).toContain("gpt-4o");
    expect(optionValues).toContain("gpt-4o-mini");
    // Selecting a suggestion still leaves the input free-typable.
    fireEvent.change(modelInput, { target: { value: "gpt-4o" } });
    expect((modelInput as HTMLInputElement).value).toBe("gpt-4o");
  });

  it("surfaces a fetch error in the provider editor when listRemoteModels rejects", async () => {
    const listRemoteModels = vi.fn(async (_providerId: string) => {
      throw new Error("Missing credential");
    });
    const dataSource = settingsTestDataSource({
      async listRemoteModels(providerId) {
        return listRemoteModels(providerId);
      },
    });
    renderSettingsView({ dataSource });

    fireEvent.click(screen.getByRole("tab", { name: "Agent" }));
    fireEvent.click(screen.getByRole("button", { name: "Add provider" }));
    fireEvent.change(screen.getByLabelText("Provider id"), {
      target: { value: "openai-prod" },
    });

    fireEvent.click(screen.getByRole("button", { name: "Fetch models" }));
    // The redaction-safe error message surfaces inside the editor (role=alert),
    // never the raw provider response body.
    expect(await screen.findByText("Missing credential")).toBeTruthy();
  });

  it("renders the max_output_tokens field in the provider editor form", async () => {
    const upsert = vi.fn(async (entry: ProviderEntry) => entry);
    const dataSource = settingsTestDataSource({
      async upsertProvider(entry) {
        return upsert(entry);
      },
      async listProviders() {
        return [];
      },
    });
    renderSettingsView({ dataSource });

    fireEvent.click(screen.getByRole("tab", { name: "Agent" }));
    fireEvent.click(screen.getByRole("button", { name: "Add provider" }));

    // The max_output_tokens field is an optional numeric input labelled by the
    // i18n string `agent.provider.maxOutputTokens` ("Max output tokens (optional)").
    const maxTokensInput = screen.getByLabelText("Max output tokens (optional)", {
      selector: "input",
    });
    expect(maxTokensInput.getAttribute("type")).toBe("number");
    // The helper text explains it overrides the default.
    expect(
      screen.getByText(
        /Overrides the provider's default output token cap/,
      ),
    ).toBeTruthy();

    // Setting a value propagates it to the persisted ProviderEntry.
    fireEvent.change(maxTokensInput, { target: { value: "8192" } });
    fireEvent.click(screen.getByRole("button", { name: "Edit" }));
    await waitFor(() => {
      expect(upsert).toHaveBeenCalledTimes(1);
    });
    const saved = upsert.mock.calls[0][0] as ProviderEntry;
    expect(saved.max_output_tokens).toBe(8192);
  });

  it("supports add, edit, delete, and test actions for image providers", async () => {
    const upsert = vi.fn(async (entry: ImageProviderEntry) => entry);
    const remove = vi.fn(async (_id: string) => savedImageProvider);
    const probe = vi.fn(async (_id: string) => ({
      ok: true,
      message: "image probe completed",
    }));
    const dataSource = settingsTestDataSource({
      async listImageProviders() {
        return [savedImageProvider];
      },
      async upsertImageProvider(entry) {
        return upsert(entry);
      },
      async deleteImageProvider(id) {
        return remove(id);
      },
      async testImageProvider(id) {
        return probe(id);
      },
    });
    renderSettingsView({ dataSource });

    const heading = await screen.findByRole("heading", {
      name: "Image providers",
    });
    const section = heading.closest("section") as HTMLElement;
    expect(await within(section).findByText("image-prod")).toBeTruthy();

    fireEvent.click(
      within(section).getByRole("button", { name: "Add image provider" }),
    );
    fireEvent.change(within(section).getByLabelText("Provider id"), {
      target: { value: "image-new" },
    });
    fireEvent.change(within(section).getByLabelText("Endpoint URL"), {
      target: { value: "https://images.example/v1" },
    });
    fireEvent.change(within(section).getByLabelText("Model"), {
      target: { value: "image-new-model" },
    });
    fireEvent.change(within(section).getByLabelText("Credential env var"), {
      target: { value: "IMAGE_API_KEY" },
    });
    fireEvent.click(
      within(section).getByRole("button", { name: "Save provider" }),
    );
    await waitFor(() => expect(upsert).toHaveBeenCalledTimes(1));
    expect(upsert.mock.calls[0][0].credential_env_var).toBe("IMAGE_API_KEY");

    expect(await within(section).findByText("image-prod")).toBeTruthy();
    fireEvent.click(within(section).getByRole("button", { name: "Edit" }));
    expect(
      (within(section).getByLabelText("Provider id") as HTMLInputElement).value,
    ).toBe("image-prod");
    fireEvent.click(within(section).getByRole("button", { name: "Cancel" }));

    fireEvent.click(
      within(section).getByRole("button", { name: "Test connection" }),
    );
    await waitFor(() => expect(probe).toHaveBeenCalledWith("image-prod"));
    expect(
      await within(section).findByText(/image probe completed/),
    ).toBeTruthy();

    fireEvent.click(within(section).getByRole("button", { name: "Delete" }));
    await waitFor(() => expect(remove).toHaveBeenCalledWith("image-prod"));
  });

  it("supports add, edit, delete, and test actions for TTS providers", async () => {
    const upsert = vi.fn(async (entry: TtsProviderEntry) => entry);
    const remove = vi.fn(async (_id: string) => savedTtsProvider);
    const probe = vi.fn(async (_id: string) => ({
      ok: false,
      message: "TTS probe failed safely",
    }));
    const dataSource = settingsTestDataSource({
      async listTtsProviders() {
        return [savedTtsProvider];
      },
      async upsertTtsProvider(entry) {
        return upsert(entry);
      },
      async deleteTtsProvider(id) {
        return remove(id);
      },
      async testTtsProvider(id) {
        return probe(id);
      },
    });
    renderSettingsView({ dataSource });

    const heading = await screen.findByRole("heading", {
      name: "TTS providers",
    });
    const section = heading.closest("section") as HTMLElement;
    expect(await within(section).findByText("tts-prod")).toBeTruthy();

    fireEvent.click(
      within(section).getByRole("button", { name: "Add TTS provider" }),
    );
    fireEvent.change(within(section).getByLabelText("Provider id"), {
      target: { value: "tts-new" },
    });
    fireEvent.change(within(section).getByLabelText("Endpoint URL"), {
      target: { value: "https://speech.example/v1" },
    });
    fireEvent.change(within(section).getByLabelText("Model"), {
      target: { value: "tts-new-model" },
    });
    fireEvent.change(within(section).getByLabelText("Credential env var"), {
      target: { value: "TTS_API_KEY" },
    });
    fireEvent.change(within(section).getByLabelText("Default voice"), {
      target: { value: "alloy" },
    });
    fireEvent.click(
      within(section).getByRole("button", { name: "Save provider" }),
    );
    await waitFor(() => expect(upsert).toHaveBeenCalledTimes(1));
    expect(upsert.mock.calls[0][0].credential_env_var).toBe("TTS_API_KEY");

    expect(await within(section).findByText("tts-prod")).toBeTruthy();
    fireEvent.click(within(section).getByRole("button", { name: "Edit" }));
    expect(
      (within(section).getByLabelText("Provider id") as HTMLInputElement).value,
    ).toBe("tts-prod");
    fireEvent.click(within(section).getByRole("button", { name: "Cancel" }));

    fireEvent.click(
      within(section).getByRole("button", { name: "Test connection" }),
    );
    await waitFor(() => expect(probe).toHaveBeenCalledWith("tts-prod"));
    expect(
      await within(section).findByText(/TTS probe failed safely/),
    ).toBeTruthy();

    fireEvent.click(within(section).getByRole("button", { name: "Delete" }));
    await waitFor(() => expect(remove).toHaveBeenCalledWith("tts-prod"));
  });

  it("renders the Prompts user/project scope toggle buttons inside the Agent tab", async () => {
    const dataSource = settingsTestDataSource({
      async listUserPromptTemplates() {
        return [userPrompt];
      },
      async listProjectPromptTemplates() {
        return [projectPrompt];
      },
    });
    renderSettingsView({ dataSource });

    fireEvent.click(screen.getByRole("tab", { name: "Agent" }));

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

  it("renders the Model selects inside the Agent tab by accessible name", async () => {
    renderSettingsView();

    fireEvent.click(screen.getByRole("tab", { name: "Agent" }));

    // Selects are labelled by the EN values of `home.aria.modelSelect` /
    // `home.aria.permissionSelect` / `home.aria.thinkingSelect` (the shared
    // selectors in ./agentConfigSelectors). Scope to <select> so the labels
    // resolve to the select elements, not the surrounding <label> wrappers.
    expect(screen.getByLabelText("Model", { selector: "select" })).toBeTruthy();
    expect(
      screen.getByLabelText("Permission level", { selector: "select" }),
    ).toBeTruthy();
    expect(
      screen.getByLabelText("Thinking level", { selector: "select" }),
    ).toBeTruthy();
    // The configured model is the currently selected option.
    expect(
      (screen.getByLabelText("Model", { selector: "select" }) as HTMLSelectElement)
        .value,
    ).toBe("local-pi");
    expect(
      (screen.getByLabelText("Permission level", { selector: "select" }) as HTMLSelectElement)
        .value,
    ).toBe("ask_every_time");
    expect(
      (screen.getByLabelText("Thinking level", { selector: "select" }) as HTMLSelectElement)
        .value,
    ).toBe("medium");
  });

  it("propagates model changes to onAgentConfigChange", async () => {
    const onAgentConfigChange = vi.fn();
    renderSettingsView({ onAgentConfigChange });

    fireEvent.click(screen.getByRole("tab", { name: "Agent" }));
    fireEvent.change(
      screen.getByLabelText("Permission level", { selector: "select" }),
      { target: { value: "full_access" } },
    );

    expect(onAgentConfigChange).toHaveBeenCalledWith(
      expect.objectContaining({ permission_level: "full_access" }),
    );
  });

  it("renders the MCP tab with real server management surface (Phase 6)", async () => {
    renderSettingsView();

    fireEvent.click(screen.getByRole("tab", { name: "MCP" }));

    // Phase 6: the MCP tab is a real server management surface, not a
    // placeholder. The "Add server" button must be present, and the empty
    // state (no servers registered) is shown — NOT the old "coming in a
    // future release" placeholder. The empty state renders after the
    // `listMcpServers()` promise resolves, so use `findByText` (async).
    expect(screen.getByRole("button", { name: "Add server" })).toBeTruthy();
    expect(
      await screen.findByText("No MCP servers registered. Add a server to begin."),
    ).toBeTruthy();
    // Regression guard: the old placeholder copy must NOT appear.
    expect(
      screen.queryByText("MCP server configuration is coming in a future release."),
    ).toBeNull();
    // No launch-promise language (no Steam publishing / platform approval).
    const mcpPanelText = screen.getByRole("tabpanel").textContent ?? "";
    expect(mcpPanelText).not.toMatch(/publish to steam|automatic.*publish|platform approval/i);
  });

  it("renders the Skills tab: shows the refresh button and lists both skills with their origin badges", async () => {
    const dataSource = settingsTestDataSource({
      async listSkills() {
        return [userSkill, claudeSkill];
      },
    });
    renderSettingsView({ dataSource });

    fireEvent.click(screen.getByRole("tab", { name: "Skills" }));

    // Refresh is the canonical action for re-scanning external roots.
    expect(
      screen.getByRole("button", { name: "Refresh index" }),
    ).toBeTruthy();
    // Both skills render by their visible name after the async `listSkills`
    // promise resolves on mount.
    expect(await screen.findByText("Council Stylist")).toBeTruthy();
    expect(await screen.findByText("Beat Architect")).toBeTruthy();
    // Origin badges render by their EN label (`agent.skill.origin.*`); the
    // SkillsSection wraps the label in "[…]" so assert the bracketed badge
    // text exactly (avoids matching "PlotForge" inside the skill description).
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
    const dataSource = settingsTestDataSource({
      async listSkills() {
        return [claudeSkill];
      },
      async importSkill(id) {
        return importSkill(id);
      },
    });
    renderSettingsView({ dataSource });

    fireEvent.click(screen.getByRole("tab", { name: "Skills" }));

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
    const dataSource = settingsTestDataSource({
      async listSkills() {
        return [userSkill];
      },
      async importSkill(id) {
        return importSkill(id);
      },
    });
    renderSettingsView({ dataSource });

    fireEvent.click(screen.getByRole("tab", { name: "Skills" }));

    // Wait for the user-library skill row to mount before asserting on the
    // Import affordance, so a slow load does not produce a false pass.
    await screen.findByText("Council Stylist");

    // Per finding L2: a skill whose origin is `plot_forge_user` is already in
    // the user library, so the Import affordance must not be a clickable
    // enabled button. This test pins the FIXED behavior so a regression that
    // re-enables Import for user-library skills fails loudly.
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
    const dataSource = settingsTestDataSource({
      async listSkills() {
        return [userSkill];
      },
      async enableSkillForProject(projectPath, skillId, enabled) {
        return enableSkillForProject(projectPath, skillId, enabled);
      },
    });
    const onAgentConfigChange = vi.fn();
    renderSettingsView({ dataSource, onAgentConfigChange });

    fireEvent.click(screen.getByRole("tab", { name: "Skills" }));
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
