use tauri::State;

use crate::{
  db::{open_connection, repository::settings_repo},
  models::{
    error::AppError,
    types::{RunCalibrationRequest, SchedulerCalibrationStatus},
  },
  services::calibration,
  AppState,
};

fn map_error(error: anyhow::Error) -> AppError {
  AppError::new("calibration_error", error.to_string())
}

#[tauri::command]
pub fn get_scheduler_calibration_status(state: State<'_, AppState>) -> Result<SchedulerCalibrationStatus, AppError> {
  let connection = open_connection(&state.db_path).map_err(AppError::from)?;
  calibration::get_calibration_status(&connection).map_err(map_error)
}

#[tauri::command]
pub fn run_scheduler_calibration(
  state: State<'_, AppState>,
  _request: RunCalibrationRequest,
) -> Result<SchedulerCalibrationStatus, AppError> {
  let connection = open_connection(&state.db_path).map_err(AppError::from)?;
  let settings = settings_repo::get_settings(&connection).map_err(map_error)?;
  calibration::run_calibration(&connection, &settings).map_err(map_error)
}
