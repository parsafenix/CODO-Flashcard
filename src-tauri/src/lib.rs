pub mod commands;
pub mod db;
pub mod models;
pub mod services;

use std::path::PathBuf;

use tauri::Manager;

#[derive(Clone)]
pub struct AppState {
  pub db_path: PathBuf,
}

fn initialize_database(handle: &tauri::AppHandle) -> Result<AppState, String> {
  let app_data_dir = handle
    .path()
    .app_data_dir()
    .map_err(|error| format!("Unable to resolve app data directory: {error}"))?;

  std::fs::create_dir_all(&app_data_dir)
    .map_err(|error| format!("Unable to create app data directory: {error}"))?;

  let db_path = app_data_dir.join("flashcard-local.sqlite");
  db::initialize_database(&db_path).map_err(|error| error.to_string())?;

  Ok(AppState { db_path })
}

pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_dialog::init())
    .setup(|app| {
      let state = initialize_database(app.handle())?;
      app.manage(state);
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      commands::analytics_commands::get_analytics,
      commands::deck_commands::list_decks,
      commands::deck_commands::get_deck,
      commands::deck_commands::create_deck,
      commands::deck_commands::update_deck,
      commands::deck_commands::delete_deck,
      commands::deck_commands::duplicate_deck,
      commands::card_commands::list_cards,
      commands::card_commands::create_card,
      commands::card_commands::update_card,
      commands::card_commands::delete_card,
      commands::import_commands::preview_import,
      commands::import_commands::commit_import,
      commands::study_commands::start_study_session,
      commands::study_commands::grade_card,
      commands::study_commands::complete_study_session,
      commands::settings_commands::get_settings,
      commands::settings_commands::update_settings,
      commands::settings_commands::create_backup,
      commands::settings_commands::reset_app_data,
      commands::settings_commands::open_data_folder,
      commands::settings_commands::export_deck
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
