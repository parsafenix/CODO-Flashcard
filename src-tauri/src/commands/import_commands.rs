use std::collections::HashSet;
use std::path::Path;

use anyhow::Context;
use tauri::State;

use crate::{
  db::{
    open_connection,
    repository::{card_repo, deck_repo, dynamic_repo},
  },
  models::{
    error::AppError,
    types::{CommitImportRequest, CreateDeckInput, ImportCommitResponse, ImportPreviewRequest, ImportPreviewResponse, ImportTarget},
  },
  services::importer,
  AppState,
};

fn map_error(error: anyhow::Error) -> AppError {
  AppError::new("import_error", error.to_string())
}

#[tauri::command]
pub fn preview_import(state: State<'_, AppState>, request: ImportPreviewRequest) -> Result<ImportPreviewResponse, AppError> {
  let connection = open_connection(&state.db_path).map_err(AppError::from)?;
  if matches!(request.target, ImportTarget::Existing { .. }) {
    importer::validate_existing_mapping_uniqueness(&request.mappings).map_err(map_error)?;
  }
  let document = importer::parse_import_file(Path::new(&request.file_path), &request.delimiter, request.has_header)
    .map_err(map_error)?;

  let (existing_keys, existing_fields) = match &request.target {
    ImportTarget::Existing { deck_id } => (
      card_repo::get_existing_dedupe_keys(&connection, *deck_id).map_err(map_error)?,
      dynamic_repo::list_deck_fields(&connection, *deck_id).map_err(map_error)?,
    ),
    ImportTarget::New { .. } => (HashSet::new(), Vec::new()),
  };

  Ok(importer::build_preview(
    document,
    &request.target,
    &existing_fields,
    existing_keys,
    &request.mappings,
    request.create_fields_from_header,
  ))
}

#[tauri::command]
pub fn commit_import(state: State<'_, AppState>, request: CommitImportRequest) -> Result<ImportCommitResponse, AppError> {
  let mut connection = open_connection(&state.db_path).map_err(AppError::from)?;
  if matches!(request.target, ImportTarget::Existing { .. }) {
    importer::validate_existing_mapping_uniqueness(&request.mappings).map_err(map_error)?;
  }
  let document = importer::parse_import_file(Path::new(&request.file_path), &request.delimiter, request.has_header)
    .map_err(map_error)?;
  let invalid = document.invalid_lines.len();
  let total_parsed = document.rows.len() + invalid;
  let transaction = connection.transaction().map_err(|error| AppError::new("import_error", error.to_string()))?;

  let (initial_existing_keys, existing_fields) = match &request.target {
    ImportTarget::Existing { deck_id } => (
      card_repo::get_existing_dedupe_keys(&transaction, *deck_id).map_err(map_error)?,
      dynamic_repo::list_deck_fields(&transaction, *deck_id).map_err(map_error)?,
    ),
    ImportTarget::New { .. } => (HashSet::new(), Vec::new()),
  };

  let preview = importer::build_preview(
    document.clone(),
    &request.target,
    &existing_fields,
    initial_existing_keys,
    &request.mappings,
    request.create_fields_from_header,
  );

  if !preview.unmapped_required_fields.is_empty() {
    return Err(AppError::new(
      "import_error",
      format!(
        "Map every required deck field before importing: {}",
        preview.unmapped_required_fields.join(", ")
      ),
    ));
  }

  let (deck_id, field_mapping) = match &request.target {
    ImportTarget::Existing { deck_id } => {
      deck_repo::get_deck(&transaction, *deck_id)
        .map_err(map_error)?
        .context("Deck not found.")
        .map_err(map_error)?;
      (
        *deck_id,
        importer::derive_existing_field_mapping(&existing_fields, &document, &request.mappings),
      )
    }
    ImportTarget::New { name, description } => {
      let field_inputs = importer::derive_new_fields(&document, request.create_fields_from_header, &request.mappings);
      let created = deck_repo::create_deck(
        &transaction,
        &CreateDeckInput {
          name: name.clone(),
          description: description.clone(),
          fields: field_inputs,
        },
      )
      .map_err(map_error)?;
      let mapping = created
        .fields
        .iter()
        .enumerate()
        .map(|(index, field)| (index, field.id))
        .collect::<std::collections::HashMap<_, _>>();
      (created.id, mapping)
    }
  };

  let preview_rows_by_line = preview
    .rows
    .iter()
    .map(|row| (row.line_number, row))
    .collect::<std::collections::HashMap<_, _>>();
  let mut imported = 0usize;
  let mut duplicates = 0usize;
  let mut skipped = 0usize;

  for row in document.rows {
    let preview_row = preview_rows_by_line
      .get(&row.line_number)
      .cloned()
      .context("Import preview row missing")
      .map_err(map_error)?;

    if preview_row.duplicate {
      duplicates += 1;
      skipped += 1;
      continue;
    }

    if !preview_row.missing_required_fields.is_empty() {
      skipped += 1;
      continue;
    }

    let values = field_mapping
      .iter()
      .filter_map(|(column_index, field_id)| {
        row.columns
          .get(*column_index)
          .map(|value| (*field_id, value.clone()))
      })
      .collect::<Vec<_>>();

    match card_repo::insert_import_card(&transaction, deck_id, &values) {
      Ok(_) => {
        imported += 1;
      }
      Err(error) => {
        if error.to_string() == "duplicate_card" {
          duplicates += 1;
          skipped += 1;
        } else {
          return Err(map_error(error));
        }
      }
    }
  }

  transaction.commit().map_err(|error| AppError::new("import_error", error.to_string()))?;

  Ok(ImportCommitResponse {
    deck_id,
    total_parsed,
    imported,
    skipped,
    invalid,
    duplicates,
  })
}
