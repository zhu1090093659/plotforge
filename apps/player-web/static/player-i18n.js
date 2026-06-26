/**
 * @fileoverview Player i18n module — owns UI chrome localization for the static
 * player. Mirrors the AGENTS.md rule that static player text is localized here;
 * it never translates authored manifest content (titles, story text, beat text).
 */

import { localStorageFor } from "./player-save.js";
import { renderPlayer } from "./player-core.js";

const playerLocaleStorageKey = "plotforge:player:locale";
const playerLocales = new Set(["en", "zh"]);

/**
 * Localized player chrome strings. Message values may be strings or template
 * functions invoked with the values passed to `t`.
 *
 * @typedef {Record<string, string | ((values: Record<string, string | number>) => string)>} PlayerMessageTable
 */

/** @type {Record<"en" | "zh", PlayerMessageTable>} */
const playerMessages = {
  en: {
    exportFailed: "Export failed to load",
    unableToLoad: "Unable to load PlotForge export",
    staticExport: "Static export",
    selectedAction: ({ actionType }) => `Selected ${actionType}`,
    storyComplete: "Story complete",
    storyEnded: "This static story path has ended.",
    sceneProgress: ({ current, total }) => `Scene ${current} of ${total}`,
    beatProgress: ({ current, total }) => `Beat ${current} of ${total}`,
    scene: "Scene",
    beat: "Beat",
    sceneArtwork: ({ title }) => `${title} scene artwork`,
    sceneAudio: "Scene audio",
  },
  zh: {
    exportFailed: "导出加载失败",
    unableToLoad: "无法加载 PlotForge 导出",
    staticExport: "静态导出",
    selectedAction: ({ actionType }) => `已选择 ${actionType}`,
    storyComplete: "故事完成",
    storyEnded: "这条静态故事路径已结束。",
    sceneProgress: ({ current, total }) => `场景 ${current} / ${total}`,
    beatProgress: ({ current, total }) => `节拍 ${current} / ${total}`,
    scene: "场景",
    beat: "节拍",
    sceneArtwork: ({ title }) => `${title} 场景图`,
    sceneAudio: "场景音频",
  },
};

/**
 * Resolved i18n facade bound to a single locale.
 *
 * @typedef {Object} PlayerI18n
 * @property {"en" | "zh"} locale
 * @property {(key: string, values?: Record<string, string | number>) => string} t
 */

/**
 * Build a locale-bound i18n facade. The locale is resolved once (request >
 * URL > localStorage > html lang > navigator) and reused for every `t` call.
 *
 * @param {Document | ShadowRoot} root
 * @param {string | undefined} requestedLocale
 * @returns {PlayerI18n}
 */
export function createPlayerI18n(root, requestedLocale) {
  const locale = resolvePlayerLocale(root, requestedLocale);
  return {
    locale,
    t(key, values = {}) {
      const message = playerMessages[locale][key] ?? playerMessages.en[key];
      return typeof message === "function" ? message(values) : message;
    },
  };
}

/**
 * @param {Document | ShadowRoot} root
 * @param {string | undefined} requestedLocale
 * @returns {"en" | "zh"}
 */
function resolvePlayerLocale(root, requestedLocale) {
  if (isPlayerLocale(requestedLocale)) {
    return requestedLocale;
  }
  const view = root.defaultView ?? globalThis.window;
  const urlLocale = localeFromSearch(view?.location?.search);
  if (urlLocale) {
    return urlLocale;
  }
  const storedLocale = localStorageFor(view)?.getItem(playerLocaleStorageKey);
  if (isPlayerLocale(storedLocale)) {
    return storedLocale;
  }
  const htmlLocale = root.documentElement?.getAttribute("lang");
  if (isPlayerLocale(htmlLocale)) {
    return htmlLocale;
  }
  return view?.navigator?.language?.toLowerCase().startsWith("zh") ? "zh" : "en";
}

/**
 * @param {string | undefined} search
 * @returns {"en" | "zh" | null}
 */
function localeFromSearch(search) {
  if (!search) {
    return null;
  }
  const params = new URLSearchParams(search);
  const value = params.get("lang") ?? params.get("language");
  return isPlayerLocale(value) ? value : null;
}

/**
 * @param {unknown} value
 * @returns {value is "en" | "zh"}
 */
function isPlayerLocale(value) {
  return typeof value === "string" && playerLocales.has(value);
}

/**
 * Reflect the active locale on the mount dataset and the document <html lang>.
 *
 * @param {Document | ShadowRoot} root
 * @param {HTMLElement} mount
 * @param {"en" | "zh"} locale
 */
export function applyLocale(root, mount, locale) {
  mount.dataset.locale = locale;
  root.documentElement?.setAttribute("lang", locale === "zh" ? "zh-CN" : "en");
}

/**
 * Wire the language <select> so it persists the choice and re-renders the player
 * in the new locale. Listens once to avoid stacking handlers across re-renders.
 *
 * @param {import("./player-types.js").ExportManifestLike} manifest
 * @param {Document | ShadowRoot} root
 * @param {HTMLElement} mount
 * @param {"en" | "zh"} locale
 */
export function wireLanguageControl(manifest, root, mount, locale) {
  const select = root.querySelector('[data-field="language-select"]');
  if (!select) {
    return;
  }
  select.value = locale;
  select.addEventListener(
    "change",
    () => {
      const nextLocale = select.value;
      if (!isPlayerLocale(nextLocale)) {
        return;
      }
      const storage = localStorageFor(root.defaultView ?? globalThis.window);
      storage?.setItem(playerLocaleStorageKey, nextLocale);
      renderPlayer(manifest, root, { locale: nextLocale });
    },
    { once: true },
  );
  mount.dataset.languageControl = "ready";
}
