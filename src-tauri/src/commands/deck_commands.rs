use tauri::State;

use crate::{
  db::{open_connection, repository::deck_repo},
  models::{
    error::AppError,
    types::{CreateDeckInput, DeckDetail, DeckSummary, UpdateDeckInput},
  },
  AppState,
};

fn map_error(error: anyhow::Error) -> AppError {
  let message = error.to_string();
  if message == "Deck name is required." {
    return AppError::field("validation", message, "name");
  }
  if message == "At least 2 active fields are required."
    || message == "At least 1 active required field is required."
    || message == "Every field needs a label."
  {
    return AppError::field("validation", message, "fields");
  }
  AppError::new("deck_error", message)
}

#[tauri::command]
pub fn list_decks(state: State<'_, AppState>, search: String) -> Result<Vec<DeckSummary>, AppError> {
  let connection = open_connection(&state.db_path).map_err(AppError::from)?;
  deck_repo::list_decks(&connection, &search).map_err(map_error)
}

#[tauri::command]
pub fn get_deck(state: State<'_, AppState>, deck_id: i64) -> Result<DeckDetail, AppError> {
  let connection = open_connection(&state.db_path).map_err(AppError::from)?;
  deck_repo::get_deck(&connection, deck_id)
    .map_err(map_error)?
    .ok_or_else(|| AppError::new("not_found", "Deck not found."))
}

#[tauri::command]
pub fn create_deck(state: State<'_, AppState>, input: CreateDeckInput) -> Result<DeckDetail, AppError> {
  let connection = open_connection(&state.db_path).map_err(AppError::from)?;
  deck_repo::create_deck(&connection, &input).map_err(map_error)
}

#[tauri::command]
pub fn update_deck(state: State<'_, AppState>, input: UpdateDeckInput) -> Result<DeckDetail, AppError> {
  let connection = open_connection(&state.db_path).map_err(AppError::from)?;
  deck_repo::update_deck(&connection, &input).map_err(map_error)
}

#[tauri::command]
pub fn delete_deck(state: State<'_, AppState>, deck_id: i64) -> Result<(), AppError> {
  let connection = open_connection(&state.db_path).map_err(AppError::from)?;
  deck_repo::delete_deck(&connection, deck_id).map_err(map_error)
}

#[tauri::command]
pub fn duplicate_deck(state: State<'_, AppState>, deck_id: i64) -> Result<DeckDetail, AppError> {
  let mut connection = open_connection(&state.db_path).map_err(AppError::from)?;
  deck_repo::duplicate_deck(&mut connection, deck_id).map_err(map_error)
}
