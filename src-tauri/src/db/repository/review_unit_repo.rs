use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::{
  models::types::{CardStatus, ReviewRating, ReviewUnitRecord, ReviewUnitState, ReviewUnitUpdate, StudyCard, StudyMode},
};

use super::dynamic_repo;

fn now_utc() -> String {
  Utc::now().to_rfc3339()
}

fn bool_to_i64(value: bool) -> i64 {
  if value { 1 } else { 0 }
}

pub fn canonical_reveal_field_ids(reveal_field_ids: &[i64]) -> Vec<i64> {
  let mut ids = reveal_field_ids.to_vec();
  ids.sort_unstable();
  ids.dedup();
  ids
}

pub fn build_direction_key(prompt_field_id: i64, reveal_field_ids: &[i64]) -> String {
  let reveal = canonical_reveal_field_ids(reveal_field_ids)
    .iter()
    .map(|field_id| field_id.to_string())
    .collect::<Vec<_>>()
    .join(",");
  format!("field:{prompt_field_id}|{reveal}")
}

fn map_review_unit(row: &Row<'_>) -> rusqlite::Result<ReviewUnitRecord> {
  Ok(ReviewUnitRecord {
    id: row.get("id")?,
    card_id: row.get("card_id")?,
    deck_id: row.get("deck_id")?,
    prompt_field_id: row.get("prompt_field_id")?,
    reveal_field_ids: dynamic_repo::parse_reveal_field_ids(&row.get::<_, String>("reveal_field_ids")?),
    direction_key: row.get("direction_key")?,
    state: ReviewUnitState::from_db(&row.get::<_, String>("state")?),
    difficulty: row.get("difficulty")?,
    stability: row.get("stability")?,
    scheduled_interval_days: row.get("scheduled_interval_days")?,
    last_reviewed_at_utc: row.get("last_reviewed_at_utc")?,
    due_at_utc: row.get("due_at_utc")?,
    lapses: row.get("lapses")?,
    successful_reviews: row.get("successful_reviews")?,
    failed_reviews: row.get("failed_reviews")?,
    total_reviews: row.get("total_reviews")?,
    same_day_reviews_count: row.get("same_day_reviews_count")?,
    average_latency_ms: row.get("average_latency_ms")?,
    last_latency_ms: row.get("last_latency_ms")?,
    hint_used_last: row.get::<_, i64>("hint_used_last")? != 0,
    confidence_last: row.get("confidence_last")?,
    suspended: row.get::<_, i64>("suspended")? != 0,
    leech: row.get::<_, i64>("leech")? != 0,
    mastered: row.get::<_, i64>("mastered")? != 0,
    learning_step_index: row.get("learning_step_index")?,
    relearning_step_index: row.get("relearning_step_index")?,
    first_reviewed_at_utc: row.get("first_reviewed_at_utc")?,
    graduated_at_utc: row.get("graduated_at_utc")?,
    mastered_at_utc: row.get("mastered_at_utc")?,
    created_at: row.get("created_at")?,
    updated_at: row.get("updated_at")?,
  })
}

fn map_study_card(row: &Row<'_>) -> rusqlite::Result<StudyCard> {
  Ok(StudyCard {
    id: row.get("id")?,
    deck_id: row.get("deck_id")?,
    review_unit_id: row.get("review_unit_id")?,
    language_1: row.get("language_1")?,
    language_2: row.get("language_2")?,
    language_3: row.get("language_3")?,
    note: row.get("note")?,
    example_sentence: row.get("example_sentence")?,
    tag: row.get("tag")?,
    values: Vec::new(),
    status: CardStatus::from_db(&row.get::<_, String>("status")?),
    next_review_at: row.get("next_review_at")?,
    review_state: ReviewUnitState::from_db(&row.get::<_, String>("review_state")?),
    due_at_utc: row.get("due_at_utc")?,
    mastered: row.get::<_, i64>("mastered")? != 0,
    leech: row.get::<_, i64>("leech")? != 0,
    suspended: row.get::<_, i64>("suspended")? != 0,
    difficulty: row.get("difficulty")?,
    stability_days: row.get("stability")?,
  })
}

fn attach_study_values(connection: &Connection, mut cards: Vec<StudyCard>) -> Result<Vec<StudyCard>> {
  let card_ids = cards.iter().map(|card| card.id).collect::<Vec<_>>();
  let by_card = dynamic_repo::get_card_values_for_cards(connection, &card_ids)?;
  for card in &mut cards {
    card.values = by_card.get(&card.id).cloned().unwrap_or_default();
  }
  Ok(cards)
}

fn derive_legacy_bootstrap(
  connection: &Connection,
  card_id: i64,
) -> Result<(ReviewUnitState, f64, f64, f64, Option<String>, Option<String>, i64, i64, i64, bool)> {
  let data = connection
    .query_row(
      "SELECT status, current_interval_minutes, ease_factor, mastery_score, last_reviewed_at, next_review_at, review_count, correct_count, wrong_count
       FROM cards
       WHERE id = ?1",
      params![card_id],
      |row| {
        Ok((
          row.get::<_, String>(0)?,
          row.get::<_, i64>(1)?,
          row.get::<_, f64>(2)?,
          row.get::<_, i64>(3)?,
          row.get::<_, Option<String>>(4)?,
          row.get::<_, Option<String>>(5)?,
          row.get::<_, i64>(6)?,
          row.get::<_, i64>(7)?,
          row.get::<_, i64>(8)?,
        ))
      },
    )
    .optional()?
    .context("Card not found while bootstrapping review unit")?;

  let state = match data.0.as_str() {
    "learning" => ReviewUnitState::Learning,
    "review" => ReviewUnitState::Review,
    "mastered" => ReviewUnitState::Review,
    _ => ReviewUnitState::New,
  };
  let interval_days = (data.1.max(0) as f64 / (24.0 * 60.0)).max(if matches!(state, ReviewUnitState::New) { 0.0 } else { 0.2 });
  let difficulty = (5.0 - ((data.2 - 2.2) * 2.0)).clamp(0.0, 10.0);
  let stability = if interval_days > 0.0 {
    interval_days
  } else if matches!(state, ReviewUnitState::Learning) {
    0.2
  } else {
    0.1
  };
  let mastered = data.3 >= 85 || data.0 == "mastered";
  Ok((state, difficulty, stability, interval_days, data.4, data.5, data.8, data.7, data.6, mastered))
}

pub fn get_review_unit(connection: &Connection, review_unit_id: i64) -> Result<Option<ReviewUnitRecord>> {
  connection
    .query_row(
      "SELECT * FROM review_units WHERE id = ?1",
      params![review_unit_id],
      map_review_unit,
    )
    .optional()
    .map_err(Into::into)
}

pub fn ensure_review_unit(
  connection: &Connection,
  card_id: i64,
  deck_id: i64,
  prompt_field_id: i64,
  reveal_field_ids: &[i64],
) -> Result<ReviewUnitRecord> {
  let reveal_field_ids = canonical_reveal_field_ids(reveal_field_ids);
  let direction_key = build_direction_key(prompt_field_id, &reveal_field_ids);
  if let Some(existing) = connection
    .query_row(
      "SELECT * FROM review_units WHERE card_id = ?1 AND prompt_field_id = ?2 AND direction_key = ?3",
      params![card_id, prompt_field_id, direction_key],
      map_review_unit,
    )
    .optional()?
  {
    return Ok(existing);
  }

  let review_unit_count = connection.query_row(
    "SELECT COUNT(*) FROM review_units WHERE card_id = ?1",
    params![card_id],
    |row| row.get::<_, i64>(0),
  )?;
  let now = now_utc();
  let (state, difficulty, stability, scheduled_interval_days, last_reviewed_at_utc, due_at_utc, lapses, successful_reviews, total_reviews, mastered) =
    if review_unit_count == 0 {
      derive_legacy_bootstrap(connection, card_id)?
    } else {
      (ReviewUnitState::New, 5.0, 0.2, 0.0, None, None, 0, 0, 0, false)
    };
  let failed_reviews = (total_reviews - successful_reviews).max(0);

  connection.execute(
    "INSERT INTO review_units (
      card_id,
      deck_id,
      prompt_field_id,
      reveal_field_ids,
      direction_key,
      state,
      difficulty,
      stability,
      scheduled_interval_days,
      last_reviewed_at_utc,
      due_at_utc,
      lapses,
      successful_reviews,
      failed_reviews,
      total_reviews,
      same_day_reviews_count,
      hint_used_last,
      confidence_last,
      suspended,
      leech,
      mastered,
      learning_step_index,
      relearning_step_index,
      created_at,
      updated_at
    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, 0, 0, NULL, 0, 0, ?16, 0, 0, ?17, ?17)",
    params![
      card_id,
      deck_id,
      prompt_field_id,
      dynamic_repo::serialize_reveal_field_ids(&reveal_field_ids),
      direction_key,
      state.as_str(),
      difficulty,
      stability,
      scheduled_interval_days,
      last_reviewed_at_utc,
      due_at_utc,
      lapses,
      successful_reviews,
      failed_reviews,
      total_reviews,
      bool_to_i64(mastered),
      now
    ],
  )?;

  get_review_unit(connection, connection.last_insert_rowid())?.context("Review unit missing after insert")
}

pub fn ensure_review_units_for_direction(
  connection: &Connection,
  deck_id: i64,
  prompt_field_id: i64,
  reveal_field_ids: &[i64],
) -> Result<()> {
  let mut statement = connection.prepare("SELECT id FROM cards WHERE deck_id = ?1 ORDER BY id ASC")?;
  let card_ids = statement
    .query_map(params![deck_id], |row| row.get::<_, i64>(0))?
    .collect::<rusqlite::Result<Vec<_>>>()?;
  for card_id in card_ids {
    let _ = ensure_review_unit(connection, card_id, deck_id, prompt_field_id, reveal_field_ids)?;
  }
  Ok(())
}

pub fn list_study_cards(
  connection: &Connection,
  deck_id: i64,
  prompt_field_id: i64,
  reveal_field_ids: &[i64],
  mode: StudyMode,
) -> Result<Vec<StudyCard>> {
  ensure_review_units_for_direction(connection, deck_id, prompt_field_id, reveal_field_ids)?;
  let direction_key = build_direction_key(prompt_field_id, reveal_field_ids);
  let now = now_utc();
  let sql = match mode {
    StudyMode::Due => {
      "SELECT
          c.id,
          c.deck_id,
          ru.id AS review_unit_id,
          c.language_1,
          c.language_2,
          c.language_3,
          c.note,
          c.example_sentence,
          c.tag,
          c.status,
          c.next_review_at,
          ru.state AS review_state,
          ru.due_at_utc,
          ru.mastered,
          ru.leech,
          ru.suspended,
          ru.difficulty,
          ru.stability
        FROM review_units ru
        INNER JOIN cards c ON c.id = ru.card_id
        WHERE ru.deck_id = ?1
          AND ru.prompt_field_id = ?2
          AND ru.direction_key = ?3
          AND ru.suspended = 0
          AND ru.state != 'new'
          AND ru.due_at_utc IS NOT NULL
          AND ru.due_at_utc <= ?4
        ORDER BY
          CASE ru.state
            WHEN 'leech' THEN 0
            WHEN 'relearning' THEN 1
            WHEN 'learning' THEN 2
            ELSE 3
          END,
          ru.due_at_utc ASC,
          c.updated_at ASC"
    }
    StudyMode::New => {
      "SELECT
          c.id,
          c.deck_id,
          ru.id AS review_unit_id,
          c.language_1,
          c.language_2,
          c.language_3,
          c.note,
          c.example_sentence,
          c.tag,
          c.status,
          c.next_review_at,
          ru.state AS review_state,
          ru.due_at_utc,
          ru.mastered,
          ru.leech,
          ru.suspended,
          ru.difficulty,
          ru.stability
        FROM review_units ru
        INNER JOIN cards c ON c.id = ru.card_id
        WHERE ru.deck_id = ?1
          AND ru.prompt_field_id = ?2
          AND ru.direction_key = ?3
          AND ru.suspended = 0
          AND ru.state = 'new'
        ORDER BY c.created_at ASC"
    }
    StudyMode::Mixed => {
      "SELECT
          c.id,
          c.deck_id,
          ru.id AS review_unit_id,
          c.language_1,
          c.language_2,
          c.language_3,
          c.note,
          c.example_sentence,
          c.tag,
          c.status,
          c.next_review_at,
          ru.state AS review_state,
          ru.due_at_utc,
          ru.mastered,
          ru.leech,
          ru.suspended,
          ru.difficulty,
          ru.stability
        FROM review_units ru
        INNER JOIN cards c ON c.id = ru.card_id
        WHERE ru.deck_id = ?1
          AND ru.prompt_field_id = ?2
          AND ru.direction_key = ?3
          AND ru.suspended = 0
          AND (
            ru.state = 'new'
            OR (ru.state != 'new' AND ru.due_at_utc IS NOT NULL AND ru.due_at_utc <= ?4)
          )
        ORDER BY
          CASE
            WHEN ru.state = 'leech' THEN 0
            WHEN ru.state = 'relearning' THEN 1
            WHEN ru.state = 'review' THEN 2
            WHEN ru.state = 'learning' THEN 3
            ELSE 4
          END,
          ru.due_at_utc ASC,
          c.created_at ASC"
    }
  };

  let mut statement = connection.prepare(sql)?;
  let cards = if matches!(mode, StudyMode::New) {
    statement
      .query_map(params![deck_id, prompt_field_id, direction_key], map_study_card)?
      .collect::<rusqlite::Result<Vec<_>>>()?
  } else {
    statement
      .query_map(params![deck_id, prompt_field_id, direction_key, now], map_study_card)?
      .collect::<rusqlite::Result<Vec<_>>>()?
  };
  attach_study_values(connection, cards)
}

pub fn count_recent_again(connection: &Connection, review_unit_id: i64, limit: usize) -> Result<i64> {
  connection
    .query_row(
      "SELECT COUNT(*)
       FROM (
         SELECT rating
         FROM review_logs
         WHERE review_unit_id = ?1
         ORDER BY reviewed_at_utc DESC
         LIMIT ?2
       ) recent
       WHERE rating = 'again'",
      params![review_unit_id, limit as i64],
      |row| row.get::<_, i64>(0),
    )
    .map_err(Into::into)
}

pub fn apply_review_update(connection: &Connection, review_unit_id: i64, update: &ReviewUnitUpdate) -> Result<()> {
  connection.execute(
    "UPDATE review_units
      SET state = ?1,
          difficulty = ?2,
          stability = ?3,
          scheduled_interval_days = ?4,
          last_reviewed_at_utc = ?5,
          due_at_utc = ?6,
          lapses = ?7,
          successful_reviews = ?8,
          failed_reviews = ?9,
          total_reviews = ?10,
          same_day_reviews_count = ?11,
          average_latency_ms = ?12,
          last_latency_ms = ?13,
          hint_used_last = ?14,
          confidence_last = ?15,
          suspended = ?16,
          leech = ?17,
          mastered = ?18,
          learning_step_index = ?19,
          relearning_step_index = ?20,
          first_reviewed_at_utc = ?21,
          graduated_at_utc = ?22,
          mastered_at_utc = ?23,
          updated_at = ?24
      WHERE id = ?25",
    params![
      update.state.as_str(),
      update.difficulty,
      update.stability,
      update.scheduled_interval_days,
      update.last_reviewed_at_utc,
      update.due_at_utc,
      update.lapses,
      update.successful_reviews,
      update.failed_reviews,
      update.total_reviews,
      update.same_day_reviews_count,
      update.average_latency_ms,
      update.last_latency_ms,
      bool_to_i64(update.hint_used_last),
      update.confidence_last,
      bool_to_i64(update.suspended),
      bool_to_i64(update.leech),
      bool_to_i64(update.mastered),
      update.learning_step_index,
      update.relearning_step_index,
      update.first_reviewed_at_utc,
      update.graduated_at_utc,
      update.mastered_at_utc,
      update.updated_at,
      review_unit_id
    ],
  )?;
  Ok(())
}

pub fn record_review_log(
  connection: &Connection,
  review_unit: &ReviewUnitRecord,
  session_id: i64,
  rating: ReviewRating,
  update: &ReviewUnitUpdate,
  latency_ms: Option<i64>,
  hint_used: bool,
  confidence: Option<f64>,
) -> Result<()> {
  let elapsed_days = review_unit
    .last_reviewed_at_utc
    .as_deref()
    .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
    .map(|last| ((Utc::now() - last.with_timezone(&Utc)).num_seconds().max(0) as f64) / 86_400.0);

  connection.execute(
    "INSERT INTO review_logs (
      review_unit_id,
      card_id,
      deck_id,
      session_id,
      reviewed_at_utc,
      rating,
      was_correct,
      state_before,
      state_after,
      retrievability_before,
      difficulty_before,
      difficulty_after,
      stability_before,
      stability_after,
      interval_before_days,
      interval_after_days,
      scheduled_due_before_utc,
      scheduled_due_after_utc,
      elapsed_days,
      latency_ms,
      hint_used,
      confidence,
      leech_before,
      leech_after
    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24)",
    params![
      review_unit.id,
      review_unit.card_id,
      review_unit.deck_id,
      if session_id > 0 { Some(session_id) } else { None },
      update.last_reviewed_at_utc,
      rating.as_str(),
      if rating.is_success() { 1 } else { 0 },
      review_unit.state.as_str(),
      update.state.as_str(),
      update.retrievability_before,
      review_unit.difficulty,
      update.difficulty,
      review_unit.stability,
      update.stability,
      review_unit.scheduled_interval_days,
      update.scheduled_interval_days,
      review_unit.due_at_utc,
      update.due_at_utc,
      elapsed_days,
      latency_ms,
      bool_to_i64(hint_used),
      confidence,
      bool_to_i64(review_unit.leech),
      bool_to_i64(update.leech)
    ],
  )?;
  Ok(())
}

pub fn sync_card_cache(connection: &Connection, card_id: i64) -> Result<()> {
  let mut statement = connection.prepare(
    "SELECT
      state,
      due_at_utc,
      last_reviewed_at_utc,
      successful_reviews,
      failed_reviews,
      total_reviews,
      difficulty,
      stability,
      scheduled_interval_days,
      mastered,
      suspended
    FROM review_units
    WHERE card_id = ?1",
  )?;

  let rows = statement.query_map(params![card_id], |row| {
    Ok((
      row.get::<_, String>(0)?,
      row.get::<_, Option<String>>(1)?,
      row.get::<_, Option<String>>(2)?,
      row.get::<_, i64>(3)?,
      row.get::<_, i64>(4)?,
      row.get::<_, i64>(5)?,
      row.get::<_, f64>(6)?,
      row.get::<_, f64>(7)?,
      row.get::<_, f64>(8)?,
      row.get::<_, i64>(9)? != 0,
      row.get::<_, i64>(10)? != 0,
    ))
  })?;

  let units = rows.collect::<rusqlite::Result<Vec<_>>>()?;
  if units.is_empty() {
    connection.execute(
      "UPDATE cards
        SET status = 'new',
            review_count = 0,
            correct_count = 0,
            wrong_count = 0,
            current_interval_minutes = 0,
            ease_factor = 2.2,
            mastery_score = 0,
            last_reviewed_at = NULL,
            next_review_at = NULL
        WHERE id = ?1",
      params![card_id],
    )?;
    return Ok(());
  }

  let active_units = units.iter().filter(|unit| !unit.10).collect::<Vec<_>>();
  let total_reviews = active_units.iter().map(|unit| unit.5).sum::<i64>();
  let correct_count = active_units.iter().map(|unit| unit.3).sum::<i64>();
  let wrong_count = active_units.iter().map(|unit| unit.4).sum::<i64>();
  let average_difficulty = if active_units.is_empty() {
    5.0
  } else {
    active_units.iter().map(|unit| unit.6).sum::<f64>() / active_units.len() as f64
  };
  let average_stability = if active_units.is_empty() {
    0.0
  } else {
    active_units.iter().map(|unit| unit.7).sum::<f64>() / active_units.len() as f64
  };
  let mastered_count = active_units.iter().filter(|unit| unit.9).count();
  let any_learning = active_units
    .iter()
    .any(|unit| matches!(unit.0.as_str(), "learning" | "relearning" | "leech"));
  let all_new = active_units.iter().all(|unit| unit.0 == "new");
  let card_status = if all_new {
    "new"
  } else if any_learning {
    "learning"
  } else if !active_units.is_empty() && mastered_count == active_units.len() {
    "mastered"
  } else {
    "review"
  };
  let last_reviewed_at = active_units
    .iter()
    .filter_map(|unit| unit.2.clone())
    .max();
  let next_review_at = active_units
    .iter()
    .filter_map(|unit| unit.1.clone())
    .min();
  let current_interval_minutes = ((average_stability.max(0.0)) * 24.0 * 60.0).round() as i64;
  let ease_factor = (2.5 - ((average_difficulty - 5.0) * 0.18)).clamp(1.3, 3.0);
  let mastery_score = if active_units.is_empty() {
    0
  } else {
    (active_units
      .iter()
      .map(|unit| if unit.9 { 100.0 } else { (unit.7.min(60.0) / 60.0) * 100.0 })
      .sum::<f64>()
      / active_units.len() as f64)
      .round() as i64
  };

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
          updated_at = ?10
      WHERE id = ?11",
    params![
      card_status,
      total_reviews,
      correct_count,
      wrong_count,
      current_interval_minutes,
      ease_factor,
      mastery_score.clamp(0, 100),
      last_reviewed_at,
      next_review_at,
      now_utc(),
      card_id
    ],
  )?;

  Ok(())
}

pub fn sync_deck_card_caches(connection: &Connection, deck_id: i64) -> Result<()> {
  let mut statement = connection.prepare("SELECT id FROM cards WHERE deck_id = ?1")?;
  let card_ids = statement
    .query_map(params![deck_id], |row| row.get::<_, i64>(0))?
    .collect::<rusqlite::Result<Vec<_>>>()?;
  for card_id in card_ids {
    sync_card_cache(connection, card_id)?;
  }
  Ok(())
}

pub fn repair_review_units(connection: &Connection) -> Result<()> {
  let mut statement = connection.prepare("SELECT id FROM decks ORDER BY id ASC")?;
  let deck_ids = statement
    .query_map([], |row| row.get::<_, i64>(0))?
    .collect::<rusqlite::Result<Vec<_>>>()?;

  for deck_id in deck_ids {
    let (prompt_field_id, reveal_field_ids) = dynamic_repo::get_study_configuration(connection, deck_id)?;
    let active_fields = dynamic_repo::get_active_fields(connection, deck_id)?;
    let prompt_field_id = prompt_field_id.unwrap_or_else(|| active_fields.first().map(|field| field.id).unwrap_or_default());
    let reveal_field_ids = if reveal_field_ids.is_empty() {
      active_fields
        .iter()
        .filter(|field| field.id != prompt_field_id)
        .map(|field| field.id)
        .collect::<Vec<_>>()
    } else {
      reveal_field_ids
    };
    if prompt_field_id > 0 && !reveal_field_ids.is_empty() {
      ensure_review_units_for_direction(connection, deck_id, prompt_field_id, &reveal_field_ids)?;
    }
    sync_deck_card_caches(connection, deck_id)?;
  }

  Ok(())
}

pub fn delete_invalid_direction_units(connection: &Connection, deck_id: i64, valid_field_ids: &[i64]) -> Result<()> {
  let valid = valid_field_ids.iter().copied().collect::<std::collections::HashSet<_>>();
  let mut statement = connection.prepare("SELECT id, prompt_field_id, reveal_field_ids FROM review_units WHERE deck_id = ?1")?;
  let rows = statement.query_map(params![deck_id], |row| {
    Ok((
      row.get::<_, i64>(0)?,
      row.get::<_, i64>(1)?,
      dynamic_repo::parse_reveal_field_ids(&row.get::<_, String>(2)?),
    ))
  })?;

  for row in rows {
    let (review_unit_id, prompt_field_id, reveal_field_ids) = row?;
    let valid_direction = valid.contains(&prompt_field_id) && reveal_field_ids.iter().all(|field_id| valid.contains(field_id));
    if !valid_direction {
      connection.execute("DELETE FROM review_units WHERE id = ?1", params![review_unit_id])?;
    }
  }
  Ok(())
}

#[cfg(test)]
mod tests {
  use std::time::{SystemTime, UNIX_EPOCH};

  use chrono::Utc;
  use rusqlite::params;

  use crate::{
    db::{initialize_database, open_connection, repository::deck_repo},
    models::types::{CardValueInput, CreateCardInput, CreateDeckInput, DeckFieldInput},
  };

  use super::{build_direction_key, canonical_reveal_field_ids, ensure_review_unit, repair_review_units};

  fn temp_db_path() -> std::path::PathBuf {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    std::env::temp_dir().join(format!("flashcard-local-review-units-{unique}.sqlite"))
  }

  #[test]
  fn direction_key_is_stable_for_reveal_order() {
    assert_eq!(canonical_reveal_field_ids(&[3, 2, 3]), vec![2, 3]);
    assert_eq!(build_direction_key(1, &[3, 2]), build_direction_key(1, &[2, 3]));
  }

  #[test]
  fn repairs_legacy_cards_into_default_direction_units() {
    let db_path = temp_db_path();
    initialize_database(&db_path).unwrap();
    let connection = open_connection(&db_path).unwrap();

    let deck = deck_repo::create_deck(
      &connection,
      &CreateDeckInput {
        name: "Review Units".into(),
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

    connection
      .execute(
        "UPDATE cards SET status = 'review', current_interval_minutes = 1440, ease_factor = 2.5, mastery_score = 72, next_review_at = ?1, review_count = 4, correct_count = 3, wrong_count = 1 WHERE id = ?2",
        params![Utc::now().to_rfc3339(), card.id],
      )
      .unwrap();

    repair_review_units(&connection).unwrap();

    let count = connection
      .query_row("SELECT COUNT(*) FROM review_units WHERE card_id = ?1", params![card.id], |row| row.get::<_, i64>(0))
      .unwrap();
    assert_eq!(count, 1);

    let unit = ensure_review_unit(&connection, card.id, deck.id, deck.fields[0].id, &[deck.fields[1].id]).unwrap();
    assert_eq!(unit.card_id, card.id);
    assert!(unit.stability > 0.9);

    let _ = std::fs::remove_file(db_path);
  }
}
