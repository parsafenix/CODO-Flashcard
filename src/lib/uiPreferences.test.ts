import { describe, expect, it } from "vitest";
import type { UiPreferences } from "./types";
import { getHiddenPanels, hidePanel, showPanel, updateHiddenPanels } from "./uiPreferences";

function buildPreferences(overrides?: Partial<UiPreferences>): UiPreferences {
  return {
    daily_coach_last_dismissed_utc_date: null,
    daily_coach_last_shown_utc_date: null,
    hidden_panels: {},
    ...overrides,
  };
}

describe("uiPreferences helpers", () => {
  it("returns hidden panels for a page and falls back to empty", () => {
    const preferences = buildPreferences({ hidden_panels: { analytics: ["streak"] } });
    expect(getHiddenPanels(preferences, "analytics")).toEqual(["streak"]);
    expect(getHiddenPanels(preferences, "settings")).toEqual([]);
  });

  it("normalizes hidden panel lists when updating", () => {
    const preferences = buildPreferences();
    expect(updateHiddenPanels(preferences, "analytics", [" weak-cards ", "streak", "streak", ""])).toEqual({
      ...preferences,
      hidden_panels: { analytics: ["streak", "weak-cards"] },
    });
  });

  it("hides and shows panels idempotently", () => {
    const preferences = buildPreferences({ hidden_panels: { analytics: ["streak"] } });
    const hidden = hidePanel(preferences, "analytics", "calibration");
    expect(hidden.hidden_panels.analytics).toEqual(["calibration", "streak"]);

    const shown = showPanel(hidden, "analytics", "streak");
    expect(shown.hidden_panels.analytics).toEqual(["calibration"]);
  });
});
