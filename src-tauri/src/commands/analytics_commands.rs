use tauri::State;

use crate::{
  db::{open_connection, repository::settings_repo},
  models::{
    error::AppError,
    types::{AnalyticsRequest, AnalyticsResponse},
  },
  services::analytics,
  AppState,
};

fn map_error(error: anyhow::Error) -> AppError {
  AppError::new("analytics_error", error.to_string())
}

#[tauri::command]
pub fn get_analytics(
  state: State<'_, AppState>,
  request: AnalyticsRequest,
) -> Result<AnalyticsResponse, AppError> {
  let connection = open_connection(&state.db_path).map_err(AppError::from)?;
  let settings = settings_repo::get_settings(&connection).map_err(map_error)?;
  analytics::get_analytics(&connection, &settings, &request).map_err(map_error)
}
