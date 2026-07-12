import { useCallback, useState } from "react";

// ---------------------------------------------------------------------------
// useStudioRail — shell-level UI state for the agent rail.
//
// Kept as a standalone hook (NOT wired through useStudioWorkspace) because the
// rail collapse toggle is pure shell-UI state: it does not depend on project
// data and should not couple the data-workspace interface to chrome concerns.
// Sinking it here keeps App.tsx thin (App.tsx only orchestrates), per the
// AGENTS.md "App.tsx only does routing/layout/form-sinking" boundary.
//
// Both chrome rails start collapsed on every app launch so the creator enters
// a focused canvas. Toggle state is intentionally session-only: reopening the
// desktop app always restores the same compact starting layout.
// ---------------------------------------------------------------------------

export interface StudioRailWorkspace {
  railCollapsed: boolean;
  toggleRail(): void;
  sidebarCollapsed: boolean;
  toggleSidebar(): void;
}

export function useStudioRail(
  initialCollapsed = true,
): StudioRailWorkspace {
  const [railCollapsed, setRailCollapsed] = useState(initialCollapsed);
  const [sidebarCollapsed, setSidebarCollapsed] = useState(true);

  const toggleRail = useCallback(() => {
    setRailCollapsed((prev) => !prev);
  }, []);

  const toggleSidebar = useCallback(() => {
    setSidebarCollapsed((prev) => !prev);
  }, []);

  return { railCollapsed, toggleRail, sidebarCollapsed, toggleSidebar };
}
