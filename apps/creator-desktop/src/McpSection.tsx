import { EmptyState, StudioPanel } from "./studioUi";
import { useStudioI18n } from "./i18n";

// ---------------------------------------------------------------------------
// McpSection — the "MCP" tab inside SettingsView.
//
// Phase A placeholder only. A full MCP client (stdio + SSE transport, server
// lifecycle, tool discovery/invoke) is Phase B work and requires:
//   1. An explicit AGENTS.md boundary carve-out for MCP server calls
//      (currently the "no networked model calls" rule blocks this).
//   2. New schema types + a plotforge-mcp crate (or plotforge-agent module).
//   3. New Studio commands + contracts + DataSource methods.
// Until those land, this tab shows an explicit "coming in a future release"
// placeholder so the Settings UI is honest about what is and isn't wired.
// ---------------------------------------------------------------------------

export function McpSection() {
  const { t } = useStudioI18n();
  return (
    <StudioPanel>
      <h4 className="mb-3 font-display text-lg font-semibold tracking-display-tight text-ink">
        {t("settings.tab.mcp")}
      </h4>
      <EmptyState>{t("settings.mcp.placeholder")}</EmptyState>
    </StudioPanel>
  );
}
