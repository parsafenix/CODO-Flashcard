use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::{
  models::types::{
    CalibrationDataSufficiency, CalibrationDiagnostics, CalibrationSplitMetrics, CalibrationWorkloadComparison,
    SchedulerCalibrationProfile, SchedulerCalibrationRunSummary, SchedulerCalibrationStatus, SchedulerParameterValue,
  },
  services::srs::{SchedulerParameters, SCHEDULER_PROFILE_VERSION},
};

fn now_utc() -> String {
  Utc::now().to_rfc3339()
}

fn bool_to_i64(value: bool) -> i64 {
  if value { 1 } else { 0 }
}

fn default_parameter_values() -> Vec<SchedulerParameterValue> {
  parameter_values(&SchedulerParameters::default())
}

pub fn parameter_values(parameters: &SchedulerParameters) -> Vec<SchedulerParameterValue> {
  let defaults = SchedulerParameters::default();
  vec![
    value("decay_scale", "Decay scale", parameters.decay_scale, defaults.decay_scale, 3.0, 25.0),
    value(
      "decay_exponent",
      "Decay exponent",
      parameters.decay_exponent,
      defaults.decay_exponent,
      0.5,
      2.5,
    ),
    value(
      "difficulty_delta",
      "Difficulty delta",
      parameters.difficulty_delta,
      defaults.difficulty_delta,
      0.05,
      1.5,
    ),
    value(
      "difficulty_mean_reversion",
      "Difficulty mean reversion",
      parameters.difficulty_mean_reversion,
      defaults.difficulty_mean_reversion,
      0.0,
      0.5,
    ),
    value(
      "difficulty_mean",
      "Difficulty mean",
      parameters.difficulty_mean,
      defaults.difficulty_mean,
      3.0,
      7.0,
    ),
    value(
      "success_base_log",
      "Success base log",
      parameters.success_base_log,
      defaults.success_base_log,
      -3.0,
      1.0,
    ),
    value(
      "success_stability_power",
      "Success stability power",
      parameters.success_stability_power,
      defaults.success_stability_power,
      0.0,
      1.2,
    ),
    value(
      "success_retrievability_weight",
      "Success retrievability weight",
      parameters.success_retrievability_weight,
      defaults.success_retrievability_weight,
      0.1,
      4.0,
    ),
    value("failure_base", "Failure base", parameters.failure_base, defaults.failure_base, 0.01, 2.0),
    value(
      "failure_difficulty_power",
      "Failure difficulty power",
      parameters.failure_difficulty_power,
      defaults.failure_difficulty_power,
      0.0,
      1.5,
    ),
    value(
      "failure_stability_power",
      "Failure stability power",
      parameters.failure_stability_power,
      defaults.failure_stability_power,
      0.1,
      1.5,
    ),
    value(
      "failure_retrievability_weight",
      "Failure retrievability weight",
      parameters.failure_retrievability_weight,
      defaults.failure_retrievability_weight,
      0.0,
      3.0,
    ),
    value("hard_bonus", "Hard bonus", parameters.hard_bonus, defaults.hard_bonus, 0.05, 1.0),
    value("good_bonus", "Good bonus", parameters.good_bonus, defaults.good_bonus, 0.4, 2.0),
    value("easy_bonus", "Easy bonus", parameters.easy_bonus, defaults.easy_bonus, 0.8, 3.0),
    value(
      "initial_difficulty_again",
      "Initial difficulty (Again)",
      parameters.initial_difficulty_again,
      defaults.initial_difficulty_again,
      2.0,
      9.0,
    ),
    value(
      "initial_difficulty_hard",
      "Initial difficulty (Hard)",
      parameters.initial_difficulty_hard,
      defaults.initial_difficulty_hard,
      2.0,
      9.0,
    ),
    value(
      "initial_difficulty_good",
      "Initial difficulty (Good)",
      parameters.initial_difficulty_good,
      defaults.initial_difficulty_good,
      2.0,
      9.0,
    ),
    value(
      "initial_difficulty_easy",
      "Initial difficulty (Easy)",
      parameters.initial_difficulty_easy,
      defaults.initial_difficulty_easy,
      2.0,
      9.0,
    ),
    value(
      "initial_stability_again",
      "Initial stability (Again)",
      parameters.initial_stability_again,
      defaults.initial_stability_again,
      1.0 / 144.0,
      0.5,
    ),
    value(
      "initial_stability_hard",
      "Initial stability (Hard)",
      parameters.initial_stability_hard,
      defaults.initial_stability_hard,
      0.02,
      1.0,
    ),
    value(
      "initial_stability_good",
      "Initial stability (Good)",
      parameters.initial_stability_good,
      defaults.initial_stability_good,
      0.05,
      3.0,
    ),
    value(
      "initial_stability_easy",
      "Initial stability (Easy)",
      parameters.initial_stability_easy,
      defaults.initial_stability_easy,
      0.1,
      5.0,
    ),
  ]
}

fn value(name: &str, label: &str, value: f64, default_value: f64, min: f64, max: f64) -> SchedulerParameterValue {
  SchedulerParameterValue {
    name: name.to_string(),
    label: label.to_string(),
    value,
    default_value,
    min,
    max,
  }
}

fn map_profile(row: &Row<'_>) -> rusqlite::Result<SchedulerCalibrationProfile> {
  let parameters: SchedulerParameters = serde_json::from_str::<SchedulerParameters>(&row.get::<_, String>("parameters_json")?)
    .unwrap_or_default()
    .validate();
  let metrics = row
    .get::<_, Option<String>>("metrics_json")?
    .and_then(|json| serde_json::from_str::<CalibrationSplitMetrics>(&json).ok());
  Ok(SchedulerCalibrationProfile {
    id: row.get("id")?,
    profile_key: row.get("profile_key")?,
    profile_version: row.get("profile_version")?,
    label: row.get("label")?,
    source: row.get("source")?,
    is_active: row.get::<_, i64>("is_active")? != 0,
    created_at: row.get("created_at")?,
    activated_at: row.get("activated_at")?,
    metrics,
    parameters: parameter_values(&parameters),
    notes: row.get("notes")?,
  })
}

fn map_run(row: &Row<'_>) -> rusqlite::Result<SchedulerCalibrationRunSummary> {
  Ok(SchedulerCalibrationRunSummary {
    id: row.get("id")?,
    status: row.get("status")?,
    accepted: row.get::<_, i64>("accepted")? != 0,
    started_at: row.get("started_at")?,
    completed_at: row.get("completed_at")?,
    used_recency_weighting: row.get::<_, i64>("used_recency_weighting")? != 0,
    recency_half_life_days: row.get("recency_half_life_days")?,
    total_events: row.get("total_events")?,
    usable_events: row.get("usable_events")?,
    filtered_events: row.get("filtered_events")?,
    distinct_review_units: row.get("distinct_review_units")?,
    deck_coverage_count: row.get("deck_coverage_count")?,
    mature_review_events: row.get("mature_review_events")?,
    failure_events: row.get("failure_events")?,
    train_events: row.get("train_events")?,
    validation_events: row.get("validation_events")?,
    test_events: row.get("test_events")?,
    split_train_end_utc: row.get("split_train_end_utc")?,
    split_validation_end_utc: row.get("split_validation_end_utc")?,
    baseline_metrics: serde_json::from_str(&row.get::<_, String>("baseline_metrics_json")?).unwrap_or_default(),
    candidate_metrics: row
      .get::<_, Option<String>>("candidate_metrics_json")?
      .and_then(|json| serde_json::from_str::<CalibrationSplitMetrics>(&json).ok()),
    diagnostics: row
      .get::<_, Option<String>>("diagnostics_json")?
      .and_then(|json| serde_json::from_str::<CalibrationDiagnostics>(&json).ok()),
    workload: row
      .get::<_, Option<String>>("workload_json")?
      .and_then(|json| serde_json::from_str::<CalibrationWorkloadComparison>(&json).ok()),
    reason: row.get("reason")?,
  })
}

pub fn ensure_default_profile(connection: &Connection) -> Result<()> {
  let now = now_utc();
  let existing_count = connection.query_row("SELECT COUNT(*) FROM scheduler_parameter_profiles", [], |row| row.get::<_, i64>(0))?;
  if existing_count == 0 {
    connection.execute(
      "INSERT INTO scheduler_parameter_profiles (
        profile_key, profile_version, label, source, is_active, parameters_json, metrics_json, notes, created_at, activated_at
      ) VALUES (?1, ?2, ?3, 'default', 1, ?4, NULL, ?5, ?6, ?6)",
      params![
        "default-codo-dsr-v2",
        SCHEDULER_PROFILE_VERSION,
        "Default scheduler profile",
        serde_json::to_string(&SchedulerParameters::default())?,
        "Factory profile shipped with the app.",
        now
      ],
    )?;
    return Ok(());
  }

  let active_count = connection.query_row(
    "SELECT COUNT(*) FROM scheduler_parameter_profiles WHERE is_active = 1",
    [],
    |row| row.get::<_, i64>(0),
  )?;
  if active_count == 0 {
    connection.execute(
      "UPDATE scheduler_parameter_profiles SET is_active = 1, activated_at = ?1 WHERE id = (
        SELECT id FROM scheduler_parameter_profiles ORDER BY id ASC LIMIT 1
      )",
      params![now],
    )?;
  }
  Ok(())
}

pub fn get_active_profile(connection: &Connection) -> Result<SchedulerCalibrationProfile> {
  connection
    .query_row(
      "SELECT * FROM scheduler_parameter_profiles WHERE is_active = 1 ORDER BY activated_at DESC, id DESC LIMIT 1",
      [],
      map_profile,
    )
    .optional()?
    .context("Active scheduler profile not found")
}

pub fn get_active_parameters(connection: &Connection) -> Result<SchedulerParameters> {
  let profile = get_active_profile(connection)?;
  let raw = profile
    .parameters
    .iter()
    .map(|value| (value.name.as_str(), value.value))
    .collect::<std::collections::HashMap<_, _>>();
  Ok(SchedulerParameters::validate(SchedulerParameters {
    decay_scale: *raw.get("decay_scale").unwrap_or(&9.0),
    decay_exponent: *raw.get("decay_exponent").unwrap_or(&1.0),
    difficulty_delta: *raw.get("difficulty_delta").unwrap_or(&0.55),
    difficulty_mean_reversion: *raw.get("difficulty_mean_reversion").unwrap_or(&0.12),
    difficulty_mean: *raw.get("difficulty_mean").unwrap_or(&5.0),
    success_base_log: *raw.get("success_base_log").unwrap_or(&-0.10),
    success_stability_power: *raw.get("success_stability_power").unwrap_or(&0.24),
    success_retrievability_weight: *raw.get("success_retrievability_weight").unwrap_or(&1.45),
    failure_base: *raw.get("failure_base").unwrap_or(&0.42),
    failure_difficulty_power: *raw.get("failure_difficulty_power").unwrap_or(&0.32),
    failure_stability_power: *raw.get("failure_stability_power").unwrap_or(&0.58),
    failure_retrievability_weight: *raw.get("failure_retrievability_weight").unwrap_or(&1.08),
    hard_bonus: *raw.get("hard_bonus").unwrap_or(&0.35),
    good_bonus: *raw.get("good_bonus").unwrap_or(&1.0),
    easy_bonus: *raw.get("easy_bonus").unwrap_or(&1.45),
    initial_difficulty_again: *raw.get("initial_difficulty_again").unwrap_or(&6.8),
    initial_difficulty_hard: *raw.get("initial_difficulty_hard").unwrap_or(&6.0),
    initial_difficulty_good: *raw.get("initial_difficulty_good").unwrap_or(&5.2),
    initial_difficulty_easy: *raw.get("initial_difficulty_easy").unwrap_or(&4.4),
    initial_stability_again: *raw.get("initial_stability_again").unwrap_or(&0.08),
    initial_stability_hard: *raw.get("initial_stability_hard").unwrap_or(&0.18),
    initial_stability_good: *raw.get("initial_stability_good").unwrap_or(&0.55),
    initial_stability_easy: *raw.get("initial_stability_easy").unwrap_or(&1.15),
  }))
}

pub fn insert_profile(
  connection: &Connection,
  profile_key: &str,
  label: &str,
  source: &str,
  parameters: &SchedulerParameters,
  metrics: Option<&CalibrationSplitMetrics>,
  notes: Option<&str>,
) -> Result<SchedulerCalibrationProfile> {
  let now = now_utc();
  connection.execute(
    "INSERT INTO scheduler_parameter_profiles (
      profile_key, profile_version, label, source, is_active, parameters_json, metrics_json, notes, created_at, activated_at
    ) VALUES (?1, ?2, ?3, ?4, 0, ?5, ?6, ?7, ?8, NULL)",
    params![
      profile_key,
      SCHEDULER_PROFILE_VERSION,
      label,
      source,
      serde_json::to_string(&parameters.validate())?,
      metrics.map(serde_json::to_string).transpose()?,
      notes,
      now
    ],
  )?;
  connection
    .query_row(
      "SELECT * FROM scheduler_parameter_profiles WHERE id = ?1",
      params![connection.last_insert_rowid()],
      map_profile,
    )
    .map_err(Into::into)
}

pub fn activate_profile(connection: &Connection, profile_id: i64) -> Result<()> {
  let now = now_utc();
  connection.execute("UPDATE scheduler_parameter_profiles SET is_active = 0 WHERE is_active = 1", [])?;
  connection.execute(
    "UPDATE scheduler_parameter_profiles SET is_active = 1, activated_at = ?1 WHERE id = ?2",
    params![now, profile_id],
  )?;
  Ok(())
}

pub fn save_calibration_run(
  connection: &Connection,
  run: &SchedulerCalibrationRunSummary,
  profile_before_id: Option<i64>,
  profile_candidate_id: Option<i64>,
  profile_after_id: Option<i64>,
) -> Result<i64> {
  connection.execute(
    "INSERT INTO scheduler_calibration_runs (
      started_at, completed_at, status, accepted, used_recency_weighting, recency_half_life_days,
      total_events, usable_events, filtered_events, distinct_review_units, deck_coverage_count,
      mature_review_events, failure_events, train_events, validation_events, test_events,
      split_train_end_utc, split_validation_end_utc, profile_before_id, profile_candidate_id, profile_after_id,
      baseline_metrics_json, candidate_metrics_json, diagnostics_json, workload_json, reason
    ) VALUES (
      ?1, ?2, ?3, ?4, ?5, ?6,
      ?7, ?8, ?9, ?10, ?11,
      ?12, ?13, ?14, ?15, ?16,
      ?17, ?18, ?19, ?20, ?21,
      ?22, ?23, ?24, ?25, ?26
    )",
    params![
      run.started_at,
      run.completed_at,
      run.status,
      bool_to_i64(run.accepted),
      bool_to_i64(run.used_recency_weighting),
      run.recency_half_life_days,
      run.total_events,
      run.usable_events,
      run.filtered_events,
      run.distinct_review_units,
      run.deck_coverage_count,
      run.mature_review_events,
      run.failure_events,
      run.train_events,
      run.validation_events,
      run.test_events,
      run.split_train_end_utc,
      run.split_validation_end_utc,
      profile_before_id,
      profile_candidate_id,
      profile_after_id,
      serde_json::to_string(&run.baseline_metrics)?,
      run.candidate_metrics.as_ref().map(serde_json::to_string).transpose()?,
      run.diagnostics.as_ref().map(serde_json::to_string).transpose()?,
      run.workload.as_ref().map(serde_json::to_string).transpose()?,
      run.reason
    ],
  )?;
  Ok(connection.last_insert_rowid())
}

pub fn get_latest_run(connection: &Connection) -> Result<Option<SchedulerCalibrationRunSummary>> {
  connection
    .query_row(
      "SELECT * FROM scheduler_calibration_runs ORDER BY id DESC LIMIT 1",
      [],
      map_run,
    )
    .optional()
    .map_err(Into::into)
}

pub fn get_status(
  connection: &Connection,
  sufficiency: CalibrationDataSufficiency,
) -> Result<SchedulerCalibrationStatus> {
  Ok(SchedulerCalibrationStatus {
    active_profile: get_active_profile(connection)?,
    latest_run: get_latest_run(connection)?,
    sufficiency,
  })
}

pub fn default_profile_preview() -> SchedulerCalibrationProfile {
  SchedulerCalibrationProfile {
    id: 0,
    profile_key: "default-codo-dsr-v2".to_string(),
    profile_version: SCHEDULER_PROFILE_VERSION.to_string(),
    label: "Default scheduler profile".to_string(),
    source: "default".to_string(),
    is_active: true,
    created_at: now_utc(),
    activated_at: Some(now_utc()),
    metrics: None,
    parameters: default_parameter_values(),
    notes: Some("Factory profile shipped with the app.".to_string()),
  }
}
