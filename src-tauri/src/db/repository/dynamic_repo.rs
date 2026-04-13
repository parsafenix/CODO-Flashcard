use std::collections::{HashMap, HashSet};

use anyhow::{anyhow, Context, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::{
  models::types::{CardValueRecord, DeckField},
  services::normalization::{build_dedupe_key, normalize_value},
};

fn bool_to_i64(value: bool) -> i64 {
  if value { 1 } else { 0 }
}

fn clean_label(value: &str, fallback: &str) -> String {
  let trimmed = value.trim();
  if trimmed.is_empty() {
    fallback.to_string()
  } else {
    trimmed.to_string()
  }
}

pub fn list_deck_fields(connection: &Connection, deck_id: i64) -> Result<Vec<DeckField>> {
  let mut statement = connection.prepare(
    "SELECT id, deck_id, label, language_code, order_index, required, active, field_type, system_key
     FROM deck_fields
     WHERE deck_id = ?1
     ORDER BY order_index ASC, id ASC",
  )?;

  let rows = statement.query_map(params![deck_id], |row| {
    Ok(DeckField {
      id: row.get("id")?,
      deck_id: row.get("deck_id")?,
      label: row.get("label")?,
      language_code: row.get("language_code")?,
      order_index: row.get("order_index")?,
      required: row.get::<_, i64>("required")? != 0,
      active: row.get::<_, i64>("active")? != 0,
      field_type: row.get("field_type")?,
      system_key: row.get("system_key")?,
    })
  })?;

  Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn get_card_values(connection: &Connection, card_id: i64) -> Result<Vec<CardValueRecord>> {
  let mut statement = connection.prepare(
    "SELECT
        cv.id,
        cv.field_id,
        df.label,
        df.language_code,
        df.order_index,
        df.required,
        df.active,
        cv.raw_value,
        cv.normalized_value,
        cv.compact_value
      FROM card_values cv
      INNER JOIN deck_fields df ON df.id = cv.field_id
      WHERE cv.card_id = ?1
      ORDER BY df.order_index ASC, df.id ASC",
  )?;

  let rows = statement.query_map(params![card_id], |row| {
    Ok(CardValueRecord {
      id: row.get(0)?,
      field_id: row.get(1)?,
      label: row.get(2)?,
      language_code: row.get(3)?,
      order_index: row.get(4)?,
      required: row.get::<_, i64>(5)? != 0,
      active: row.get::<_, i64>(6)? != 0,
      value: row.get(7)?,
      normalized_value: row.get(8)?,
      compact_value: row.get(9)?,
    })
  })?;

  Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn get_card_values_for_cards(connection: &Connection, card_ids: &[i64]) -> Result<HashMap<i64, Vec<CardValueRecord>>> {
  let mut map = HashMap::new();
  for card_id in card_ids {
    map.insert(*card_id, get_card_values(connection, *card_id)?);
  }
  Ok(map)
}

fn insert_legacy_field(
  connection: &Connection,
  deck_id: i64,
  label: &str,
  language_code: Option<&str>,
  order_index: i64,
  required: bool,
  active: bool,
  system_key: &str,
) -> Result<i64> {
  connection.execute(
    "INSERT INTO deck_fields (deck_id, label, language_code, order_index, required, active, field_type, system_key)
      VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'text', ?7)",
    params![
      deck_id,
      label,
      language_code,
      order_index,
      bool_to_i64(required),
      bool_to_i64(active),
      system_key
    ],
  )?;
  Ok(connection.last_insert_rowid())
}

fn ensure_legacy_fields_for_deck(connection: &Connection, deck_id: i64) -> Result<()> {
  let count = connection.query_row(
    "SELECT COUNT(*) FROM deck_fields WHERE deck_id = ?1",
    params![deck_id],
    |row| row.get::<_, i64>(0),
  )?;

  if count > 0 {
    return Ok(());
  }

  let (label_1, label_2, label_3, has_note, has_example, has_tag) = connection.query_row(
    "SELECT
        language_1_label,
        language_2_label,
        language_3_label,
        EXISTS(SELECT 1 FROM cards c WHERE c.deck_id = d.id AND TRIM(IFNULL(c.note, '')) != ''),
        EXISTS(SELECT 1 FROM cards c WHERE c.deck_id = d.id AND TRIM(IFNULL(c.example_sentence, '')) != ''),
        EXISTS(SELECT 1 FROM cards c WHERE c.deck_id = d.id AND TRIM(IFNULL(c.tag, '')) != '')
      FROM decks d
      WHERE d.id = ?1",
    params![deck_id],
    |row| {
      Ok((
        row.get::<_, String>(0)?,
        row.get::<_, String>(1)?,
        row.get::<_, String>(2)?,
        row.get::<_, i64>(3)? != 0,
        row.get::<_, i64>(4)? != 0,
        row.get::<_, i64>(5)? != 0,
      ))
    },
  )?;

  insert_legacy_field(connection, deck_id, &clean_label(&label_1, "Field 1"), None, 0, true, true, "legacy_language_1")?;
  insert_legacy_field(connection, deck_id, &clean_label(&label_2, "Field 2"), None, 1, true, true, "legacy_language_2")?;
  insert_legacy_field(connection, deck_id, &clean_label(&label_3, "Field 3"), None, 2, true, true, "legacy_language_3")?;
  insert_legacy_field(connection, deck_id, "Note", Some("notes"), 3, false, has_note, "legacy_note")?;
  insert_legacy_field(
    connection,
    deck_id,
    "Example sentence",
    Some("example"),
    4,
    false,
    has_example,
    "legacy_example_sentence",
  )?;
  insert_legacy_field(connection, deck_id, "Tag", Some("tag"), 5, false, has_tag, "legacy_tag")?;

  Ok(())
}

fn get_field_by_system_key(connection: &Connection, deck_id: i64, system_key: &str) -> Result<Option<i64>> {
  connection
    .query_row(
      "SELECT id FROM deck_fields WHERE deck_id = ?1 AND system_key = ?2",
      params![deck_id, system_key],
      |row| row.get::<_, i64>(0),
    )
    .optional()
    .map_err(Into::into)
}

fn ensure_legacy_card_values(connection: &Connection, deck_id: i64, card_id: i64) -> Result<()> {
  let (language_1, language_2, language_3, note, example_sentence, tag) = connection.query_row(
    "SELECT language_1, language_2, language_3, note, example_sentence, tag FROM cards WHERE id = ?1 AND deck_id = ?2",
    params![card_id, deck_id],
    |row| {
      Ok((
        row.get::<_, String>(0)?,
        row.get::<_, String>(1)?,
        row.get::<_, String>(2)?,
        row.get::<_, Option<String>>(3)?,
        row.get::<_, Option<String>>(4)?,
        row.get::<_, Option<String>>(5)?,
      ))
    },
  )?;

  let mappings = vec![
    ("legacy_language_1", language_1),
    ("legacy_language_2", language_2),
    ("legacy_language_3", language_3),
    ("legacy_note", note.unwrap_or_default()),
    ("legacy_example_sentence", example_sentence.unwrap_or_default()),
    ("legacy_tag", tag.unwrap_or_default()),
  ];

  for (system_key, raw) in mappings {
    if raw.trim().is_empty() {
      continue;
    }
    let Some(field_id) = get_field_by_system_key(connection, deck_id, system_key)? else {
      continue;
    };

    let exists = connection.query_row(
      "SELECT COUNT(*) FROM card_values WHERE card_id = ?1 AND field_id = ?2",
      params![card_id, field_id],
      |row| row.get::<_, i64>(0),
    )?;
    if exists > 0 {
      continue;
    }

    let normalized = normalize_value(&raw);
    connection.execute(
      "INSERT INTO card_values (card_id, field_id, raw_value, normalized_value, compact_value)
       VALUES (?1, ?2, ?3, ?4, ?5)",
      params![card_id, field_id, raw.trim(), normalized.normalized, normalized.compact],
    )?;
  }

  Ok(())
}

pub fn get_required_active_fields(connection: &Connection, deck_id: i64) -> Result<Vec<DeckField>> {
  Ok(list_deck_fields(connection, deck_id)?
    .into_iter()
    .filter(|field| field.active && field.required)
    .collect())
}

pub fn ensure_valid_deck_fields(fields: &[DeckField]) -> Result<()> {
  let active_fields = fields.iter().filter(|field| field.active).count();
  let required_active_fields = fields.iter().filter(|field| field.active && field.required).count();
  anyhow::ensure!(active_fields >= 2, "At least 2 active fields are required.");
  anyhow::ensure!(required_active_fields >= 1, "At least 1 active required field is required.");
  Ok(())
}

fn map_values_by_field(values: &[CardValueRecord]) -> HashMap<i64, CardValueRecord> {
  values.iter().cloned().map(|value| (value.field_id, value)).collect()
}

fn dedupe_parts_for_card(fields: &[DeckField], values_by_field: &HashMap<i64, CardValueRecord>) -> Vec<String> {
  fields
    .iter()
    .filter(|field| field.active && field.required)
    .map(|field| {
      values_by_field
        .get(&field.id)
        .map(|value| value.normalized_value.clone())
        .unwrap_or_default()
    })
    .collect()
}

pub fn compute_card_dedupe_key(fields: &[DeckField], values: &[CardValueRecord]) -> String {
  build_dedupe_key(&dedupe_parts_for_card(fields, &map_values_by_field(values)))
}

fn compatibility_cache_values(fields: &[DeckField], values_by_field: &HashMap<i64, CardValueRecord>) -> (String, String, String, Option<String>, Option<String>, Option<String>) {
  let active_fields = fields.iter().filter(|field| field.active).collect::<Vec<_>>();
  let preview_values = active_fields
    .iter()
    .take(3)
    .map(|field| {
      values_by_field
        .get(&field.id)
        .map(|value| value.value.clone())
        .unwrap_or_default()
    })
    .collect::<Vec<_>>();

  let lookup_system = |system_key: &str| {
    fields
      .iter()
      .find(|field| field.system_key.as_deref() == Some(system_key))
      .and_then(|field| values_by_field.get(&field.id))
      .map(|value| value.value.clone())
      .filter(|value| !value.trim().is_empty())
  };

  (
    preview_values.first().cloned().unwrap_or_default(),
    preview_values.get(1).cloned().unwrap_or_default(),
    preview_values.get(2).cloned().unwrap_or_default(),
    lookup_system("legacy_note"),
    lookup_system("legacy_example_sentence"),
    lookup_system("legacy_tag"),
  )
}

pub fn sync_card_compatibility_cache(connection: &Connection, deck_id: i64, card_id: i64) -> Result<()> {
  let fields = list_deck_fields(connection, deck_id)?;
  let values = get_card_values(connection, card_id)?;
  let values_by_field = map_values_by_field(&values);
  let dedupe_key = compute_card_dedupe_key(&fields, &values);
  let (language_1, language_2, language_3, note, example_sentence, tag) = compatibility_cache_values(&fields, &values_by_field);
  let normalized_1 = normalize_value(&language_1);
  let normalized_2 = normalize_value(&language_2);
  let normalized_3 = normalize_value(&language_3);

  connection.execute(
    "UPDATE cards
      SET language_1 = ?1,
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
          dedupe_key = ?13
      WHERE id = ?14",
    params![
      language_1,
      language_2,
      language_3,
      note,
      example_sentence,
      tag,
      normalized_1.normalized,
      normalized_2.normalized,
      normalized_3.normalized,
      normalized_1.compact,
      normalized_2.compact,
      normalized_3.compact,
      dedupe_key,
      card_id
    ],
  )?;

  Ok(())
}

pub fn recompute_deck_card_caches(connection: &Connection, deck_id: i64) -> Result<()> {
  let fields = list_deck_fields(connection, deck_id)?;
  ensure_valid_deck_fields(&fields)?;

  let mut statement = connection.prepare("SELECT id FROM cards WHERE deck_id = ?1 ORDER BY id ASC")?;
  let card_ids = statement
    .query_map(params![deck_id], |row| row.get::<_, i64>(0))?
    .collect::<rusqlite::Result<Vec<_>>>()?;

  let mut seen = HashSet::new();
  for card_id in card_ids {
    let values = get_card_values(connection, card_id)?;
    let dedupe_key = compute_card_dedupe_key(&fields, &values);
    if !seen.insert(dedupe_key.clone()) {
      return Err(anyhow!(
        "Updating this deck schema would create duplicate cards across the active required fields."
      ));
    }
    sync_card_compatibility_cache(connection, deck_id, card_id)?;
  }

  if connection
    .query_row(
      "SELECT study_prompt_field_id FROM decks WHERE id = ?1",
      params![deck_id],
      |row| row.get::<_, Option<i64>>(0),
    )?
    .is_none()
  {
    let active_fields = fields.iter().filter(|field| field.active).collect::<Vec<_>>();
    if let Some(prompt_field) = active_fields.first() {
      let reveal = active_fields.iter().skip(1).map(|field| field.id.to_string()).collect::<Vec<_>>();
      connection.execute(
        "UPDATE decks SET study_prompt_field_id = ?1, study_reveal_field_ids = ?2 WHERE id = ?3",
        params![prompt_field.id, format!("[{}]", reveal.join(",")), deck_id],
      )?;
    }
  }

  Ok(())
}

pub fn repair_dynamic_model(connection: &Connection) -> Result<()> {
  let mut statement = connection.prepare("SELECT id FROM decks ORDER BY id ASC")?;
  let deck_ids = statement
    .query_map([], |row| row.get::<_, i64>(0))?
    .collect::<rusqlite::Result<Vec<_>>>()?;

  for deck_id in deck_ids {
    ensure_legacy_fields_for_deck(connection, deck_id)?;

    let mut card_statement = connection.prepare("SELECT id FROM cards WHERE deck_id = ?1 ORDER BY id ASC")?;
    let card_ids = card_statement
      .query_map(params![deck_id], |row| row.get::<_, i64>(0))?
      .collect::<rusqlite::Result<Vec<_>>>()?;

    for card_id in card_ids {
      ensure_legacy_card_values(connection, deck_id, card_id)?;

      let values = get_card_values(connection, card_id)?;
      for value in &values {
        let normalized = normalize_value(&value.value);
        connection.execute(
          "UPDATE card_values SET normalized_value = ?1, compact_value = ?2 WHERE id = ?3",
          params![normalized.normalized, normalized.compact, value.id],
        )?;
      }
    }

    recompute_deck_card_caches(connection, deck_id)?;
  }

  Ok(())
}

pub fn upsert_card_values(
  connection: &Connection,
  card_id: i64,
  values: &[(i64, String)],
) -> Result<()> {
  for (field_id, raw_value) in values {
    let trimmed = raw_value.trim();
    let exists = connection.query_row(
      "SELECT id FROM card_values WHERE card_id = ?1 AND field_id = ?2",
      params![card_id, field_id],
      |row| row.get::<_, i64>(0),
    ).optional()?;

    if trimmed.is_empty() {
      if exists.is_some() {
        connection.execute(
          "DELETE FROM card_values WHERE card_id = ?1 AND field_id = ?2",
          params![card_id, field_id],
        )?;
      }
      continue;
    }

    let normalized = normalize_value(trimmed);
    if let Some(value_id) = exists {
      connection.execute(
        "UPDATE card_values
          SET raw_value = ?1, normalized_value = ?2, compact_value = ?3
          WHERE id = ?4",
        params![trimmed, normalized.normalized, normalized.compact, value_id],
      )?;
    } else {
      connection.execute(
        "INSERT INTO card_values (card_id, field_id, raw_value, normalized_value, compact_value)
          VALUES (?1, ?2, ?3, ?4, ?5)",
        params![card_id, field_id, trimmed, normalized.normalized, normalized.compact],
      )?;
    }
  }
  Ok(())
}

pub fn ensure_required_fields_present(
  fields: &[DeckField],
  values: &HashMap<i64, String>,
) -> Result<()> {
  for field in fields.iter().filter(|field| field.active && field.required) {
    let value = values.get(&field.id).map(|value| value.trim()).unwrap_or("");
    if value.is_empty() {
      return Err(anyhow!("Required field missing: {}", field.label));
    }
  }
  Ok(())
}

pub fn get_active_fields(connection: &Connection, deck_id: i64) -> Result<Vec<DeckField>> {
  Ok(list_deck_fields(connection, deck_id)?
    .into_iter()
    .filter(|field| field.active)
    .collect())
}

pub fn parse_reveal_field_ids(raw: &str) -> Vec<i64> {
  raw
    .trim()
    .trim_start_matches('[')
    .trim_end_matches(']')
    .split(',')
    .filter_map(|part| part.trim().parse::<i64>().ok())
    .collect()
}

pub fn serialize_reveal_field_ids(field_ids: &[i64]) -> String {
  format!(
    "[{}]",
    field_ids
      .iter()
      .map(|field_id| field_id.to_string())
      .collect::<Vec<_>>()
      .join(",")
  )
}

pub fn delete_fields(connection: &Connection, deck_id: i64, deleted_field_ids: &[i64]) -> Result<()> {
  for field_id in deleted_field_ids {
    let has_values = connection.query_row(
      "SELECT COUNT(*) FROM card_values cv
        INNER JOIN deck_fields df ON df.id = cv.field_id
        WHERE df.deck_id = ?1 AND cv.field_id = ?2",
      params![deck_id, field_id],
      |row| row.get::<_, i64>(0),
    )?;
    if has_values > 0 {
      connection.execute("DELETE FROM card_values WHERE field_id = ?1", params![field_id])?;
    }
    connection.execute("DELETE FROM deck_fields WHERE id = ?1 AND deck_id = ?2", params![field_id, deck_id])?;
  }
  Ok(())
}

pub fn active_field_labels(fields: &[DeckField]) -> [String; 3] {
  let active = fields.iter().filter(|field| field.active).collect::<Vec<_>>();
  [
    active.first().map(|field| field.label.clone()).unwrap_or_else(|| "Field 1".to_string()),
    active.get(1).map(|field| field.label.clone()).unwrap_or_else(|| "Field 2".to_string()),
    active.get(2).map(|field| field.label.clone()).unwrap_or_else(|| "Field 3".to_string()),
  ]
}

pub fn ensure_field_belongs_to_deck(connection: &Connection, deck_id: i64, field_id: i64) -> Result<()> {
  let exists = connection.query_row(
    "SELECT COUNT(*) FROM deck_fields WHERE id = ?1 AND deck_id = ?2",
    params![field_id, deck_id],
    |row| row.get::<_, i64>(0),
  )?;
  anyhow::ensure!(exists > 0, "Selected field does not belong to this deck.");
  Ok(())
}

pub fn get_study_configuration(connection: &Connection, deck_id: i64) -> Result<(Option<i64>, Vec<i64>)> {
  connection
    .query_row(
      "SELECT study_prompt_field_id, study_reveal_field_ids FROM decks WHERE id = ?1",
      params![deck_id],
      |row| {
        let prompt_field_id = row.get::<_, Option<i64>>(0)?;
        let reveal_raw = row.get::<_, String>(1)?;
        Ok((prompt_field_id, parse_reveal_field_ids(&reveal_raw)))
      },
    )
    .optional()?
    .context("Deck not found")
}

pub fn save_study_configuration(connection: &Connection, deck_id: i64, prompt_field_id: i64, reveal_field_ids: &[i64]) -> Result<()> {
  connection.execute(
    "UPDATE decks
      SET study_prompt_field_id = ?1,
          study_reveal_field_ids = ?2
      WHERE id = ?3",
    params![prompt_field_id, serialize_reveal_field_ids(reveal_field_ids), deck_id],
  )?;
  Ok(())
}

#[cfg(test)]
mod tests {
  use std::time::{SystemTime, UNIX_EPOCH};

  use rusqlite::params;

  use crate::db::{initialize_database, open_connection};

  use super::{list_deck_fields, repair_dynamic_model};

  fn temp_db_path() -> std::path::PathBuf {
    let unique = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .unwrap()
      .as_nanos();
    std::env::temp_dir().join(format!("flashcard-local-dynamic-repo-{unique}.sqlite"))
  }

  #[test]
  fn repairs_legacy_decks_and_cards_idempotently() {
    let db_path = temp_db_path();
    initialize_database(&db_path).unwrap();
    let connection = open_connection(&db_path).unwrap();

    connection.execute(
      "INSERT INTO decks (
        id,
        name,
        description,
        language_1_label,
        language_2_label,
        language_3_label,
        created_at,
        updated_at,
        study_reveal_field_ids
      ) VALUES (1, 'Legacy deck', NULL, 'Persian', 'English', 'Italian', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z', '[]')",
      [],
    ).unwrap();

    connection.execute(
      "INSERT INTO cards (
        id,
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
      ) VALUES (
        11,
        1,
        'سلام',
        'Hello',
        'Ciao',
        'greeting',
        'سلام دوست من',
        'basic',
        '',
        '',
        '',
        '',
        '',
        '',
        '',
        '2026-01-01T00:00:00Z',
        '2026-01-01T00:00:00Z',
        'review',
        1440,
        2.4,
        72
      )",
      [],
    ).unwrap();

    repair_dynamic_model(&connection).unwrap();
    repair_dynamic_model(&connection).unwrap();

    let fields = list_deck_fields(&connection, 1).unwrap();
    assert_eq!(fields.len(), 6);
    assert_eq!(fields[0].label, "Persian");
    assert_eq!(fields[1].label, "English");
    assert_eq!(fields[2].label, "Italian");

    let value_count = connection.query_row(
      "SELECT COUNT(*) FROM card_values WHERE card_id = 11",
      [],
      |row| row.get::<_, i64>(0),
    ).unwrap();
    assert_eq!(value_count, 6);

    let note_value = connection.query_row(
      "SELECT raw_value
       FROM card_values cv
       INNER JOIN deck_fields df ON df.id = cv.field_id
       WHERE cv.card_id = 11 AND df.system_key = 'legacy_note'",
      [],
      |row| row.get::<_, String>(0),
    ).unwrap();
    assert_eq!(note_value, "greeting");

    let (dedupe_key, language_1, language_2, language_3) = connection.query_row(
      "SELECT dedupe_key, language_1, language_2, language_3 FROM cards WHERE id = ?1",
      params![11],
      |row| {
        Ok((
          row.get::<_, String>(0)?,
          row.get::<_, String>(1)?,
          row.get::<_, String>(2)?,
          row.get::<_, String>(3)?,
        ))
      },
    ).unwrap();
    assert!(!dedupe_key.is_empty());
    assert_eq!(language_1, "سلام");
    assert_eq!(language_2, "Hello");
    assert_eq!(language_3, "Ciao");

    let _ = std::fs::remove_file(db_path);
  }
}
