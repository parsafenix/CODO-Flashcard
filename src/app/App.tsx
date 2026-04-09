import { useEffect, useState } from "react";
import { Navigate, NavLink, Outlet, Route, Routes, useLocation, useNavigate } from "react-router-dom";
import { Button } from "../components/ui/Button";
import { ToastProvider } from "../components/ui/ToastProvider";
import darkLogo from "../assets/branding/icons/DARK-LOGO.png";
import lightLogo from "../assets/branding/icons/LIGHT-LOGO.png";
import { AnalyticsPage } from "../features/analytics/AnalyticsPage";
import { DeckDetailPage } from "../features/decks/DeckDetailPage";
import { DeckLibraryPage } from "../features/decks/DeckLibraryPage";
import { SettingsPage } from "../features/settings/SettingsPage";
import { StudySessionPage } from "../features/study/StudySessionPage";
import { api } from "../lib/api";
import type { AppSettings } from "../lib/types";
import { AppContext } from "./AppContext";
import { useAppContext } from "./AppContext";

function AppShell() {
  const location = useLocation();
  const navigate = useNavigate();
  const { settings, setSettings } = useAppContext();
  const [reminderVisible, setReminderVisible] = useState(false);

  async function syncReminderState() {
    if (!settings.reminder_enabled) {
      setReminderVisible(false);
      return;
    }

    try {
      const analytics = await api.getAnalytics({ period_days: 7 });
      setReminderVisible(analytics.reminder.should_show);
    } catch {
      setReminderVisible(false);
    }
  }

  useEffect(() => {
    void syncReminderState();
  }, [settings.reminder_enabled, settings.reminder_time, settings.reminder_last_acknowledged_date]);

  useEffect(() => {
    const listener = () => {
      if (document.visibilityState === "visible") {
        void syncReminderState();
      }
    };

    document.addEventListener("visibilitychange", listener);
    window.addEventListener("focus", listener);
    return () => {
      document.removeEventListener("visibilitychange", listener);
      window.removeEventListener("focus", listener);
    };
  }, [settings.reminder_enabled, settings.reminder_time, settings.reminder_last_acknowledged_date]);

  async function dismissReminderForToday() {
    try {
      const analytics = await api.getAnalytics({ period_days: 7 });
      const nextSettings = await api.updateSettings({
        ...settings,
        reminder_last_acknowledged_date: analytics.reminder.today_utc_date,
      });
      setSettings(nextSettings);
      setReminderVisible(false);
    } catch {
      setReminderVisible(false);
    }
  }

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div>
          <div className="brand">
            <div className="brand__logo-frame" aria-hidden="true">
              <img src={darkLogo} alt="" className="brand__logo brand__logo--dark" />
              <img src={lightLogo} alt="" className="brand__logo brand__logo--light" />
            </div>
            <div>
              <h1>CODO</h1>
              <p>CODO: Flashcard</p>
            </div>
          </div>

          <nav className="nav-list" aria-label="Primary">
            <NavLink to="/" className={({ isActive }) => (isActive ? "nav-link nav-link--active" : "nav-link")}>
              Library
            </NavLink>
            <NavLink
              to="/analytics"
              className={({ isActive }) => (isActive ? "nav-link nav-link--active" : "nav-link")}
            >
              Analytics
            </NavLink>
            <NavLink
              to="/settings"
              className={({ isActive }) => (isActive ? "nav-link nav-link--active" : "nav-link")}
            >
              Settings
            </NavLink>
          </nav>
        </div>

        <div className="sidebar__footer">
          <div className="surface-muted">
            <div className="surface-muted__label">Current view</div>
            <div className="surface-muted__value">
              {location.pathname.startsWith("/study")
                ? "Study session"
                : location.pathname.startsWith("/analytics")
                  ? "Analytics"
                : location.pathname.startsWith("/settings")
                  ? "Preferences"
                  : location.pathname.startsWith("/decks")
                    ? "Deck details"
                    : "Library"}
            </div>
          </div>
        </div>
      </aside>

      <main className="app-main">
        {reminderVisible ? (
          <div className="reminder-banner">
            <div>
              <strong>Reminder</strong>
              <p>Your reminder time has passed and due review cards are waiting.</p>
            </div>
            <div className="dialog-actions">
              <Button variant="secondary" onClick={() => navigate("/")}>
                Open library
              </Button>
              <Button variant="ghost" onClick={() => void dismissReminderForToday()}>
                Dismiss today
              </Button>
            </div>
          </div>
        ) : null}
        <Outlet />
      </main>
    </div>
  );
}

export function App() {
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function refreshSettings() {
    try {
      const nextSettings = await api.getSettings();
      setSettings(nextSettings);
      setError(null);
    } catch (err) {
      const message = err instanceof Error ? err.message : "Unable to load settings.";
      setError(message);
    }
  }

  useEffect(() => {
    void refreshSettings();
  }, []);

  useEffect(() => {
    if (!settings) {
      return;
    }

    document.documentElement.dataset.theme = settings.theme;
  }, [settings]);

  if (error) {
    return (
      <div className="boot-screen">
        <h1>CODO: Flashcard</h1>
        <p>{error}</p>
        <Button onClick={() => void refreshSettings()}>Retry</Button>
      </div>
    );
  }

  if (!settings) {
    return (
      <div className="boot-screen">
        <h1>CODO: Flashcard</h1>
        <p>Loading your local workspace...</p>
      </div>
    );
  }

  return (
    <AppContext.Provider value={{ settings, setSettings, refreshSettings }}>
      <ToastProvider>
        <Routes>
          <Route element={<AppShell />}>
            <Route index element={<DeckLibraryPage />} />
            <Route path="analytics" element={<AnalyticsPage />} />
            <Route path="decks/:deckId" element={<DeckDetailPage />} />
            <Route path="study/:deckId" element={<StudySessionPage />} />
            <Route path="settings" element={<SettingsPage />} />
            <Route path="*" element={<Navigate to="/" replace />} />
          </Route>
        </Routes>
      </ToastProvider>
    </AppContext.Provider>
  );
}
