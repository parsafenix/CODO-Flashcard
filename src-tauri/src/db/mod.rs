mod migrations;
pub mod repository;

use std::path::Path;

use anyhow::Result;
use rusqlite::Connection;

pub fn open_connection(path: &Path) -> Result<Connection> {
  let connection = Connection::open(path)?;
  connection.pragma_update(None, "foreign_keys", "ON")?;
  connection.pragma_update(None, "journal_mode", "WAL")?;
  connection.busy_timeout(std::time::Duration::from_secs(5))?;
  Ok(connection)
}

pub fn initialize_database(path: &Path) -> Result<()> {
  let connection = open_connection(path)?;
  migrations::run_migrations(&connection)?;
  repository::dynamic_repo::repair_dynamic_model(&connection)?;
  repository::settings_repo::ensure_default_settings(&connection)?;
  Ok(())
}
