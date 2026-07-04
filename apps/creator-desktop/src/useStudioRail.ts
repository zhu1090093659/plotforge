import { useCallback, useEffect, useRef, useState } from "react";

// ---------------------------------------------------------------------------
// useStudioRail — shell-level UI state for the agent rail.
//
// Kept as a standalone hook (NOT wired through useStudioWorkspace) because the
// rail collapse toggle is pure shell-UI state: it does not depend on project
// data and should not couple the data-workspace interface to chrome concerns.
// Sinking it here keeps App.tsx thin (App.tsx only orchestrates), per the
// AGENTS.md "App.tsx only does routing/layout/form-sinking" boundary.
//
// Responsive default: on a wide desktop (xl / >=1280px) the rail starts
// expanded so the director entry point is visible; on a narrower viewport
// (lg, 1024–1280px) it starts collapsed so the main content area is not
// squeezed into a scroll-prone narrow column. The user's explicit toggle is
// then persisted to localStorage and wins on subsequent loads. In jsdom (where
// matchMedia is unavailable) the rail defaults to expanded so existing
// behavior-based tests keep seeing the Agent rail.
// ---------------------------------------------------------------------------

const railStorageKey = "plotforge:creator-desktop:rail-collapsed";

export interface StudioRailWorkspace {
  railCollapsed: boolean;
  toggleRail(): void;
}

export function useStudioRail(
  initialCollapsed: boolean | undefined = undefined,
): StudioRailWorkspace {
  const [railCollapsed, setRailCollapsed] = useState<boolean>(() =>
    resolveInitialCollapsed(initialCollapsed),
  );

  // Only persist an explicit user toggle — skip the very first effect run so
  // the auto-resolved default (responsive / jsdom) is not written back as if
  // it were a deliberate choice. A subsequent load then re-derives the
  // default from viewport, instead of being locked to the first load's width.
  const initialized = useRef(false);
  useEffect(() => {
    if (!initialized.current) {
      initialized.current = true;
      return;
    }
    const storage = storageFor(globalThis.window);
    if (storage) {
      storage.setItem(railStorageKey, String(railCollapsed));
    }
  }, [railCollapsed]);

  const toggleRail = useCallback(() => {
    setRailCollapsed((prev) => !prev);
  }, []);

  return { railCollapsed, toggleRail };
}

function resolveInitialCollapsed(
  initialCollapsed: boolean | undefined,
): boolean {
  // An explicit caller argument wins (used by tests and deterministic
  // call sites). When omitted (the App shell path), consult localStorage so
  // the user's last explicit toggle persists, then fall back to a responsive
  // default based on viewport width.
  if (initialCollapsed !== undefined) return initialCollapsed;
  const storage = storageFor(globalThis.window);
  const stored = storage?.getItem(railStorageKey);
  if (stored === "true") return true;
  if (stored === "false") return false;
  // No stored preference: pick a sensible responsive default. Only expand by
  // default on genuinely wide desktops; collapse on lg (1024–1280px) so the
  // 3-column layout does not squeeze main content into a scroll-prone column.
  const view = globalThis.window;
  if (view && typeof view.matchMedia === "function") {
    return !view.matchMedia("(min-width: 1280px)").matches;
  }
  // jsdom / non-browser: default expanded to preserve existing test behavior.
  return false;
}

function storageFor(view: Window | undefined): Storage | null {
  const storage = view?.localStorage;
  if (
    storage &&
    typeof storage.getItem === "function" &&
    typeof storage.setItem === "function"
  ) {
    return storage;
  }
  return null;
}
