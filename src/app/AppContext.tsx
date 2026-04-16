import { createContext, useContext } from "react";
import type { AppSettings, UiPreferences } from "../lib/types";

export interface AppContextValue {
  settings: AppSettings;
  setSettings: (settings: AppSettings) => void;
  refreshSettings: () => Promise<void>;
  uiPreferences: UiPreferences;
  setUiPreferences: (preferences: UiPreferences) => void;
  refreshUiPreferences: () => Promise<void>;
  persistUiPreferences: (preferences: UiPreferences) => Promise<UiPreferences>;
}

export const AppContext = createContext<AppContextValue | null>(null);

export function useAppContext(): AppContextValue {
  const value = useContext(AppContext);
  if (!value) {
    throw new Error("App context is not available.");
  }
  return value;
}
