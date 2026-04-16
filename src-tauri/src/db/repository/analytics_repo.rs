use std::collections::HashMap;

use anyhow::Result;
use chrono::{Days, NaiveDate, Utc};
use rusqlite::{params, Connection};

use crate::{
  models::types::{
    CardStatus, ContentQualityAnalytics, LearningBalance, LearningOutcomeAnalytics, OverviewMetrics, ProgressPoint,
    RetentionForecastPoint, SchedulerHealthAnalytics, WeakCardAnalytics, WeakCardPreviewField,
  },
  services::srs,
};

#[derive(Debug, Clone)]
pub struct DailyCoachDeckSignal {
  pub deck_id: i64,
  pub deck_name: String,
  pub last_studied_at: Option<String>,
  pub due_cards: i64,
  pub overdue_cards: i64,
  pub new_cards: i64,
  pub weak_direction_count: i64,
  pub upcoming_due_7d: i64,
}

fn now_utc() -> String {
  Utc::now().to_rfc3339()
}

fn round_percent(numerator: i64, denominator: i64) -> i64 {
  if denominator <= 0 {
    0
  } else {
    ((numerator as f64 / denominator as f64) * 100.0).round() as i64
  }
}

fn round_ratio(numerator: f64, denominator: f64) -> f64 {
  if denominator <= 0.0 {
    0.0
  } else {
    ((numerator / denominator) * 100.0).round() / 100.0
  }
}

fn round_float(value: f64) -> f64 {
  (value * 100.0).round() / 100.0
}

pub fn get_overview_metrics(connection: &Connection) -> Result<OverviewMetrics> {
  let now = now_utc();
  let (total_cards, new_cards, due_cards, mastered_cards) = connection.query_row(
    "SELECT
      COUNT(*) AS total_cards,
      COALESCE(SUM(CASE WHEN status = 'new' THEN 1 ELSE 0 END), 0) AS new_cards,
      COALESCE(SUM(CASE WHEN status != 'new' AND next_review_at IS NOT NULL AND next_review_at <= ?1 THEN 1 ELSE 0 END), 0) AS due_cards,
      COALESCE(SUM(CASE WHEN status = 'mastered' THEN 1 ELSE 0 END), 0) AS mastered_cards
    FROM cards",
    params![now],
    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?, row.get::<_, i64>(3)?)),
  )?;

  let (total_reviews_completed, total_correct_reviews) = connection.query_row(
    "SELECT COUNT(*) AS total_reviews_completed, COALESCE(SUM(knew_it), 0) AS total_correct_reviews FROM review_history",
    [],
    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
  )?;

  let review_accuracy_percent = round_percent(total_correct_reviews, total_reviews_completed);

  Ok(OverviewMetrics {
    total_cards,
    new_cards,
    due_cards,
    mastered_cards,
    total_reviews_completed,
    review_accuracy_percent,
    retention_score_percent: review_accuracy_percent,
  })
}

pub fn get_progress_points(connection: &Connection, period_days: i64) -> Result<Vec<ProgressPoint>> {
  let today = Utc::now().date_naive();
  let start = today.checked_sub_days(Days::new((period_days.saturating_sub(1)) as u64)).unwrap_or(today);
  let end_exclusive = today.checked_add_days(Days::new(1)).unwrap_or(today);

  let mut statement = connection.prepare(
    "SELECT
      substr(reviewed_at, 1, 10) AS utc_date,
      COUNT(*) AS reviews_completed,
      COALESCE(CAST(ROUND(SUM(knew_it) * 100.0 / COUNT(*)) AS INTEGER), 0) AS accuracy_percent,
      COALESCE(SUM(CASE WHEN previous_status = 'new' AND new_status != 'new' THEN 1 ELSE 0 END), 0) AS new_cards_learned
    FROM review_history
    WHERE reviewed_at >= ?1 AND reviewed_at < ?2
    GROUP BY substr(reviewed_at, 1, 10)
    ORDER BY utc_date ASC",
  )?;

  let rows = statement.query_map(params![start.to_string(), end_exclusive.to_string()], |row| {
    Ok(ProgressPoint {
      utc_date: row.get("utc_date")?,
      reviews_completed: row.get("reviews_completed")?,
      accuracy_percent: row.get("accuracy_percent")?,
      new_cards_learned: row.get("new_cards_learned")?,
    })
  })?;

  let mut by_date = HashMap::new();
  for row in rows {
    let point = row?;
    by_date.insert(point.utc_date.clone(), point);
  }

  let mut points = Vec::new();
  let mut current = start;
  while current < end_exclusive {
    let date_key = current.to_string();
    points.push(by_date.remove(&date_key).unwrap_or(ProgressPoint {
      utc_date: date_key,
      reviews_completed: 0,
      accuracy_percent: 0,
      new_cards_learned: 0,
    }));
    current = current.checked_add_days(Days::new(1)).unwrap_or(end_exclusive);
  }

  Ok(points)
}

pub fn get_learning_balance(connection: &Connection, period_days: i64) -> Result<LearningBalance> {
  let today = Utc::now().date_naive();
  let start = today.checked_sub_days(Days::new((period_days.saturating_sub(1)) as u64)).unwrap_or(today);
  let end_exclusive = today.checked_add_days(Days::new(1)).unwrap_or(today);

  let (new_card_reviews, review_card_reviews) = connection.query_row(
    "SELECT
      COALESCE(SUM(CASE WHEN previous_status = 'new' THEN 1 ELSE 0 END), 0) AS new_card_reviews,
      COALESCE(SUM(CASE WHEN previous_status != 'new' THEN 1 ELSE 0 END), 0) AS review_card_reviews
    FROM review_history
    WHERE reviewed_at >= ?1 AND reviewed_at < ?2",
    params![start.to_string(), end_exclusive.to_string()],
    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
  )?;

  let total = new_card_reviews + review_card_reviews;
  Ok(LearningBalance {
    new_card_reviews,
    review_card_reviews,
    new_card_percent: round_percent(new_card_reviews, total),
    review_card_percent: round_percent(review_card_reviews, total),
  })
}

pub fn get_review_dates(connection: &Connection) -> Result<Vec<NaiveDate>> {
  let mut statement = connection.prepare(
    "SELECT DISTINCT substr(reviewed_at, 1, 10) AS utc_date
     FROM review_history
     ORDER BY utc_date ASC",
  )?;

  let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
  let mut dates = Vec::new();
  for row in rows {
    let value = row?;
    if let Ok(date) = NaiveDate::parse_from_str(&value, "%Y-%m-%d") {
      dates.push(date);
    }
  }
  Ok(dates)
}

pub fn get_today_review_count(connection: &Connection) -> Result<i64> {
  let today = Utc::now().date_naive().to_string();
  connection
    .query_row(
      "SELECT COUNT(*) FROM review_history WHERE substr(reviewed_at, 1, 10) = ?1",
      params![today],
      |row| row.get::<_, i64>(0),
    )
    .map_err(Into::into)
}

pub fn get_daily_coach_signals(connection: &Connection) -> Result<Vec<DailyCoachDeckSignal>> {
  let now = now_utc();
  let mut statement = connection.prepare(
    "SELECT
      d.id,
      d.name,
      d.last_studied_at,
      COALESCE((
        SELECT COUNT(*)
        FROM review_units ru
        WHERE ru.deck_id = d.id
          AND ru.suspended = 0
          AND ru.state != 'new'
          AND ru.due_at_utc IS NOT NULL
          AND ru.due_at_utc <= ?1
      ), 0) AS due_cards,
      COALESCE((
        SELECT COUNT(*)
        FROM review_units ru
        WHERE ru.deck_id = d.id
          AND ru.suspended = 0
          AND ru.state != 'new'
          AND ru.due_at_utc IS NOT NULL
          AND ru.due_at_utc < datetime(?1, '-1 day')
      ), 0) AS overdue_cards,
      COALESCE((
        SELECT COUNT(*)
        FROM cards c
        WHERE c.deck_id = d.id
          AND c.status = 'new'
      ), 0) AS new_cards,
      COALESCE((
        SELECT COUNT(*)
        FROM review_units ru
        WHERE ru.deck_id = d.id
          AND ru.suspended = 0
          AND (
            ru.leech = 1
            OR ru.state IN ('relearning', 'leech')
            OR ru.failed_reviews >= 2
            OR (ru.total_reviews > 0 AND ru.stability < 3.0)
          )
      ), 0) AS weak_direction_count,
      COALESCE((
        SELECT COUNT(*)
        FROM review_units ru
        WHERE ru.deck_id = d.id
          AND ru.suspended = 0
          AND ru.state != 'new'
          AND ru.due_at_utc IS NOT NULL
          AND ru.due_at_utc > ?1
          AND ru.due_at_utc <= datetime(?1, '+7 day')
      ), 0) AS upcoming_due_7d
    FROM decks d
    ORDER BY d.name COLLATE NOCASE ASC",
  )?;

  let rows = statement.query_map(params![now], |row| {
    Ok(DailyCoachDeckSignal {
      deck_id: row.get(0)?,
      deck_name: row.get(1)?,
      last_studied_at: row.get(2)?,
      due_cards: row.get(3)?,
      overdue_cards: row.get(4)?,
      new_cards: row.get(5)?,
      weak_direction_count: row.get(6)?,
      upcoming_due_7d: row.get(7)?,
    })
  })?;

  Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn preview_field_is_context(language_code: Option<&str>, label: &str) -> bool {
  let code = language_code.unwrap_or_default().to_lowercase();
  let label = label.to_lowercase();
  ["example", "notes", "note", "definition", "context"]
    .iter()
    .any(|token| code.contains(token) || label.contains(token))
}

fn get_weak_card_preview_fields(connection: &Connection, card_id: i64) -> Result<Vec<WeakCardPreviewField>> {
  let mut statement = connection.prepare(
    "SELECT
      df.label,
      df.language_code,
      cv.raw_value AS value
    FROM card_values cv
    INNER JOIN deck_fields df ON df.id = cv.field_id
    WHERE cv.card_id = ?1
      AND df.active = 1
      AND TRIM(cv.raw_value) != ''
    ORDER BY df.order_index ASC",
  )?;

  let rows = statement.query_map(params![card_id], |row| {
    let label = row.get::<_, String>("label")?;
    let language_code = row.get::<_, Option<String>>("language_code")?;
    let value = row.get::<_, String>("value")?;
    Ok(WeakCardPreviewField {
      is_context: preview_field_is_context(language_code.as_deref(), &label),
      label,
      value,
    })
  })?;

  Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?.into_iter().take(4).collect())
}

pub fn get_weak_cards(connection: &Connection, limit: usize) -> Result<Vec<WeakCardAnalytics>> {
  let mut statement = connection.prepare(
    "SELECT
      ru.id AS review_unit_id,
      c.id AS card_id,
      c.deck_id,
      d.name AS deck_name,
      c.language_1,
      c.language_2,
      c.language_3,
      c.status,
      ru.state AS review_state,
      ru.failed_reviews,
      ru.lapses,
      ru.difficulty,
      ru.stability,
      ru.leech,
      COALESCE((
        SELECT CAST(ROUND(AVG(CASE WHEN recent.was_correct = 1 THEN 100.0 ELSE 0 END)) AS INTEGER)
        FROM (
          SELECT was_correct
          FROM review_logs rl2
          WHERE rl2.review_unit_id = ru.id
          ORDER BY reviewed_at_utc DESC
          LIMIT 10
        ) recent
      ), 0) AS recent_success_rate_percent
    FROM review_units ru
    INNER JOIN cards c ON c.id = ru.card_id
    INNER JOIN decks d ON d.id = ru.deck_id
    WHERE ru.total_reviews > 0 OR ru.lapses > 0 OR ru.state IN ('learning', 'relearning', 'leech') OR ru.leech = 1",
  )?;

  let rows = statement.query_map([], |row| {
    let stability = row.get::<_, f64>("stability")?;
    Ok(WeakCardAnalytics {
      review_unit_id: row.get("review_unit_id")?,
      card_id: row.get("card_id")?,
      deck_id: row.get("deck_id")?,
      deck_name: row.get("deck_name")?,
      language_1: row.get("language_1")?,
      language_2: row.get("language_2")?,
      language_3: row.get("language_3")?,
      preview_fields: Vec::new(),
      status: CardStatus::from_db(&row.get::<_, String>("status")?),
      review_state: crate::models::types::ReviewUnitState::from_db(&row.get::<_, String>("review_state")?),
      wrong_count: row.get("failed_reviews")?,
      mastery_score: ((stability.min(60.0) / 60.0) * 100.0).round() as i64,
      relearn_count: row.get("lapses")?,
      recent_success_rate_percent: row.get("recent_success_rate_percent")?,
      difficulty_score: 0,
      difficulty: row.get("difficulty")?,
      stability_days: stability,
      leech: row.get::<_, i64>("leech")? != 0,
      needs_attention: false,
    })
  })?;

  let mut cards = rows
    .collect::<rusqlite::Result<Vec<_>>>()?
    .into_iter()
    .map(|mut card| {
      let low_mastery_penalty = (100 - card.mastery_score).clamp(0, 100);
      let low_recent_success_penalty = (100 - card.recent_success_rate_percent).clamp(0, 100);
      let stability_penalty = (20.0 - card.stability_days.min(20.0)).round() as i64;
      let leech_penalty = if card.leech { 28 } else { 0 };
      card.difficulty_score = (card.difficulty * 10.0).round() as i64
        + card.wrong_count * 12
        + card.relearn_count * 18
        + low_mastery_penalty
        + (low_recent_success_penalty / 2)
        + stability_penalty
        + leech_penalty;
      card.needs_attention =
        card.wrong_count >= 2 || card.mastery_score < 50 || card.relearn_count >= 2 || card.recent_success_rate_percent < 60 || card.leech;
      card
    })
    .filter(|card| card.difficulty_score > 0)
    .collect::<Vec<_>>();

  cards.sort_by(|left, right| {
    right
      .difficulty_score
      .cmp(&left.difficulty_score)
      .then(right.wrong_count.cmp(&left.wrong_count))
      .then_with(|| right.difficulty.total_cmp(&left.difficulty))
  });
  cards.truncate(limit);
  for card in &mut cards {
    card.preview_fields = get_weak_card_preview_fields(connection, card.card_id)?;
  }
  Ok(cards)
}

fn average_direction_accuracy(connection: &Connection, recognition: bool) -> Result<i64> {
  let mut statement = connection.prepare(
    "SELECT
      rl.was_correct,
      prompt.label,
      prompt.language_code
     FROM review_logs rl
     INNER JOIN review_units ru ON ru.id = rl.review_unit_id
     INNER JOIN deck_fields prompt ON prompt.id = ru.prompt_field_id",
  )?;
  let rows = statement.query_map([], |row| {
    Ok((
      row.get::<_, i64>(0)?,
      row.get::<_, String>(1)?,
      row.get::<_, Option<String>>(2)?,
    ))
  })?;

  let mut total = 0_i64;
  let mut correct = 0_i64;
  for row in rows {
    let (was_correct, label, language_code) = row?;
    let prompt_is_recognition = preview_field_is_context(language_code.as_deref(), &label);
    if prompt_is_recognition == recognition {
      total += 1;
      correct += was_correct;
    }
  }

  Ok(round_percent(correct, total))
}

pub fn get_learning_outcomes(connection: &Connection) -> Result<LearningOutcomeAnalytics> {
  let first_pass_success_rate_percent = connection.query_row(
    "SELECT
      COALESCE(CAST(ROUND(AVG(CASE WHEN rating != 'again' THEN 100.0 ELSE 0 END)) AS INTEGER), 0)
     FROM review_logs rl
     WHERE rl.id IN (
       SELECT MIN(id) FROM review_logs GROUP BY review_unit_id
     )",
    [],
    |row| row.get::<_, i64>(0),
  )?;

  let today = Utc::now().date_naive();
  let retention_window = |days: i64| -> Result<i64> {
    let start = today.checked_sub_days(Days::new(days as u64)).unwrap_or(today);
    connection
      .query_row(
        "SELECT
          COALESCE(CAST(ROUND(AVG(CASE WHEN was_correct = 1 THEN 100.0 ELSE 0 END)) AS INTEGER), 0)
         FROM review_logs
         WHERE state_before = 'review'
           AND reviewed_at_utc >= ?1",
        params![start.to_string()],
        |row| row.get::<_, i64>(0),
      )
      .map_err(Into::into)
  };

  let average_time_to_graduation_days = connection.query_row(
    "SELECT COALESCE(AVG(julianday(graduated_at_utc) - julianday(first_reviewed_at_utc)), 0.0)
     FROM review_units
     WHERE graduated_at_utc IS NOT NULL AND first_reviewed_at_utc IS NOT NULL",
    [],
    |row| row.get::<_, f64>(0),
  )?;
  let average_time_to_mastery_days = connection.query_row(
    "SELECT COALESCE(AVG(julianday(mastered_at_utc) - julianday(first_reviewed_at_utc)), 0.0)
     FROM review_units
     WHERE mastered_at_utc IS NOT NULL AND first_reviewed_at_utc IS NOT NULL",
    [],
    |row| row.get::<_, f64>(0),
  )?;
  let (total_logs, again_logs) = connection.query_row(
    "SELECT COUNT(*), COALESCE(SUM(CASE WHEN rating = 'again' THEN 1 ELSE 0 END), 0) FROM review_logs",
    [],
    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
  )?;
  let (total_units, leech_units, retained_units) = connection.query_row(
    "SELECT
       COUNT(*),
       COALESCE(SUM(CASE WHEN leech = 1 OR state = 'leech' THEN 1 ELSE 0 END), 0),
       COALESCE(SUM(CASE WHEN stability >= 7.0 THEN 1 ELSE 0 END), 0)
     FROM review_units",
    [],
    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?)),
  )?;

  Ok(LearningOutcomeAnalytics {
    first_pass_success_rate_percent,
    recognition_accuracy_percent: average_direction_accuracy(connection, true)?,
    production_accuracy_percent: average_direction_accuracy(connection, false)?,
    retention_7d_percent: retention_window(7)?,
    retention_30d_percent: retention_window(30)?,
    average_time_to_graduation_days: round_float(average_time_to_graduation_days),
    average_time_to_mastery_days: round_float(average_time_to_mastery_days),
    lapse_rate_percent: round_percent(again_logs, total_logs),
    leech_rate_percent: round_percent(leech_units, total_units),
    review_burden_per_retained_item: round_ratio(total_logs as f64, retained_units.max(1) as f64),
  })
}

pub fn get_scheduler_health(
  connection: &Connection,
  desired_retention: f64,
  parameters: &crate::services::srs::SchedulerParameters,
) -> Result<SchedulerHealthAnalytics> {
  let (predicted_recall_percent, actual_recall_percent, successful_growth_percent, review_lapse_rate_percent) = connection.query_row(
    "SELECT
      COALESCE(CAST(ROUND(AVG(retrievability_before) * 100.0) AS INTEGER), 0),
      COALESCE(CAST(ROUND(AVG(CASE WHEN was_correct = 1 THEN 100.0 ELSE 0 END)) AS INTEGER), 0),
      COALESCE(CAST(ROUND(AVG(
        CASE
          WHEN was_correct = 1 AND stability_before > 0 THEN ((stability_after - stability_before) / stability_before) * 100.0
          ELSE NULL
        END
      )) AS INTEGER), 0),
      COALESCE(CAST(ROUND(AVG(
        CASE
          WHEN state_before = 'review' THEN CASE WHEN rating = 'again' THEN 100.0 ELSE 0 END
          ELSE NULL
        END
      )) AS INTEGER), 0)
      FROM review_logs",
    [],
    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?, row.get::<_, i64>(3)?)),
  )?;

  let (average_stability_days, average_difficulty) = connection.query_row(
    "SELECT COALESCE(AVG(stability), 0.0), COALESCE(AVG(difficulty), 0.0)
     FROM review_units
     WHERE suspended = 0",
    [],
    |row| Ok((row.get::<_, f64>(0)?, row.get::<_, f64>(1)?)),
  )?;

  let now = now_utc();
  let (due_forecast_7d, due_forecast_30d) = connection.query_row(
    "SELECT
      COALESCE(SUM(CASE WHEN due_at_utc IS NOT NULL AND due_at_utc > ?1 AND due_at_utc <= datetime(?1, '+7 day') THEN 1 ELSE 0 END), 0),
      COALESCE(SUM(CASE WHEN due_at_utc IS NOT NULL AND due_at_utc > ?1 AND due_at_utc <= datetime(?1, '+30 day') THEN 1 ELSE 0 END), 0)
     FROM review_units
     WHERE suspended = 0 AND state != 'new'",
    params![now],
    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
  )?;

  let (on_time_success_percent, overdue_success_percent) = connection.query_row(
    "SELECT
      COALESCE(CAST(ROUND(AVG(
        CASE
          WHEN scheduled_due_before_utc IS NOT NULL AND reviewed_at_utc <= scheduled_due_before_utc THEN CASE WHEN was_correct = 1 THEN 100.0 ELSE 0 END
          ELSE NULL
        END
      )) AS INTEGER), 0),
      COALESCE(CAST(ROUND(AVG(
        CASE
          WHEN scheduled_due_before_utc IS NOT NULL AND reviewed_at_utc > scheduled_due_before_utc THEN CASE WHEN was_correct = 1 THEN 100.0 ELSE 0 END
          ELSE NULL
        END
      )) AS INTEGER), 0)
     FROM review_logs",
    [],
    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
  )?;

  let mut retention_sensitivity = Vec::new();
  for target in [0.85_f64, desired_retention.clamp(0.85, 0.95), 0.95_f64] {
    let mut statement = connection.prepare("SELECT stability FROM review_units WHERE suspended = 0 AND state != 'new'")?;
    let rows = statement.query_map([], |row| row.get::<_, f64>(0))?;
    let estimated_due_next_30_days = rows
      .collect::<rusqlite::Result<Vec<_>>>()?
      .into_iter()
      .filter(|stability| srs::interval_from_stability_with_parameters(*stability, target, parameters) <= 30.0)
      .count() as i64;
    retention_sensitivity.push(RetentionForecastPoint {
      desired_retention: target,
      estimated_due_next_30_days,
    });
  }

  Ok(SchedulerHealthAnalytics {
    predicted_recall_percent,
    actual_recall_percent,
    average_stability_days: round_float(average_stability_days),
    average_difficulty: round_float(average_difficulty),
    successful_stability_growth_percent: successful_growth_percent,
    review_lapse_rate_percent,
    overdue_success_percent,
    on_time_success_percent,
    due_forecast_7d,
    due_forecast_30d,
    workload_forecast_per_day_7d: round_ratio(due_forecast_7d as f64, 7.0),
    workload_forecast_per_day_30d: round_ratio(due_forecast_30d as f64, 30.0),
    retention_sensitivity,
  })
}

pub fn get_content_quality(connection: &Connection, weak_cards: &[WeakCardAnalytics]) -> Result<ContentQualityAnalytics> {
  let repeated_again_count = connection.query_row(
    "SELECT COUNT(*)
     FROM review_units ru
     WHERE (
       SELECT COUNT(*)
       FROM (
         SELECT rating
         FROM review_logs rl
         WHERE rl.review_unit_id = ru.id
         ORDER BY reviewed_at_utc DESC
         LIMIT 6
       ) recent
       WHERE rating = 'again'
     ) >= 3",
    [],
    |row| row.get::<_, i64>(0),
  )?;
  let leech_count = connection.query_row(
    "SELECT COUNT(*) FROM review_units WHERE leech = 1 OR state = 'leech'",
    [],
    |row| row.get::<_, i64>(0),
  )?;
  let contextual_support_count = connection.query_row(
    "SELECT COUNT(DISTINCT ru.id)
     FROM review_units ru
     WHERE EXISTS (
       SELECT 1
       FROM json_each(ru.reveal_field_ids) rid
       INNER JOIN deck_fields df ON df.id = rid.value
       WHERE LOWER(IFNULL(df.language_code, '')) LIKE '%example%'
          OR LOWER(IFNULL(df.language_code, '')) LIKE '%note%'
          OR LOWER(IFNULL(df.language_code, '')) LIKE '%definition%'
          OR LOWER(df.label) LIKE '%example%'
          OR LOWER(df.label) LIKE '%note%'
          OR LOWER(df.label) LIKE '%definition%'
     )",
    [],
    |row| row.get::<_, i64>(0),
  )?;

  Ok(ContentQualityAnalytics {
    hardest_direction_count: weak_cards.len() as i64,
    repeated_again_count,
    leech_count,
    contextual_support_count,
  })
}

#[cfg(test)]
mod tests {
  use std::time::{SystemTime, UNIX_EPOCH};

  use chrono::{Duration, Utc};
  use rusqlite::{params, Connection};

  use crate::db::{initialize_database, open_connection, repository::deck_repo};
  use crate::models::types::{CardValueInput, CreateCardInput, CreateDeckInput, DeckFieldInput};

  use super::{
    get_content_quality, get_learning_balance, get_learning_outcomes, get_overview_metrics, get_progress_points, get_review_dates,
    get_scheduler_health, get_today_review_count, get_weak_cards,
  };

  fn temp_db_path() -> std::path::PathBuf {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    std::env::temp_dir().join(format!("flashcard-local-analytics-{unique}.sqlite"))
  }

  fn seed_review_history(connection: &Connection, card_id: i64, deck_id: i64, days_ago: i64, knew_it: bool, previous_status: &str, new_status: &str) {
    let reviewed_at = (Utc::now() - Duration::days(days_ago)).to_rfc3339();
    connection.execute(
      "INSERT INTO review_history (
        card_id, deck_id, session_id, reviewed_at, knew_it, previous_status, new_status,
        previous_interval_minutes, new_interval_minutes, previous_ease_factor, new_ease_factor,
        previous_mastery_score, new_mastery_score
      ) VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?6, 10, 10, 2.2, 2.2, 40, 60)",
      params![card_id, deck_id, reviewed_at, if knew_it { 1 } else { 0 }, previous_status, new_status],
    ).unwrap();
  }

  #[test]
  fn analytics_queries_cover_overview_progress_and_scheduler_health() {
    let db_path = temp_db_path();
    initialize_database(&db_path).unwrap();
    let connection = open_connection(&db_path).unwrap();

    let deck = deck_repo::create_deck(
      &connection,
      &CreateDeckInput {
        name: "Analytics".into(),
        description: None,
        fields: vec![
          DeckFieldInput {
            id: None,
            label: "Persian".into(),
            language_code: Some("persian".into()),
            order_index: 0,
            required: true,
            active: true,
            field_type: Some("text".into()),
          },
          DeckFieldInput {
            id: None,
            label: "English".into(),
            language_code: Some("english".into()),
            order_index: 1,
            required: true,
            active: true,
            field_type: Some("text".into()),
          },
        ],
      },
    )
    .unwrap();

    let card = crate::db::repository::card_repo::create_card(
      &connection,
      &CreateCardInput {
        deck_id: deck.id,
        values: vec![
          CardValueInput {
            field_id: deck.fields[0].id,
            value: "سلام".into(),
          },
          CardValueInput {
            field_id: deck.fields[1].id,
            value: "Hello".into(),
          },
        ],
      },
    )
    .unwrap();

    connection.execute(
      "INSERT INTO review_units (
        card_id, deck_id, prompt_field_id, reveal_field_ids, direction_key, state, difficulty, stability,
        scheduled_interval_days, due_at_utc, lapses, successful_reviews, failed_reviews, total_reviews,
        same_day_reviews_count, hint_used_last, suspended, leech, mastered, learning_step_index, relearning_step_index,
        created_at, updated_at
      ) VALUES (?1, ?2, ?3, '[2]', 'field:1|2', 'review', 6.0, 5.0, 5.0, ?4, 2, 3, 2, 5, 0, 0, 0, 1, 0, 0, 0, ?5, ?5)",
      params![card.id, deck.id, deck.fields[0].id, Utc::now().to_rfc3339(), Utc::now().to_rfc3339()],
    ).unwrap();
    let review_unit_id = connection.last_insert_rowid();
    connection.execute(
      "INSERT INTO review_logs (
        review_unit_id, card_id, deck_id, session_id, reviewed_at_utc, rating, was_correct,
        state_before, state_after, retrievability_before, difficulty_before, difficulty_after,
        stability_before, stability_after, interval_before_days, interval_after_days,
        scheduled_due_before_utc, scheduled_due_after_utc, elapsed_days, latency_ms, hint_used, confidence, leech_before, leech_after
      ) VALUES (?1, ?2, ?3, NULL, ?4, 'again', 0, 'review', 'leech', 0.62, 6.0, 6.8, 5.0, 1.2, 5.0, 0.01, ?4, ?4, 5.0, 1200, 0, NULL, 0, 1)",
      params![review_unit_id, card.id, deck.id, Utc::now().to_rfc3339()],
    ).unwrap();

    seed_review_history(&connection, card.id, deck.id, 0, false, "review", "learning");

    let overview = get_overview_metrics(&connection).unwrap();
    assert_eq!(overview.total_cards, 1);

    let progress = get_progress_points(&connection, 7).unwrap();
    assert_eq!(progress.len(), 7);

    let balance = get_learning_balance(&connection, 7).unwrap();
    assert_eq!(balance.review_card_reviews, 1);

    let weak_cards = get_weak_cards(&connection, 5).unwrap();
    assert_eq!(weak_cards[0].card_id, card.id);
    assert!(weak_cards[0].needs_attention);

    let outcomes = get_learning_outcomes(&connection).unwrap();
    assert!(outcomes.lapse_rate_percent >= 0);

    let scheduler = get_scheduler_health(&connection, 0.90, &crate::services::srs::SchedulerParameters::default()).unwrap();
    assert!(scheduler.average_difficulty >= 0.0);

    let content = get_content_quality(&connection, &weak_cards).unwrap();
    assert!(content.leech_count >= 1);

    let review_dates = get_review_dates(&connection).unwrap();
    assert!(!review_dates.is_empty());
    assert_eq!(get_today_review_count(&connection).unwrap(), 1);

    let _ = std::fs::remove_file(db_path);
  }
}
