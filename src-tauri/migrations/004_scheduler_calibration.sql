CREATE TABLE IF NOT EXISTS scheduler_parameter_profiles (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  profile_key TEXT NOT NULL UNIQUE,
  profile_version TEXT NOT NULL,
  label TEXT NOT NULL,
  source TEXT NOT NULL CHECK (source IN ('default', 'calibrated')),
  is_active INTEGER NOT NULL DEFAULT 0,
  parameters_json TEXT NOT NULL,
  metrics_json TEXT,
  notes TEXT,
  created_at TEXT NOT NULL,
  activated_at TEXT
);

CREATE TABLE IF NOT EXISTS scheduler_calibration_runs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  started_at TEXT NOT NULL,
  completed_at TEXT,
  status TEXT NOT NULL CHECK (status IN ('insufficient_data', 'completed', 'accepted', 'rejected', 'failed')),
  accepted INTEGER NOT NULL DEFAULT 0,
  used_recency_weighting INTEGER NOT NULL DEFAULT 0,
  recency_half_life_days REAL,
  total_events INTEGER NOT NULL DEFAULT 0,
  usable_events INTEGER NOT NULL DEFAULT 0,
  filtered_events INTEGER NOT NULL DEFAULT 0,
  distinct_review_units INTEGER NOT NULL DEFAULT 0,
  deck_coverage_count INTEGER NOT NULL DEFAULT 0,
  mature_review_events INTEGER NOT NULL DEFAULT 0,
  failure_events INTEGER NOT NULL DEFAULT 0,
  train_events INTEGER NOT NULL DEFAULT 0,
  validation_events INTEGER NOT NULL DEFAULT 0,
  test_events INTEGER NOT NULL DEFAULT 0,
  split_train_end_utc TEXT,
  split_validation_end_utc TEXT,
  profile_before_id INTEGER REFERENCES scheduler_parameter_profiles(id) ON DELETE SET NULL,
  profile_candidate_id INTEGER REFERENCES scheduler_parameter_profiles(id) ON DELETE SET NULL,
  profile_after_id INTEGER REFERENCES scheduler_parameter_profiles(id) ON DELETE SET NULL,
  baseline_metrics_json TEXT,
  candidate_metrics_json TEXT,
  diagnostics_json TEXT,
  workload_json TEXT,
  reason TEXT
);

CREATE INDEX IF NOT EXISTS idx_scheduler_profiles_active ON scheduler_parameter_profiles(is_active, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_scheduler_runs_completed ON scheduler_calibration_runs(completed_at DESC, accepted DESC);
