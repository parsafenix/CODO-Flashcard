use std::collections::HashSet;
use std::path::Path;

use anyhow::Context;
use tauri::State;

use crate::{
  db::{
    open_connection,
    repository::{card_repo, deck_repo},
  },
  models::{
    error::AppError,
    types::{
      CommitImportRequest, CreateDeckInput, ImportCommitResponse, ImportPreviewRequest, ImportPreviewResponse,
      ImportTarget,
    },
  },
  services::importer,
  AppState,
};

fn map_error(error: anyhow::Error) -> AppError {
  AppError::new("import_error", error.to_string())
}

fn get_existing_labels(
  connection: &rusqlite::Connection,
  deck_id: i64,
) -> Result<[String; 3], anyhow::Error> {
  let deck = deck_repo::get_deck(connection, deck_id)?.context("Deck not found.")?;
  Ok([
    deck.language_1_label,
    deck.language_2_label,
    deck.language_3_label,
  ])
}

#[tauri::command]
pub fn preview_import(
  state: State<'_, AppState>,
  request: ImportPreviewRequest,
) -> Result<ImportPreviewResponse, AppError> {
  let connection = open_connection(&state.db_path).map_err(AppError::from)?;
  let document = importer::parse_import_file(Path::new(&request.file_path), &request.delimiter, request.has_header)
    .map_err(map_error)?;

  let (existing_keys, existing_labels) = match &request.target {
    ImportTarget::Existing { deck_id } => (
      card_repo::get_existing_dedupe_keys(&connection, *deck_id).map_err(map_error)?,
      Some(get_existing_labels(&connection, *deck_id).map_err(map_error)?),
    ),
    ImportTarget::New { .. } => (HashSet::new(), None),
  };

  Ok(importer::build_preview(
    document,
    &request.target,
    existing_keys,
    existing_labels,
  ))
}

#[tauri::command]
pub fn commit_import(
  state: State<'_, AppState>,
  request: CommitImportRequest,
) -> Result<ImportCommitResponse, AppError> {
  let mut connection = open_connection(&state.db_path).map_err(AppError::from)?;
  let document = importer::parse_import_file(Path::new(&request.file_path), &request.delimiter, request.has_header)
    .map_err(map_error)?;
  let invalid = document.invalid_lines.len();
  let total_parsed = document.cards.len() + invalid;
  let transaction = connection.transaction().map_err(|error| AppError::new("import_error", error.to_string()))?;

  let deck_id = match &request.target {
    ImportTarget::Existing { deck_id } => {
      deck_repo::get_deck(&transaction, *deck_id)
        .map_err(map_error)?
        .context("Deck not found.")
        .map_err(map_error)?;
      if request.apply_header_labels_to_existing {
        if let Some(header_labels) = document.header_labels.clone() {
          deck_repo::update_deck_labels(&transaction, *deck_id, header_labels).map_err(map_error)?;
        }
      }
      *deck_id
    }
    ImportTarget::New {
      name,
      description,
      language_1_label,
      language_2_label,
      language_3_label,
    } => {
      let mut create_input = CreateDeckInput {
        name: name.clone(),
        description: description.clone(),
        language_1_label: language_1_label.clone(),
        language_2_label: language_2_label.clone(),
        language_3_label: language_3_label.clone(),
      };
      if let Some(header) = document.header_labels.clone() {
        create_input.language_1_label = Some(header[0].clone());
        create_input.language_2_label = Some(header[1].clone());
        create_input.language_3_label = Some(header[2].clone());
      }
      let created = deck_repo::create_deck(&transaction, &create_input).map_err(map_error)?;
      created.id
    }
  };

  let mut existing_keys = card_repo::get_existing_dedupe_keys(&transaction, deck_id).map_err(map_error)?;
  let mut seen_batch = HashSet::new();
  let mut imported = 0usize;
  let mut duplicates = 0usize;

  for card in document.cards {
    if existing_keys.contains(&card.dedupe_key) || !seen_batch.insert(card.dedupe_key.clone()) {
      duplicates += 1;
      continue;
    }

    match card_repo::insert_import_card(
      &transaction,
      deck_id,
      &card.language_1,
      &card.language_2,
      &card.language_3,
    ) {
      Ok(_) => {
        imported += 1;
        existing_keys.insert(card.dedupe_key);
      }
      Err(error) => {
        if error.to_string() == "duplicate_card" {
          duplicates += 1;
        } else {
          return Err(map_error(error));
        }
      }
    }
  }

  transaction
    .commit()
    .map_err(|error| AppError::new("import_error", error.to_string()))?;

  Ok(ImportCommitResponse {
    deck_id,
    total_parsed,
    imported,
    skipped: duplicates,
    invalid,
    duplicates,
  })
}
