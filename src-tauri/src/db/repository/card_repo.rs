use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::{params, params_from_iter, ffi, Connection, Error, OptionalExtension, Row, ToSql};

use crate::models::types::{
  CardFilter, CardListQuery, CardRecord, CardSchedulingRecord, CardSort, CardStatus, CreateCardInput,
  StudyCard, StudyMode, UpdateCardInput,
};

use super::dynamic_repo;

fn now_utc() -> String {
  Utc::now().to_rfc3339()
}

fn temporary_dedupe_key(deck_id: i64) -> String {
  let tick = Utc::now()
    .timestamp_nanos_opt()
    .unwrap_or_else(|| Utc::now().timestamp_micros() * 1_000);
  format!("pending:{deck_id}:{tick}")
}

fn map_card_base(row: &Row<'_>) -> rusqlite::Result<CardRecord> {
  Ok(CardRecord {
    id: row.get("id")?,
    deck_id: row.get("deck_id")?,
    language_1: row.get("language_1")?,
    language_2: row.get("language_2")?,
    language_3: row.get("language_3")?,
    note: row.get("note")?,
    example_sentence: row.get("example_sentence")?,
    tag: row.get("tag")?,
    values: Vec::new(),
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
    values: Vec::new(),
    status: CardStatus::from_db(&row.get::<_, String>("status")?),
    next_review_at: row.get("next_review_at")?,
  })
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

fn attach_values(connection: &Connection, mut cards: Vec<CardRecord>) -> Result<Vec<CardRecord>> {
  let card_ids = cards.iter().map(|card| card.id).collect::<Vec<_>>();
  let by_card = dynamic_repo::get_card_values_for_cards(connection, &card_ids)?;
  for card in &mut cards {
    card.values = by_card.get(&card.id).cloned().unwrap_or_default();
  }
  Ok(cards)
}

fn attach_study_values(connection: &Connection, mut cards: Vec<StudyCard>) -> Result<Vec<StudyCard>> {
  let card_ids = cards.iter().map(|card| card.id).collect::<Vec<_>>();
  let by_card = dynamic_repo::get_card_values_for_cards(connection, &card_ids)?;
  for card in &mut cards {
    card.values = by_card.get(&card.id).cloned().unwrap_or_default();
  }
  Ok(cards)
}

fn save_card(connection: &Connection, id: Option<i64>, deck_id: i64, values: &[(i64, String)]) -> Result<CardRecord> {
  let fields = dynamic_repo::list_deck_fields(connection, deck_id)?;
  let values_map = values.iter().cloned().collect::<HashMap<_, _>>();
  dynamic_repo::ensure_required_fields_present(&fields, &values_map)?;
  for field_id in values_map.keys() {
    dynamic_repo::ensure_field_belongs_to_deck(connection, deck_id, *field_id)?;
  }

  let card_id = if let Some(card_id) = id {
    let rows = connection.execute(
      "UPDATE cards SET updated_at = ?1 WHERE id = ?2 AND deck_id = ?3",
      params![now_utc(), card_id, deck_id],
    )?;
    if rows == 0 {
      return Err(not_found_error());
    }
    card_id
  } else {
    let created_at = now_utc();
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
      ) VALUES (?1, '', '', '', NULL, NULL, NULL, '', '', '', '', '', '', ?2, ?3, ?3, 'new', 0, 2.2, 0)",
      params![deck_id, temporary_dedupe_key(deck_id), created_at],
    )
    .map_err(|error| if is_unique_constraint(&error) { duplicate_error() } else { error.into() })?;
    connection.last_insert_rowid()
  };

  let submitted_field_ids = values.iter().map(|(field_id, _)| *field_id).collect::<HashSet<_>>();
  let mut current_statement = connection.prepare("SELECT field_id FROM card_values WHERE card_id = ?1")?;
  let current_field_ids = current_statement
    .query_map(params![card_id], |row| row.get::<_, i64>(0))?
    .collect::<rusqlite::Result<Vec<_>>>()?;
  for field_id in current_field_ids {
    if !submitted_field_ids.contains(&field_id) {
      connection.execute(
        "DELETE FROM card_values WHERE card_id = ?1 AND field_id = ?2",
        params![card_id, field_id],
      )?;
    }
  }

  dynamic_repo::upsert_card_values(connection, card_id, values)?;
  dynamic_repo::sync_card_compatibility_cache(connection, deck_id, card_id)
    .map_err(|error| if error.to_string().contains("UNIQUE constraint") { duplicate_error() } else { error })?;

  get_card_in_deck(connection, card_id, deck_id)?.context("Card not found after save")
}

pub fn list_cards(connection: &Connection, query: &CardListQuery) -> Result<Vec<CardRecord>> {
  let search_normalized = query
    .search
    .as_deref()
    .map(crate::services::normalization::normalize_text)
    .unwrap_or_default();
  let search_compact = query
    .search
    .as_deref()
    .map(crate::services::normalization::compact_text)
    .unwrap_or_default();
  let like_normalized = format!("%{}%", search_normalized);
  let like_compact = format!("%{}%", search_compact);
  let now = now_utc();

  let mut sql = String::from(
    "SELECT * FROM cards c
      WHERE c.deck_id = ?1
        AND (?2 = '' OR EXISTS (
          SELECT 1
          FROM card_values cv
          INNER JOIN deck_fields df ON df.id = cv.field_id
          WHERE cv.card_id = c.id
            AND df.active = 1
            AND (cv.normalized_value LIKE ?3 OR cv.compact_value LIKE ?4)
        ))",
  );

  let filter = query.filter.unwrap_or(CardFilter::All);
  match filter {
    CardFilter::All => {}
    CardFilter::New => sql.push_str(" AND c.status = 'new'"),
    CardFilter::Due => sql.push_str(" AND c.status != 'new' AND c.next_review_at IS NOT NULL AND c.next_review_at <= ?5"),
    CardFilter::Mastered => sql.push_str(" AND c.status = 'mastered'"),
    CardFilter::Weak => sql.push_str(" AND c.review_count > 0 AND (c.wrong_count > c.correct_count OR c.mastery_score < 40)"),
  }

  sql.push_str(" ORDER BY ");
  sql.push_str(match query.sort.unwrap_or(CardSort::UpdatedDesc) {
    CardSort::UpdatedDesc => "c.updated_at DESC",
    CardSort::CreatedDesc => "c.created_at DESC",
    CardSort::NextReviewAsc => "CASE WHEN c.next_review_at IS NULL THEN 1 ELSE 0 END, c.next_review_at ASC",
    CardSort::PrimaryFieldAsc => "c.language_1_normalized ASC",
  });

  let mut statement = connection.prepare(&sql)?;
  let cards = if matches!(filter, CardFilter::Due) {
    let params_vec: Vec<&dyn ToSql> = vec![&query.deck_id, &search_normalized, &like_normalized, &like_compact, &now];
    statement
      .query_map(params_from_iter(params_vec), map_card_base)?
      .collect::<rusqlite::Result<Vec<_>>>()?
  } else {
    let params_vec: Vec<&dyn ToSql> = vec![&query.deck_id, &search_normalized, &like_normalized, &like_compact];
    statement
      .query_map(params_from_iter(params_vec), map_card_base)?
      .collect::<rusqlite::Result<Vec<_>>>()?
  };

  attach_values(connection, cards)
}

pub fn get_card(connection: &Connection, card_id: i64) -> Result<Option<CardRecord>> {
  let card = connection
    .query_row("SELECT * FROM cards WHERE id = ?1", params![card_id], map_card_base)
    .optional()?;
  Ok(match card {
    Some(card) => Some(attach_values(connection, vec![card])?.remove(0)),
    None => None,
  })
}

pub fn get_card_in_deck(connection: &Connection, card_id: i64, deck_id: i64) -> Result<Option<CardRecord>> {
  let card = connection
    .query_row(
      "SELECT * FROM cards WHERE id = ?1 AND deck_id = ?2",
      params![card_id, deck_id],
      map_card_base,
    )
    .optional()?;
  Ok(match card {
    Some(card) => Some(attach_values(connection, vec![card])?.remove(0)),
    None => None,
  })
}

pub fn create_card(connection: &Connection, input: &CreateCardInput) -> Result<CardRecord> {
  let values = input
    .values
    .iter()
    .map(|value| (value.field_id, value.value.clone()))
    .collect::<Vec<_>>();
  save_card(connection, None, input.deck_id, &values)
}

pub fn update_card(connection: &Connection, input: &UpdateCardInput) -> Result<CardRecord> {
  let values = input
    .values
    .iter()
    .map(|value| (value.field_id, value.value.clone()))
    .collect::<Vec<_>>();
  save_card(connection, Some(input.id), input.deck_id, &values)
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

pub fn insert_import_card(connection: &Connection, deck_id: i64, values: &[(i64, String)]) -> Result<()> {
  save_card(connection, None, deck_id, values).map(|_| ()).map_err(|error| {
    if error.to_string() == "duplicate_card" {
      duplicate_error()
    } else {
      error
    }
  })
}

pub fn get_cards_for_study(connection: &Connection, deck_id: i64, mode: StudyMode) -> Result<Vec<StudyCard>> {
  let now = now_utc();
  let sql = match mode {
    StudyMode::Due => {
      "SELECT id, deck_id, language_1, language_2, language_3, note, example_sentence, tag, status, next_review_at
        FROM cards
        WHERE deck_id = ?1 AND status != 'new' AND next_review_at IS NOT NULL AND next_review_at <= ?2
        ORDER BY next_review_at ASC, updated_at ASC"
    }
    StudyMode::New => {
      "SELECT id, deck_id, language_1, language_2, language_3, note, example_sentence, tag, status, next_review_at
        FROM cards
        WHERE deck_id = ?1 AND status = 'new'
        ORDER BY created_at ASC"
    }
    StudyMode::Mixed => {
      "SELECT id, deck_id, language_1, language_2, language_3, note, example_sentence, tag, status, next_review_at
        FROM cards
        WHERE deck_id = ?1 AND (
          status = 'new' OR (status != 'new' AND next_review_at IS NOT NULL AND next_review_at <= ?2)
        )
        ORDER BY CASE WHEN status = 'new' THEN 1 ELSE 0 END, next_review_at ASC, created_at ASC"
    }
  };

  let mut statement = connection.prepare(sql)?;
  let cards = statement
    .query_map(params![deck_id, now], map_study_card)?
    .collect::<rusqlite::Result<Vec<_>>>()?;
  attach_study_values(connection, cards)
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

  use rusqlite::Connection;

  use crate::{
    db::{initialize_database, open_connection, repository::deck_repo},
    models::types::{CardFilter, CardListQuery, CardSort, CardValueInput, CreateCardInput, CreateDeckInput, DeckFieldInput, UpdateCardInput},
  };

  use super::{create_card, list_cards, update_card};

  fn temp_db_path() -> std::path::PathBuf {
    let unique = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .unwrap()
      .as_nanos();
    std::env::temp_dir().join(format!("flashcard-local-card-repo-{unique}.sqlite"))
  }

  fn create_test_deck(connection: &Connection, name: &str) -> i64 {
    deck_repo::create_deck(
      connection,
      &CreateDeckInput {
        name: name.to_string(),
        description: None,
        fields: vec![
          DeckFieldInput {
            id: None,
            label: "Persian".to_string(),
            language_code: Some("persian".to_string()),
            order_index: 0,
            required: true,
            active: true,
            field_type: Some("text".to_string()),
          },
          DeckFieldInput {
            id: None,
            label: "English".to_string(),
            language_code: Some("english".to_string()),
            order_index: 1,
            required: true,
            active: true,
            field_type: Some("text".to_string()),
          },
          DeckFieldInput {
            id: None,
            label: "Example".to_string(),
            language_code: Some("example".to_string()),
            order_index: 2,
            required: false,
            active: true,
            field_type: Some("text".to_string()),
          },
        ],
      },
    )
    .unwrap()
    .id
  }

  #[test]
  fn update_respects_deck_id_and_rejects_wrong_target() {
    let db_path = temp_db_path();
    initialize_database(&db_path).unwrap();
    let connection = open_connection(&db_path).unwrap();

    let deck_a = deck_repo::get_deck(&connection, create_test_deck(&connection, "Deck A")).unwrap().unwrap();
    let deck_b = deck_repo::get_deck(&connection, create_test_deck(&connection, "Deck B")).unwrap().unwrap();

    let card = create_card(
      &connection,
      &CreateCardInput {
        deck_id: deck_a.id,
        values: vec![
          CardValueInput {
            field_id: deck_a.fields[0].id,
            value: "سلام".to_string(),
          },
          CardValueInput {
            field_id: deck_a.fields[1].id,
            value: "Hello".to_string(),
          },
        ],
      },
    )
    .unwrap();

    let error = update_card(
      &connection,
      &UpdateCardInput {
        id: card.id,
        deck_id: deck_b.id,
        values: vec![
          CardValueInput {
            field_id: deck_b.fields[0].id,
            value: "سلام".to_string(),
          },
          CardValueInput {
            field_id: deck_b.fields[1].id,
            value: "Hello".to_string(),
          },
        ],
      },
    )
    .unwrap_err();

    assert_eq!(error.to_string(), "card_not_found");
    let _ = std::fs::remove_file(db_path);
  }

  #[test]
  fn search_matches_dynamic_compact_persian_variants() {
    let db_path = temp_db_path();
    initialize_database(&db_path).unwrap();
    let connection = open_connection(&db_path).unwrap();
    let deck = deck_repo::get_deck(&connection, create_test_deck(&connection, "Deck")).unwrap().unwrap();

    create_card(
      &connection,
      &CreateCardInput {
        deck_id: deck.id,
        values: vec![
          CardValueInput {
            field_id: deck.fields[0].id,
            value: "كتاب\u{200C}ها".to_string(),
          },
          CardValueInput {
            field_id: deck.fields[1].id,
            value: "Books".to_string(),
          },
        ],
      },
    )
    .unwrap();

    let results = list_cards(
      &connection,
      &CardListQuery {
        deck_id: deck.id,
        search: Some("کتابها".to_string()),
        filter: Some(CardFilter::All),
        sort: Some(CardSort::PrimaryFieldAsc),
      },
    )
    .unwrap();

    assert_eq!(results.len(), 1);
    let _ = std::fs::remove_file(db_path);
  }

  #[test]
  fn duplicate_detection_uses_required_active_fields_only() {
    let db_path = temp_db_path();
    initialize_database(&db_path).unwrap();
    let connection = open_connection(&db_path).unwrap();

    let deck = deck_repo::create_deck(
      &connection,
      &CreateDeckInput {
        name: "Dynamic duplicate deck".to_string(),
        description: None,
        fields: vec![
          DeckFieldInput {
            id: None,
            label: "Word".to_string(),
            language_code: Some("english".to_string()),
            order_index: 0,
            required: true,
            active: true,
            field_type: Some("text".to_string()),
          },
          DeckFieldInput {
            id: None,
            label: "Meaning".to_string(),
            language_code: Some("persian".to_string()),
            order_index: 1,
            required: true,
            active: true,
            field_type: Some("text".to_string()),
          },
          DeckFieldInput {
            id: None,
            label: "Notes".to_string(),
            language_code: Some("notes".to_string()),
            order_index: 2,
            required: false,
            active: true,
            field_type: Some("text".to_string()),
          },
        ],
      },
    )
    .unwrap();

    create_card(
      &connection,
      &CreateCardInput {
        deck_id: deck.id,
        values: vec![
          CardValueInput {
            field_id: deck.fields[0].id,
            value: "book".to_string(),
          },
          CardValueInput {
            field_id: deck.fields[1].id,
            value: "کتاب".to_string(),
          },
          CardValueInput {
            field_id: deck.fields[2].id,
            value: "first note".to_string(),
          },
        ],
      },
    )
    .unwrap();

    let duplicate_error = create_card(
      &connection,
      &CreateCardInput {
        deck_id: deck.id,
        values: vec![
          CardValueInput {
            field_id: deck.fields[0].id,
            value: "book".to_string(),
          },
          CardValueInput {
            field_id: deck.fields[1].id,
            value: "کتاب".to_string(),
          },
          CardValueInput {
            field_id: deck.fields[2].id,
            value: "second note".to_string(),
          },
        ],
      },
    )
    .unwrap_err();

    assert_eq!(duplicate_error.to_string(), "duplicate_card");
    let _ = std::fs::remove_file(db_path);
  }
}
