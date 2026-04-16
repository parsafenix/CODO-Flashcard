use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::models::types::{ReviewRating, ReviewUnitRecord, ReviewUnitState, ReviewUnitUpdate, SchedulerReviewInput};

const MIN_STABILITY_DAYS: f64 = 1.0 / 144.0;
const MASTERED_STABILITY_DAYS: f64 = 45.0;
const MASTERED_SUCCESS_REVIEWS: i64 = 8;
pub const SCHEDULER_PROFILE_VERSION: &str = "codo_dsr_v2";

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(default)]
pub struct SchedulerParameters {
  pub decay_scale: f64,
  pub decay_exponent: f64,
  pub difficulty_delta: f64,
  pub difficulty_mean_reversion: f64,
  pub difficulty_mean: f64,
  pub success_base_log: f64,
  pub success_stability_power: f64,
  pub success_retrievability_weight: f64,
  pub failure_base: f64,
  pub failure_difficulty_power: f64,
  pub failure_stability_power: f64,
  pub failure_retrievability_weight: f64,
  pub hard_bonus: f64,
  pub good_bonus: f64,
  pub easy_bonus: f64,
  pub initial_difficulty_again: f64,
  pub initial_difficulty_hard: f64,
  pub initial_difficulty_good: f64,
  pub initial_difficulty_easy: f64,
  pub initial_stability_again: f64,
  pub initial_stability_hard: f64,
  pub initial_stability_good: f64,
  pub initial_stability_easy: f64,
}

impl Default for SchedulerParameters {
  fn default() -> Self {
    Self::validate(Self {
      decay_scale: 9.0,
      decay_exponent: 1.0,
      difficulty_delta: 0.55,
      difficulty_mean_reversion: 0.12,
      difficulty_mean: 5.0,
      success_base_log: -0.10,
      success_stability_power: 0.24,
      success_retrievability_weight: 1.45,
      failure_base: 0.42,
      failure_difficulty_power: 0.32,
      failure_stability_power: 0.58,
      failure_retrievability_weight: 1.08,
      hard_bonus: 0.35,
      good_bonus: 1.0,
      easy_bonus: 1.45,
      initial_difficulty_again: 6.8,
      initial_difficulty_hard: 6.0,
      initial_difficulty_good: 5.2,
      initial_difficulty_easy: 4.4,
      initial_stability_again: 0.08,
      initial_stability_hard: 0.18,
      initial_stability_good: 0.55,
      initial_stability_easy: 1.15,
    })
  }
}

impl SchedulerParameters {
  pub fn validate(mut self) -> Self {
    self.decay_scale = self.decay_scale.clamp(3.0, 25.0);
    self.decay_exponent = self.decay_exponent.clamp(0.5, 2.5);
    self.difficulty_delta = self.difficulty_delta.clamp(0.05, 1.5);
    self.difficulty_mean_reversion = self.difficulty_mean_reversion.clamp(0.0, 0.5);
    self.difficulty_mean = self.difficulty_mean.clamp(3.0, 7.0);
    self.success_base_log = self.success_base_log.clamp(-3.0, 1.0);
    self.success_stability_power = self.success_stability_power.clamp(0.0, 1.2);
    self.success_retrievability_weight = self.success_retrievability_weight.clamp(0.1, 4.0);
    self.failure_base = self.failure_base.clamp(0.01, 2.0);
    self.failure_difficulty_power = self.failure_difficulty_power.clamp(0.0, 1.5);
    self.failure_stability_power = self.failure_stability_power.clamp(0.1, 1.5);
    self.failure_retrievability_weight = self.failure_retrievability_weight.clamp(0.0, 3.0);

    self.hard_bonus = self.hard_bonus.clamp(0.05, 1.0);
    self.good_bonus = self.good_bonus.clamp((self.hard_bonus + 0.05).min(2.0), 2.0);
    self.easy_bonus = self.easy_bonus.clamp((self.good_bonus + 0.05).min(3.0), 3.0);

    self.initial_difficulty_again = self.initial_difficulty_again.clamp(2.0, 9.0);
    self.initial_difficulty_hard = self
      .initial_difficulty_hard
      .clamp(2.0, (self.initial_difficulty_again - 0.05).max(2.0));
    self.initial_difficulty_good = self
      .initial_difficulty_good
      .clamp(2.0, (self.initial_difficulty_hard - 0.05).max(2.0));
    self.initial_difficulty_easy = self
      .initial_difficulty_easy
      .clamp(2.0, (self.initial_difficulty_good - 0.05).max(2.0));

    self.initial_stability_again = self.initial_stability_again.clamp(MIN_STABILITY_DAYS, 0.5);
    self.initial_stability_hard = self.initial_stability_hard.clamp(self.initial_stability_again + 0.01, 1.0);
    self.initial_stability_good = self.initial_stability_good.clamp(self.initial_stability_hard + 0.01, 3.0);
    self.initial_stability_easy = self.initial_stability_easy.clamp(self.initial_stability_good + 0.01, 5.0);
    self
  }
}

fn clamp_difficulty(value: f64) -> f64 {
  value.clamp(0.0, 10.0)
}

fn clamp_stability(value: f64) -> f64 {
  value.max(MIN_STABILITY_DAYS)
}

fn round_days(value: f64) -> f64 {
  (value * 100.0).round() / 100.0
}

fn round_metric(value: f64) -> f64 {
  (value * 1000.0).round() / 1000.0
}

fn parse_utc(value: &str) -> Option<DateTime<Utc>> {
  DateTime::parse_from_rfc3339(value).ok().map(|timestamp| timestamp.with_timezone(&Utc))
}

fn duration_from_days(days: f64) -> Duration {
  let total_minutes = (days.max(MIN_STABILITY_DAYS) * 24.0 * 60.0).round() as i64;
  Duration::minutes(total_minutes.max(1))
}

fn format_due_at(reviewed_at: &DateTime<Utc>, interval_days: f64) -> String {
  (*reviewed_at + duration_from_days(interval_days)).to_rfc3339()
}

fn steps_in_days(steps_minutes: &[i64]) -> Vec<f64> {
  steps_minutes
    .iter()
    .map(|minutes| (*minutes as f64 / (24.0 * 60.0)).max(MIN_STABILITY_DAYS))
    .collect()
}

fn current_step_due(steps: &[f64], index: i64) -> f64 {
  let safe_index = index.clamp(0, (steps.len().saturating_sub(1)) as i64) as usize;
  steps.get(safe_index).copied().unwrap_or(MIN_STABILITY_DAYS)
}

pub fn retrievability_with_parameters(elapsed_days: f64, stability_days: f64, parameters: &SchedulerParameters) -> f64 {
  let elapsed = elapsed_days.max(0.0);
  let stability = clamp_stability(stability_days);
  (1.0 + elapsed / (parameters.decay_scale * stability))
    .powf(-parameters.decay_exponent)
    .clamp(0.0, 1.0)
}

pub fn retrievability(elapsed_days: f64, stability_days: f64) -> f64 {
  retrievability_with_parameters(elapsed_days, stability_days, &SchedulerParameters::default())
}

pub fn interval_from_stability_with_parameters(
  stability_days: f64,
  desired_retention: f64,
  parameters: &SchedulerParameters,
) -> f64 {
  let stability = clamp_stability(stability_days);
  let retention = desired_retention.clamp(0.85, 0.95);
  round_days(parameters.decay_scale * stability * (retention.powf(-1.0 / parameters.decay_exponent) - 1.0))
}

pub fn interval_from_stability(stability_days: f64, desired_retention: f64) -> f64 {
  interval_from_stability_with_parameters(stability_days, desired_retention, &SchedulerParameters::default())
}

fn rating_bonus(rating: ReviewRating, parameters: &SchedulerParameters) -> f64 {
  match rating {
    ReviewRating::Again => 0.0,
    ReviewRating::Hard => parameters.hard_bonus,
    ReviewRating::Good => parameters.good_bonus,
    ReviewRating::Easy => parameters.easy_bonus,
  }
}

fn update_difficulty(current_difficulty: f64, rating: ReviewRating, parameters: &SchedulerParameters) -> f64 {
  let shifted = current_difficulty - parameters.difficulty_delta * ((rating.score() - 3) as f64);
  let reverted = shifted * (1.0 - parameters.difficulty_mean_reversion) + parameters.difficulty_mean * parameters.difficulty_mean_reversion;
  round_metric(clamp_difficulty(reverted))
}

fn update_stability_success(
  difficulty: f64,
  stability: f64,
  retrievability_before: f64,
  rating: ReviewRating,
  parameters: &SchedulerParameters,
) -> f64 {
  let base_stability = clamp_stability(stability);
  let delta = parameters.success_base_log.exp()
    * (11.0 - clamp_difficulty(difficulty))
    * base_stability.powf(-parameters.success_stability_power)
    * ((parameters.success_retrievability_weight * (1.0 - retrievability_before)).exp() - 1.0)
    * rating_bonus(rating, parameters);
  round_metric(clamp_stability(base_stability * (1.0 + delta.max(0.02))))
}

fn update_stability_failure(
  difficulty: f64,
  stability: f64,
  retrievability_before: f64,
  parameters: &SchedulerParameters,
) -> f64 {
  let adjusted_difficulty = clamp_difficulty(difficulty).max(0.3);
  let base_stability = clamp_stability(stability);
  let reduced = parameters.failure_base
    * adjusted_difficulty.powf(-parameters.failure_difficulty_power)
    * ((base_stability + 1.0).powf(parameters.failure_stability_power) - 1.0)
    * (parameters.failure_retrievability_weight * (1.0 - retrievability_before)).exp();
  round_metric(clamp_stability(reduced))
}

fn derived_mastered(successful_reviews: i64, lapses: i64, stability_days: f64, leech: bool) -> bool {
  !leech
    && successful_reviews >= MASTERED_SUCCESS_REVIEWS
    && lapses <= successful_reviews / 2
    && stability_days >= MASTERED_STABILITY_DAYS
}

fn initial_difficulty_with_parameters(rating: ReviewRating, parameters: &SchedulerParameters) -> f64 {
  match rating {
    ReviewRating::Again => parameters.initial_difficulty_again,
    ReviewRating::Hard => parameters.initial_difficulty_hard,
    ReviewRating::Good => parameters.initial_difficulty_good,
    ReviewRating::Easy => parameters.initial_difficulty_easy,
  }
}

fn initial_stability_with_parameters(rating: ReviewRating, parameters: &SchedulerParameters) -> f64 {
  match rating {
    ReviewRating::Again => parameters.initial_stability_again,
    ReviewRating::Hard => parameters.initial_stability_hard,
    ReviewRating::Good => parameters.initial_stability_good,
    ReviewRating::Easy => parameters.initial_stability_easy,
  }
}

fn elapsed_days(unit: &ReviewUnitRecord, reviewed_at: &DateTime<Utc>) -> f64 {
  unit
    .last_reviewed_at_utc
    .as_deref()
    .and_then(parse_utc)
    .map(|last| ((*reviewed_at - last).num_seconds().max(0) as f64) / 86_400.0)
    .unwrap_or(0.0)
}

fn next_same_day_count(unit: &ReviewUnitRecord, reviewed_at: &DateTime<Utc>) -> i64 {
  let today = reviewed_at.date_naive();
  unit
    .last_reviewed_at_utc
    .as_deref()
    .and_then(parse_utc)
    .map(|last| {
      if last.date_naive() == today {
        unit.same_day_reviews_count + 1
      } else {
        1
      }
    })
    .unwrap_or(1)
}

fn next_average_latency(unit: &ReviewUnitRecord, latency_ms: Option<i64>) -> Option<f64> {
  latency_ms.map(|latency| match unit.average_latency_ms {
    Some(current) if unit.total_reviews > 0 => {
      ((current * unit.total_reviews as f64) + latency as f64) / (unit.total_reviews as f64 + 1.0)
    }
    _ => latency as f64,
  })
}

fn learning_state_after_failure(unit: &ReviewUnitRecord, next_lapses: i64, input: &SchedulerReviewInput) -> (ReviewUnitState, bool) {
  let becomes_leech = next_lapses >= input.leech_lapse_threshold || input.recent_again_count >= 2;
  if becomes_leech || unit.leech || matches!(unit.state, ReviewUnitState::Leech) {
    (ReviewUnitState::Leech, true)
  } else if matches!(unit.state, ReviewUnitState::Review | ReviewUnitState::Relearning) {
    (ReviewUnitState::Relearning, false)
  } else {
    (ReviewUnitState::Learning, false)
  }
}

fn build_step_update(
  unit: &ReviewUnitRecord,
  input: &SchedulerReviewInput,
  parameters: &SchedulerParameters,
  reviewed_at: &DateTime<Utc>,
  learning_steps: &[f64],
  relearning_steps: &[f64],
) -> ReviewUnitUpdate {
  let retrievability_before = if unit.total_reviews == 0 {
    1.0
  } else {
    retrievability_with_parameters(elapsed_days(unit, reviewed_at), unit.stability, parameters)
  };
  let last_reviewed_at_utc = reviewed_at.to_rfc3339();
  let average_latency_ms = next_average_latency(unit, input.latency_ms);
  let same_day_reviews_count = next_same_day_count(unit, reviewed_at);
  let total_reviews = unit.total_reviews + 1;
  let successful_reviews = unit.successful_reviews + if input.rating.is_success() { 1 } else { 0 };
  let failed_reviews = unit.failed_reviews + if input.rating.is_success() { 0 } else { 1 };
  let updated_at = last_reviewed_at_utc.clone();
  let first_reviewed_at_utc = unit.first_reviewed_at_utc.clone().or_else(|| Some(last_reviewed_at_utc.clone()));

  if !input.rating.is_success() {
    let next_lapses = unit.lapses + 1;
    let (state, leech) = learning_state_after_failure(unit, next_lapses, input);
    let due_days = current_step_due(relearning_steps, 0);
    return ReviewUnitUpdate {
      state,
      difficulty: update_difficulty(unit.difficulty.max(initial_difficulty_with_parameters(input.rating, parameters)), input.rating, parameters),
      stability: update_stability_failure(
        unit.difficulty.max(initial_difficulty_with_parameters(input.rating, parameters)),
        unit.stability.max(initial_stability_with_parameters(input.rating, parameters)),
        retrievability_before,
        parameters,
      ),
      scheduled_interval_days: round_days(due_days),
      last_reviewed_at_utc,
      due_at_utc: Some(format_due_at(reviewed_at, due_days)),
      lapses: next_lapses,
      successful_reviews,
      failed_reviews,
      total_reviews,
      same_day_reviews_count,
      average_latency_ms,
      last_latency_ms: input.latency_ms,
      hint_used_last: input.hint_used,
      confidence_last: input.confidence,
      suspended: false,
      leech,
      mastered: false,
      learning_step_index: 0,
      relearning_step_index: 0,
      first_reviewed_at_utc,
      graduated_at_utc: unit.graduated_at_utc.clone(),
      mastered_at_utc: None,
      updated_at,
      retrievability_before,
      newly_mastered: false,
    };
  }

  let current_steps = if matches!(unit.state, ReviewUnitState::Learning | ReviewUnitState::New) {
    learning_steps
  } else {
    relearning_steps
  };
  let current_index = if matches!(unit.state, ReviewUnitState::Learning | ReviewUnitState::New) {
    unit.learning_step_index
  } else {
    unit.relearning_step_index
  };
  let advance_by = if matches!(input.rating, ReviewRating::Easy) { 2 } else { 1 };
  let target_index = match input.rating {
    ReviewRating::Hard => current_index,
    _ => current_index + advance_by,
  };
  let current_due_days = current_step_due(current_steps, current_index);
  let seeded_stability = unit
    .stability
    .max(current_due_days)
    .max(initial_stability_with_parameters(input.rating, parameters));
  let difficulty = update_difficulty(
    unit.difficulty.max(initial_difficulty_with_parameters(input.rating, parameters)),
    input.rating,
    parameters,
  );
  let stability = update_stability_success(
    difficulty,
    seeded_stability,
    retrievability_before,
    input.rating,
    parameters,
  );

  if (target_index as usize) >= current_steps.len() {
    let graduated_at_utc = unit.graduated_at_utc.clone().or_else(|| Some(last_reviewed_at_utc.clone()));
    let scheduled_interval_days = interval_from_stability_with_parameters(
      stability.max(current_steps.last().copied().unwrap_or(stability)),
      input.desired_retention,
      parameters,
    );
    let mastered = derived_mastered(successful_reviews, unit.lapses, stability, false);
    return ReviewUnitUpdate {
      state: ReviewUnitState::Review,
      difficulty,
      stability,
      scheduled_interval_days,
      last_reviewed_at_utc: last_reviewed_at_utc.clone(),
      due_at_utc: Some(format_due_at(reviewed_at, scheduled_interval_days)),
      lapses: unit.lapses,
      successful_reviews,
      failed_reviews,
      total_reviews,
      same_day_reviews_count,
      average_latency_ms,
      last_latency_ms: input.latency_ms,
      hint_used_last: input.hint_used,
      confidence_last: input.confidence,
      suspended: false,
      leech: false,
      mastered,
      learning_step_index: unit.learning_step_index,
      relearning_step_index: unit.relearning_step_index,
      first_reviewed_at_utc,
      graduated_at_utc,
      mastered_at_utc: if mastered {
        unit.mastered_at_utc.clone().or_else(|| Some(last_reviewed_at_utc.clone()))
      } else {
        None
      },
      updated_at,
      retrievability_before,
      newly_mastered: mastered && !unit.mastered,
    };
  }

  let scheduled_interval_days = current_step_due(current_steps, target_index);
  let state = if matches!(unit.state, ReviewUnitState::Learning | ReviewUnitState::New) {
    ReviewUnitState::Learning
  } else if matches!(unit.state, ReviewUnitState::Leech) {
    ReviewUnitState::Leech
  } else {
    ReviewUnitState::Relearning
  };
  ReviewUnitUpdate {
    state,
    difficulty,
    stability,
    scheduled_interval_days: round_days(scheduled_interval_days),
    last_reviewed_at_utc: last_reviewed_at_utc.clone(),
    due_at_utc: Some(format_due_at(reviewed_at, scheduled_interval_days)),
    lapses: unit.lapses,
    successful_reviews,
    failed_reviews,
    total_reviews,
    same_day_reviews_count,
    average_latency_ms,
    last_latency_ms: input.latency_ms,
    hint_used_last: input.hint_used,
    confidence_last: input.confidence,
    suspended: false,
    leech: matches!(state, ReviewUnitState::Leech),
    mastered: false,
    learning_step_index: if matches!(state, ReviewUnitState::Learning) { target_index } else { unit.learning_step_index },
    relearning_step_index: if matches!(state, ReviewUnitState::Learning) { unit.relearning_step_index } else { target_index },
    first_reviewed_at_utc,
    graduated_at_utc: unit.graduated_at_utc.clone(),
    mastered_at_utc: None,
    updated_at,
    retrievability_before,
    newly_mastered: false,
  }
}

fn build_review_update(
  unit: &ReviewUnitRecord,
  input: &SchedulerReviewInput,
  parameters: &SchedulerParameters,
  reviewed_at: &DateTime<Utc>,
  relearning_steps: &[f64],
) -> ReviewUnitUpdate {
  let retrievability_before = retrievability_with_parameters(elapsed_days(unit, reviewed_at), unit.stability, parameters);
  let last_reviewed_at_utc = reviewed_at.to_rfc3339();
  let average_latency_ms = next_average_latency(unit, input.latency_ms);
  let same_day_reviews_count = next_same_day_count(unit, reviewed_at);
  let total_reviews = unit.total_reviews + 1;
  let updated_at = last_reviewed_at_utc.clone();
  let first_reviewed_at_utc = unit.first_reviewed_at_utc.clone().or_else(|| Some(last_reviewed_at_utc.clone()));

  if matches!(input.rating, ReviewRating::Again) {
    let difficulty = update_difficulty(unit.difficulty, input.rating, parameters);
    let stability = update_stability_failure(difficulty, unit.stability, retrievability_before, parameters);
    let lapses = unit.lapses + 1;
    let failed_reviews = unit.failed_reviews + 1;
    let (state, leech) = learning_state_after_failure(unit, lapses, input);
    let scheduled_interval_days = current_step_due(relearning_steps, 0);
    return ReviewUnitUpdate {
      state,
      difficulty,
      stability,
      scheduled_interval_days: round_days(scheduled_interval_days),
      last_reviewed_at_utc: last_reviewed_at_utc.clone(),
      due_at_utc: Some(format_due_at(reviewed_at, scheduled_interval_days)),
      lapses,
      successful_reviews: unit.successful_reviews,
      failed_reviews,
      total_reviews,
      same_day_reviews_count,
      average_latency_ms,
      last_latency_ms: input.latency_ms,
      hint_used_last: input.hint_used,
      confidence_last: input.confidence,
      suspended: false,
      leech,
      mastered: false,
      learning_step_index: unit.learning_step_index,
      relearning_step_index: 0,
      first_reviewed_at_utc,
      graduated_at_utc: unit.graduated_at_utc.clone(),
      mastered_at_utc: None,
      updated_at,
      retrievability_before,
      newly_mastered: false,
    };
  }

  let difficulty = update_difficulty(unit.difficulty, input.rating, parameters);
  let stability = update_stability_success(difficulty, unit.stability, retrievability_before, input.rating, parameters);
  let scheduled_interval_days = interval_from_stability_with_parameters(stability, input.desired_retention, parameters);
  let successful_reviews = unit.successful_reviews + 1;
  let leech = unit.leech && stability < 7.0;
  let mastered = derived_mastered(successful_reviews, unit.lapses, stability, leech);
  ReviewUnitUpdate {
    state: ReviewUnitState::Review,
    difficulty,
    stability,
    scheduled_interval_days,
    last_reviewed_at_utc: last_reviewed_at_utc.clone(),
    due_at_utc: Some(format_due_at(reviewed_at, scheduled_interval_days)),
    lapses: unit.lapses,
    successful_reviews,
    failed_reviews: unit.failed_reviews,
    total_reviews,
    same_day_reviews_count,
    average_latency_ms,
    last_latency_ms: input.latency_ms,
    hint_used_last: input.hint_used,
    confidence_last: input.confidence,
    suspended: false,
    leech,
    mastered,
    learning_step_index: unit.learning_step_index,
    relearning_step_index: unit.relearning_step_index,
    first_reviewed_at_utc,
    graduated_at_utc: unit.graduated_at_utc.clone().or_else(|| Some(last_reviewed_at_utc.clone())),
    mastered_at_utc: if mastered {
      unit.mastered_at_utc.clone().or_else(|| Some(last_reviewed_at_utc.clone()))
    } else {
      None
    },
    updated_at,
    retrievability_before,
    newly_mastered: mastered && !unit.mastered,
  }
}

pub fn schedule_review_with_parameters(
  unit: &ReviewUnitRecord,
  input: &SchedulerReviewInput,
  parameters: &SchedulerParameters,
) -> ReviewUnitUpdate {
  let reviewed_at = parse_utc(&input.reviewed_at_utc).unwrap_or_else(Utc::now);
  let learning_steps = steps_in_days(&input.learning_steps_minutes);
  let relearning_steps = steps_in_days(&input.relearning_steps_minutes);

  match unit.state {
    ReviewUnitState::New | ReviewUnitState::Learning | ReviewUnitState::Relearning | ReviewUnitState::Leech => {
      build_step_update(unit, input, parameters, &reviewed_at, &learning_steps, &relearning_steps)
    }
    ReviewUnitState::Review => build_review_update(unit, input, parameters, &reviewed_at, &relearning_steps),
  }
}

pub fn schedule_review(unit: &ReviewUnitRecord, input: &SchedulerReviewInput) -> ReviewUnitUpdate {
  let parameters = SchedulerParameters::default();
  schedule_review_with_parameters(unit, input, &parameters)
}

#[cfg(test)]
mod tests {
  use chrono::Utc;

  use crate::models::types::{ReviewRating, ReviewUnitRecord, ReviewUnitState, SchedulerReviewInput};

  use super::{interval_from_stability, retrievability, schedule_review};

  fn base_unit(state: ReviewUnitState) -> ReviewUnitRecord {
    ReviewUnitRecord {
      id: 1,
      card_id: 1,
      deck_id: 1,
      prompt_field_id: 10,
      reveal_field_ids: vec![11],
      direction_key: "field:10|11".to_string(),
      state,
      difficulty: 5.0,
      stability: 1.0,
      scheduled_interval_days: 1.0,
      last_reviewed_at_utc: Some((Utc::now() - chrono::Duration::days(1)).to_rfc3339()),
      due_at_utc: Some(Utc::now().to_rfc3339()),
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
      leech: false,
      mastered: false,
      learning_step_index: 0,
      relearning_step_index: 0,
      first_reviewed_at_utc: None,
      graduated_at_utc: None,
      mastered_at_utc: None,
      created_at: Utc::now().to_rfc3339(),
      updated_at: Utc::now().to_rfc3339(),
    }
  }

  fn review_input(rating: ReviewRating) -> SchedulerReviewInput {
    SchedulerReviewInput {
      rating,
      reviewed_at_utc: Utc::now().to_rfc3339(),
      latency_ms: Some(1200),
      hint_used: false,
      confidence: None,
      desired_retention: 0.90,
      learning_steps_minutes: vec![10, 24 * 60, 3 * 24 * 60],
      relearning_steps_minutes: vec![10, 24 * 60],
      recent_again_count: 0,
      leech_lapse_threshold: 8,
    }
  }

  #[test]
  fn retrievability_decays_monotonically() {
    let first = retrievability(1.0, 3.0);
    let later = retrievability(5.0, 3.0);
    assert!(first > later);
    assert!(later > 0.0);
  }

  #[test]
  fn interval_grows_with_stability() {
    assert!(interval_from_stability(10.0, 0.90) > interval_from_stability(2.0, 0.90));
  }

  #[test]
  fn new_card_good_rating_enters_learning_steps() {
    let unit = base_unit(ReviewUnitState::New);
    let update = schedule_review(&unit, &review_input(ReviewRating::Good));
    assert_eq!(update.state, ReviewUnitState::Learning);
    assert!(update.scheduled_interval_days >= 1.0);
  }

  #[test]
  fn mature_again_moves_to_relearning() {
    let mut unit = base_unit(ReviewUnitState::Review);
    unit.successful_reviews = 5;
    unit.total_reviews = 5;
    unit.stability = 14.0;
    let update = schedule_review(&unit, &review_input(ReviewRating::Again));
    assert!(matches!(update.state, ReviewUnitState::Relearning | ReviewUnitState::Leech));
    assert!(update.stability < unit.stability);
  }

  #[test]
  fn easy_review_increases_stability() {
    let mut unit = base_unit(ReviewUnitState::Review);
    unit.successful_reviews = 8;
    unit.total_reviews = 8;
    unit.stability = 20.0;
    let update = schedule_review(&unit, &review_input(ReviewRating::Easy));
    assert!(update.stability > unit.stability);
    assert!(update.scheduled_interval_days > unit.scheduled_interval_days);
  }

  #[test]
  fn repeated_again_can_trigger_leech_rehab() {
    let mut unit = base_unit(ReviewUnitState::Review);
    unit.successful_reviews = 3;
    unit.failed_reviews = 2;
    unit.total_reviews = 5;
    unit.lapses = 7;
    let mut input = review_input(ReviewRating::Again);
    input.recent_again_count = 2;
    let update = schedule_review(&unit, &input);
    assert_eq!(update.state, ReviewUnitState::Leech);
    assert!(update.leech);
  }

  #[test]
  fn same_day_reviews_increment_counter() {
    let mut unit = base_unit(ReviewUnitState::Learning);
    unit.last_reviewed_at_utc = Some(Utc::now().to_rfc3339());
    unit.same_day_reviews_count = 1;
    let update = schedule_review(&unit, &review_input(ReviewRating::Hard));
    assert_eq!(update.same_day_reviews_count, 2);
  }
}
