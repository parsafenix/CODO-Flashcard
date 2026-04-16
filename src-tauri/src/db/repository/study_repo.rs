use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};

use crate::models::types::{
  CompleteStudySessionInput, ReviewRating, ReviewUnitRecord, ReviewUnitState, ReviewUnitUpdate, SessionRecord, SessionSummary,
  StudySessionOptions, UiLanguage,
};

use super::dynamic_repo;

fn now_utc() -> String {
  Utc::now().to_rfc3339()
}

pub fn start_session(connection: &Connection, options: &StudySessionOptions) -> Result<i64> {
  dynamic_repo::save_study_configuration(connection, options.deck_id, options.prompt_field_id, &options.reveal_field_ids)?;
  connection.execute(
    "INSERT INTO study_sessions (deck_id, started_at, mode, prompt_language, prompt_field_id, reveal_field_ids)
      VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    params![
      options.deck_id,
      now_utc(),
      options.mode.as_str(),
      format!("field:{}", options.prompt_field_id),
      options.prompt_field_id,
      dynamic_repo::serialize_reveal_field_ids(&options.reveal_field_ids),
    ],
  )?;
  Ok(connection.last_insert_rowid())
}

pub fn record_review_history(
  connection: &Connection,
  session_id: i64,
  review_unit: &ReviewUnitRecord,
  rating: ReviewRating,
  update: &ReviewUnitUpdate,
) -> Result<()> {
  let previous_status = compatibility_status(review_unit.state, review_unit.mastered);
  let new_status = compatibility_status(update.state, update.mastered);
  let previous_interval_minutes = (review_unit.scheduled_interval_days * 24.0 * 60.0).round() as i64;
  let new_interval_minutes = (update.scheduled_interval_days * 24.0 * 60.0).round() as i64;
  let previous_mastery = compatibility_mastery_score(review_unit);
  let new_mastery = compatibility_mastery_score_from_update(update);
  connection.execute(
    "INSERT INTO review_history (
      card_id,
      deck_id,
      session_id,
      reviewed_at,
      knew_it,
      previous_status,
      new_status,
      previous_interval_minutes,
      new_interval_minutes,
      previous_ease_factor,
      new_ease_factor,
      previous_mastery_score,
      new_mastery_score
    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
    params![
      review_unit.card_id,
      review_unit.deck_id,
      session_id,
      now_utc(),
      if rating.is_success() { 1 } else { 0 },
      previous_status.as_str(),
      new_status.as_str(),
      previous_interval_minutes,
      new_interval_minutes,
      compatibility_ease_factor(review_unit.difficulty),
      compatibility_ease_factor(update.difficulty),
      previous_mastery,
      new_mastery
    ],
  )?;
  Ok(())
}

fn session_suggestion(language: UiLanguage, remaining_due_cards: i64, wrong_count: i64, correct_count: i64) -> String {
  if remaining_due_cards > 0 {
    return match language {
      UiLanguage::Fa => "هنوز کارت‌های موعددار دارید. یک مرور کوتاه دیگر کمک می‌کند بهتر تثبیت شوند.".to_string(),
      _ => "You still have due cards waiting. A short follow-up round will help lock them in.".to_string(),
    };
  }

  if wrong_count > correct_count {
    return match language {
      UiLanguage::Fa => "یک استراحت کوتاه داشته باشید و بعد یک جلسه‌ی mixed دیگر بروید تا کارت‌های سخت‌تر بهتر جا بیفتند.".to_string(),
      _ => "Take a short break, then run a new mixed session to reinforce the harder cards.".to_string(),
    };
  }

  match language {
    UiLanguage::Fa => "جلسه‌ی خوبی بود. بعداً برای مرور زمان‌بندی‌شده‌ی بعدی برگردید.".to_string(),
    _ => "Nice session. Come back later for the next scheduled review window.".to_string(),
  }
}

pub fn complete_session(connection: &Connection, input: &CompleteStudySessionInput, language: UiLanguage) -> Result<SessionSummary> {
  let now = now_utc();
  let session = get_session_record(connection, input.session_id)?;
  let accuracy_percent = if input.studied_count == 0 {
    0
  } else {
    ((input.correct_count as f64 / input.studied_count as f64) * 100.0).round() as i64
  };

  connection.execute(
    "UPDATE study_sessions
      SET completed_at = ?1,
          studied_count = ?2,
          correct_count = ?3,
          wrong_count = ?4,
          newly_mastered_count = ?5,
          accuracy_percent = ?6
      WHERE id = ?7",
    params![
      now,
      input.studied_count,
      input.correct_count,
      input.wrong_count,
      input.newly_mastered_count,
      accuracy_percent,
      input.session_id
    ],
  )?;

  connection.execute(
    "UPDATE decks SET last_studied_at = ?1, updated_at = ?1 WHERE id = ?2",
    params![now, input.deck_id],
  )?;

  let remaining_due_cards = connection.query_row(
    "SELECT COUNT(*)
      FROM review_units
      WHERE deck_id = ?1
        AND prompt_field_id = ?2
        AND reveal_field_ids = ?3
        AND suspended = 0
        AND state != 'new'
        AND due_at_utc IS NOT NULL
        AND due_at_utc <= ?4",
    params![
      input.deck_id,
      session.prompt_field_id,
      dynamic_repo::serialize_reveal_field_ids(&session.reveal_field_ids),
      now
    ],
    |row| row.get::<_, i64>(0),
  )?;

  let suggestion = session_suggestion(language, remaining_due_cards, input.wrong_count, input.correct_count);

  Ok(SessionSummary {
    session_id: input.session_id,
    studied_count: input.studied_count,
    correct_count: input.correct_count,
    wrong_count: input.wrong_count,
    accuracy_percent,
    newly_mastered_count: input.newly_mastered_count,
    remaining_due_cards,
    suggestion,
  })
}

pub fn get_session_record(connection: &Connection, session_id: i64) -> Result<SessionRecord> {
  connection
    .query_row(
      "SELECT id, deck_id, prompt_field_id, reveal_field_ids FROM study_sessions WHERE id = ?1",
      params![session_id],
      |row| {
        Ok(SessionRecord {
          id: row.get("id")?,
          deck_id: row.get("deck_id")?,
          prompt_field_id: row.get("prompt_field_id")?,
          reveal_field_ids: dynamic_repo::parse_reveal_field_ids(&row.get::<_, String>("reveal_field_ids")?),
        })
      },
    )
    .optional()?
    .context("Study session not found")
}

fn compatibility_status(state: ReviewUnitState, mastered: bool) -> crate::models::types::CardStatus {
  if mastered {
    return crate::models::types::CardStatus::Mastered;
  }

  match state {
    ReviewUnitState::New => crate::models::types::CardStatus::New,
    ReviewUnitState::Review => crate::models::types::CardStatus::Review,
    ReviewUnitState::Learning | ReviewUnitState::Relearning | ReviewUnitState::Leech => crate::models::types::CardStatus::Learning,
  }
}

fn compatibility_ease_factor(difficulty: f64) -> f64 {
  (2.6 - ((difficulty - 5.0) * 0.18)).clamp(1.3, 3.0)
}

fn compatibility_mastery_score(unit: &ReviewUnitRecord) -> i64 {
  if unit.mastered {
    100
  } else {
    ((unit.stability.min(60.0) / 60.0) * 100.0).round() as i64
  }
}

fn compatibility_mastery_score_from_update(update: &ReviewUnitUpdate) -> i64 {
  if update.mastered {
    100
  } else {
    ((update.stability.min(60.0) / 60.0) * 100.0).round() as i64
  }
}
