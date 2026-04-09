use tauri::State;

use crate::{
  db::{open_connection, repository::card_repo},
  models::{
    error::AppError,
    types::{CardListQuery, CardRecord, CreateCardInput, UpdateCardInput},
  },
  AppState,
};

fn map_error(error: anyhow::Error) -> AppError {
  let message = error.to_string();
  if message == "duplicate_card" {
    return AppError::field(
      "duplicate_card",
      "A card with the same normalized 3-language tuple already exists in this deck.",
      "language_1",
    );
  }
  if message == "card_not_found" {
    return AppError::new("not_found", "Card not found.");
  }
  if message.contains("Language 1") {
    return AppError::field("validation", message, "language_1");
  }
  if message.contains("Language 2") {
    return AppError::field("validation", message, "language_2");
  }
  if message.contains("Language 3") {
    return AppError::field("validation", message, "language_3");
  }
  AppError::new("card_error", message)
}

#[tauri::command]
pub fn list_cards(state: State<'_, AppState>, query: CardListQuery) -> Result<Vec<CardRecord>, AppError> {
  let connection = open_connection(&state.db_path).map_err(AppError::from)?;
  card_repo::list_cards(&connection, &query).map_err(map_error)
}

#[tauri::command]
pub fn create_card(state: State<'_, AppState>, input: CreateCardInput) -> Result<CardRecord, AppError> {
  let connection = open_connection(&state.db_path).map_err(AppError::from)?;
  card_repo::create_card(&connection, &input).map_err(map_error)
}

#[tauri::command]
pub fn update_card(state: State<'_, AppState>, input: UpdateCardInput) -> Result<CardRecord, AppError> {
  let connection = open_connection(&state.db_path).map_err(AppError::from)?;
  card_repo::update_card(&connection, &input).map_err(map_error)
}

#[tauri::command]
pub fn delete_card(state: State<'_, AppState>, card_id: i64) -> Result<(), AppError> {
  let connection = open_connection(&state.db_path).map_err(AppError::from)?;
  card_repo::delete_card(&connection, card_id).map_err(map_error)
}
