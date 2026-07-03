import { useCallback, useState } from "react";

// ---------------------------------------------------------------------------
// useStudioRail — shell-level UI state for the agent rail.
//
// Kept as a standalone hook (NOT wired through useStudioWorkspace) because the
// rail collapse toggle is pure shell-UI state: it does not depend on project
// data and should not couple the data-workspace interface to chrome concerns.
// Sinking it here keeps App.tsx thin (App.tsx only orchestrates), per the
// AGENTS.md "App.tsx only does routing/layout/form-sinking" boundary.
// ---------------------------------------------------------------------------

export interface StudioRailWorkspace {
  railCollapsed: boolean;
  toggleRail(): void;
}

export function useStudioRail(initialCollapsed = false): StudioRailWorkspace {
  const [railCollapsed, setRailCollapsed] = useState(initialCollapsed);
  const toggleRail = useCallback(() => {
    setRailCollapsed((prev) => !prev);
  }, []);
  return { railCollapsed, toggleRail };
}
