use std::{fs, path::{Path, PathBuf}};

use anyhow::Result;
use chrono::Utc;
use rusqlite::Connection;

use crate::models::types::BackupResult;

fn escape_sqlite_string(value: &str) -> String {
  value.replace('\'', "''")
}

pub fn create_backup(connection: &Connection, destination_directory: &Path) -> Result<BackupResult> {
  fs::create_dir_all(destination_directory)?;
  let timestamp = Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
  let db_filename = format!("flashcard-local-backup-{timestamp}.sqlite");
  let db_output_path = destination_directory.join(db_filename);
  let escaped_path = escape_sqlite_string(&db_output_path.to_string_lossy());
  connection.execute_batch(&format!("VACUUM INTO '{escaped_path}'"))?;

  let manifest_path = destination_directory.join(format!("flashcard-local-backup-{timestamp}.json"));
  let manifest = serde_json::json!({
    "created_at": Utc::now().to_rfc3339(),
    "database_file": db_output_path.file_name().and_then(|name| name.to_str()).unwrap_or_default(),
    "type": "sqlite-backup"
  });
  fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)?;

  Ok(BackupResult {
    output_path: db_output_path.to_string_lossy().to_string(),
    manifest_path: manifest_path.to_string_lossy().to_string(),
  })
}

pub fn sidecar_paths(db_path: &Path) -> Vec<PathBuf> {
  vec![
    db_path.to_path_buf(),
    db_path.with_extension("sqlite-wal"),
    db_path.with_extension("sqlite-shm"),
  ]
}
