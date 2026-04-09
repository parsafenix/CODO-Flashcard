use std::{collections::HashSet, fs, path::Path};

use anyhow::{bail, Result};

use crate::{
  models::types::{
    ImportPreviewResponse, ImportPreviewRow, ImportPreviewSummary, ImportTarget, InvalidImportLine,
    ParsedImportCard, ParsedImportDocument,
  },
  services::normalization::normalize_card_fields,
};

pub fn parse_import_file(path: &Path, delimiter: &str, has_header: bool) -> Result<ParsedImportDocument> {
  let delimiter = delimiter.trim();
  if delimiter.is_empty() {
    bail!("Import delimiter cannot be empty");
  }

  let content = fs::read_to_string(path)?;
  let mut header_labels = None;
  let mut cards = Vec::new();
  let mut invalid_lines = Vec::new();

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

    let columns: Vec<String> = trimmed
      .split(delimiter)
      .map(|column| column.trim().to_string())
      .collect();

    if columns.len() != 3 || columns.iter().any(|column| column.is_empty()) {
      invalid_lines.push(InvalidImportLine {
        line_number,
        raw: trimmed.to_string(),
        reason: "Expected exactly 3 non-empty columns.".to_string(),
      });
      continue;
    }

    if has_header && header_labels.is_none() {
      header_labels = Some([
        columns[0].clone(),
        columns[1].clone(),
        columns[2].clone(),
      ]);
      continue;
    }

    let normalized = normalize_card_fields(&columns[0], &columns[1], &columns[2]);
    cards.push(ParsedImportCard {
      line_number,
      language_1: columns[0].clone(),
      language_2: columns[1].clone(),
      language_3: columns[2].clone(),
      dedupe_key: normalized.dedupe_key,
    });
  }

  Ok(ParsedImportDocument {
    header_labels,
    cards,
    invalid_lines,
  })
}

pub fn build_preview(
  document: ParsedImportDocument,
  target: &ImportTarget,
  existing_keys: HashSet<String>,
  existing_labels: Option<[String; 3]>,
) -> ImportPreviewResponse {
  let mut seen_batch_keys = HashSet::new();
  let mut rows = Vec::new();
  let mut duplicates = 0usize;

  for card in document.cards {
    let duplicate_reason = if existing_keys.contains(&card.dedupe_key) {
      Some("Already exists in this deck.".to_string())
    } else if !seen_batch_keys.insert(card.dedupe_key.clone()) {
      Some("Repeated within this import file.".to_string())
    } else {
      None
    };

    let duplicate = duplicate_reason.is_some();
    if duplicate {
      duplicates += 1;
    }

    rows.push(ImportPreviewRow {
      line_number: card.line_number,
      language_1: card.language_1,
      language_2: card.language_2,
      language_3: card.language_3,
      duplicate,
      duplicate_reason,
    });
  }

  let valid = rows.len();
  let invalid = document.invalid_lines.len();
  let importable = valid.saturating_sub(duplicates);
  let can_update_existing_labels = matches!(target, ImportTarget::Existing { .. })
    && document
      .header_labels
      .as_ref()
      .zip(existing_labels)
      .map(|(header, existing)| header != &existing)
      .unwrap_or(false);

  ImportPreviewResponse {
    rows,
    invalid_lines: document.invalid_lines,
    summary: ImportPreviewSummary {
      total_parsed: valid + invalid,
      valid,
      invalid,
      duplicates,
      importable,
    },
    header_labels: document.header_labels,
    can_update_existing_labels,
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
  fn parses_bom_and_crlf() {
    let unique = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .unwrap()
      .as_nanos();
    let path = std::env::temp_dir().join(format!("flashcard-import-{unique}.txt"));
    fs::write(
      &path,
      "\u{FEFF}Persian | English | Italian\r\n\u{0633}\u{0644}\u{0627}\u{0645} | Hello | Ciao\r\n",
    )
    .unwrap();

    let parsed = parse_import_file(&path, "|", true).unwrap();
    assert_eq!(parsed.header_labels.unwrap()[0], "Persian");
    assert_eq!(parsed.cards.len(), 1);

    let _ = fs::remove_file(path);
  }
}
