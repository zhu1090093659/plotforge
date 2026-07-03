import { useEffect, useMemo, useRef, useState, type KeyboardEvent } from "react";
import { CornerDownLeft, Search } from "lucide-react";
import { useStudioI18n } from "./i18n";

// ---------------------------------------------------------------------------
// StudioCommandPalette — the global ⌘K action surface.
//
// Single input that doubles as a command filter AND a project-path entry:
// if the typed query does not match any navigation/action (or looks
// path-like via separators/extensions), an "Open project '<query>'" action
// is offered so the creator can load any path without a dedicated header
// text field (Phase D: the header path input is removed in favour of this
// surface). When a query matches a nav action and is not path-like, the
// open-project action is appended LAST so the matched nav action stays
// first (the default active option) and Enter runs it.
//
// Accessibility: role="dialog" + aria-modal, autofocus input, WAI-ARIA
// listbox/option semantics, full keyboard navigation (ArrowUp/Down/Enter/Esc,
// Home/End), and Escape to close. The backdrop closes on click.
// ---------------------------------------------------------------------------

export interface PaletteAction {
  id: string;
  label: string;
  hint?: string;
  run(): void;
}

export interface StudioCommandPaletteProps {
  open: boolean;
  onClose(): void;
  actions: PaletteAction[];
  onOpenProject(path: string): void;
}

export function StudioCommandPalette({
  open,
  onClose,
  actions,
  onOpenProject,
}: StudioCommandPaletteProps) {
  const { t } = useStudioI18n();
  const [query, setQuery] = useState("");
  const [activeIndex, setActiveIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLUListElement>(null);

  // Reset query + selection whenever the palette opens.
  useEffect(() => {
    if (open) {
      setQuery("");
      setActiveIndex(0);
    }
  }, [open]);

  // Autofocus the input when opened.
  useEffect(() => {
    if (open) {
      inputRef.current?.focus();
    }
  }, [open]);

  const trimmed = query.trim();
  const lowerQuery = query.toLowerCase();

  const filtered = useMemo(
    () =>
      actions.filter((action) =>
        action.label.toLowerCase().includes(lowerQuery),
      ),
    [actions, lowerQuery],
  );

  // The "open project" action replaces the removed header path input. To avoid
  // hijacking a query that matches a real navigation action, only inject it
  // when the query looks path-like (contains a path separator or extension) OR
  // no filtered action matches. When injected alongside matches, it goes LAST
  // so the first matched nav action stays at index 0 (the default active one).
  const looksPathLike = /[\\/]/.test(trimmed) || /\.[a-z0-9]+$/i.test(trimmed);
  const openProjectAction: PaletteAction | null = trimmed
    ? {
        id: "__open-project",
        label: t("palette.openProject", { path: trimmed }),
        hint: t("palette.openProjectHint"),
        run: () => onOpenProject(trimmed),
      }
    : null;

  const list: PaletteAction[] = (() => {
    if (!openProjectAction) return filtered;
    if (looksPathLike || filtered.length === 0) {
      return [openProjectAction, ...filtered];
    }
    return [...filtered, openProjectAction];
  })();

  // Clamp activeIndex when the list shrinks.
  const safeIndex = Math.min(activeIndex, Math.max(0, list.length - 1));

  function runAction(action: PaletteAction) {
    action.run();
    onClose();
  }

  function handleKeyDown(event: KeyboardEvent<HTMLInputElement>) {
    switch (event.key) {
      case "ArrowDown":
        event.preventDefault();
        setActiveIndex((prev) => Math.min(prev + 1, list.length - 1));
        break;
      case "ArrowUp":
        event.preventDefault();
        setActiveIndex((prev) => Math.max(prev - 1, 0));
        break;
      case "Home":
        event.preventDefault();
        setActiveIndex(0);
        break;
      case "End":
        event.preventDefault();
        setActiveIndex(Math.max(0, list.length - 1));
        break;
      case "Enter": {
        event.preventDefault();
        const action = list[safeIndex];
        if (action) {
          runAction(action);
        }
        break;
      }
      case "Escape":
        event.preventDefault();
        onClose();
        break;
    }
  }

  // Keep the active option scrolled into view during keyboard navigation.
  useEffect(() => {
    const listEl = listRef.current;
    if (!listEl) return;
    const active = listEl.children[safeIndex] as HTMLElement | undefined;
    if (typeof active?.scrollIntoView === "function") {
      active.scrollIntoView({ block: "nearest" });
    }
  }, [safeIndex]);

  if (!open) return null;

  const listId = "studio-command-palette-list";

  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center bg-ink/30 px-4 pt-24"
      onClick={onClose}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-label={t("palette.title")}
        onClick={(event) => event.stopPropagation()}
        className="w-full max-w-xl overflow-hidden rounded-lg border border-canvas-200 bg-canvas-50 shadow-studio-panel"
      >
        <div className="flex items-center gap-2 border-b border-canvas-200 px-4 py-3">
          <Search aria-hidden size={18} className="text-ink/45" />
          <input
            ref={inputRef}
            type="text"
            role="combobox"
            aria-expanded="true"
            aria-controls={listId}
            aria-autocomplete="list"
            aria-label={t("palette.placeholder")}
            aria-activedescendant={
              list[safeIndex] ? `palette-option-${safeIndex}` : undefined
            }
            value={query}
            onChange={(event) => {
              setQuery(event.target.value);
              setActiveIndex(0);
            }}
            onKeyDown={handleKeyDown}
            placeholder={t("palette.placeholder")}
            className="min-w-0 flex-1 bg-transparent text-sm text-ink outline-none placeholder:text-ink/40"
          />
        </div>

        {list.length > 0 ? (
          <ul
            id={listId}
            ref={listRef}
            role="listbox"
            aria-label={t("palette.title")}
            className="max-h-80 overflow-auto py-1"
          >
            {list.map((action, index) => {
              const isActive = index === safeIndex;
              return (
                <li
                  key={action.id}
                  id={`palette-option-${index}`}
                  role="option"
                  aria-selected={isActive}
                >
                  <button
                    type="button"
                    onClick={() => runAction(action)}
                    onMouseEnter={() => setActiveIndex(index)}
                    className={[
                      "flex w-full items-center justify-between gap-3 px-4 py-2 text-left text-sm transition",
                      isActive
                        ? "bg-violet-500/15 text-ink"
                        : "text-ink/80 hover:bg-canvas-100",
                    ].join(" ")}
                  >
                    <span className="min-w-0 truncate">{action.label}</span>
                    {action.hint ? (
                      <span className="flex shrink-0 items-center gap-1 text-xs text-ink/45">
                        {isActive ? (
                          <CornerDownLeft aria-hidden size={12} />
                        ) : null}
                        {action.hint}
                      </span>
                    ) : null}
                  </button>
                </li>
              );
            })}
          </ul>
        ) : (
          <p className="px-4 py-6 text-center text-sm text-ink/45">
            {t("palette.empty")}
          </p>
        )}
      </div>
    </div>
  );
}
