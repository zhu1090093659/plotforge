import { afterEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import {
  enDictionary,
  StudioI18nProvider,
  translate,
  useStudioI18n,
  zhDictionary,
} from "./i18n";

describe("creator desktop i18n", () => {
  it("translates dotted keys to zh", () => {
    expect(translate("common.save", "zh")).toBe("保存");
    expect(translate("trace.traceEvidence", "zh")).toBe("追踪证据");
    expect(translate("export.exportPackage", "zh")).toBe("导出包");
    expect(translate("launchpad.projectOverview", "zh")).toBe("项目概览");
  });

  it("returns en value for en locale", () => {
    expect(translate("common.save", "en")).toBe("Save");
    expect(translate("trace.traceEvidence", "en")).toBe("Trace Evidence");
  });

  it("interpolates params into zh and en", () => {
    expect(translate("common.files", "zh", { count: 5 })).toBe("5 个文件");
    expect(translate("common.files", "en", { count: 5 })).toBe("5 files");
    expect(
      translate("common.scenesRulesCharacters", "zh", {
        count: 3,
        rules: 10,
        characters: 4,
      }),
    ).toBe("3 个场景，10 条规则，4 个角色");
    expect(
      translate("common.projectWaitingProof", "zh", { project: "MyGame" }),
    ).toBe("MyGame / 等待首次证明运行");
  });

  it("falls back to en when zh key is missing", () => {
    expect(translate("brand.studio", "zh")).toBe("PlotForge Studio");
  });

  it("returns the key itself when missing from both dictionaries", () => {
    expect(translate("nonexistent.key", "zh")).toBe("nonexistent.key");
    expect(translate("nonexistent.key", "en")).toBe("nonexistent.key");
  });

  it("escapes regex metacharacters in param names without throwing", () => {
    expect(
      translate("common.files", "en", { ["a.b"]: 7 } as Record<
        string,
        string | number
      >),
    ).toBe("{count} files");
    expect(translate("common.files", "en", { count: 7 })).toBe("7 files");
  });

  it("renders zh text in a component under zh locale", () => {
    function TestComponent() {
      const { t } = useStudioI18n();
      return <p>{t("common.save")}</p>;
    }
    render(
      <StudioI18nProvider defaultLocale="zh">
        <TestComponent />
      </StudioI18nProvider>,
    );
    expect(screen.getByText("保存")).toBeTruthy();
  });

  it("switches locale when setLocale is called from a user event", () => {
    function TestComponent() {
      const { t, locale, setLocale } = useStudioI18n();
      return (
        <button type="button" onClick={() => setLocale("zh")}>
          <span>{locale}</span>
          <span>{t("common.save")}</span>
        </button>
      );
    }
    render(
      <StudioI18nProvider defaultLocale="en">
        <TestComponent />
      </StudioI18nProvider>,
    );
    expect(screen.getByText("en")).toBeTruthy();
    expect(screen.getByText("Save")).toBeTruthy();
    fireEvent.click(screen.getByRole("button"));
    expect(screen.getByText("zh")).toBeTruthy();
    expect(screen.getByText("保存")).toBeTruthy();
  });

  it("en and zh dictionaries have the same key set", () => {
    const enKeys = new Set(Object.keys(enDictionary));
    const zhKeys = new Set(Object.keys(zhDictionary));
    const missingInZh = [...enKeys].filter((key) => !zhKeys.has(key));
    const missingInEn = [...zhKeys].filter((key) => !enKeys.has(key));
    expect(missingInZh).toEqual([]);
    expect(missingInEn).toEqual([]);
  });

  it("every en key resolves to a non-empty value", () => {
    for (const [key, value] of Object.entries(enDictionary)) {
      expect(typeof value).toBe("string");
      expect(value.length).toBeGreaterThan(0);
    }
  });

  it("every en template placeholder is supplied by at least one known param set", () => {
    const placeholderRe = /\{([a-zA-Z0-9_]+)\}/g;
    const templatesWithParams: Array<[string, string[]]> = [];
    for (const [key, value] of Object.entries(enDictionary)) {
      const placeholders = new Set<string>();
      let match: RegExpExecArray | null;
      placeholderRe.lastIndex = 0;
      while ((match = placeholderRe.exec(value)) !== null) {
        placeholders.add(match[1]);
      }
      if (placeholders.size > 0) {
        templatesWithParams.push([key, [...placeholders]]);
      }
    }
    expect(templatesWithParams.length).toBeGreaterThan(0);
    for (const [key, placeholders] of templatesWithParams) {
      for (const placeholder of placeholders) {
        const rendered = translate(key, "en", {
          [placeholder]: "X",
        } as Record<string, string | number>);
        expect(rendered).not.toContain(`{${placeholder}}`);
      }
    }
  });

  it("zh values do not leak untranslated English words (regression: 测试 alternate ending)", () => {
    // Catch un-translated English words inside zh chrome strings. Allows a
    // narrow whitelist of intentional English: brand/product names, file
    // extensions, technical identifiers, units, and widely-accepted
    // borrow words. A run of >=3 ASCII letters that is not whitelisted is
    // treated as a leak. Adjust the whitelist only when adding a new
    // intentional English term, never to silence a real leak.
    const wordRe = /[a-zA-Z]{3,}/g;
    const whitelist = new Set([
      // Brand / product names
      "PlotForge",
      "Studio",
      "Workshop",
      "Steam",
      "Tauri",
      // File formats / protocols / acronyms
      "json",
      "toml",
      "html",
      "http",
      "css",
      "ipc",
      "zip",
      "web",
      "dev",
      // Technical identifiers / contract names kept verbatim
      "RuntimeTrace",
      "PlayOnceReport",
      "ExportManifest",
      "StudioDataSource",
      "Markdown",
      "Provider",
      // Widely-accepted borrow words in zh tech UI
      "adapter",
      "agent",
      "bridge",
      "characters",
      "check",
      "contract",
      "contracts",
      "council",
      "count",
      "envoy",
      "error",
      "export",
      "fallback",
      "files",
      "grain",
      "git",
      "harvest",
      "index",
      "initial",
      "kinds",
      "list",
      "loyalty",
      "manifest",
      "max",
      "media",
      "min",
      "mock",
      "once",
      "open",
      "passed",
      "path",
      "play",
      "plotforge",
      "page",
      "project",
      "provider",
      "resource",
      "review",
      "rules",
      "run",
      "runtime",
      "scene",
      "schema",
      "score",
      "source",
      "static",
      "status",
      "storage",
      "studio",
      "total",
      "trace",
      "worker",
      // Technical identifiers referenced verbatim in zh docs
      "add",
      // Language name kept verbatim (it names the English language itself)
      "English",
    ]);
    const leaks: Array<[string, string]> = [];
    for (const [key, value] of Object.entries(zhDictionary)) {
      let match: RegExpExecArray | null;
      wordRe.lastIndex = 0;
      while ((match = wordRe.exec(value)) !== null) {
        const word = match[0];
        if (!whitelist.has(word) && !whitelist.has(word.toLowerCase())) {
          leaks.push([key, word]);
        }
      }
    }
    expect(leaks).toEqual([]);
  });

  afterEach(() => {
    cleanup();
    if (
      typeof window !== "undefined" &&
      window.localStorage?.removeItem instanceof Function
    ) {
      window.localStorage.removeItem("plotforge:creator-desktop:locale");
    }
  });
});
