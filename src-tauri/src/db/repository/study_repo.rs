use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};

use crate::models::types::{CompleteStudySessionInput, GradeCardResponse, SessionRecord, SessionSummary, StudySessionOptions};

fn now_utc() -> String {
  Utc::now().to_rfc3339()
}

pub fn start_session(connection: &Connection, options: &StudySessionOptions) -> Result<i64> {
  connection.execute(
    "INSERT INTO study_sessions (deck_id, started_at, mode, prompt_language)
      VALUES (?1, ?2, ?3, ?4)",
    params![
      options.deck_id,
      now_utc(),
      options.mode.as_str(),
      options.prompt_language.as_db_field(),
    ],
  )?;
  Ok(connection.last_insert_rowid())
}

pub fn record_review_history(
  connection: &Connection,
  session_id: i64,
  response: &GradeCardResponse,
  card: &crate::models::types::CardSchedulingRecord,
  knew_it: bool,
) -> Result<()> {
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
      card.id,
      card.deck_id,
      session_id,
      now_utc(),
      if knew_it { 1 } else { 0 },
      card.status.as_str(),
      response.status.as_str(),
      card.current_interval_minutes,
      response.current_interval_minutes,
      card.ease_factor,
      response.ease_factor,
      card.mastery_score,
      response.mastery_score
    ],
  )?;
  Ok(())
}

pub fn apply_scheduling_update(
  connection: &Connection,
  card_id: i64,
  update: &crate::models::types::SchedulingUpdate,
) -> Result<()> {
  connection.execute(
    "UPDATE cards
      SET status = ?1,
          review_count = ?2,
          correct_count = ?3,
          wrong_count = ?4,
          current_interval_minutes = ?5,
          ease_factor = ?6,
          mastery_score = ?7,
          last_reviewed_at = ?8,
          next_review_at = ?9,
          updated_at = ?8
      WHERE id = ?10",
    params![
      update.status.as_str(),
      update.review_count,
      update.correct_count,
      update.wrong_count,
      update.current_interval_minutes,
      update.ease_factor,
      update.mastery_score,
      update.last_reviewed_at,
      update.next_review_at,
      card_id
    ],
  )?;
  Ok(())
}

pub fn complete_session(connection: &Connection, input: &CompleteStudySessionInput) -> Result<SessionSummary> {
  let now = now_utc();
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
    "SELECT COUNT(*) FROM cards
      WHERE deck_id = ?1 AND status != 'new' AND next_review_at IS NOT NULL AND next_review_at <= ?2",
    params![input.deck_id, now],
    |row| row.get::<_, i64>(0),
  )?;

  let suggestion = if remaining_due_cards > 0 {
    "You still have due cards waiting. A short follow-up round will help lock them in.".to_string()
  } else if input.wrong_count > input.correct_count {
    "Take a short break, then run a new mixed session to reinforce the harder cards.".to_string()
  } else {
    "Nice session. Come back later for the next scheduled review window.".to_string()
  };

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
      "SELECT id, deck_id FROM study_sessions WHERE id = ?1",
      params![session_id],
      |row| {
        Ok(SessionRecord {
          id: row.get("id")?,
          deck_id: row.get("deck_id")?,
        })
      },
    )
    .optional()?
    .context("Study session not found")
}
