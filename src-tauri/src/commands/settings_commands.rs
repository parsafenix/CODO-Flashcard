use std::{fs, path::PathBuf, process::Command};

use tauri::State;

use crate::{
  db::{initialize_database, open_connection},
  models::{
    error::AppError,
    types::{AppSettings, BackupResult, ExportDeckInput},
  },
  services::{backup, exporter},
  AppState,
};

fn map_error(error: anyhow::Error) -> AppError {
  AppError::new("settings_error", error.to_string())
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, AppError> {
  let connection = open_connection(&state.db_path).map_err(AppError::from)?;
  crate::db::repository::settings_repo::get_settings(&connection).map_err(map_error)
}

#[tauri::command]
pub fn update_settings(state: State<'_, AppState>, settings: AppSettings) -> Result<AppSettings, AppError> {
  let connection = open_connection(&state.db_path).map_err(AppError::from)?;
  crate::db::repository::settings_repo::save_settings(&connection, &settings).map_err(map_error)
}

#[tauri::command]
pub fn export_deck(state: State<'_, AppState>, input: ExportDeckInput) -> Result<(), AppError> {
  let connection = open_connection(&state.db_path).map_err(AppError::from)?;
  exporter::export_deck(&connection, &input).map_err(map_error)
}

#[tauri::command]
pub fn create_backup(
  state: State<'_, AppState>,
  directory_path: String,
) -> Result<BackupResult, AppError> {
  let connection = open_connection(&state.db_path).map_err(AppError::from)?;
  let result = backup::create_backup(&connection, PathBuf::from(directory_path).as_path()).map_err(map_error)?;

  let mut settings = crate::db::repository::settings_repo::get_settings(&connection).map_err(map_error)?;
  settings.last_backup_directory = Some(
    PathBuf::from(&result.output_path)
      .parent()
      .map(|path| path.to_string_lossy().to_string())
      .unwrap_or_default(),
  );
  let _ = crate::db::repository::settings_repo::save_settings(&connection, &settings);

  Ok(result)
}

#[tauri::command]
pub fn reset_app_data(state: State<'_, AppState>) -> Result<(), AppError> {
  let sidecars = backup::sidecar_paths(&state.db_path);
  for path in sidecars {
    if path.exists() {
      fs::remove_file(path).map_err(|error| AppError::new("settings_error", error.to_string()))?;
    }
  }
  initialize_database(&state.db_path).map_err(AppError::from)
}

#[tauri::command]
pub fn open_data_folder(state: State<'_, AppState>) -> Result<String, AppError> {
  let directory = state
    .db_path
    .parent()
    .map(PathBuf::from)
    .ok_or_else(|| AppError::new("settings_error", "App data directory not found."))?;

  #[cfg(target_os = "windows")]
  let mut command = {
    let mut command = Command::new("explorer");
    command.arg(directory.as_os_str());
    command
  };

  #[cfg(target_os = "macos")]
  let mut command = {
    let mut command = Command::new("open");
    command.arg(directory.as_os_str());
    command
  };

  #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
  let mut command = {
    let mut command = Command::new("xdg-open");
    command.arg(directory.as_os_str());
    command
  };

  command
    .spawn()
    .map_err(|error| AppError::new("settings_error", error.to_string()))?;

  Ok(directory.to_string_lossy().to_string())
}
