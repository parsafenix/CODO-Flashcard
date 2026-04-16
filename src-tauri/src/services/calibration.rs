use std::collections::{HashMap, HashSet, VecDeque};

use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use rusqlite::Connection;

use crate::{
  db::repository::scheduler_repo,
  models::types::{
    AppSettings, CalibrationBreakdownRow, CalibrationCurvePoint, CalibrationDataSufficiency, CalibrationDiagnostics,
    CalibrationMetrics, CalibrationSplitMetrics, CalibrationWorkloadComparison, CalibrationWorkloadForecast, ReviewRating,
    ReviewUnitRecord, ReviewUnitState, SchedulerCalibrationRunSummary, SchedulerCalibrationStatus, SchedulerReviewInput,
  },
  services::srs::{self, SchedulerParameters},
};

const EPSILON: f64 = 1e-6;
const MIN_USABLE_EVENTS: i64 = 400;
const MIN_DISTINCT_REVIEW_UNITS: i64 = 40;
const MIN_MATURE_REVIEW_EVENTS: i64 = 120;
const MIN_FAILURE_EVENTS: i64 = 25;

const MIN_LOG_LOSS_IMPROVEMENT: f64 = 0.002;
const MAX_RMSE_REGRESSION: f64 = 0.01;
const MAX_TEST_LOG_LOSS_REGRESSION: f64 = 0.005;
const MAX_WORKLOAD_INCREASE_RATIO: f64 = 1.5;

#[derive(Clone, Copy, PartialEq, Eq)]
enum SplitSegment {
  Train,
  Validation,
  Test,
}

#[derive(Clone)]
struct CalibrationEvent {
  id: i64,
  review_unit_id: i64,
  deck_id: i64,
  reviewed_at: DateTime<Utc>,
  reviewed_at_utc: String,
  rating: ReviewRating,
  was_correct: bool,
  state_before: ReviewUnitState,
  difficulty_before: f64,
  stability_before: f64,
  interval_before_days: f64,
  elapsed_days: Option<f64>,
  latency_ms: Option<i64>,
  hint_used: bool,
  confidence: Option<f64>,
  segment: SplitSegment,
}

#[derive(Default)]
struct Dataset {
  events_by_unit: HashMap<i64, Vec<CalibrationEvent>>,
  sufficiency: CalibrationDataSufficiency,
  split_train_end_utc: Option<String>,
  split_validation_end_utc: Option<String>,
}

#[derive(Default)]
struct RawCounters {
  total_events: i64,
  filtered_events: i64,
  usable_events: i64,
  distinct_review_units: i64,
  deck_coverage_count: i64,
  mature_review_events: i64,
  failure_events: i64,
}

#[derive(Clone)]
struct Observation {
  probability: f64,
  outcome: bool,
  weight: f64,
  state: ReviewUnitState,
  rating: ReviewRating,
  elapsed_days: f64,
}

#[derive(Default)]
struct SegmentAccumulator {
  observations: Vec<Observation>,
}

#[derive(Default)]
struct EvaluationArtifacts {
  training: SegmentAccumulator,
  validation: SegmentAccumulator,
  test: SegmentAccumulator,
}

#[derive(Default, Clone)]
struct SegmentCounts {
  train: i64,
  validation: i64,
  test: i64,
}

#[derive(Clone)]
struct EvaluationReport {
  metrics: CalibrationSplitMetrics,
  diagnostics: CalibrationDiagnostics,
  train_objective: f64,
}

#[derive(Clone, Copy)]
enum ParameterId {
  DecayScale,
  DecayExponent,
  DifficultyDelta,
  DifficultyMeanReversion,
  DifficultyMean,
  SuccessBaseLog,
  SuccessStabilityPower,
  SuccessRetrievabilityWeight,
  FailureBase,
  FailureDifficultyPower,
  FailureStabilityPower,
  FailureRetrievabilityWeight,
  HardBonus,
  GoodBonus,
  EasyBonus,
  InitialDifficultyAgain,
  InitialDifficultyHard,
  InitialDifficultyGood,
  InitialDifficultyEasy,
  InitialStabilityAgain,
  InitialStabilityHard,
  InitialStabilityGood,
  InitialStabilityEasy,
}

#[derive(Clone, Copy)]
struct ParameterSpec {
  id: ParameterId,
  step: f64,
}

fn now_utc() -> String {
  Utc::now().to_rfc3339()
}

fn rating_from_db(value: &str) -> ReviewRating {
  match value {
    "again" => ReviewRating::Again,
    "hard" => ReviewRating::Hard,
    "easy" => ReviewRating::Easy,
    _ => ReviewRating::Good,
  }
}

fn parse_timestamp(value: &str) -> Option<DateTime<Utc>> {
  DateTime::parse_from_rfc3339(value).ok().map(|timestamp| timestamp.with_timezone(&Utc))
}

fn clamp_probability(value: f64) -> f64 {
  value.clamp(EPSILON, 1.0 - EPSILON)
}

fn round_float(value: f64) -> f64 {
  (value * 10_000.0).round() / 10_000.0
}

fn recency_weight(event: &CalibrationEvent, latest_training_timestamp: Option<DateTime<Utc>>, settings: &AppSettings) -> f64 {
  if !settings.calibration_use_recency_weighting {
    return 1.0;
  }
  let Some(latest) = latest_training_timestamp else {
    return 1.0;
  };
  let age_days = (latest - event.reviewed_at).num_seconds().max(0) as f64 / 86_400.0;
  let half_life = settings.calibration_recency_half_life_days.max(14) as f64;
  (-(std::f64::consts::LN_2 * age_days / half_life)).exp().clamp(0.1, 1.0)
}

fn weighted_log_loss(observations: &[Observation]) -> f64 {
  if observations.is_empty() {
    return 0.0;
  }
  let mut weighted_sum = 0.0;
  let mut total_weight = 0.0;
  for observation in observations {
    let y = if observation.outcome { 1.0 } else { 0.0 };
    let p = clamp_probability(observation.probability);
    weighted_sum += observation.weight * (-(y * p.ln() + (1.0 - y) * (1.0 - p).ln()));
    total_weight += observation.weight;
  }
  if total_weight <= 0.0 {
    0.0
  } else {
    weighted_sum / total_weight
  }
}

fn compute_metrics(observations: &[Observation]) -> CalibrationMetrics {
  if observations.is_empty() {
    return CalibrationMetrics::default();
  }

  let log_loss = weighted_log_loss(
    &observations
      .iter()
      .map(|observation| Observation { weight: 1.0, ..observation.clone() })
      .collect::<Vec<_>>(),
  );
  let brier_score = observations
    .iter()
    .map(|observation| {
      let y = if observation.outcome { 1.0 } else { 0.0 };
      (observation.probability - y).powi(2)
    })
    .sum::<f64>()
    / observations.len() as f64;

  let bins = build_curve(observations, 10);
  let rmse_bins = if bins.is_empty() {
    0.0
  } else {
    (bins
      .iter()
      .map(|point| (point.average_predicted - point.actual_rate).powi(2))
      .sum::<f64>()
      / bins.len() as f64)
      .sqrt()
  };

  let auc = compute_auc(observations);
  let (calibration_slope, calibration_intercept) = calibration_regression(observations);

  CalibrationMetrics {
    event_count: observations.len() as i64,
    log_loss: round_float(log_loss),
    rmse_bins: round_float(rmse_bins),
    auc: round_float(auc),
    brier_score: round_float(brier_score),
    calibration_slope: calibration_slope.map(round_float),
    calibration_intercept: calibration_intercept.map(round_float),
  }
}

fn compute_auc(observations: &[Observation]) -> f64 {
  let positives = observations.iter().filter(|observation| observation.outcome).count();
  let negatives = observations.len().saturating_sub(positives);
  if positives == 0 || negatives == 0 {
    return 0.5;
  }

  let mut ranked = observations
    .iter()
    .enumerate()
    .map(|(index, observation)| (index, observation.probability, observation.outcome))
    .collect::<Vec<_>>();
  ranked.sort_by(|left, right| left.1.total_cmp(&right.1));

  let mut rank_sum = 0.0;
  for (rank, (_, _, outcome)) in ranked.iter().enumerate() {
    if *outcome {
      rank_sum += (rank + 1) as f64;
    }
  }
  (rank_sum - (positives * (positives + 1) / 2) as f64) / (positives as f64 * negatives as f64)
}

fn calibration_regression(observations: &[Observation]) -> (Option<f64>, Option<f64>) {
  if observations.len() < 5 {
    return (None, None);
  }
  let xs = observations
    .iter()
    .map(|observation| clamp_probability(observation.probability))
    .map(|probability| (probability / (1.0 - probability)).ln())
    .collect::<Vec<_>>();
  let ys = observations
    .iter()
    .map(|observation| if observation.outcome { 1.0 } else { 0.0 })
    .collect::<Vec<_>>();
  let x_mean = xs.iter().sum::<f64>() / xs.len() as f64;
  let y_mean = ys.iter().sum::<f64>() / ys.len() as f64;
  let variance = xs.iter().map(|value| (value - x_mean).powi(2)).sum::<f64>();
  if variance <= EPSILON {
    return (None, None);
  }
  let covariance = xs
    .iter()
    .zip(ys.iter())
    .map(|(x, y)| (x - x_mean) * (y - y_mean))
    .sum::<f64>();
  let slope = covariance / variance;
  let intercept = y_mean - slope * x_mean;
  (Some(slope), Some(intercept))
}

fn build_curve(observations: &[Observation], bins: usize) -> Vec<CalibrationCurvePoint> {
  let mut grouped = vec![(0.0_f64, 0.0_f64, 0_i64); bins];
  for observation in observations {
    let bin = ((observation.probability * bins as f64).floor() as usize).min(bins.saturating_sub(1));
    grouped[bin].0 += observation.probability;
    grouped[bin].1 += if observation.outcome { 1.0 } else { 0.0 };
    grouped[bin].2 += 1;
  }

  grouped
    .into_iter()
    .enumerate()
    .filter(|(_, (_, _, count))| *count > 0)
    .map(|(index, (predicted_sum, actual_sum, count))| {
      let start = index as f64 / bins as f64;
      let end = (index + 1) as f64 / bins as f64;
      CalibrationCurvePoint {
        bin_index: index as i64,
        label: format!("{start:.1}-{end:.1}"),
        average_predicted: round_float(predicted_sum / count as f64),
        actual_rate: round_float(actual_sum / count as f64),
        event_count: count,
      }
    })
    .collect()
}

fn build_breakdown(observations: &[Observation], labeler: impl Fn(&Observation) -> String) -> Vec<CalibrationBreakdownRow> {
  let mut groups = HashMap::<String, Vec<Observation>>::new();
  for observation in observations {
    groups.entry(labeler(observation)).or_default().push(observation.clone());
  }

  let mut rows = groups
    .into_iter()
    .map(|(label, group)| {
      let metrics = compute_metrics(&group);
      CalibrationBreakdownRow {
        label,
        event_count: metrics.event_count,
        average_predicted: round_float(group.iter().map(|observation| observation.probability).sum::<f64>() / group.len() as f64),
        actual_rate: round_float(group.iter().filter(|observation| observation.outcome).count() as f64 / group.len() as f64),
        log_loss: metrics.log_loss,
        brier_score: metrics.brier_score,
      }
    })
    .collect::<Vec<_>>();

  rows.sort_by(|left, right| right.event_count.cmp(&left.event_count).then_with(|| left.label.cmp(&right.label)));
  rows
}

fn elapsed_band_label(elapsed_days: f64) -> String {
  match elapsed_days {
    days if days < 1.0 => "0-1 day".to_string(),
    days if days < 3.0 => "1-3 days".to_string(),
    days if days < 7.0 => "3-7 days".to_string(),
    days if days < 14.0 => "7-14 days".to_string(),
    days if days < 30.0 => "14-30 days".to_string(),
    _ => "30+ days".to_string(),
  }
}

fn build_diagnostics(observations: &[Observation]) -> CalibrationDiagnostics {
  CalibrationDiagnostics {
    curve: build_curve(observations, 10),
    error_by_state: build_breakdown(observations, |observation| observation.state.as_str().to_string()),
    error_by_rating: build_breakdown(observations, |observation| observation.rating.as_str().to_string()),
    retention_by_elapsed_band: build_breakdown(observations, |observation| elapsed_band_label(observation.elapsed_days)),
  }
}

fn evaluate_dataset(dataset: &Dataset, settings: &AppSettings, parameters: &SchedulerParameters) -> EvaluationReport {
  let latest_training_timestamp = dataset
    .events_by_unit
    .values()
    .flat_map(|events| events.iter())
    .filter(|event| matches!(event.segment, SplitSegment::Train))
    .map(|event| event.reviewed_at)
    .max();

  let mut evaluation = EvaluationArtifacts::default();

  for events in dataset.events_by_unit.values() {
    let mut recent_ratings = VecDeque::with_capacity(6);
    let Some(first) = events.first() else {
      continue;
    };
    let mut unit = seed_review_unit(first, settings);

    for event in events {
      let input = SchedulerReviewInput {
        rating: event.rating,
        reviewed_at_utc: event.reviewed_at_utc.clone(),
        latency_ms: event.latency_ms,
        hint_used: event.hint_used,
        confidence: event.confidence,
        desired_retention: settings.desired_retention,
        learning_steps_minutes: settings.learning_steps_minutes.clone(),
        relearning_steps_minutes: settings.relearning_steps_minutes.clone(),
        recent_again_count: recent_ratings.iter().filter(|rating| matches!(rating, ReviewRating::Again)).count() as i64,
        leech_lapse_threshold: settings.leech_lapse_threshold as i64,
      };
      let update = srs::schedule_review_with_parameters(&unit, &input, parameters);
      let observation = Observation {
        probability: clamp_probability(update.retrievability_before),
        outcome: event.was_correct,
        weight: recency_weight(event, latest_training_timestamp, settings),
        state: event.state_before,
        rating: event.rating,
        elapsed_days: event.elapsed_days.unwrap_or(0.0).max(0.0),
      };
      match event.segment {
        SplitSegment::Train => evaluation.training.observations.push(observation),
        SplitSegment::Validation => evaluation.validation.observations.push(observation),
        SplitSegment::Test => evaluation.test.observations.push(observation),
      }
      recent_ratings.push_front(event.rating);
      if recent_ratings.len() > 6 {
        recent_ratings.pop_back();
      }
      unit = apply_update(unit, &update);
    }
  }

  let metrics = CalibrationSplitMetrics {
    training: compute_metrics(&evaluation.training.observations),
    validation: compute_metrics(&evaluation.validation.observations),
    test: compute_metrics(&evaluation.test.observations),
  };
  let diagnostics = build_diagnostics(&evaluation.validation.observations);

  EvaluationReport {
    train_objective: weighted_log_loss(&evaluation.training.observations),
    metrics,
    diagnostics,
  }
}

fn seed_review_unit(event: &CalibrationEvent, settings: &AppSettings) -> ReviewUnitRecord {
  let inferred_last_review = event.elapsed_days.and_then(|elapsed| {
    if elapsed <= 0.0 {
      None
    } else {
      Some((event.reviewed_at - Duration::seconds((elapsed * 86_400.0).round() as i64)).to_rfc3339())
    }
  });
  let learning_step_index = if matches!(event.state_before, ReviewUnitState::New | ReviewUnitState::Learning) {
    infer_step_index(event.interval_before_days, &settings.learning_steps_minutes)
  } else {
    0
  };
  let relearning_step_index = if matches!(event.state_before, ReviewUnitState::Relearning | ReviewUnitState::Leech) {
    infer_step_index(event.interval_before_days, &settings.relearning_steps_minutes)
  } else {
    0
  };

  ReviewUnitRecord {
    id: event.review_unit_id,
    card_id: 0,
    deck_id: event.deck_id,
    prompt_field_id: 0,
    reveal_field_ids: Vec::new(),
    direction_key: format!("calibration:{}", event.review_unit_id),
    state: event.state_before,
    difficulty: event.difficulty_before.clamp(0.0, 10.0),
    stability: event.stability_before.max(1.0 / 144.0),
    scheduled_interval_days: event.interval_before_days.max(0.0),
    last_reviewed_at_utc: inferred_last_review,
    due_at_utc: Some(event.reviewed_at_utc.clone()),
    lapses: 0,
    successful_reviews: 0,
    failed_reviews: 0,
    total_reviews: 0,
    same_day_reviews_count: 0,
    average_latency_ms: None,
    last_latency_ms: None,
    hint_used_last: false,
    confidence_last: None,
    suspended: false,
    leech: matches!(event.state_before, ReviewUnitState::Leech),
    mastered: false,
    learning_step_index,
    relearning_step_index,
    first_reviewed_at_utc: None,
    graduated_at_utc: None,
    mastered_at_utc: None,
    created_at: event.reviewed_at_utc.clone(),
    updated_at: event.reviewed_at_utc.clone(),
  }
}

fn infer_step_index(interval_before_days: f64, steps_minutes: &[i64]) -> i64 {
  let steps = steps_minutes
    .iter()
    .enumerate()
    .map(|(index, step)| (index as i64, *step as f64 / (24.0 * 60.0)))
    .collect::<Vec<_>>();
  steps
    .into_iter()
    .min_by(|left, right| (left.1 - interval_before_days).abs().total_cmp(&(right.1 - interval_before_days).abs()))
    .map(|(index, _)| index)
    .unwrap_or(0)
}

fn apply_update(mut unit: ReviewUnitRecord, update: &crate::models::types::ReviewUnitUpdate) -> ReviewUnitRecord {
  unit.state = update.state;
  unit.difficulty = update.difficulty;
  unit.stability = update.stability;
  unit.scheduled_interval_days = update.scheduled_interval_days;
  unit.last_reviewed_at_utc = Some(update.last_reviewed_at_utc.clone());
  unit.due_at_utc = update.due_at_utc.clone();
  unit.lapses = update.lapses;
  unit.successful_reviews = update.successful_reviews;
  unit.failed_reviews = update.failed_reviews;
  unit.total_reviews = update.total_reviews;
  unit.same_day_reviews_count = update.same_day_reviews_count;
  unit.average_latency_ms = update.average_latency_ms;
  unit.last_latency_ms = update.last_latency_ms;
  unit.hint_used_last = update.hint_used_last;
  unit.confidence_last = update.confidence_last;
  unit.suspended = update.suspended;
  unit.leech = update.leech;
  unit.mastered = update.mastered;
  unit.learning_step_index = update.learning_step_index;
  unit.relearning_step_index = update.relearning_step_index;
  unit.first_reviewed_at_utc = update.first_reviewed_at_utc.clone();
  unit.graduated_at_utc = update.graduated_at_utc.clone();
  unit.mastered_at_utc = update.mastered_at_utc.clone();
  unit.updated_at = update.updated_at.clone();
  unit
}

fn build_dataset(connection: &Connection) -> Result<Dataset> {
  let mut statement = connection.prepare(
    "SELECT
      id,
      review_unit_id,
      deck_id,
      reviewed_at_utc,
      rating,
      was_correct,
      state_before,
      difficulty_before,
      stability_before,
      interval_before_days,
      elapsed_days,
      latency_ms,
      hint_used,
      confidence
     FROM review_logs
     ORDER BY reviewed_at_utc ASC, id ASC",
  )?;

  let rows = statement.query_map([], |row| {
    Ok((
      row.get::<_, i64>(0)?,
      row.get::<_, i64>(1)?,
      row.get::<_, i64>(2)?,
      row.get::<_, String>(3)?,
      row.get::<_, String>(4)?,
      row.get::<_, i64>(5)? != 0,
      row.get::<_, String>(6)?,
      row.get::<_, f64>(7)?,
      row.get::<_, f64>(8)?,
      row.get::<_, f64>(9)?,
      row.get::<_, Option<f64>>(10)?,
      row.get::<_, Option<i64>>(11)?,
      row.get::<_, i64>(12)? != 0,
      row.get::<_, Option<f64>>(13)?,
    ))
  })?;

  let mut raw_events = Vec::new();
  let mut seen = HashSet::new();
  let mut counters = RawCounters::default();
  let mut deck_ids = HashSet::new();

  for row in rows {
    counters.total_events += 1;
    let (id, review_unit_id, deck_id, reviewed_at_utc, rating, was_correct, state_before, difficulty_before, stability_before, interval_before_days, elapsed_days, latency_ms, hint_used, confidence) =
      row?;
    let signature = format!("{review_unit_id}|{reviewed_at_utc}|{rating}|{state_before}|{difficulty_before:.6}|{stability_before:.6}");
    if !seen.insert(signature) {
      counters.filtered_events += 1;
      continue;
    }
    let Some(reviewed_at) = parse_timestamp(&reviewed_at_utc) else {
      counters.filtered_events += 1;
      continue;
    };
    if !(0.0..=10.0).contains(&difficulty_before) || !stability_before.is_finite() || stability_before <= 0.0 || interval_before_days < 0.0 {
      counters.filtered_events += 1;
      continue;
    }
    let rating = rating_from_db(&rating);
    if was_correct != rating.is_success() {
      counters.filtered_events += 1;
      continue;
    }

    deck_ids.insert(deck_id);
    raw_events.push(CalibrationEvent {
      id,
      review_unit_id,
      deck_id,
      reviewed_at,
      reviewed_at_utc,
      rating,
      was_correct,
      state_before: ReviewUnitState::from_db(&state_before),
      difficulty_before,
      stability_before,
      interval_before_days,
      elapsed_days,
      latency_ms,
      hint_used,
      confidence,
      segment: SplitSegment::Train,
    });
  }

  raw_events.sort_by(|left, right| left.reviewed_at.cmp(&right.reviewed_at).then(left.id.cmp(&right.id)));
  counters.usable_events = raw_events.len() as i64;
  counters.filtered_events = counters.total_events - counters.usable_events;
  counters.distinct_review_units = raw_events.iter().map(|event| event.review_unit_id).collect::<HashSet<_>>().len() as i64;
  counters.deck_coverage_count = deck_ids.len() as i64;
  counters.mature_review_events = raw_events
    .iter()
    .filter(|event| matches!(event.state_before, ReviewUnitState::Review))
    .count() as i64;
  counters.failure_events = raw_events.iter().filter(|event| !event.was_correct).count() as i64;

  let enough_data = counters.usable_events >= MIN_USABLE_EVENTS
    && counters.distinct_review_units >= MIN_DISTINCT_REVIEW_UNITS
    && counters.mature_review_events >= MIN_MATURE_REVIEW_EVENTS
    && counters.failure_events >= MIN_FAILURE_EVENTS;

  let mut split_train_end_utc = None;
  let mut split_validation_end_utc = None;
  if !raw_events.is_empty() {
    let train_end = ((raw_events.len() as f64) * 0.70).floor() as usize;
    let validation_end = ((raw_events.len() as f64) * 0.85).floor() as usize;
    let train_end = train_end.clamp(1, raw_events.len().saturating_sub(2));
    let validation_end = validation_end.clamp(train_end + 1, raw_events.len().saturating_sub(1));
    split_train_end_utc = raw_events.get(train_end - 1).map(|event| event.reviewed_at_utc.clone());
    split_validation_end_utc = raw_events.get(validation_end - 1).map(|event| event.reviewed_at_utc.clone());

    for (index, event) in raw_events.iter_mut().enumerate() {
      event.segment = if index < train_end {
        SplitSegment::Train
      } else if index < validation_end {
        SplitSegment::Validation
      } else {
        SplitSegment::Test
      };
    }
  }

  let mut events_by_unit = HashMap::<i64, Vec<CalibrationEvent>>::new();
  for event in raw_events {
    events_by_unit.entry(event.review_unit_id).or_default().push(event);
  }
  for events in events_by_unit.values_mut() {
    events.sort_by(|left, right| left.reviewed_at.cmp(&right.reviewed_at).then(left.id.cmp(&right.id)));
  }

  Ok(Dataset {
    events_by_unit,
    sufficiency: CalibrationDataSufficiency {
      enough_data,
      minimum_usable_events: MIN_USABLE_EVENTS,
      minimum_distinct_review_units: MIN_DISTINCT_REVIEW_UNITS,
      minimum_mature_review_events: MIN_MATURE_REVIEW_EVENTS,
      minimum_failure_events: MIN_FAILURE_EVENTS,
      total_events: counters.total_events,
      usable_events: counters.usable_events,
      filtered_events: counters.filtered_events,
      distinct_review_units: counters.distinct_review_units,
      deck_coverage_count: counters.deck_coverage_count,
      mature_review_events: counters.mature_review_events,
      failure_events: counters.failure_events,
    },
    split_train_end_utc,
    split_validation_end_utc,
  })
}

fn parameter_specs() -> Vec<ParameterSpec> {
  vec![
    ParameterSpec { id: ParameterId::DecayScale, step: 1.5 },
    ParameterSpec { id: ParameterId::DecayExponent, step: 0.15 },
    ParameterSpec { id: ParameterId::DifficultyDelta, step: 0.08 },
    ParameterSpec { id: ParameterId::DifficultyMeanReversion, step: 0.03 },
    ParameterSpec { id: ParameterId::DifficultyMean, step: 0.2 },
    ParameterSpec { id: ParameterId::SuccessBaseLog, step: 0.15 },
    ParameterSpec { id: ParameterId::SuccessStabilityPower, step: 0.04 },
    ParameterSpec { id: ParameterId::SuccessRetrievabilityWeight, step: 0.12 },
    ParameterSpec { id: ParameterId::FailureBase, step: 0.06 },
    ParameterSpec { id: ParameterId::FailureDifficultyPower, step: 0.04 },
    ParameterSpec { id: ParameterId::FailureStabilityPower, step: 0.04 },
    ParameterSpec { id: ParameterId::FailureRetrievabilityWeight, step: 0.08 },
    ParameterSpec { id: ParameterId::HardBonus, step: 0.05 },
    ParameterSpec { id: ParameterId::GoodBonus, step: 0.07 },
    ParameterSpec { id: ParameterId::EasyBonus, step: 0.08 },
    ParameterSpec { id: ParameterId::InitialDifficultyAgain, step: 0.15 },
    ParameterSpec { id: ParameterId::InitialDifficultyHard, step: 0.12 },
    ParameterSpec { id: ParameterId::InitialDifficultyGood, step: 0.12 },
    ParameterSpec { id: ParameterId::InitialDifficultyEasy, step: 0.10 },
    ParameterSpec { id: ParameterId::InitialStabilityAgain, step: 0.03 },
    ParameterSpec { id: ParameterId::InitialStabilityHard, step: 0.04 },
    ParameterSpec { id: ParameterId::InitialStabilityGood, step: 0.05 },
    ParameterSpec { id: ParameterId::InitialStabilityEasy, step: 0.06 },
  ]
}

fn get_parameter(parameters: &SchedulerParameters, id: ParameterId) -> f64 {
  match id {
    ParameterId::DecayScale => parameters.decay_scale,
    ParameterId::DecayExponent => parameters.decay_exponent,
    ParameterId::DifficultyDelta => parameters.difficulty_delta,
    ParameterId::DifficultyMeanReversion => parameters.difficulty_mean_reversion,
    ParameterId::DifficultyMean => parameters.difficulty_mean,
    ParameterId::SuccessBaseLog => parameters.success_base_log,
    ParameterId::SuccessStabilityPower => parameters.success_stability_power,
    ParameterId::SuccessRetrievabilityWeight => parameters.success_retrievability_weight,
    ParameterId::FailureBase => parameters.failure_base,
    ParameterId::FailureDifficultyPower => parameters.failure_difficulty_power,
    ParameterId::FailureStabilityPower => parameters.failure_stability_power,
    ParameterId::FailureRetrievabilityWeight => parameters.failure_retrievability_weight,
    ParameterId::HardBonus => parameters.hard_bonus,
    ParameterId::GoodBonus => parameters.good_bonus,
    ParameterId::EasyBonus => parameters.easy_bonus,
    ParameterId::InitialDifficultyAgain => parameters.initial_difficulty_again,
    ParameterId::InitialDifficultyHard => parameters.initial_difficulty_hard,
    ParameterId::InitialDifficultyGood => parameters.initial_difficulty_good,
    ParameterId::InitialDifficultyEasy => parameters.initial_difficulty_easy,
    ParameterId::InitialStabilityAgain => parameters.initial_stability_again,
    ParameterId::InitialStabilityHard => parameters.initial_stability_hard,
    ParameterId::InitialStabilityGood => parameters.initial_stability_good,
    ParameterId::InitialStabilityEasy => parameters.initial_stability_easy,
  }
}

fn set_parameter(parameters: &mut SchedulerParameters, id: ParameterId, value: f64) {
  match id {
    ParameterId::DecayScale => parameters.decay_scale = value,
    ParameterId::DecayExponent => parameters.decay_exponent = value,
    ParameterId::DifficultyDelta => parameters.difficulty_delta = value,
    ParameterId::DifficultyMeanReversion => parameters.difficulty_mean_reversion = value,
    ParameterId::DifficultyMean => parameters.difficulty_mean = value,
    ParameterId::SuccessBaseLog => parameters.success_base_log = value,
    ParameterId::SuccessStabilityPower => parameters.success_stability_power = value,
    ParameterId::SuccessRetrievabilityWeight => parameters.success_retrievability_weight = value,
    ParameterId::FailureBase => parameters.failure_base = value,
    ParameterId::FailureDifficultyPower => parameters.failure_difficulty_power = value,
    ParameterId::FailureStabilityPower => parameters.failure_stability_power = value,
    ParameterId::FailureRetrievabilityWeight => parameters.failure_retrievability_weight = value,
    ParameterId::HardBonus => parameters.hard_bonus = value,
    ParameterId::GoodBonus => parameters.good_bonus = value,
    ParameterId::EasyBonus => parameters.easy_bonus = value,
    ParameterId::InitialDifficultyAgain => parameters.initial_difficulty_again = value,
    ParameterId::InitialDifficultyHard => parameters.initial_difficulty_hard = value,
    ParameterId::InitialDifficultyGood => parameters.initial_difficulty_good = value,
    ParameterId::InitialDifficultyEasy => parameters.initial_difficulty_easy = value,
    ParameterId::InitialStabilityAgain => parameters.initial_stability_again = value,
    ParameterId::InitialStabilityHard => parameters.initial_stability_hard = value,
    ParameterId::InitialStabilityGood => parameters.initial_stability_good = value,
    ParameterId::InitialStabilityEasy => parameters.initial_stability_easy = value,
  }
}

fn optimize_parameters(dataset: &Dataset, settings: &AppSettings, baseline: &SchedulerParameters) -> SchedulerParameters {
  let mut best = baseline.validate();
  let mut best_score = evaluate_dataset(dataset, settings, &best).train_objective;
  let mut steps = parameter_specs();

  for _ in 0..6 {
    let mut improved = false;
    for spec in &steps {
      let current_value = get_parameter(&best, spec.id);
      for direction in [-1.0_f64, 1.0_f64] {
        let mut candidate = best;
        set_parameter(&mut candidate, spec.id, current_value + direction * spec.step);
        let candidate = candidate.validate();
        let score = evaluate_dataset(dataset, settings, &candidate).train_objective;
        if score + EPSILON < best_score {
          best = candidate;
          best_score = score;
          improved = true;
        }
      }
    }

    if !improved {
      for spec in &mut steps {
        spec.step *= 0.5;
      }
    }
  }

  best.validate()
}

fn build_workload_forecast(connection: &Connection, desired_retention: f64, parameters: &SchedulerParameters) -> Result<CalibrationWorkloadForecast> {
  let mut statement = connection.prepare(
    "SELECT stability
     FROM review_units
     WHERE suspended = 0 AND state != 'new'",
  )?;
  let rows = statement.query_map([], |row| row.get::<_, f64>(0))?;
  let mut due_next_7d = 0_i64;
  let mut due_next_30d = 0_i64;
  let mut retention_sum = 0.0;
  let mut count = 0_i64;
  for row in rows {
    let stability = row?;
    let interval = srs::interval_from_stability_with_parameters(stability, desired_retention, parameters);
    let predicted = srs::retrievability_with_parameters(interval, stability, parameters);
    if interval <= 7.0 {
      due_next_7d += 1;
    }
    if interval <= 30.0 {
      due_next_30d += 1;
    }
    retention_sum += predicted;
    count += 1;
  }

  Ok(CalibrationWorkloadForecast {
    due_next_7d,
    due_next_30d,
    expected_recall_at_due_percent: if count == 0 {
      0
    } else {
      ((retention_sum / count as f64) * 100.0).round() as i64
    },
  })
}

fn build_workload_comparison(
  connection: &Connection,
  desired_retention: f64,
  active: &SchedulerParameters,
  candidate: &SchedulerParameters,
) -> Result<CalibrationWorkloadComparison> {
  let active_forecast = build_workload_forecast(connection, desired_retention, active)?;
  let candidate_forecast = build_workload_forecast(connection, desired_retention, candidate)?;
  Ok(CalibrationWorkloadComparison {
    workload_change_percent_7d: if active_forecast.due_next_7d <= 0 {
      0.0
    } else {
      round_float(((candidate_forecast.due_next_7d - active_forecast.due_next_7d) as f64 / active_forecast.due_next_7d as f64) * 100.0)
    },
    workload_change_percent_30d: if active_forecast.due_next_30d <= 0 {
      0.0
    } else {
      round_float(((candidate_forecast.due_next_30d - active_forecast.due_next_30d) as f64 / active_forecast.due_next_30d as f64) * 100.0)
    },
    active: active_forecast,
    candidate: candidate_forecast,
  })
}

fn build_segment_counts(dataset: &Dataset) -> SegmentCounts {
  let mut counts = SegmentCounts::default();
  for events in dataset.events_by_unit.values() {
    for event in events {
      match event.segment {
        SplitSegment::Train => counts.train += 1,
        SplitSegment::Validation => counts.validation += 1,
        SplitSegment::Test => counts.test += 1,
      }
    }
  }
  counts
}

fn should_accept_candidate(
  baseline: &EvaluationReport,
  candidate: &EvaluationReport,
  workload: &CalibrationWorkloadComparison,
) -> (bool, String) {
  if candidate.metrics.validation.log_loss > baseline.metrics.validation.log_loss - MIN_LOG_LOSS_IMPROVEMENT {
    return (false, "Validation log loss did not improve enough.".to_string());
  }
  if candidate.metrics.validation.rmse_bins > baseline.metrics.validation.rmse_bins + MAX_RMSE_REGRESSION {
    return (false, "Validation RMSE (bins) regressed beyond the allowed guardrail.".to_string());
  }
  if candidate.metrics.test.log_loss > baseline.metrics.test.log_loss + MAX_TEST_LOG_LOSS_REGRESSION {
    return (false, "Test log loss regressed, so the new fit was not activated.".to_string());
  }
  if workload.active.due_next_30d > 0
    && workload.candidate.due_next_30d as f64 > workload.active.due_next_30d as f64 * MAX_WORKLOAD_INCREASE_RATIO
  {
    return (false, "The fitted parameters would increase the 30-day review load too aggressively.".to_string());
  }
  (true, "Validation metrics improved and workload stayed within the safety guardrails.".to_string())
}

pub fn get_calibration_status(connection: &Connection) -> Result<SchedulerCalibrationStatus> {
  let dataset = build_dataset(connection)?;
  scheduler_repo::get_status(connection, dataset.sufficiency)
}

pub fn run_calibration(connection: &Connection, settings: &AppSettings) -> Result<SchedulerCalibrationStatus> {
  let dataset = build_dataset(connection)?;
  let started_at = now_utc();
  let segment_counts = build_segment_counts(&dataset);
  let active_profile = scheduler_repo::get_active_profile(connection).unwrap_or_else(|_| scheduler_repo::default_profile_preview());
  let active_parameters = scheduler_repo::get_active_parameters(connection).unwrap_or_default();
  let baseline_report = evaluate_dataset(&dataset, settings, &active_parameters);

  let latest_run = if !dataset.sufficiency.enough_data {
    let run = SchedulerCalibrationRunSummary {
      id: 0,
      status: "insufficient_data".to_string(),
      accepted: false,
      started_at: started_at.clone(),
      completed_at: Some(now_utc()),
      used_recency_weighting: settings.calibration_use_recency_weighting,
      recency_half_life_days: Some(settings.calibration_recency_half_life_days as f64),
      total_events: dataset.sufficiency.total_events,
      usable_events: dataset.sufficiency.usable_events,
      filtered_events: dataset.sufficiency.filtered_events,
      distinct_review_units: dataset.sufficiency.distinct_review_units,
      deck_coverage_count: dataset.sufficiency.deck_coverage_count,
      mature_review_events: dataset.sufficiency.mature_review_events,
      failure_events: dataset.sufficiency.failure_events,
      train_events: segment_counts.train,
      validation_events: segment_counts.validation,
      test_events: segment_counts.test,
      split_train_end_utc: dataset.split_train_end_utc.clone(),
      split_validation_end_utc: dataset.split_validation_end_utc.clone(),
      baseline_metrics: baseline_report.metrics.clone(),
      candidate_metrics: None,
      diagnostics: Some(baseline_report.diagnostics.clone()),
      workload: None,
      reason: Some("There is not enough reliable local review data to fit a statistically meaningful parameter update yet.".to_string()),
    };
    scheduler_repo::save_calibration_run(connection, &run, Some(active_profile.id), None, Some(active_profile.id))?;
    scheduler_repo::get_latest_run(connection)?
  } else {
    let candidate_parameters = optimize_parameters(&dataset, settings, &active_parameters);
    let candidate_report = evaluate_dataset(&dataset, settings, &candidate_parameters);
    let workload = build_workload_comparison(connection, settings.desired_retention, &active_parameters, &candidate_parameters)?;
    let (accepted, reason) = should_accept_candidate(&baseline_report, &candidate_report, &workload);
    let candidate_profile = scheduler_repo::insert_profile(
      connection,
      &format!("calibrated-{}", Utc::now().format("%Y%m%d%H%M%S")),
      "Calibrated scheduler profile",
      "calibrated",
      &candidate_parameters,
      Some(&candidate_report.metrics),
      Some("Fitted locally from review_logs with time-series validation."),
    )?;
    let profile_after_id = if accepted {
      scheduler_repo::activate_profile(connection, candidate_profile.id)?;
      candidate_profile.id
    } else {
      active_profile.id
    };

    let run = SchedulerCalibrationRunSummary {
      id: 0,
      status: if accepted { "accepted".to_string() } else { "rejected".to_string() },
      accepted,
      started_at: started_at.clone(),
      completed_at: Some(now_utc()),
      used_recency_weighting: settings.calibration_use_recency_weighting,
      recency_half_life_days: Some(settings.calibration_recency_half_life_days as f64),
      total_events: dataset.sufficiency.total_events,
      usable_events: dataset.sufficiency.usable_events,
      filtered_events: dataset.sufficiency.filtered_events,
      distinct_review_units: dataset.sufficiency.distinct_review_units,
      deck_coverage_count: dataset.sufficiency.deck_coverage_count,
      mature_review_events: dataset.sufficiency.mature_review_events,
      failure_events: dataset.sufficiency.failure_events,
      train_events: segment_counts.train,
      validation_events: segment_counts.validation,
      test_events: segment_counts.test,
      split_train_end_utc: dataset.split_train_end_utc.clone(),
      split_validation_end_utc: dataset.split_validation_end_utc.clone(),
      baseline_metrics: baseline_report.metrics.clone(),
      candidate_metrics: Some(candidate_report.metrics.clone()),
      diagnostics: Some(candidate_report.diagnostics.clone()),
      workload: Some(workload),
      reason: Some(reason),
    };
    scheduler_repo::save_calibration_run(
      connection,
      &run,
      Some(active_profile.id),
      Some(candidate_profile.id),
      Some(profile_after_id),
    )?;
    scheduler_repo::get_latest_run(connection)?
  };

  Ok(SchedulerCalibrationStatus {
    active_profile: scheduler_repo::get_active_profile(connection).unwrap_or(active_profile),
    latest_run,
    sufficiency: dataset.sufficiency,
  })
}

#[cfg(test)]
mod tests {
  use chrono::{Duration, Utc};
  use rusqlite::params;

  use crate::{
    db::{initialize_database, open_connection, repository::scheduler_repo},
    models::types::AppSettings,
  };

  use super::{build_dataset, get_calibration_status, run_calibration, weighted_log_loss};

  fn temp_db_path(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("codo-calibration-{name}-{}.sqlite", Utc::now().timestamp_nanos_opt().unwrap_or_default()))
  }

  fn seed_logs(connection: &rusqlite::Connection, unit_count: i64, reviews_per_unit: i64) {
    let now = Utc::now();
    connection.execute(
      "INSERT INTO decks (id, name, description, language_1_label, language_2_label, language_3_label, created_at, updated_at)
       VALUES (1, 'Calibration', NULL, 'Front', 'Back', 'Context', ?1, ?1)",
      params![now.to_rfc3339()],
    ).unwrap();
    connection.execute(
      "INSERT INTO deck_fields (id, deck_id, label, language_code, order_index, required, active, field_type, system_key)
       VALUES
       (1, 1, 'Front', 'front', 0, 1, 1, 'text', NULL),
       (2, 1, 'Back', 'back', 1, 1, 1, 'text', NULL)",
      [],
    ).unwrap();
    for unit_index in 0..unit_count {
      let card_id = unit_index + 1;
      connection.execute(
        "INSERT INTO cards (
          id, deck_id, language_1, language_2, language_3, note, example_sentence, tag,
          language_1_normalized, language_2_normalized, language_3_normalized,
          language_1_compact, language_2_compact, language_3_compact,
          dedupe_key, created_at, updated_at, status
        ) VALUES (?1, 1, ?2, ?3, '', NULL, NULL, NULL, ?2, ?3, '', ?2, ?3, '', ?4, ?5, ?5, 'review')",
        params![
          card_id,
          format!("front-{card_id}"),
          format!("back-{card_id}"),
          format!("dedupe-{card_id}"),
          now.to_rfc3339()
        ],
      ).unwrap();
      connection.execute(
        "INSERT INTO review_units (
          id, card_id, deck_id, prompt_field_id, reveal_field_ids, direction_key, state, difficulty, stability,
          scheduled_interval_days, due_at_utc, lapses, successful_reviews, failed_reviews, total_reviews,
          same_day_reviews_count, hint_used_last, confidence_last, suspended, leech, mastered,
          learning_step_index, relearning_step_index, created_at, updated_at
        ) VALUES (?1, ?2, 1, 1, '[2]', 'field:1|2', 'review', 5.0, 5.0, 5.0, ?3, 0, 0, 0, 0, 0, 0, NULL, 0, 0, 0, 0, 0, ?3, ?3)",
        params![card_id, card_id, now.to_rfc3339()],
      ).unwrap();

      for review_index in 0..reviews_per_unit {
        let reviewed_at = (now - Duration::days((unit_count - unit_index + reviews_per_unit - review_index) as i64)).to_rfc3339();
        let rating = if review_index % 5 == 0 { "again" } else if review_index % 4 == 0 { "hard" } else { "good" };
        let was_correct = if rating == "again" { 0 } else { 1 };
        connection.execute(
          "INSERT INTO review_logs (
            review_unit_id, card_id, deck_id, session_id, reviewed_at_utc, rating, was_correct, state_before, state_after,
            retrievability_before, difficulty_before, difficulty_after, stability_before, stability_after,
            interval_before_days, interval_after_days, scheduled_due_before_utc, scheduled_due_after_utc,
            elapsed_days, latency_ms, hint_used, confidence, leech_before, leech_after
          ) VALUES (?1, ?2, 1, NULL, ?3, ?4, ?5, 'review', ?6, 0.7, 5.0, 5.1, 5.0, 5.2, 5.0, 6.0, ?3, ?3, 5.0, 1200, 0, NULL, 0, 0)",
          params![card_id, card_id, reviewed_at, rating, was_correct, if was_correct == 1 { "review" } else { "relearning" }],
        ).unwrap();
      }
    }
  }

  #[test]
  fn weighted_log_loss_is_stable() {
    let observations = vec![
      super::Observation { probability: 0.8, outcome: true, weight: 1.0, state: crate::models::types::ReviewUnitState::Review, rating: crate::models::types::ReviewRating::Good, elapsed_days: 1.0 },
      super::Observation { probability: 0.2, outcome: false, weight: 0.5, state: crate::models::types::ReviewUnitState::Review, rating: crate::models::types::ReviewRating::Again, elapsed_days: 2.0 },
    ];
    let loss = weighted_log_loss(&observations);
    assert!(loss > 0.0);
    assert!(loss < 1.0);
  }

  #[test]
  fn calibration_reports_insufficient_data_before_threshold() {
    let db_path = temp_db_path("insufficient");
    initialize_database(&db_path).unwrap();
    let connection = open_connection(&db_path).unwrap();
    scheduler_repo::ensure_default_profile(&connection).unwrap();
    let status = get_calibration_status(&connection).unwrap();
    assert!(!status.sufficiency.enough_data);
    let _ = std::fs::remove_file(db_path);
  }

  #[test]
  fn calibration_run_records_latest_result() {
    let db_path = temp_db_path("accepted");
    initialize_database(&db_path).unwrap();
    let connection = open_connection(&db_path).unwrap();
    scheduler_repo::ensure_default_profile(&connection).unwrap();
    seed_logs(&connection, 60, 8);
    let status = run_calibration(&connection, &AppSettings::default()).unwrap();
    assert!(status.latest_run.is_some());
    assert!(status.active_profile.id > 0);
    let dataset = build_dataset(&connection).unwrap();
    assert!(dataset.sufficiency.usable_events >= 400);
    let _ = std::fs::remove_file(db_path);
  }
}
