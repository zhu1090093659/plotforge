import { useCallback, useEffect, useRef, useState } from "react";
import type {
  AgentSessionConfig,
  ModelOption,
} from "../../../contracts/plotforge";
import type { StudioDataSource } from "./studioDataSource";
import { errorMessage } from "./errorMessage";

// ---------------------------------------------------------------------------
// useAgentConfig — persisted agent session configuration for the home page.
//
// Loads the per-project `AgentSessionConfig` (model_id, permission_level,
// thinking_level) from the Rust backend on project load and persists every
// selector change back to `.plotforge/agent-config.json`. The config is a
// redaction-safe capability surface — never carries credentials — so the
// frontend can freely read/write it.
//
// Real provider routing enforcement lives in `plotforge-studio` /
// `plotforge-agent`; this hook only manages the UI state + persistence.
// ---------------------------------------------------------------------------

const defaultAgentConfig: AgentSessionConfig = {
  model_id: "local-pi",
  permission_level: "ask_every_time",
  thinking_level: "medium",
};

export interface AgentConfigWorkspace {
  /** Available model options for the selector. */
  availableModels: ModelOption[];
  /** Persisted agent session config. */
  agentConfig: AgentSessionConfig;
  /** True while the initial config is loading. */
  loading: boolean;
  /** Persist a new config (debounced internally to avoid write churn). */
  setAgentConfig(config: AgentSessionConfig): void;
  /** Last persist error (cleared on next successful save). */
  saveError: string | null;
}

export function useAgentConfig(
  dataSource: StudioDataSource,
  loadedPath: string,
): AgentConfigWorkspace {
  const [availableModels, setAvailableModels] = useState<ModelOption[]>([]);
  const [agentConfig, setAgentConfigState] = useState<AgentSessionConfig>(
    defaultAgentConfig,
  );
  const [loading, setLoading] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);

  // Load available models once (static list).
  useEffect(() => {
    let cancelled = false;
    dataSource
      .listAvailableModels()
      .then((models) => {
        if (!cancelled) setAvailableModels(models);
      })
      .catch(() => {
        // Models list is a static fallback; silently keep empty on error.
      });
    return () => {
      cancelled = true;
    };
  }, [dataSource]);

  // Load persisted config when the loaded path changes.
  useEffect(() => {
    if (!loadedPath) {
      setAgentConfigState(defaultAgentConfig);
      return;
    }
    let cancelled = false;
    setLoading(true);
    dataSource
      .getAgentSessionConfig(loadedPath)
      .then((config) => {
        if (!cancelled) setAgentConfigState(config);
      })
      .catch(() => {
        // If the read fails, fall back to defaults — never block the home page.
        if (!cancelled) setAgentConfigState(defaultAgentConfig);
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [dataSource, loadedPath]);

  // Debounced persistence: avoid a write on every keystroke when the config
  // changes. We persist 300ms after the last `setAgentConfig` call.
  const persistTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const persistPath = useRef<string>(loadedPath);

  useEffect(() => {
    persistPath.current = loadedPath;
  }, [loadedPath]);

  const setAgentConfig = useCallback(
    (config: AgentSessionConfig) => {
      setAgentConfigState(config);
      const path = persistPath.current;
      if (!path) return;
      if (persistTimer.current) clearTimeout(persistTimer.current);
      persistTimer.current = setTimeout(() => {
        dataSource
          .setAgentSessionConfig(path, config)
          .then(() => setSaveError(null))
          .catch((source) => setSaveError(errorMessage(source)));
      }, 300);
    },
    [dataSource],
  );

  return {
    availableModels,
    agentConfig,
    loading,
    setAgentConfig,
    saveError,
  };
}
