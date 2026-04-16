import { open } from "@tauri-apps/plugin-dialog";
import { useEffect, useMemo, useState } from "react";
import { useAppContext } from "../../app/AppContext";
import { Button } from "../../components/ui/Button";
import { ConfirmDialog } from "../../components/ui/ConfirmDialog";
import { HiddenPanelsBar } from "../../components/ui/HiddenPanelsBar";
import { PanelCard } from "../../components/ui/PanelCard";
import { useToast } from "../../components/ui/ToastProvider";
import { api } from "../../lib/api";
import { useI18n } from "../../lib/i18n";
import { localizeAppMessage, localizeCalibrationStatus } from "../../lib/messages";
import { usePanelVisibility } from "../../lib/usePanelVisibility";
import type { AppSettings, FieldPresetKind, SchedulerCalibrationStatus, StudyMode, Theme, UiLanguage } from "../../lib/types";

export function SettingsPage() {
  const { settings, setSettings, refreshSettings } = useAppContext();
  const { notify } = useToast();
  const { t } = useI18n();
  const [draft, setDraft] = useState<AppSettings>(settings);
  const [saving, setSaving] = useState(false);
  const [calibrationStatus, setCalibrationStatus] = useState<SchedulerCalibrationStatus | null>(null);
  const [calibrationLoading, setCalibrationLoading] = useState(false);
  const [resetOpen, setResetOpen] = useState(false);
  const [newPresetLabel, setNewPresetLabel] = useState("");
  const [newPresetKind, setNewPresetKind] = useState<FieldPresetKind>("language");
  const panelLabels = [
    { id: "appearance", label: t("settings.appearance") },
    { id: "study-defaults", label: t("settings.studyDefaults") },
    { id: "imports-backups", label: t("settings.importsBackups") },
    { id: "reminder", label: t("settings.reminder") },
    { id: "calibration", label: t("settings.calibrationTitle") },
    { id: "field-presets", label: t("settings.fieldPresets") },
    { id: "about", label: t("settings.about") },
    { id: "danger", label: t("settings.dangerTitle") },
  ];
  const { visiblePanels, hiddenPanels, hidePanel, showPanel } = usePanelVisibility("settings", panelLabels);

  useEffect(() => {
    setDraft(settings);
  }, [settings]);

  useEffect(() => {
    void loadCalibrationStatus();
  }, []);

  const hasUnsavedChanges = useMemo(() => JSON.stringify(draft) !== JSON.stringify(settings), [draft, settings]);

  async function loadCalibrationStatus() {
    try {
      const status = await api.getSchedulerCalibrationStatus();
      setCalibrationStatus(status);
    } catch {
      setCalibrationStatus(null);
    }
  }

  async function saveSettings() {
    setSaving(true);
    try {
      const nextSettings = await api.updateSettings(draft);
      setSettings(nextSettings);
      setDraft(nextSettings);
      notify(t("settings.saved"), "success");
    } catch (err) {
      const message = localizeAppMessage(
        typeof err === "object" && err && "message" in err ? String(err.message) : t("settings.saveError"),
        t
      );
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
      const message = localizeAppMessage(
        typeof err === "object" && err && "message" in err ? String(err.message) : t("settings.backupError"),
        t
      );
      notify(message, "error");
    }
  }

  async function handleOpenDataFolder() {
    try {
      await api.openDataFolder();
      notify(t("settings.openFolderSuccess"), "info");
    } catch (err) {
      const message = localizeAppMessage(
        typeof err === "object" && err && "message" in err ? String(err.message) : t("settings.openFolderError"),
        t
      );
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
      const message = localizeAppMessage(
        typeof err === "object" && err && "message" in err ? String(err.message) : t("settings.resetError"),
        t
      );
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

  async function handleRunCalibration() {
    setCalibrationLoading(true);
    try {
      const status = await api.runSchedulerCalibration();
      setCalibrationStatus(status);
      notify(
        status.latest_run?.accepted ? t("settings.calibrationAccepted") : t("settings.calibrationFinished"),
        status.latest_run?.accepted ? "success" : "info"
      );
    } catch (err) {
      const message = localizeAppMessage(
        typeof err === "object" && err && "message" in err ? String(err.message) : t("settings.calibrationError"),
        t
      );
      notify(message, "error");
    } finally {
      setCalibrationLoading(false);
    }
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

      <HiddenPanelsBar panels={hiddenPanels} onShow={(panelId) => void showPanel(panelId)} />

      <section className="panel-masonry">
        {visiblePanels.some((panel) => panel.id === "appearance") ? (
          <div className="panel-masonry__item">
            <PanelCard title={t("settings.appearance")} onHide={() => void hidePanel("appearance")}>
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
            </PanelCard>
          </div>
        ) : null}

        {visiblePanels.some((panel) => panel.id === "study-defaults") ? (
          <div className="panel-masonry__item">
            <PanelCard title={t("settings.studyDefaults")} onHide={() => void hidePanel("study-defaults")}>
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

                <label className="field">
                  <span>{t("settings.desiredRetention")}</span>
                  <input
                    type="number"
                    min={0.85}
                    max={0.95}
                    step={0.01}
                    value={draft.desired_retention}
                    onChange={(event) => setDraft((current) => ({ ...current, desired_retention: Number(event.target.value) }))}
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
            </PanelCard>
          </div>
        ) : null}

        {visiblePanels.some((panel) => panel.id === "imports-backups") ? (
          <div className="panel-masonry__item">
            <PanelCard title={t("settings.importsBackups")} onHide={() => void hidePanel("imports-backups")}>
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
            </PanelCard>
          </div>
        ) : null}

        {visiblePanels.some((panel) => panel.id === "reminder") ? (
          <div className="panel-masonry__item">
            <PanelCard title={t("settings.reminder")} onHide={() => void hidePanel("reminder")}>
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
            </PanelCard>
          </div>
        ) : null}

        {visiblePanels.some((panel) => panel.id === "calibration") ? (
          <div className="panel-masonry__item">
            <PanelCard title={t("settings.calibrationTitle")} onHide={() => void hidePanel("calibration")}>
              <div className="form-stack">
                <p>{t("settings.calibrationDescription")}</p>
                <div className="surface-muted">
                  <div className="surface-muted__label">{t("settings.calibrationActiveProfile")}</div>
                  <p>{calibrationStatus?.active_profile.label ?? t("common.loading")}</p>
                  <p className="form-helper-text">
                    {calibrationStatus
                      ? t("settings.calibrationProfileMeta", {
                          source: calibrationStatus.active_profile.source,
                          version: calibrationStatus.active_profile.profile_version,
                        })
                      : t("settings.calibrationLoading")}
                  </p>
                </div>

                <div className="stat-grid stat-grid--compact analytics-mini-stats">
                  <div className="stat-card">
                    <div className="stat-card__label">{t("settings.calibrationUsableReviews")}</div>
                    <div className="stat-card__value">{calibrationStatus?.sufficiency.usable_events ?? "..."}</div>
                  </div>
                  <div className="stat-card">
                    <div className="stat-card__label">{t("settings.calibrationReviewUnits")}</div>
                    <div className="stat-card__value">{calibrationStatus?.sufficiency.distinct_review_units ?? "..."}</div>
                  </div>
                  <div className="stat-card">
                    <div className="stat-card__label">{t("settings.calibrationMatureReviews")}</div>
                    <div className="stat-card__value">{calibrationStatus?.sufficiency.mature_review_events ?? "..."}</div>
                  </div>
                  <div className="stat-card">
                    <div className="stat-card__label">{t("settings.calibrationFailures")}</div>
                    <div className="stat-card__value">{calibrationStatus?.sufficiency.failure_events ?? "..."}</div>
                  </div>
                </div>

                <div className="surface-muted">
                  <div className="surface-muted__label">{t("settings.calibrationReadiness")}</div>
                  <p>
                    {calibrationStatus?.sufficiency.enough_data
                      ? t("settings.calibrationReady")
                      : t("settings.calibrationNotReady", {
                          reviews: calibrationStatus?.sufficiency.minimum_usable_events ?? 0,
                          units: calibrationStatus?.sufficiency.minimum_distinct_review_units ?? 0,
                        })}
                  </p>
                  {calibrationStatus?.latest_run?.reason ? (
                    <p className="form-helper-text">{localizeAppMessage(calibrationStatus.latest_run.reason, t)}</p>
                  ) : null}
                </div>

                {calibrationStatus?.latest_run ? (
                  <div className="surface-muted">
                    <div className="surface-muted__label">{t("settings.calibrationLastRun")}</div>
                    <p>
                      {t("settings.calibrationLastRunSummary", {
                        status: localizeCalibrationStatus(calibrationStatus.latest_run.status, t),
                        logLoss:
                          calibrationStatus.latest_run.candidate_metrics?.validation.log_loss ??
                          calibrationStatus.latest_run.baseline_metrics.validation.log_loss,
                      })}
                    </p>
                  </div>
                ) : null}

                <label className="field field--checkbox">
                  <input
                    type="checkbox"
                    checked={draft.calibration_use_recency_weighting}
                    onChange={(event) =>
                      setDraft((current) => ({ ...current, calibration_use_recency_weighting: event.target.checked }))
                    }
                  />
                  <span>{t("settings.calibrationRecencyWeighting")}</span>
                </label>

                <label className="field">
                  <span>{t("settings.calibrationHalfLife")}</span>
                  <input
                    type="number"
                    min={14}
                    max={720}
                    value={draft.calibration_recency_half_life_days}
                    disabled={!draft.calibration_use_recency_weighting}
                    onChange={(event) =>
                      setDraft((current) => ({ ...current, calibration_recency_half_life_days: Number(event.target.value) }))
                    }
                  />
                </label>

                <div className="dialog-actions dialog-actions--start">
                  <Button
                    onClick={() => void handleRunCalibration()}
                    disabled={calibrationLoading || !calibrationStatus?.sufficiency.enough_data || hasUnsavedChanges}
                  >
                    {calibrationLoading ? t("common.loading") : t("settings.runCalibration")}
                  </Button>
                  <Button variant="ghost" onClick={() => void loadCalibrationStatus()}>
                    {t("common.retry")}
                  </Button>
                </div>
                {hasUnsavedChanges ? <p className="form-helper-text">{t("settings.calibrationSaveFirst")}</p> : null}
              </div>
            </PanelCard>
          </div>
        ) : null}

        {visiblePanels.some((panel) => panel.id === "field-presets") ? (
          <div className="panel-masonry__item">
            <PanelCard
              title={t("settings.fieldPresets")}
              description={t("settings.fieldPresetsHelp")}
              onHide={() => void hidePanel("field-presets")}
            >
              <div className="form-stack">
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
            </PanelCard>
          </div>
        ) : null}

        {visiblePanels.some((panel) => panel.id === "about") ? (
          <div className="panel-masonry__item">
            <PanelCard title={t("settings.about")} onHide={() => void hidePanel("about")}>
              <div className="form-stack settings-about">
                <div className="surface-muted">
                  <div className="surface-muted__label">{t("settings.aboutApp")}</div>
                  <p>{t("app.brandFull")}</p>
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
                  <p>{t("settings.aboutQuote")}</p>
                </div>
              </div>
            </PanelCard>
          </div>
        ) : null}

        {visiblePanels.some((panel) => panel.id === "danger") ? (
          <div className="panel-masonry__item">
            <PanelCard title={t("settings.dangerTitle")} className="surface-panel--danger" onHide={() => void hidePanel("danger")}>
              <p>{t("settings.dangerDescription")}</p>
              <Button variant="danger" onClick={() => setResetOpen(true)}>
                {t("settings.resetAppData")}
              </Button>
            </PanelCard>
          </div>
        ) : null}
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
