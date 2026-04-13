use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::models::types::{CreateDeckInput, DeckDetail, DeckField, DeckFieldInput, DeckSummary, UpdateDeckInput};

use super::dynamic_repo;

fn now_utc() -> String {
  Utc::now().to_rfc3339()
}

fn temporary_dedupe_key(deck_id: i64, source_card_id: i64) -> String {
  let tick = Utc::now()
    .timestamp_nanos_opt()
    .unwrap_or_else(|| Utc::now().timestamp_micros() * 1_000);
  format!("pending:{deck_id}:{source_card_id}:{tick}")
}

fn clean_text(value: Option<&String>) -> Option<String> {
  value
    .map(|text| text.trim().to_string())
    .filter(|text| !text.is_empty())
}

fn sanitize_field_inputs(fields: &[DeckFieldInput]) -> Result<Vec<DeckFieldInput>> {
  let mut sanitized = fields
    .iter()
    .enumerate()
    .map(|(index, field)| DeckFieldInput {
      id: field.id,
      label: field.label.trim().to_string(),
      language_code: field.language_code.as_ref().map(|value| value.trim().to_string()).filter(|value| !value.is_empty()),
      order_index: index as i64,
      required: field.required,
      active: field.active,
      field_type: Some(field.field_type.clone().unwrap_or_else(|| "text".to_string())),
    })
    .collect::<Vec<_>>();

  anyhow::ensure!(!sanitized.is_empty(), "At least 2 active fields are required.");
  anyhow::ensure!(
    sanitized.iter().all(|field| !field.label.is_empty()),
    "Every field needs a label."
  );

  let preview_fields = sanitized
    .iter()
    .enumerate()
    .map(|(index, field)| DeckField {
      id: field.id.unwrap_or(-1 - index as i64),
      deck_id: 0,
      label: field.label.clone(),
      language_code: field.language_code.clone(),
      order_index: index as i64,
      required: field.required,
      active: field.active,
      field_type: field.field_type.clone().unwrap_or_else(|| "text".to_string()),
      system_key: None,
    })
    .collect::<Vec<_>>();
  dynamic_repo::ensure_valid_deck_fields(&preview_fields)?;

  for (index, field) in sanitized.iter_mut().enumerate() {
    field.order_index = index as i64;
  }

  Ok(sanitized)
}

fn map_deck_base(row: &Row<'_>) -> rusqlite::Result<DeckSummary> {
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
    study_prompt_field_id: row.get("study_prompt_field_id")?,
    study_reveal_field_ids: dynamic_repo::parse_reveal_field_ids(&row.get::<_, String>("study_reveal_field_ids")?),
    fields: Vec::new(),
  })
}

fn attach_fields(connection: &Connection, mut deck: DeckSummary) -> Result<DeckSummary> {
  deck.fields = dynamic_repo::list_deck_fields(connection, deck.id)?;
  let compatibility_labels = dynamic_repo::active_field_labels(&deck.fields);
  deck.language_1_label = compatibility_labels[0].clone();
  deck.language_2_label = compatibility_labels[1].clone();
  deck.language_3_label = compatibility_labels[2].clone();
  Ok(deck)
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
        d.study_prompt_field_id,
        d.study_reveal_field_ids,
        (SELECT COUNT(*) FROM cards c WHERE c.deck_id = d.id) AS total_cards,
        (SELECT COUNT(*) FROM cards c WHERE c.deck_id = d.id AND c.status != 'new' AND c.next_review_at IS NOT NULL AND c.next_review_at <= ?1) AS due_cards,
        (SELECT COUNT(*) FROM cards c WHERE c.deck_id = d.id AND c.status = 'new') AS new_cards,
        (SELECT COUNT(*) FROM cards c WHERE c.deck_id = d.id AND c.status = 'mastered') AS mastered_cards
      FROM decks d
      WHERE (?2 = '%%' OR d.name LIKE ?2 OR IFNULL(d.description, '') LIKE ?2)
      ORDER BY d.updated_at DESC",
  )?;

  let rows = statement.query_map(params![now, like], map_deck_base)?;
  let mut decks = Vec::new();
  for row in rows {
    decks.push(attach_fields(connection, row?)?);
  }
  Ok(decks)
}

pub fn get_deck(connection: &Connection, deck_id: i64) -> Result<Option<DeckDetail>> {
  let now = now_utc();
  let deck = connection
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
          d.study_prompt_field_id,
          d.study_reveal_field_ids,
          (SELECT COUNT(*) FROM cards c WHERE c.deck_id = d.id) AS total_cards,
          (SELECT COUNT(*) FROM cards c WHERE c.deck_id = d.id AND c.status != 'new' AND c.next_review_at IS NOT NULL AND c.next_review_at <= ?1) AS due_cards,
          (SELECT COUNT(*) FROM cards c WHERE c.deck_id = d.id AND c.status = 'new') AS new_cards,
          (SELECT COUNT(*) FROM cards c WHERE c.deck_id = d.id AND c.status = 'mastered') AS mastered_cards
        FROM decks d
        WHERE d.id = ?2",
      params![now, deck_id],
      map_deck_base,
    )
    .optional()?;

  Ok(match deck {
    Some(deck) => Some(attach_fields(connection, deck)?),
    None => None,
  })
}

pub fn create_deck(connection: &Connection, input: &CreateDeckInput) -> Result<DeckDetail> {
  let name = input.name.trim();
  anyhow::ensure!(!name.is_empty(), "Deck name is required.");
  let fields = sanitize_field_inputs(&input.fields)?;
  let now = now_utc();
  let compatibility_labels = dynamic_repo::active_field_labels(
    &fields
      .iter()
      .enumerate()
      .map(|(index, field)| DeckField {
        id: -1 - index as i64,
        deck_id: 0,
        label: field.label.clone(),
        language_code: field.language_code.clone(),
        order_index: field.order_index,
        required: field.required,
        active: field.active,
        field_type: field.field_type.clone().unwrap_or_else(|| "text".to_string()),
        system_key: None,
      })
      .collect::<Vec<_>>(),
  );

  connection.execute(
    "INSERT INTO decks (
      name,
      description,
      language_1_label,
      language_2_label,
      language_3_label,
      created_at,
      updated_at,
      study_reveal_field_ids
    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, '[]')",
    params![
      name,
      clean_text(input.description.as_ref()),
      compatibility_labels[0].clone(),
      compatibility_labels[1].clone(),
      compatibility_labels[2].clone(),
      now
    ],
  )?;

  let deck_id = connection.last_insert_rowid();
  for field in fields {
    connection.execute(
      "INSERT INTO deck_fields (deck_id, label, language_code, order_index, required, active, field_type)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
      params![
        deck_id,
        field.label,
        field.language_code,
        field.order_index,
        if field.required { 1 } else { 0 },
        if field.active { 1 } else { 0 },
        field.field_type.unwrap_or_else(|| "text".to_string())
      ],
    )?;
  }

  dynamic_repo::recompute_deck_card_caches(connection, deck_id)?;
  get_deck(connection, deck_id)?.context("Failed to fetch created deck")
}

pub fn update_deck(connection: &Connection, input: &UpdateDeckInput) -> Result<DeckDetail> {
  let name = input.name.trim();
  anyhow::ensure!(!name.is_empty(), "Deck name is required.");
  let fields = sanitize_field_inputs(&input.fields)?;
  anyhow::ensure!(get_deck(connection, input.id)?.is_some(), "Deck not found.");

  let transaction = connection.unchecked_transaction()?;

  transaction.execute(
    "UPDATE decks
      SET name = ?1,
          description = ?2,
          updated_at = ?3
      WHERE id = ?4",
    params![name, clean_text(input.description.as_ref()), now_utc(), input.id],
  )?;

  dynamic_repo::delete_fields(&transaction, input.id, &input.deleted_field_ids)?;

  for field in fields {
    if let Some(field_id) = field.id {
      transaction.execute(
        "UPDATE deck_fields
          SET label = ?1,
              language_code = ?2,
              order_index = ?3,
              required = ?4,
              active = ?5,
              field_type = ?6
          WHERE id = ?7 AND deck_id = ?8",
        params![
          field.label,
          field.language_code,
          field.order_index,
          if field.required { 1 } else { 0 },
          if field.active { 1 } else { 0 },
          field.field_type.unwrap_or_else(|| "text".to_string()),
          field_id,
          input.id
        ],
      )?;
    } else {
      transaction.execute(
        "INSERT INTO deck_fields (deck_id, label, language_code, order_index, required, active, field_type)
          VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
          input.id,
          field.label,
          field.language_code,
          field.order_index,
          if field.required { 1 } else { 0 },
          if field.active { 1 } else { 0 },
          field.field_type.unwrap_or_else(|| "text".to_string())
        ],
      )?;
    }
  }

  let updated_fields = dynamic_repo::list_deck_fields(&transaction, input.id)?;
  let compatibility_labels = dynamic_repo::active_field_labels(&updated_fields);
  transaction.execute(
    "UPDATE decks
      SET language_1_label = ?1,
          language_2_label = ?2,
          language_3_label = ?3
      WHERE id = ?4",
    params![
      compatibility_labels[0].clone(),
      compatibility_labels[1].clone(),
      compatibility_labels[2].clone(),
      input.id
    ],
  )?;

  let (prompt_field_id, reveal_field_ids) = dynamic_repo::get_study_configuration(&transaction, input.id)?;
  let active_field_ids = updated_fields
    .iter()
    .filter(|field| field.active)
    .map(|field| field.id)
    .collect::<Vec<_>>();
  let prompt_invalid = prompt_field_id.map(|field_id| !active_field_ids.contains(&field_id)).unwrap_or(true);
  let reveal_invalid = reveal_field_ids.is_empty() || reveal_field_ids.iter().any(|field_id| !active_field_ids.contains(field_id));
  if prompt_invalid || reveal_invalid {
    transaction.execute(
      "UPDATE decks SET study_prompt_field_id = NULL, study_reveal_field_ids = '[]' WHERE id = ?1",
      params![input.id],
    )?;
  }

  dynamic_repo::recompute_deck_card_caches(&transaction, input.id)?;
  transaction.commit()?;
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

  transaction.execute(
    "INSERT INTO decks (
      name,
      description,
      language_1_label,
      language_2_label,
      language_3_label,
      created_at,
      updated_at,
      study_reveal_field_ids
    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, '[]')",
    params![
      format!("{} Copy", source.name),
      source.description,
      source.language_1_label,
      source.language_2_label,
      source.language_3_label,
      now
    ],
  )?;
  let new_deck_id = transaction.last_insert_rowid();

  let mut field_id_map = std::collections::HashMap::new();
  for field in &source.fields {
    transaction.execute(
      "INSERT INTO deck_fields (deck_id, label, language_code, order_index, required, active, field_type, system_key)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
      params![
        new_deck_id,
        field.label,
        field.language_code,
        field.order_index,
        if field.required { 1 } else { 0 },
        if field.active { 1 } else { 0 },
        field.field_type,
        field.system_key
      ],
    )?;
    field_id_map.insert(field.id, transaction.last_insert_rowid());
  }

  let source_cards = {
    let mut card_statement = transaction.prepare(
      "SELECT id, created_at, updated_at FROM cards WHERE deck_id = ?1 ORDER BY id ASC",
    )?;
    let rows = card_statement
      .query_map(params![deck_id], |row| {
        Ok((
          row.get::<_, i64>(0)?,
          row.get::<_, String>(1)?,
          row.get::<_, String>(2)?,
        ))
      })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()?
  };

  for (source_card_id, _, _) in source_cards {
    let created_at = now_utc();
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
      ) VALUES (?1, '', '', '', NULL, NULL, NULL, '', '', '', '', '', '', ?2, ?3, ?3, 'new', 0, 2.2, 0, 0, 0, 0)",
      params![new_deck_id, temporary_dedupe_key(new_deck_id, source_card_id), created_at],
    )?;
    let new_card_id = transaction.last_insert_rowid();

    let values = dynamic_repo::get_card_values(&transaction, source_card_id)?;
    for value in values {
      let target_field_id = field_id_map
        .get(&value.field_id)
        .copied()
        .context("Missing duplicated field mapping")?;
      transaction.execute(
        "INSERT INTO card_values (card_id, field_id, raw_value, normalized_value, compact_value)
          VALUES (?1, ?2, ?3, ?4, ?5)",
        params![new_card_id, target_field_id, value.value, value.normalized_value, value.compact_value],
      )?;
    }
  }

  transaction.commit()?;
  dynamic_repo::recompute_deck_card_caches(connection, new_deck_id)?;
  get_deck(connection, new_deck_id)?.context("Duplicated deck missing")
}
