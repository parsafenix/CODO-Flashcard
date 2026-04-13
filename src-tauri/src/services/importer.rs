use std::{collections::{HashMap, HashSet}, fs, path::Path};

use anyhow::{bail, Result};

use crate::{
  models::types::{
    DeckField, DeckFieldInput, ImportColumnMapping, ImportDetectedColumn, ImportPreviewResponse, ImportPreviewRow,
    ImportPreviewSummary, ImportTarget, InvalidImportLine, ParsedImportDocument, ParsedImportRow,
  },
  services::normalization::{build_dedupe_key, normalize_text},
};

pub fn parse_import_file(path: &Path, delimiter: &str, has_header: bool) -> Result<ParsedImportDocument> {
  let delimiter = delimiter.trim();
  if delimiter.is_empty() {
    bail!("Import delimiter cannot be empty");
  }

  let content = fs::read_to_string(path)?;
  let mut header_labels = None;
  let mut rows = Vec::new();
  let mut invalid_lines = Vec::new();
  let mut expected_columns: Option<usize> = None;

  for (index, raw_line) in content.lines().enumerate() {
    let line_number = index + 1;
    let mut line = raw_line.to_string();
    if index == 0 {
      line = line.trim_start_matches('\u{FEFF}').to_string();
    }

    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
      continue;
    }

    let columns = trimmed
      .split(delimiter)
      .map(|column| column.trim().to_string())
      .collect::<Vec<_>>();

    if columns.len() < 2 {
      invalid_lines.push(InvalidImportLine {
        line_number,
        raw: trimmed.to_string(),
        reason: "Expected at least 2 columns.".to_string(),
      });
      continue;
    }

    if has_header && header_labels.is_none() {
      expected_columns = Some(columns.len());
      header_labels = Some(columns);
      continue;
    }

    if let Some(expected) = expected_columns {
      if columns.len() != expected {
        invalid_lines.push(InvalidImportLine {
          line_number,
          raw: trimmed.to_string(),
          reason: format!("Expected {expected} columns but found {}.", columns.len()),
        });
        continue;
      }
    } else {
      expected_columns = Some(columns.len());
    }

    rows.push(ParsedImportRow { line_number, columns });
  }

  Ok(ParsedImportDocument {
    header_labels,
    column_count: expected_columns.unwrap_or(0),
    rows,
    invalid_lines,
  })
}

fn default_new_fields(document: &ParsedImportDocument, create_fields_from_header: bool) -> Vec<DeckFieldInput> {
  let header = document.header_labels.clone().unwrap_or_default();
  (0..document.column_count)
    .map(|index| DeckFieldInput {
      id: None,
      label: if create_fields_from_header {
        header.get(index).cloned().filter(|value| !value.trim().is_empty()).unwrap_or_else(|| format!("Field {}", index + 1))
      } else {
        format!("Field {}", index + 1)
      },
      language_code: None,
      order_index: index as i64,
      required: index < 2,
      active: true,
      field_type: Some("text".to_string()),
    })
    .collect()
}

fn mapping_labels(document: &ParsedImportDocument) -> Vec<ImportDetectedColumn> {
  let header = document.header_labels.clone().unwrap_or_default();
  (0..document.column_count)
    .map(|index| ImportDetectedColumn {
      column_index: index,
      label: header.get(index).cloned().filter(|value| !value.trim().is_empty()).unwrap_or_else(|| format!("Column {}", index + 1)),
    })
    .collect()
}

pub fn derive_new_fields(document: &ParsedImportDocument, create_fields_from_header: bool, mappings: &[ImportColumnMapping]) -> Vec<DeckFieldInput> {
  let mut fields = default_new_fields(document, create_fields_from_header);
  for mapping in mappings {
    if let Some(field) = fields.get_mut(mapping.column_index) {
      if let Some(label) = &mapping.label {
        if !label.trim().is_empty() {
          field.label = label.trim().to_string();
        }
      }
      if let Some(language_code) = &mapping.language_code {
        field.language_code = Some(language_code.trim().to_string()).filter(|value| !value.is_empty());
      }
      if let Some(required) = mapping.required {
        field.required = required;
      }
      if let Some(active) = mapping.active {
        field.active = active;
      }
    }
  }
  for (index, field) in fields.iter_mut().enumerate() {
    field.order_index = index as i64;
  }
  fields
}

pub fn derive_existing_field_mapping(
  fields: &[DeckField],
  document: &ParsedImportDocument,
  mappings: &[ImportColumnMapping],
) -> HashMap<usize, i64> {
  if !mappings.is_empty() {
    return mappings
      .iter()
      .filter_map(|mapping| mapping.field_id.map(|field_id| (mapping.column_index, field_id)))
      .collect();
  }

  let active_fields = fields.iter().filter(|field| field.active).collect::<Vec<_>>();
  (0..document.column_count)
    .filter_map(|index| active_fields.get(index).map(|field| (index, field.id)))
    .collect()
}

pub fn validate_existing_mapping_uniqueness(mappings: &[ImportColumnMapping]) -> Result<()> {
  let mut seen = HashSet::new();
  for mapping in mappings {
    if let Some(field_id) = mapping.field_id {
      if !seen.insert(field_id) {
        bail!("Each active deck field can only be mapped once.");
      }
    }
  }
  Ok(())
}

fn build_row_dedupe(parts: &[String]) -> String {
  build_dedupe_key(parts)
}

pub fn build_preview(
  document: ParsedImportDocument,
  target: &ImportTarget,
  existing_fields: &[DeckField],
  existing_keys: HashSet<String>,
  mappings: &[ImportColumnMapping],
  create_fields_from_header: bool,
) -> ImportPreviewResponse {
  let detected_columns = mapping_labels(&document);
  let suggested_new_fields = derive_new_fields(&document, create_fields_from_header, mappings);

  let required_existing_fields = existing_fields
    .iter()
    .filter(|field| field.active && field.required)
    .cloned()
    .collect::<Vec<_>>();
  let existing_mapping = derive_existing_field_mapping(existing_fields, &document, mappings);
  let unmapped_required_fields = match target {
    ImportTarget::Existing { .. } => required_existing_fields
      .iter()
      .filter(|field| !existing_mapping.values().any(|field_id| field_id == &field.id))
      .map(|field| field.label.clone())
      .collect::<Vec<_>>(),
    ImportTarget::New { .. } => Vec::new(),
  };

  let mut rows = Vec::new();
  let mut seen_batch = HashSet::new();
  let mut duplicates = 0usize;
  let mut missing_required = 0usize;

  for row in &document.rows {
    let mut duplicate = false;
    let mut duplicate_reason = None;
    let mut missing_fields = Vec::new();
    let mut dedupe_parts = Vec::new();

    match target {
      ImportTarget::Existing { .. } => {
        for field in &required_existing_fields {
          let mapped_column = existing_mapping
            .iter()
            .find_map(|(column_index, field_id)| if *field_id == field.id { Some(*column_index) } else { None });
          let value = mapped_column.and_then(|column_index| row.columns.get(column_index)).map(|value| value.trim()).unwrap_or("");
          if value.is_empty() {
            missing_fields.push(field.label.clone());
          }
          dedupe_parts.push(normalize_text(value));
        }
      }
      ImportTarget::New { .. } => {
        for (index, field) in suggested_new_fields.iter().enumerate() {
          if !field.active {
            continue;
          }
          let value = row.columns.get(index).map(|value| value.trim()).unwrap_or("");
          if field.required && value.is_empty() {
            missing_fields.push(field.label.clone());
          }
          if field.required {
            dedupe_parts.push(normalize_text(value));
          }
        }
      }
    }

    if missing_fields.is_empty() {
      let dedupe_key = build_row_dedupe(&dedupe_parts);
      if existing_keys.contains(&dedupe_key) {
        duplicate = true;
        duplicate_reason = Some("Matches another card by this deck's active required fields.".to_string());
      } else if !seen_batch.insert(dedupe_key) {
        duplicate = true;
        duplicate_reason = Some("Repeated in this file for the same active required fields.".to_string());
      }
    }

    if duplicate {
      duplicates += 1;
    }
    if !missing_fields.is_empty() {
      missing_required += 1;
    }

    rows.push(ImportPreviewRow {
      line_number: row.line_number,
      columns: row.columns.clone(),
      duplicate,
      duplicate_reason,
      missing_required_fields: missing_fields,
    });
  }

  let valid = document.rows.len();
  let invalid = document.invalid_lines.len();
  let importable = rows
    .iter()
    .filter(|row| !row.duplicate && row.missing_required_fields.is_empty())
    .count();
  let ready_for_commit = importable > 0
    && match target {
      ImportTarget::Existing { .. } => unmapped_required_fields.is_empty(),
      ImportTarget::New { .. } => true,
    };

  ImportPreviewResponse {
    detected_columns,
    rows,
    invalid_lines: document.invalid_lines,
    summary: ImportPreviewSummary {
      total_parsed: valid + invalid,
      valid,
      invalid,
      duplicates,
      missing_required,
      importable,
    },
    suggested_new_fields: suggested_new_fields.clone(),
    unmapped_required_fields,
    ready_for_commit,
  }
}

#[cfg(test)]
mod tests {
  use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
  };

  use super::parse_import_file;

  #[test]
  fn parses_bom_crlf_and_variable_columns() {
    let unique = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .unwrap()
      .as_nanos();
    let path = std::env::temp_dir().join(format!("flashcard-import-{unique}.txt"));
    fs::write(
      &path,
      "\u{FEFF}Persian | English | Example\r\nسلام | Hello | Hi there\r\nکتاب | Book | \r\n",
    )
    .unwrap();

    let parsed = parse_import_file(&path, "|", true).unwrap();
    assert_eq!(parsed.header_labels.unwrap()[0], "Persian");
    assert_eq!(parsed.column_count, 3);
    assert_eq!(parsed.rows.len(), 2);

    let _ = fs::remove_file(path);
  }
}
