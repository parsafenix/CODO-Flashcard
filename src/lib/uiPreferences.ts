import type { UiPreferences } from "./types";

export function getHiddenPanels(preferences: UiPreferences, pageKey: string): string[] {
  return preferences.hidden_panels[pageKey] ?? [];
}

export function updateHiddenPanels(
  preferences: UiPreferences,
  pageKey: string,
  panelIds: string[]
): UiPreferences {
  const unique = [...new Set(panelIds.map((value) => value.trim()).filter(Boolean))].sort();
  return {
    ...preferences,
    hidden_panels: {
      ...preferences.hidden_panels,
      [pageKey]: unique,
    },
  };
}

export function hidePanel(preferences: UiPreferences, pageKey: string, panelId: string): UiPreferences {
  return updateHiddenPanels(preferences, pageKey, [...getHiddenPanels(preferences, pageKey), panelId]);
}

export function showPanel(preferences: UiPreferences, pageKey: string, panelId: string): UiPreferences {
  return updateHiddenPanels(
    preferences,
    pageKey,
    getHiddenPanels(preferences, pageKey).filter((value) => value !== panelId)
  );
}
