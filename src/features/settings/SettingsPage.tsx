import { open } from "@tauri-apps/plugin-dialog";
import { useEffect, useState } from "react";
import { useAppContext } from "../../app/AppContext";
import { Button } from "../../components/ui/Button";
import { ConfirmDialog } from "../../components/ui/ConfirmDialog";
import { useToast } from "../../components/ui/ToastProvider";
import { api } from "../../lib/api";
import type { AppSettings, PromptLanguage, StudyMode, Theme } from "../../lib/types";

export function SettingsPage() {
  const { settings, setSettings, refreshSettings } = useAppContext();
  const { notify } = useToast();
  const [draft, setDraft] = useState<AppSettings>(settings);
  const [saving, setSaving] = useState(false);
  const [resetOpen, setResetOpen] = useState(false);

  useEffect(() => {
    setDraft(settings);
  }, [settings]);

  async function saveSettings() {
    setSaving(true);
    try {
      const nextSettings = await api.updateSettings(draft);
      setSettings(nextSettings);
      setDraft(nextSettings);
      notify("Settings saved.", "success");
    } catch (err) {
      const message = typeof err === "object" && err && "message" in err ? String(err.message) : "Unable to save settings.";
      notify(message, "error");
    } finally {
      setSaving(false);
    }
  }

  async function handleBackup() {
    const selected = await open({
      directory: true,
      multiple: false,
    });

    if (typeof selected !== "string") {
      return;
    }

    try {
      const result = await api.createBackup(selected);
      notify(`Backup created at ${result.output_path}`, "success");
      await refreshSettings();
    } catch (err) {
      const message = typeof err === "object" && err && "message" in err ? String(err.message) : "Unable to create backup.";
      notify(message, "error");
    }
  }

  async function handleOpenDataFolder() {
    try {
      await api.openDataFolder();
      notify("Opened the local data folder.", "info");
    } catch (err) {
      const message = typeof err === "object" && err && "message" in err ? String(err.message) : "Unable to open the data folder.";
      notify(message, "error");
    }
  }

  async function handleReset() {
    try {
      await api.resetAppData();
      await refreshSettings();
      setResetOpen(false);
      notify("App data reset. The database was recreated locally.", "success");
    } catch (err) {
      const message = typeof err === "object" && err && "message" in err ? String(err.message) : "Unable to reset app data.";
      notify(message, "error");
    }
  }

  return (
    <>
      <section className="page-header">
        <div>
          <p className="eyebrow">Settings</p>
          <h1>Preferences</h1>
          <p>Local defaults for theme, study behavior, imports, backup actions, reminders, and project information.</p>
        </div>
      </section>

      <section className="settings-grid">
        <div className="surface-panel">
          <h2>Appearance</h2>
          <div className="form-stack">
            <label className="field">
              <span>Theme</span>
              <select
                value={draft.theme}
                onChange={(event) => setDraft((current) => ({ ...current, theme: event.target.value as Theme }))}
              >
                <option value="dark">Dark</option>
                <option value="light">Light</option>
              </select>
            </label>
          </div>
        </div>

        <div className="surface-panel">
          <h2>Study defaults</h2>
          <div className="form-stack">
            <div className="field-grid field-grid--dual">
              <label className="field">
                <span>Default prompt language</span>
                <select
                  value={draft.default_prompt_language}
                  onChange={(event) =>
                    setDraft((current) => ({
                      ...current,
                      default_prompt_language: event.target.value as PromptLanguage,
                    }))
                  }
                >
                  <option value="language_1">Language 1</option>
                  <option value="language_2">Language 2</option>
                  <option value="language_3">Language 3</option>
                </select>
              </label>

              <label className="field">
                <span>Default study mode</span>
                <select
                  value={draft.default_study_mode}
                  onChange={(event) =>
                    setDraft((current) => ({
                      ...current,
                      default_study_mode: event.target.value as StudyMode,
                    }))
                  }
                >
                  <option value="mixed">Mixed</option>
                  <option value="due">Due only</option>
                  <option value="new">New only</option>
                </select>
              </label>
            </div>

            <label className="field">
              <span>Cards per session</span>
              <input
                type="number"
                min={1}
                max={200}
                value={draft.cards_per_session}
                onChange={(event) =>
                  setDraft((current) => ({
                    ...current,
                    cards_per_session: Number(event.target.value),
                  }))
                }
              />
            </label>

            <label className="field">
              <span>Daily review goal</span>
              <input
                type="number"
                min={1}
                max={500}
                value={draft.daily_review_goal}
                onChange={(event) =>
                  setDraft((current) => ({
                    ...current,
                    daily_review_goal: Number(event.target.value),
                  }))
                }
              />
            </label>

            <label className="field field--checkbox">
              <input
                type="checkbox"
                checked={draft.random_order}
                onChange={(event) => setDraft((current) => ({ ...current, random_order: event.target.checked }))}
              />
              <span>Randomize card order by default</span>
            </label>

            <label className="field field--checkbox">
              <input
                type="checkbox"
                checked={draft.reverse_mode}
                onChange={(event) => setDraft((current) => ({ ...current, reverse_mode: event.target.checked }))}
              />
              <span>Enable reverse mode by default</span>
            </label>

            <label className="field field--checkbox">
              <input
                type="checkbox"
                checked={draft.reveal_all_on_flip}
                onChange={(event) => setDraft((current) => ({ ...current, reveal_all_on_flip: event.target.checked }))}
              />
              <span>Reveal all answer fields on flip</span>
            </label>
          </div>
        </div>

        <div className="surface-panel">
          <h2>Imports and backups</h2>
          <div className="form-stack">
            <label className="field">
              <span>Import delimiter</span>
              <input
                value={draft.import_delimiter}
                maxLength={3}
                onChange={(event) => setDraft((current) => ({ ...current, import_delimiter: event.target.value }))}
              />
            </label>

            <div className="surface-muted">
              <div className="surface-muted__label">Last backup folder</div>
              <p>{draft.last_backup_directory || "No backup created yet."}</p>
            </div>

            <div className="dialog-actions dialog-actions--start">
              <Button variant="secondary" onClick={() => void handleBackup()}>
                Create backup
              </Button>
              <Button variant="ghost" onClick={() => void handleOpenDataFolder()}>
                Open data folder
              </Button>
            </div>
          </div>
        </div>

        <div className="surface-panel">
          <h2>Reminder</h2>
          <div className="form-stack">
            <label className="field field--checkbox">
              <input
                type="checkbox"
                checked={draft.reminder_enabled}
                onChange={(event) => setDraft((current) => ({ ...current, reminder_enabled: event.target.checked }))}
              />
              <span>Enable local study reminder</span>
            </label>

            <label className="field">
              <span>Reminder time</span>
              <input
                type="time"
                value={draft.reminder_time}
                onChange={(event) => setDraft((current) => ({ ...current, reminder_time: event.target.value }))}
              />
            </label>

            <div className="surface-muted">
              <div className="surface-muted__label">Reminder behavior</div>
              <p>When the app is opened or becomes active after this time, it shows a local reminder if due cards are waiting.</p>
            </div>
          </div>
        </div>

        <div className="surface-panel surface-panel--danger">
          <h2>Danger zone</h2>
          <p>Reset the local SQLite database and recreate the app with default settings.</p>
          <Button variant="danger" onClick={() => setResetOpen(true)}>
            Reset app data
          </Button>
        </div>

        <div className="surface-panel">
          <h2>About CODO</h2>
          <div className="form-stack settings-about">
            <div className="surface-muted">
              <div className="surface-muted__label">App</div>
              <p>CODO: Flashcard</p>
            </div>

            <div className="surface-muted settings-about__copy">
              <div className="surface-muted__label">About</div>
              <p>
                Flashcard Local is a lightweight, fully offline flashcard app designed for simple and effective vocabulary
                learning.
              </p>
              <p>
                It is built to be accessible to everyone, completely free to use, and runs entirely on your device
                without any accounts or internet connection. The app focuses on speed, privacy, and a clean learning
                experience.
              </p>
              <p>
                This project was developed with the help of AI, with the goal of creating a practical, minimal, and
                reliable tool for everyday language learning.
              </p>
            </div>

            <div className="surface-muted">
              <div className="surface-muted__label">Developer</div>
              <p>PARSA FALAHATI</p>
              <a
                className="external-link"
                href="https://www.linkedin.com/in/parsa-falahati"
                target="_blank"
                rel="noreferrer"
              >
                <span className="external-link__icon" aria-hidden="true">
                  <svg viewBox="0 0 24 24" role="img" focusable="false">
                    <path
                      fill="currentColor"
                      d="M6.94 8.5H3.56V19h3.38V8.5Zm.22-3.25a1.96 1.96 0 1 0-3.92 0 1.96 1.96 0 0 0 3.92 0ZM20.5 13.01c0-3.22-1.72-4.72-4.02-4.72-1.85 0-2.67 1.02-3.13 1.73V8.5H9.97c.04 1.01 0 10.5 0 10.5h3.38v-5.86c0-.31.02-.62.11-.84.25-.62.81-1.25 1.76-1.25 1.24 0 1.74.94 1.74 2.32V19H20.5v-5.99Z"
                    />
                  </svg>
                </span>
                <span>Visit LinkedIn</span>
              </a>
            </div>

            <div className="surface-muted">
              <div className="surface-muted__label">Message</div>
              <p>به امید فردایی بهتر ♥️</p>
            </div>
          </div>
        </div>
      </section>

      <div className="dialog-actions dialog-actions--end page-actions">
        <Button variant="secondary" onClick={() => setDraft(settings)}>
          Revert changes
        </Button>
        <Button onClick={() => void saveSettings()} disabled={saving}>
          {saving ? "Saving..." : "Save settings"}
        </Button>
      </div>

      <ConfirmDialog
        open={resetOpen}
        title="Reset local app data"
        description="This deletes your local deck database and recreates an empty one. Back up your data first if you want to keep it."
        confirmLabel="Reset app data"
        onCancel={() => setResetOpen(false)}
        onConfirm={() => void handleReset()}
      />
    </>
  );
}
