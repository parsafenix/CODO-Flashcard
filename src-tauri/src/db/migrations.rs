use anyhow::Result;
use rusqlite::{params, Connection};

const MIGRATIONS: [(&str, &str); 4] = [
  ("001_initial.sql", include_str!("../../migrations/001_initial.sql")),
  ("002_dynamic_fields.sql", include_str!("../../migrations/002_dynamic_fields.sql")),
  ("003_review_units.sql", include_str!("../../migrations/003_review_units.sql")),
  ("004_scheduler_calibration.sql", include_str!("../../migrations/004_scheduler_calibration.sql")),
];

pub fn run_migrations(connection: &Connection) -> Result<()> {
  connection.execute(
    "CREATE TABLE IF NOT EXISTS schema_migrations (
      version TEXT PRIMARY KEY,
      applied_at TEXT NOT NULL
    )",
    [],
  )?;

  for (version, sql) in MIGRATIONS {
    let applied = connection.query_row(
      "SELECT COUNT(*) FROM schema_migrations WHERE version = ?1",
      [version],
      |row| row.get::<_, i64>(0),
    )?;

    if applied == 0 {
      connection.execute_batch(sql)?;
      connection.execute(
        "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, datetime('now'))",
        params![version],
      )?;
    }
  }

  Ok(())
}
