use anyhow::Result;
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};

use crate::models::types::AppSettings;

fn now_utc() -> String {
  Utc::now().to_rfc3339()
}

pub fn ensure_default_settings(connection: &Connection) -> Result<()> {
  let exists = connection
    .query_row(
      "SELECT COUNT(*) FROM app_settings WHERE key = 'app'",
      [],
      |row| row.get::<_, i64>(0),
    )?;

  if exists == 0 {
    let settings = serde_json::to_string(&AppSettings::default())?;
    connection.execute(
      "INSERT INTO app_settings (key, value, updated_at) VALUES ('app', ?1, ?2)",
      params![settings, now_utc()],
    )?;
  }

  Ok(())
}

pub fn get_settings(connection: &Connection) -> Result<AppSettings> {
  let raw = connection
    .query_row("SELECT value FROM app_settings WHERE key = 'app'", [], |row| row.get::<_, String>(0))
    .optional()?;

  let settings = raw
    .and_then(|json| serde_json::from_str::<AppSettings>(&json).ok())
    .unwrap_or_default()
    .validate();

  save_settings(connection, &settings)?;
  Ok(settings)
}

pub fn save_settings(connection: &Connection, settings: &AppSettings) -> Result<AppSettings> {
  let normalized = settings.clone().validate();
  connection.execute(
    "INSERT INTO app_settings (key, value, updated_at)
      VALUES ('app', ?1, ?2)
      ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
    params![serde_json::to_string(&normalized)?, now_utc()],
  )?;
  Ok(normalized)
}

