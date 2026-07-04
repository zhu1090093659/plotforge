import { useCallback, useEffect, useState } from "react";
import type { GitBranchInfo } from "../../../contracts/plotforge";
import type { StudioDataSource } from "./studioDataSource";
import { errorMessage } from "./errorMessage";

// ---------------------------------------------------------------------------
// useGitInfo — local git workspace surface for the home page.
//
// Reads the current branch + branch list for the loaded project path, and
// switches branches via `git checkout`. A path that is not a git repo is
// not an error: `currentBranch` becomes `null` and `branches` becomes `[]`,
// so the home page can render a "(no git)" chip without a jarring error
// banner. Switch failures ARE surfaced (never silent) via `switchError`.
//
// Real git operations are shelled out by the Rust backend
// (`plotforge-studio::git_*`); this hook only manages fetch/switch state.
// ---------------------------------------------------------------------------

export interface GitInfoWorkspace {
  /** Current branch name, or null when the path is not a git repo / not loaded. */
  currentBranch: string | null;
  /** Local branch list (empty when not a repo). */
  branches: GitBranchInfo[];
  /** True while a branch switch is in flight (a `git checkout` is running). */
  switchingBranch: boolean;
  /** True while the branch list is being fetched (initial load / refresh). */
  fetching: boolean;
  /** Last switch error message (cleared on next successful switch). */
  switchError: string | null;
  /** Switch to a local branch. Returns the new branch name on success. */
  switchBranch(branch: string): Promise<string | null>;
  /** Re-fetch the branch list (e.g. after an external git operation). */
  refreshBranches(): Promise<void>;
}

export function useGitInfo(
  dataSource: StudioDataSource,
  loadedPath: string,
  onBranchSwitched?: () => void,
): GitInfoWorkspace {
  const [currentBranch, setCurrentBranch] = useState<string | null>(null);
  const [branches, setBranches] = useState<GitBranchInfo[]>([]);
  const [switchingBranch, setSwitchingBranch] = useState(false);
  const [fetching, setFetching] = useState(false);
  const [switchError, setSwitchError] = useState<string | null>(null);

  const refreshBranches = useCallback(async () => {
    if (!loadedPath) {
      setCurrentBranch(null);
      setBranches([]);
      return;
    }
    // Fetch loading is distinct from switch loading: a plain read should not
    // disable the branch chip with a "switching" spinner.
    setFetching(true);
    try {
      const [branch, list] = await Promise.all([
        dataSource.gitCurrentBranch(loadedPath).catch(() => null),
        dataSource.gitListBranches(loadedPath).catch(() => [] as GitBranchInfo[]),
      ]);
      setCurrentBranch(branch);
      setBranches(list);
    } finally {
      setFetching(false);
    }
  }, [dataSource, loadedPath]);

  useEffect(() => {
    void refreshBranches();
  }, [refreshBranches]);

  const switchBranch = useCallback(
    async (branch: string): Promise<string | null> => {
      if (!loadedPath) return null;
      setSwitchingBranch(true);
      setSwitchError(null);
      try {
        const result = await dataSource.gitSwitchBranch(loadedPath, branch);
        setCurrentBranch(result.branch);
        // Refresh the branch list so `is_current` markers stay accurate.
        const list = await dataSource
          .gitListBranches(loadedPath)
          .catch(() => [] as GitBranchInfo[]);
        setBranches(list);
        onBranchSwitched?.();
        return result.branch;
      } catch (source) {
        const message = errorMessage(source);
        setSwitchError(message);
        return null;
      } finally {
        setSwitchingBranch(false);
      }
    },
    [dataSource, loadedPath, onBranchSwitched],
  );

  return {
    currentBranch,
    branches,
    switchingBranch,
    fetching,
    switchError,
    switchBranch,
    refreshBranches,
  };
}
