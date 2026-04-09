use std::collections::HashSet;

use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::{params, params_from_iter, ffi, Connection, Error, OptionalExtension, Row, ToSql};

use crate::{
  models::types::{
    CardFilter, CardListQuery, CardRecord, CardSchedulingRecord, CardSort, CardStatus, CreateCardInput,
    StudyCard, UpdateCardInput,
  },
  services::normalization::{compact_text, normalize_card_fields, normalize_text},
};

fn now_utc() -> String {
  Utc::now().to_rfc3339()
}

fn map_card_record(row: &Row<'_>) -> rusqlite::Result<CardRecord> {
  Ok(CardRecord {
    id: row.get("id")?,
    deck_id: row.get("deck_id")?,
    language_1: row.get("language_1")?,
    language_2: row.get("language_2")?,
    language_3: row.get("language_3")?,
    note: row.get("note")?,
    example_sentence: row.get("example_sentence")?,
    tag: row.get("tag")?,
    created_at: row.get("created_at")?,
    updated_at: row.get("updated_at")?,
    last_reviewed_at: row.get("last_reviewed_at")?,
    next_review_at: row.get("next_review_at")?,
    review_count: row.get("review_count")?,
    correct_count: row.get("correct_count")?,
    wrong_count: row.get("wrong_count")?,
    current_interval_minutes: row.get("current_interval_minutes")?,
    ease_factor: row.get("ease_factor")?,
    mastery_score: row.get("mastery_score")?,
    status: CardStatus::from_db(&row.get::<_, String>("status")?),
  })
}

fn map_study_card(row: &Row<'_>) -> rusqlite::Result<StudyCard> {
  Ok(StudyCard {
    id: row.get("id")?,
    deck_id: row.get("deck_id")?,
    language_1: row.get("language_1")?,
    language_2: row.get("language_2")?,
    language_3: row.get("language_3")?,
    note: row.get("note")?,
    example_sentence: row.get("example_sentence")?,
    tag: row.get("tag")?,
    status: CardStatus::from_db(&row.get::<_, String>("status")?),
    next_review_at: row.get("next_review_at")?,
  })
}

fn normalize_optional(value: &Option<String>) -> Option<String> {
  value
    .as_ref()
    .map(|text| text.trim().to_string())
    .filter(|text| !text.is_empty())
}

fn duplicate_error() -> anyhow::Error {
  anyhow::anyhow!("duplicate_card")
}

fn not_found_error() -> anyhow::Error {
  anyhow::anyhow!("card_not_found")
}

fn is_unique_constraint(error: &Error) -> bool {
  matches!(
    error,
    Error::SqliteFailure(code, _)
      if code.extended_code == ffi::SQLITE_CONSTRAINT_UNIQUE
        || code.extended_code == ffi::SQLITE_CONSTRAINT_PRIMARYKEY
        || code.extended_code == ffi::SQLITE_CONSTRAINT
  )
}

fn save_card(
  connection: &Connection,
  id: Option<i64>,
  deck_id: i64,
  language_1: &str,
  language_2: &str,
  language_3: &str,
  note: Option<String>,
  example_sentence: Option<String>,
  tag: Option<String>,
) -> Result<CardRecord> {
  anyhow::ensure!(!language_1.trim().is_empty(), "Language 1 is required.");
  anyhow::ensure!(!language_2.trim().is_empty(), "Language 2 is required.");
  anyhow::ensure!(!language_3.trim().is_empty(), "Language 3 is required.");

  let normalized = normalize_card_fields(language_1, language_2, language_3);
  let now = now_utc();

  let rows_affected = if let Some(card_id) = id {
    connection.execute(
      "UPDATE cards SET
        language_1 = ?1,
        language_2 = ?2,
        language_3 = ?3,
        note = ?4,
        example_sentence = ?5,
        tag = ?6,
        language_1_normalized = ?7,
        language_2_normalized = ?8,
        language_3_normalized = ?9,
        language_1_compact = ?10,
        language_2_compact = ?11,
        language_3_compact = ?12,
        dedupe_key = ?13,
        updated_at = ?14
      WHERE id = ?15 AND deck_id = ?16",
      params![
        language_1.trim(),
        language_2.trim(),
        language_3.trim(),
        note,
        example_sentence,
        tag,
        normalized.language_1_normalized,
        normalized.language_2_normalized,
        normalized.language_3_normalized,
        normalized.language_1_compact,
        normalized.language_2_compact,
        normalized.language_3_compact,
        normalized.dedupe_key,
        now,
        card_id,
        deck_id
      ],
    )
  } else {
    connection.execute(
      "INSERT INTO cards (
        deck_id,
        language_1,
        language_2,
        language_3,
        note,
        example_sentence,
        tag,
        language_1_normalized,
        language_2_normalized,
        language_3_normalized,
        language_1_compact,
        language_2_compact,
        language_3_compact,
        dedupe_key,
        created_at,
        updated_at,
        status,
        current_interval_minutes,
        ease_factor,
        mastery_score
      ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?15, 'new', 0, 2.2, 0)",
      params![
        deck_id,
        language_1.trim(),
        language_2.trim(),
        language_3.trim(),
        note,
        example_sentence,
        tag,
        normalized.language_1_normalized,
        normalized.language_2_normalized,
        normalized.language_3_normalized,
        normalized.language_1_compact,
        normalized.language_2_compact,
        normalized.language_3_compact,
        normalized.dedupe_key,
        now
      ],
    )
  }
  .map_err(|error| if is_unique_constraint(&error) { duplicate_error() } else { error.into() })?;

  if id.is_some() && rows_affected == 0 {
    return Err(not_found_error());
  }

  let final_id = id.unwrap_or_else(|| connection.last_insert_rowid());
  get_card_in_deck(connection, final_id, deck_id)?.context("Card not found after save")
}

pub fn list_cards(connection: &Connection, query: &CardListQuery) -> Result<Vec<CardRecord>> {
  let search_normalized = query.search.as_deref().map(normalize_text).unwrap_or_default();
  let search_compact = query.search.as_deref().map(compact_text).unwrap_or_default();
  let like_normalized = format!("%{}%", search_normalized);
  let like_compact = format!("%{}%", search_compact);
  let now = now_utc();

  let mut sql = String::from(
    "SELECT * FROM cards
      WHERE deck_id = ?1
        AND (?2 = '' OR
          language_1_normalized LIKE ?3 OR language_2_normalized LIKE ?3 OR language_3_normalized LIKE ?3 OR
          language_1_compact LIKE ?4 OR language_2_compact LIKE ?4 OR language_3_compact LIKE ?4)",
  );

  let filter = query.filter.unwrap_or(CardFilter::All);
  match filter {
    CardFilter::All => {}
    CardFilter::New => sql.push_str(" AND status = 'new'"),
    CardFilter::Due => sql.push_str(" AND status != 'new' AND next_review_at IS NOT NULL AND next_review_at <= ?5"),
    CardFilter::Mastered => sql.push_str(" AND status = 'mastered'"),
    CardFilter::Weak => sql.push_str(" AND review_count > 0 AND (wrong_count > correct_count OR mastery_score < 40)"),
  }

  sql.push_str(" ORDER BY ");
  sql.push_str(match query.sort.unwrap_or(CardSort::UpdatedDesc) {
    CardSort::UpdatedDesc => "updated_at DESC",
    CardSort::CreatedDesc => "created_at DESC",
    CardSort::NextReviewAsc => "CASE WHEN next_review_at IS NULL THEN 1 ELSE 0 END, next_review_at ASC",
    CardSort::Language1Asc => "language_1_normalized ASC",
  });

  let mut statement = connection.prepare(&sql)?;
  if matches!(filter, CardFilter::Due) {
    let params_vec: Vec<&dyn ToSql> = vec![
      &query.deck_id,
      &search_normalized,
      &like_normalized,
      &like_compact,
      &now,
    ];
    let rows = statement.query_map(params_from_iter(params_vec), map_card_record)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
  } else {
    let params_vec: Vec<&dyn ToSql> = vec![&query.deck_id, &search_normalized, &like_normalized, &like_compact];
    let rows = statement.query_map(params_from_iter(params_vec), map_card_record)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
  }
}

pub fn get_card(connection: &Connection, card_id: i64) -> Result<Option<CardRecord>> {
  connection
    .query_row("SELECT * FROM cards WHERE id = ?1", params![card_id], map_card_record)
    .optional()
    .map_err(Into::into)
}

pub fn get_card_in_deck(connection: &Connection, card_id: i64, deck_id: i64) -> Result<Option<CardRecord>> {
  connection
    .query_row(
      "SELECT * FROM cards WHERE id = ?1 AND deck_id = ?2",
      params![card_id, deck_id],
      map_card_record,
    )
    .optional()
    .map_err(Into::into)
}

pub fn create_card(connection: &Connection, input: &CreateCardInput) -> Result<CardRecord> {
  save_card(
    connection,
    None,
    input.deck_id,
    &input.language_1,
    &input.language_2,
    &input.language_3,
    normalize_optional(&input.note),
    normalize_optional(&input.example_sentence),
    normalize_optional(&input.tag),
  )
}

pub fn update_card(connection: &Connection, input: &UpdateCardInput) -> Result<CardRecord> {
  save_card(
    connection,
    Some(input.id),
    input.deck_id,
    &input.language_1,
    &input.language_2,
    &input.language_3,
    normalize_optional(&input.note),
    normalize_optional(&input.example_sentence),
    normalize_optional(&input.tag),
  )
}

pub fn delete_card(connection: &Connection, card_id: i64) -> Result<()> {
  connection.execute("DELETE FROM cards WHERE id = ?1", params![card_id])?;
  Ok(())
}

pub fn get_existing_dedupe_keys(connection: &Connection, deck_id: i64) -> Result<HashSet<String>> {
  let mut statement = connection.prepare("SELECT dedupe_key FROM cards WHERE deck_id = ?1")?;
  let rows = statement.query_map(params![deck_id], |row| row.get::<_, String>(0))?;
  Ok(rows.collect::<rusqlite::Result<HashSet<_>>>()?)
}

pub fn insert_import_card(
  connection: &Connection,
  deck_id: i64,
  language_1: &str,
  language_2: &str,
  language_3: &str,
) -> Result<()> {
  let normalized = normalize_card_fields(language_1, language_2, language_3);
  let now = now_utc();
  match connection.execute(
    "INSERT INTO cards (
      deck_id,
      language_1,
      language_2,
      language_3,
      language_1_normalized,
      language_2_normalized,
      language_3_normalized,
      language_1_compact,
      language_2_compact,
      language_3_compact,
      dedupe_key,
      created_at,
      updated_at,
      status,
      current_interval_minutes,
      ease_factor,
      mastery_score
    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12, 'new', 0, 2.2, 0)",
    params![
      deck_id,
      language_1.trim(),
      language_2.trim(),
      language_3.trim(),
      normalized.language_1_normalized,
      normalized.language_2_normalized,
      normalized.language_3_normalized,
      normalized.language_1_compact,
      normalized.language_2_compact,
      normalized.language_3_compact,
      normalized.dedupe_key,
      now
    ],
  ) {
    Ok(_) => {}
    Err(error) if is_unique_constraint(&error) => return Err(duplicate_error()),
    Err(other) => return Err(other.into()),
  }
  Ok(())
}

pub fn get_cards_for_study(
  connection: &Connection,
  deck_id: i64,
  mode: crate::models::types::StudyMode,
) -> Result<Vec<StudyCard>> {
  let now = now_utc();
  let sql = match mode {
    crate::models::types::StudyMode::Due => {
      "SELECT id, deck_id, language_1, language_2, language_3, note, example_sentence, tag, status, next_review_at
        FROM cards
        WHERE deck_id = ?1 AND status != 'new' AND next_review_at IS NOT NULL AND next_review_at <= ?2
        ORDER BY next_review_at ASC, updated_at ASC"
    }
    crate::models::types::StudyMode::New => {
      "SELECT id, deck_id, language_1, language_2, language_3, note, example_sentence, tag, status, next_review_at
        FROM cards
        WHERE deck_id = ?1 AND status = 'new'
        ORDER BY created_at ASC"
    }
    crate::models::types::StudyMode::Mixed => {
      "SELECT id, deck_id, language_1, language_2, language_3, note, example_sentence, tag, status, next_review_at
        FROM cards
        WHERE deck_id = ?1 AND (
          status = 'new' OR (status != 'new' AND next_review_at IS NOT NULL AND next_review_at <= ?2)
        )
        ORDER BY CASE WHEN status = 'new' THEN 1 ELSE 0 END, next_review_at ASC, created_at ASC"
    }
  };

  let mut statement = connection.prepare(sql)?;
  let rows = statement.query_map(params![deck_id, now], map_study_card)?;
  Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn get_scheduling_record(connection: &Connection, card_id: i64) -> Result<Option<CardSchedulingRecord>> {
  connection
    .query_row(
      "SELECT id, deck_id, status, review_count, correct_count, wrong_count, current_interval_minutes, ease_factor, mastery_score, last_reviewed_at, next_review_at
        FROM cards
        WHERE id = ?1",
      params![card_id],
      |row| {
        Ok(CardSchedulingRecord {
          id: row.get("id")?,
          deck_id: row.get("deck_id")?,
          status: CardStatus::from_db(&row.get::<_, String>("status")?),
          review_count: row.get("review_count")?,
          correct_count: row.get("correct_count")?,
          wrong_count: row.get("wrong_count")?,
          current_interval_minutes: row.get("current_interval_minutes")?,
          ease_factor: row.get("ease_factor")?,
          mastery_score: row.get("mastery_score")?,
          last_reviewed_at: row.get("last_reviewed_at")?,
          next_review_at: row.get("next_review_at")?,
        })
      },
    )
    .optional()
    .map_err(Into::into)
}

#[cfg(test)]
mod tests {
  use std::time::{SystemTime, UNIX_EPOCH};

  use crate::{
    db::{initialize_database, open_connection, repository::deck_repo},
    models::types::{CardFilter, CardListQuery, CardSort, CreateCardInput, CreateDeckInput, UpdateCardInput},
  };

  use super::{create_card, list_cards, update_card};

  fn temp_db_path() -> std::path::PathBuf {
    let unique = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .unwrap()
      .as_nanos();
    std::env::temp_dir().join(format!("flashcard-local-card-repo-{unique}.sqlite"))
  }

  #[test]
  fn update_respects_deck_id_and_rejects_wrong_target() {
    let db_path = temp_db_path();
    initialize_database(&db_path).unwrap();
    let connection = open_connection(&db_path).unwrap();

    let deck_a = deck_repo::create_deck(
      &connection,
      &CreateDeckInput {
        name: "Deck A".to_string(),
        description: None,
        language_1_label: None,
        language_2_label: None,
        language_3_label: None,
      },
    )
    .unwrap();

    let deck_b = deck_repo::create_deck(
      &connection,
      &CreateDeckInput {
        name: "Deck B".to_string(),
        description: None,
        language_1_label: None,
        language_2_label: None,
        language_3_label: None,
      },
    )
    .unwrap();

    let card = create_card(
      &connection,
      &CreateCardInput {
        deck_id: deck_a.id,
        language_1: "\u{0633}\u{0644}\u{0627}\u{0645}".to_string(),
        language_2: "Hello".to_string(),
        language_3: "Ciao".to_string(),
        note: None,
        example_sentence: None,
        tag: None,
      },
    )
    .unwrap();

    let error = update_card(
      &connection,
      &UpdateCardInput {
        id: card.id,
        deck_id: deck_b.id,
        language_1: "\u{0633}\u{0644}\u{0627}\u{0645}".to_string(),
        language_2: "Hello".to_string(),
        language_3: "Ciao".to_string(),
        note: None,
        example_sentence: None,
        tag: None,
      },
    )
    .unwrap_err();

    assert_eq!(error.to_string(), "card_not_found");

    let _ = std::fs::remove_file(db_path);
  }

  #[test]
  fn search_matches_compact_persian_variants() {
    let db_path = temp_db_path();
    initialize_database(&db_path).unwrap();
    let connection = open_connection(&db_path).unwrap();

    let deck = deck_repo::create_deck(
      &connection,
      &CreateDeckInput {
        name: "Deck".to_string(),
        description: None,
        language_1_label: None,
        language_2_label: None,
        language_3_label: None,
      },
    )
    .unwrap();

    create_card(
      &connection,
      &CreateCardInput {
        deck_id: deck.id,
        language_1: "\u{0643}\u{062A}\u{0627}\u{0628}\u{200C}\u{0647}\u{0627}".to_string(),
        language_2: "Books".to_string(),
        language_3: "Libri".to_string(),
        note: None,
        example_sentence: None,
        tag: None,
      },
    )
    .unwrap();

    let results = list_cards(
      &connection,
      &CardListQuery {
        deck_id: deck.id,
        search: Some("\u{06A9}\u{062A}\u{0627}\u{0628}\u{0647}\u{0627}".to_string()),
        filter: Some(CardFilter::All),
        sort: Some(CardSort::Language1Asc),
      },
    )
    .unwrap();

    assert_eq!(results.len(), 1);

    let _ = std::fs::remove_file(db_path);
  }
}
