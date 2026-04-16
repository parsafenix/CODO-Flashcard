import { useMemo } from "react";
import { useAppContext } from "../app/AppContext";
import { getHiddenPanels, hidePanel as buildHidePanelPreferences, showPanel as buildShowPanelPreferences } from "./uiPreferences";

export function usePanelVisibility(pageKey: string, panels: Array<{ id: string; label: string }>) {
  const { uiPreferences, persistUiPreferences } = useAppContext();
  const hiddenIds = useMemo(() => new Set(getHiddenPanels(uiPreferences, pageKey)), [pageKey, uiPreferences]);

  return {
    visiblePanels: panels.filter((panel) => !hiddenIds.has(panel.id)),
    hiddenPanels: panels.filter((panel) => hiddenIds.has(panel.id)),
    hidePanel: async (panelId: string) =>
      persistUiPreferences(buildHidePanelPreferences(uiPreferences, pageKey, panelId)),
    showPanel: async (panelId: string) =>
      persistUiPreferences(buildShowPanelPreferences(uiPreferences, pageKey, panelId)),
  };
}
