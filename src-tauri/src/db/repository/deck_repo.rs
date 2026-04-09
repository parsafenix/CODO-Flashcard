use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::models::types::{CreateDeckInput, DeckDetail, DeckSummary, UpdateDeckInput};

fn now_utc() -> String {
  Utc::now().to_rfc3339()
}

fn clean_label(value: Option<String>, fallback: &str) -> String {
  value
    .map(|label| label.trim().to_string())
    .filter(|label| !label.is_empty())
    .unwrap_or_else(|| fallback.to_string())
}

fn map_deck_summary(row: &Row<'_>) -> rusqlite::Result<DeckSummary> {
  Ok(DeckSummary {
    id: row.get("id")?,
    name: row.get("name")?,
    description: row.get("description")?,
    language_1_label: row.get("language_1_label")?,
    language_2_label: row.get("language_2_label")?,
    language_3_label: row.get("language_3_label")?,
    created_at: row.get("created_at")?,
    updated_at: row.get("updated_at")?,
    last_studied_at: row.get("last_studied_at")?,
    total_cards: row.get("total_cards")?,
    due_cards: row.get("due_cards")?,
    new_cards: row.get("new_cards")?,
    mastered_cards: row.get("mastered_cards")?,
  })
}

pub fn list_decks(connection: &Connection, search: &str) -> Result<Vec<DeckSummary>> {
  let like = format!("%{}%", search.trim());
  let now = now_utc();
  let mut statement = connection.prepare(
    "SELECT
        d.id,
        d.name,
        d.description,
        d.language_1_label,
        d.language_2_label,
        d.language_3_label,
        d.created_at,
        d.updated_at,
        d.last_studied_at,
        (SELECT COUNT(*) FROM cards c WHERE c.deck_id = d.id) AS total_cards,
        (SELECT COUNT(*) FROM cards c WHERE c.deck_id = d.id AND c.status != 'new' AND c.next_review_at IS NOT NULL AND c.next_review_at <= ?1) AS due_cards,
        (SELECT COUNT(*) FROM cards c WHERE c.deck_id = d.id AND c.status = 'new') AS new_cards,
        (SELECT COUNT(*) FROM cards c WHERE c.deck_id = d.id AND c.status = 'mastered') AS mastered_cards
      FROM decks d
      WHERE (?2 = '%%' OR d.name LIKE ?2 OR IFNULL(d.description, '') LIKE ?2)
      ORDER BY d.updated_at DESC",
  )?;

  let rows = statement.query_map(params![now, like], map_deck_summary)?;
  Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn get_deck(connection: &Connection, deck_id: i64) -> Result<Option<DeckDetail>> {
  let now = now_utc();
  connection
    .query_row(
      "SELECT
          d.id,
          d.name,
          d.description,
          d.language_1_label,
          d.language_2_label,
          d.language_3_label,
          d.created_at,
          d.updated_at,
          d.last_studied_at,
          (SELECT COUNT(*) FROM cards c WHERE c.deck_id = d.id) AS total_cards,
          (SELECT COUNT(*) FROM cards c WHERE c.deck_id = d.id AND c.status != 'new' AND c.next_review_at IS NOT NULL AND c.next_review_at <= ?1) AS due_cards,
          (SELECT COUNT(*) FROM cards c WHERE c.deck_id = d.id AND c.status = 'new') AS new_cards,
          (SELECT COUNT(*) FROM cards c WHERE c.deck_id = d.id AND c.status = 'mastered') AS mastered_cards
        FROM decks d
        WHERE d.id = ?2",
      params![now, deck_id],
      map_deck_summary,
    )
    .optional()
    .map_err(Into::into)
}

pub fn create_deck(connection: &Connection, input: &CreateDeckInput) -> Result<DeckDetail> {
  let name = input.name.trim();
  anyhow::ensure!(!name.is_empty(), "Deck name is required.");
  let now = now_utc();

  connection.execute(
    "INSERT INTO decks (
      name,
      description,
      language_1_label,
      language_2_label,
      language_3_label,
      created_at,
      updated_at
    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
    params![
      name,
      input.description.as_ref().map(|value| value.trim().to_string()).filter(|value| !value.is_empty()),
      clean_label(input.language_1_label.clone(), "Persian"),
      clean_label(input.language_2_label.clone(), "English"),
      clean_label(input.language_3_label.clone(), "Italian"),
      now
    ],
  )?;

  let deck_id = connection.last_insert_rowid();
  get_deck(connection, deck_id)?.context("Failed to fetch created deck")
}

pub fn update_deck(connection: &Connection, input: &UpdateDeckInput) -> Result<DeckDetail> {
  let name = input.name.trim();
  anyhow::ensure!(!name.is_empty(), "Deck name is required.");

  connection.execute(
    "UPDATE decks
      SET name = ?1,
          description = ?2,
          language_1_label = ?3,
          language_2_label = ?4,
          language_3_label = ?5,
          updated_at = ?6
      WHERE id = ?7",
    params![
      name,
      input.description.as_ref().map(|value| value.trim().to_string()).filter(|value| !value.is_empty()),
      clean_label(input.language_1_label.clone(), "Persian"),
      clean_label(input.language_2_label.clone(), "English"),
      clean_label(input.language_3_label.clone(), "Italian"),
      now_utc(),
      input.id
    ],
  )?;

  get_deck(connection, input.id)?.context("Deck not found after update")
}

pub fn delete_deck(connection: &Connection, deck_id: i64) -> Result<()> {
  connection.execute("DELETE FROM decks WHERE id = ?1", params![deck_id])?;
  Ok(())
}

pub fn duplicate_deck(connection: &mut Connection, deck_id: i64) -> Result<DeckDetail> {
  let source = get_deck(connection, deck_id)?.context("Deck not found")?;
  let transaction = connection.transaction()?;
  let now = now_utc();
  let copied_name = format!("{} Copy", source.name);

  transaction.execute(
    "INSERT INTO decks (
      name,
      description,
      language_1_label,
      language_2_label,
      language_3_label,
      created_at,
      updated_at
    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
    params![
      copied_name,
      source.description,
      source.language_1_label,
      source.language_2_label,
      source.language_3_label,
      now
    ],
  )?;

  let new_deck_id = transaction.last_insert_rowid();
  transaction.execute(
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
      mastery_score,
      review_count,
      correct_count,
      wrong_count
    )
    SELECT
      ?1,
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
      ?2,
      ?2,
      'new',
      0,
      2.2,
      0,
      0,
      0,
      0
    FROM cards
    WHERE deck_id = ?3",
    params![new_deck_id, now, deck_id],
  )?;

  transaction.commit()?;
  get_deck(connection, new_deck_id)?.context("Duplicated deck missing")
}

pub fn update_deck_labels(connection: &Connection, deck_id: i64, labels: [String; 3]) -> Result<()> {
  let rows_affected = connection.execute(
    "UPDATE decks
      SET language_1_label = ?1,
          language_2_label = ?2,
          language_3_label = ?3,
          updated_at = ?4
      WHERE id = ?5",
    params![
      clean_label(Some(labels[0].clone()), "Persian"),
      clean_label(Some(labels[1].clone()), "English"),
      clean_label(Some(labels[2].clone()), "Italian"),
      now_utc(),
      deck_id
    ],
  )?;
  anyhow::ensure!(rows_affected > 0, "Deck not found.");
  Ok(())
}
