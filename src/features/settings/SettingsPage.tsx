import { open } from "@tauri-apps/plugin-dialog";
import { useEffect, useMemo, useState } from "react";
import { useAppContext } from "../../app/AppContext";
import { Button } from "../../components/ui/Button";
import { ConfirmDialog } from "../../components/ui/ConfirmDialog";
import { useToast } from "../../components/ui/ToastProvider";
import { api } from "../../lib/api";
import { useI18n } from "../../lib/i18n";
import type { AppSettings, FieldPresetKind, StudyMode, Theme, UiLanguage } from "../../lib/types";

export function SettingsPage() {
  const { settings, setSettings, refreshSettings } = useAppContext();
  const { notify } = useToast();
  const { t } = useI18n();
  const [draft, setDraft] = useState<AppSettings>(settings);
  const [saving, setSaving] = useState(false);
  const [resetOpen, setResetOpen] = useState(false);
  const [newPresetLabel, setNewPresetLabel] = useState("");
  const [newPresetKind, setNewPresetKind] = useState<FieldPresetKind>("language");

  useEffect(() => {
    setDraft(settings);
  }, [settings]);

  const hasUnsavedChanges = useMemo(() => JSON.stringify(draft) !== JSON.stringify(settings), [draft, settings]);

  async function saveSettings() {
    setSaving(true);
    try {
      const nextSettings = await api.updateSettings(draft);
      setSettings(nextSettings);
      setDraft(nextSettings);
      notify(t("settings.saved"), "success");
    } catch (err) {
      const message = typeof err === "object" && err && "message" in err ? String(err.message) : t("settings.saveError");
      notify(message, "error");
    } finally {
      setSaving(false);
    }
  }

  async function handleBackup() {
    const selected = await open({ directory: true, multiple: false });
    if (typeof selected !== "string") {
      return;
    }

    try {
      const result = await api.createBackup(selected);
      notify(t("settings.backupCreated", { path: result.output_path }), "success");
      await refreshSettings();
    } catch (err) {
      const message = typeof err === "object" && err && "message" in err ? String(err.message) : t("settings.backupError");
      notify(message, "error");
    }
  }

  async function handleOpenDataFolder() {
    try {
      await api.openDataFolder();
      notify(t("settings.openFolderSuccess"), "info");
    } catch (err) {
      const message = typeof err === "object" && err && "message" in err ? String(err.message) : t("settings.openFolderError");
      notify(message, "error");
    }
  }

  async function handleReset() {
    try {
      await api.resetAppData();
      await refreshSettings();
      setResetOpen(false);
      notify(t("settings.resetSuccess"), "success");
    } catch (err) {
      const message = typeof err === "object" && err && "message" in err ? String(err.message) : t("settings.resetError");
      notify(message, "error");
    }
  }

  function addPreset() {
    const label = newPresetLabel.trim();
    if (!label) {
      return;
    }

    const baseId = label
      .toLowerCase()
      .normalize("NFKC")
      .replace(/[^\p{L}\p{N}\s-]+/gu, "")
      .trim()
      .replace(/\s+/g, "-");
    let id = baseId || `preset-${draft.field_presets.length + 1}`;
    let suffix = 2;
    while (draft.field_presets.some((preset) => preset.id === id)) {
      id = `${baseId || "preset"}-${suffix}`;
      suffix += 1;
    }

    setDraft((current) => ({
      ...current,
      field_presets: [...current.field_presets, { id, label, kind: newPresetKind }],
    }));
    setNewPresetLabel("");
    setNewPresetKind("language");
  }

  return (
    <>
      <section className="page-header">
        <div>
          <p className="eyebrow">{t("nav.settings")}</p>
          <h1>{t("settings.title")}</h1>
          <p>{t("settings.description")}</p>
        </div>
      </section>

      <section className="settings-grid">
        <div className="surface-panel">
          <h2>{t("settings.appearance")}</h2>
          <div className="form-stack">
            <label className="field">
              <span>{t("settings.theme")}</span>
              <select value={draft.theme} onChange={(event) => setDraft((current) => ({ ...current, theme: event.target.value as Theme }))}>
                <option value="dark">{t("settings.theme.dark")}</option>
                <option value="light">{t("settings.theme.light")}</option>
              </select>
            </label>

            <label className="field">
              <span>{t("settings.uiLanguage")}</span>
              <select
                value={draft.ui_language}
                onChange={(event) => setDraft((current) => ({ ...current, ui_language: event.target.value as UiLanguage }))}
              >
                <option value="en">{t("settings.language.en")}</option>
                <option value="fa">{t("settings.language.fa")}</option>
                <option value="it">{t("settings.language.it")}</option>
              </select>
            </label>
          </div>
        </div>

        <div className="surface-panel">
          <h2>{t("settings.studyDefaults")}</h2>
          <div className="form-stack">
            <label className="field">
              <span>{t("settings.defaultStudyMode")}</span>
              <select
                value={draft.default_study_mode}
                onChange={(event) => setDraft((current) => ({ ...current, default_study_mode: event.target.value as StudyMode }))}
              >
                <option value="mixed">{t("study.mode.mixed")}</option>
                <option value="due">{t("study.mode.due")}</option>
                <option value="new">{t("study.mode.new")}</option>
              </select>
            </label>

            <label className="field">
              <span>{t("settings.cardsPerSession")}</span>
              <input
                type="number"
                min={1}
                max={200}
                value={draft.cards_per_session}
                onChange={(event) => setDraft((current) => ({ ...current, cards_per_session: Number(event.target.value) }))}
              />
            </label>

            <label className="field">
              <span>{t("settings.dailyGoal")}</span>
              <input
                type="number"
                min={1}
                max={500}
                value={draft.daily_review_goal}
                onChange={(event) => setDraft((current) => ({ ...current, daily_review_goal: Number(event.target.value) }))}
              />
            </label>

            <label className="field field--checkbox">
              <input
                type="checkbox"
                checked={draft.random_order}
                onChange={(event) => setDraft((current) => ({ ...current, random_order: event.target.checked }))}
              />
              <span>{t("settings.randomOrderDefault")}</span>
            </label>

            <label className="field field--checkbox">
              <input
                type="checkbox"
                checked={draft.reverse_mode}
                onChange={(event) => setDraft((current) => ({ ...current, reverse_mode: event.target.checked }))}
              />
              <span>{t("settings.reverseModeDefault")}</span>
            </label>
          </div>
        </div>

        <div className="surface-panel">
          <h2>{t("settings.importsBackups")}</h2>
          <div className="form-stack">
            <label className="field">
              <span>{t("settings.importDelimiter")}</span>
              <input
                value={draft.import_delimiter}
                maxLength={3}
                onChange={(event) => setDraft((current) => ({ ...current, import_delimiter: event.target.value }))}
              />
            </label>

            <div className="surface-muted">
              <div className="surface-muted__label">{t("settings.importBackupLastFolder")}</div>
              <p>{draft.last_backup_directory || t("settings.importBackupEmpty")}</p>
            </div>

            <div className="dialog-actions dialog-actions--start">
              <Button variant="secondary" onClick={() => void handleBackup()}>
                {t("settings.createBackup")}
              </Button>
              <Button variant="ghost" onClick={() => void handleOpenDataFolder()}>
                {t("settings.openDataFolder")}
              </Button>
            </div>
          </div>
        </div>

        <div className="surface-panel">
          <h2>{t("settings.reminder")}</h2>
          <div className="form-stack">
            <label className="field field--checkbox">
              <input
                type="checkbox"
                checked={draft.reminder_enabled}
                onChange={(event) => setDraft((current) => ({ ...current, reminder_enabled: event.target.checked }))}
              />
              <span>{t("settings.enableReminder")}</span>
            </label>

            <label className="field">
              <span>{t("settings.reminderTime")}</span>
              <input
                type="time"
                value={draft.reminder_time}
                onChange={(event) => setDraft((current) => ({ ...current, reminder_time: event.target.value }))}
              />
            </label>
          </div>
        </div>

        <div className="surface-panel">
          <h2>{t("settings.fieldPresets")}</h2>
          <div className="form-stack">
            <p>{t("settings.fieldPresetsHelp")}</p>
            {draft.field_presets.map((preset, index) => (
              <div key={`${preset.id}-${index}`} className="schema-field-row">
                <label className="field field--grow">
                  <span>{t("settings.presetLabel")}</span>
                  <input
                    dir="auto"
                    value={preset.label}
                    onChange={(event) =>
                      setDraft((current) => ({
                        ...current,
                        field_presets: current.field_presets.map((item, itemIndex) =>
                          itemIndex === index ? { ...item, label: event.target.value } : item
                        ),
                      }))
                    }
                  />
                </label>
                <label className="field">
                  <span>{t("settings.presetType")}</span>
                  <select
                    value={preset.kind}
                    onChange={(event) =>
                      setDraft((current) => ({
                        ...current,
                        field_presets: current.field_presets.map((item, itemIndex) =>
                          itemIndex === index ? { ...item, kind: event.target.value as FieldPresetKind } : item
                        ),
                      }))
                    }
                  >
                    <option value="language">{t("settings.presetType.language")}</option>
                    <option value="custom">{t("settings.presetType.custom")}</option>
                  </select>
                </label>
                <Button
                  type="button"
                  variant="danger"
                  onClick={() =>
                    setDraft((current) => ({
                      ...current,
                      field_presets: current.field_presets.filter((_, itemIndex) => itemIndex !== index),
                    }))
                  }
                >
                  {t("common.delete")}
                </Button>
              </div>
            ))}

            <div className="schema-field-row">
              <label className="field field--grow">
                <span>{t("settings.newPreset")}</span>
                <input dir="auto" value={newPresetLabel} onChange={(event) => setNewPresetLabel(event.target.value)} />
              </label>
              <label className="field">
                <span>{t("settings.presetType")}</span>
                <select value={newPresetKind} onChange={(event) => setNewPresetKind(event.target.value as FieldPresetKind)}>
                  <option value="language">{t("settings.presetType.language")}</option>
                  <option value="custom">{t("settings.presetType.custom")}</option>
                </select>
              </label>
              <Button type="button" onClick={addPreset}>
                {t("common.add")}
              </Button>
            </div>
          </div>
        </div>

        <div className="surface-panel surface-panel--danger">
          <h2>{t("settings.dangerTitle")}</h2>
          <p>{t("settings.dangerDescription")}</p>
          <Button variant="danger" onClick={() => setResetOpen(true)}>
            {t("settings.resetAppData")}
          </Button>
        </div>

        <div className="surface-panel">
          <h2>{t("settings.about")}</h2>
          <div className="form-stack settings-about">
            <div className="surface-muted">
              <div className="surface-muted__label">{t("settings.aboutApp")}</div>
              <p>CODO: Flashcard</p>
            </div>

            <div className="surface-muted settings-about__copy">
              <div className="surface-muted__label">{t("settings.aboutCopy")}</div>
              <p>{t("settings.aboutDescription1")}</p>
              <p>{t("settings.aboutDescription2")}</p>
              <p>{t("settings.aboutDescription3")}</p>
            </div>

            <div className="surface-muted">
              <div className="surface-muted__label">{t("settings.aboutDeveloper")}</div>
              <p>PARSA FALAHATI</p>
              <a className="external-link" href="https://www.linkedin.com/in/parsa-falahati" target="_blank" rel="noreferrer">
                <span className="external-link__icon" aria-hidden="true">
                  <svg viewBox="0 0 24 24" role="img" focusable="false">
                    <path
                      fill="currentColor"
                      d="M6.94 8.5H3.56V19h3.38V8.5Zm.22-3.25a1.96 1.96 0 1 0-3.92 0 1.96 1.96 0 0 0 3.92 0ZM20.5 13.01c0-3.22-1.72-4.72-4.02-4.72-1.85 0-2.67 1.02-3.13 1.73V8.5H9.97c.04 1.01 0 10.5 0 10.5h3.38v-5.86c0-.31.02-.62.11-.84.25-.62.81-1.25 1.76-1.25 1.24 0 1.74.94 1.74 2.32V19H20.5v-5.99Z"
                    />
                  </svg>
                </span>
                <span>{t("settings.aboutLinkedIn")}</span>
              </a>
            </div>

            <div className="surface-muted">
              <div className="surface-muted__label">{t("settings.aboutMessage")}</div>
              <p>به امید فردایی بهتر ♥️</p>
            </div>
          </div>
        </div>
      </section>

      <div className="dialog-actions dialog-actions--end page-actions">
        {hasUnsavedChanges ? <div className="form-helper-text">{t("settings.unsavedChanges")}</div> : null}
        <Button variant="secondary" onClick={() => setDraft(settings)}>
          {t("settings.revertChanges")}
        </Button>
        <Button onClick={() => void saveSettings()} disabled={saving}>
          {saving ? t("common.loading") : t("common.save")}
        </Button>
      </div>

      <ConfirmDialog
        open={resetOpen}
        title={t("settings.resetConfirmTitle")}
        description={t("settings.resetConfirmDescription")}
        confirmLabel={t("settings.resetAppData")}
        onCancel={() => setResetOpen(false)}
        onConfirm={() => void handleReset()}
      />
    </>
  );
}
