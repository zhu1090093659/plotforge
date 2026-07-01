import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { RuleDraft, RulesEditDocument } from "../../../contracts/plotforge";
import { RulesView } from "./RulesView";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

afterEach(() => cleanup());

function emptyDraft(): RuleDraft {
  return { id: "", action_type: "", resource_key: "", amount: 0 };
}

function renderRulesView(
  overrides: Partial<React.ComponentProps<typeof RulesView>> = {},
) {
  const defaults = {
    rulesEditDocument: null,
    resourceKeys: ["treasury", "army_morale"],
    saving: false,
    formStatus: null,
    ruleDraft: emptyDraft(),
    onRuleDraftChange: vi.fn(),
    onSave: vi.fn(),
    onCreateRuleFromDraft: vi.fn(),
    onUpdateRule: vi.fn(),
    ...overrides,
  };

  return render(<RulesView {...defaults} />);
}

const demoDocument: RulesEditDocument = {
  rules: [
    {
      id: "gain-treasury",
      action_type: "gain",
      conditions: [{ kind: "resource_at_most", key: "treasury", value: 20 }],
      effects: [{ kind: "add_resource", key: "treasury", amount: 10 }],
    },
    {
      id: "trigger-crisis",
      action_type: "crisis",
      conditions: [{ kind: "flag_equals", key: "crisis_active", value: false }],
      effects: [{ kind: "trigger_event", event: "city_crisis" }],
    },
    {
      id: "set-order",
      action_type: "set",
      conditions: [],
      effects: [{ kind: "set_resource", key: "public_order", value: 50 }],
    },
  ],
};

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("RulesView", () => {
  it("shows 'Rule edit document not loaded.' when document is null", () => {
    renderRulesView();
    expect(screen.getByText("Rule edit document not loaded.")).toBeTruthy();
  });

  it("shows rule count when document is loaded", () => {
    renderRulesView({ rulesEditDocument: demoDocument });
    expect(screen.getByText("3 rules")).toBeTruthy();
  });

  it("renders collapsible toggle buttons for each rule using rule id as label", () => {
    renderRulesView({ rulesEditDocument: demoDocument });
    // getAllByRole handles the case where each rule id appears once as a button label
    const gainBtns = screen.getAllByRole("button", { name: /gain-treasury/ });
    expect(gainBtns.length).toBeGreaterThan(0);
    const crisisBtns = screen.getAllByRole("button", { name: /trigger-crisis/ });
    expect(crisisBtns.length).toBeGreaterThan(0);
    const orderBtns = screen.getAllByRole("button", { name: /set-order/ });
    expect(orderBtns.length).toBeGreaterThan(0);
  });

  it("expanding a rule card reveals human-readable condition and effect labels", () => {
    renderRulesView({ rulesEditDocument: demoDocument });

    // Expand gain-treasury card (first button with that name)
    const btns = screen.getAllByRole("button", { name: /gain-treasury/ });
    fireEvent.click(btns[0]);

    // Effect: add treasury +10 — should see the +10 text
    expect(screen.getByText("+10")).toBeTruthy();
  });

  it("does NOT display raw JSON.stringify output for conditions or effects", () => {
    renderRulesView({ rulesEditDocument: demoDocument });
    // Expand all cards
    screen.getAllByRole("button", { name: /gain-treasury/ }).forEach((b) =>
      fireEvent.click(b),
    );
    screen.getAllByRole("button", { name: /trigger-crisis/ }).forEach((b) =>
      fireEvent.click(b),
    );
    screen.getAllByRole("button", { name: /set-order/ }).forEach((b) =>
      fireEvent.click(b),
    );

    // Should not see raw JSON bracket syntax from JSON.stringify
    const fullText = document.body.textContent ?? "";
    expect(fullText).not.toContain('"kind"');
    expect(fullText).not.toContain('"resource_at_most"');
  });

  it("shows trigger_event effect label with event name", () => {
    renderRulesView({ rulesEditDocument: demoDocument });
    screen.getAllByRole("button", { name: /trigger-crisis/ }).forEach((b) =>
      fireEvent.click(b),
    );
    expect(screen.getByText("city_crisis")).toBeTruthy();
  });

  it("shows set_resource effect label with value", () => {
    renderRulesView({ rulesEditDocument: demoDocument });
    screen.getAllByRole("button", { name: /set-order/ }).forEach((b) =>
      fireEvent.click(b),
    );
    expect(screen.getByText("public_order")).toBeTruthy();
    // Value 50 appears in the DOM (may appear multiple times)
    expect(screen.getAllByText("50").length).toBeGreaterThan(0);
  });

  it("shows '(none)' for rules with no conditions", () => {
    renderRulesView({ rulesEditDocument: demoDocument });
    screen.getAllByRole("button", { name: /set-order/ }).forEach((b) =>
      fireEvent.click(b),
    );
    expect(screen.getByText("(none)")).toBeTruthy();
  });

  it("Add Rule collapsible is collapsed by default", () => {
    renderRulesView({ rulesEditDocument: demoDocument });
    // 'New rule id' input should not be visible until Add Rule is opened
    expect(screen.queryByLabelText("New rule id")).toBeNull();
  });

  it("expanding Add Rule collapsible reveals the form", () => {
    renderRulesView({ rulesEditDocument: demoDocument });
    fireEvent.click(screen.getByRole("button", { name: "Add Rule" }));
    expect(screen.getByLabelText("New rule id")).toBeTruthy();
    expect(screen.getByLabelText("New rule action type")).toBeTruthy();
    expect(screen.getByLabelText("New rule resource")).toBeTruthy();
    expect(screen.getByLabelText("New rule amount")).toBeTruthy();
  });

  it("populates resource dropdown from resourceKeys prop", () => {
    renderRulesView({ rulesEditDocument: demoDocument });
    fireEvent.click(screen.getByRole("button", { name: "Add Rule" }));
    expect(screen.getByRole("option", { name: "treasury" })).toBeTruthy();
    expect(screen.getByRole("option", { name: "army_morale" })).toBeTruthy();
  });

  it("shows the add_resource limitation note in the form", () => {
    renderRulesView({ rulesEditDocument: demoDocument });
    fireEvent.click(screen.getByRole("button", { name: "Add Rule" }));
    expect(
      screen.getByText(/only add_resource effect is supported/),
    ).toBeTruthy();
  });

  it("calls onCreateRuleFromDraft when Create Rule is clicked", () => {
    const onCreateRuleFromDraft = vi.fn();
    renderRulesView({
      rulesEditDocument: demoDocument,
      onCreateRuleFromDraft,
    });
    fireEvent.click(screen.getByRole("button", { name: "Add Rule" }));
    fireEvent.click(screen.getByRole("button", { name: "Create Rule" }));
    expect(onCreateRuleFromDraft).toHaveBeenCalledOnce();
  });

  it("calls onRuleDraftChange when draft fields change", () => {
    const onRuleDraftChange = vi.fn();
    renderRulesView({ rulesEditDocument: demoDocument, onRuleDraftChange });
    fireEvent.click(screen.getByRole("button", { name: "Add Rule" }));
    fireEvent.change(screen.getByLabelText("New rule id"), {
      target: { value: "harvest-grain" },
    });
    expect(onRuleDraftChange).toHaveBeenCalledWith(
      expect.objectContaining({ id: "harvest-grain" }),
    );
  });

  it("calls onSave when Save Rules is clicked", () => {
    const onSave = vi.fn();
    renderRulesView({ rulesEditDocument: demoDocument, onSave });
    fireEvent.click(screen.getByRole("button", { name: "Save Rules" }));
    expect(onSave).toHaveBeenCalledOnce();
  });

  it("calls onUpdateRule when a rule field is changed", () => {
    const onUpdateRule = vi.fn();
    renderRulesView({ rulesEditDocument: demoDocument, onUpdateRule });
    // Expand the first rule card to access its inputs
    const btns = screen.getAllByRole("button", { name: /gain-treasury/ });
    fireEvent.click(btns[0]);
    fireEvent.change(screen.getByLabelText("Rule id 1"), {
      target: { value: "gain-treasury-updated" },
    });
    expect(onUpdateRule).toHaveBeenCalledWith(0, {
      id: "gain-treasury-updated",
    });
  });

  it("Save Rules button is disabled while saving", () => {
    renderRulesView({ rulesEditDocument: demoDocument, saving: true });
    // There should be exactly one "Save Rules" button
    const saveButton = screen.getByRole("button", {
      name: "Save Rules",
    }) as HTMLButtonElement;
    expect(saveButton.disabled).toBe(true);
  });

  it("displays success formStatus message", () => {
    renderRulesView({
      rulesEditDocument: demoDocument,
      formStatus: {
        section: "rules",
        tone: "success",
        message: "Rules saved.",
      },
    });
    expect(screen.getByText("Rules saved.")).toBeTruthy();
  });

  it("does not display formStatus from a different section", () => {
    renderRulesView({
      rulesEditDocument: demoDocument,
      formStatus: {
        section: "characters",
        tone: "success",
        message: "Characters saved.",
      },
    });
    expect(screen.queryByText("Characters saved.")).toBeNull();
  });
});
