import { describe, expect, it } from "vitest";
import { translate } from "./i18n";

describe("creator desktop i18n", () => {
  it("translates legacy Studio chrome that is still rendered by App panels", () => {
    expect(translate("Source Artifacts", "zh")).toBe("源产物");
    expect(translate("Artifact Text Editor", "zh")).toBe("产物文本编辑器");
    expect(translate("Asset Maintenance", "zh")).toBe("资产维护");
    expect(translate("No source file selected", "zh")).toBe("未选择源文件");
    expect(
      translate("No asset records or scene background paths found.", "zh"),
    ).toBe("未找到资产记录或场景背景路径。");
    expect(translate("Save Visual Bible", "zh")).toBe("保存视觉设定");
    expect(translate("No Visual Bible style cards in project data.", "zh")).toBe(
      "项目数据中没有视觉设定风格卡。",
    );
  });

  it("translates proof, trace, and boundary evidence chrome", () => {
    expect(translate("Run a playtest turn to create proof", "zh")).toBe(
      "运行试玩回合以创建证明",
    );
    expect(translate("No playable result", "zh")).toBe("没有可玩结果");
    expect(translate("Package Evidence Summary", "zh")).toBe("包证据摘要");
    expect(translate("AI Usage Disclosure", "zh")).toBe("AI 使用披露");
    expect(
      translate(
        "Browser mode uses the HTTP dev bridge backed by plotforge-studio.",
        "zh",
      ),
    ).toBe("浏览器模式使用由 plotforge-studio 支撑的 HTTP dev bridge。");
  });

  it("translates common dynamic counts and backend IO errors", () => {
    expect(translate("0 style cards", "zh")).toBe("0 张风格卡");
    expect(translate("0 voice cards", "zh")).toBe("0 张语音卡");
    expect(
      translate(
        "/tmp/starter-project: io error at /tmp/starter-project/game.toml: No such file or directory (os error 2)",
        "zh",
      ),
    ).toBe(
      "/tmp/starter-project: 在 /tmp/starter-project/game.toml 发生 IO 错误：没有这个文件或目录（os error 2）",
    );
  });
});
