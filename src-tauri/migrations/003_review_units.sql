CREATE TABLE IF NOT EXISTS review_units (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  card_id INTEGER NOT NULL REFERENCES cards(id) ON DELETE CASCADE,
  deck_id INTEGER NOT NULL REFERENCES decks(id) ON DELETE CASCADE,
  prompt_field_id INTEGER NOT NULL REFERENCES deck_fields(id) ON DELETE CASCADE,
  reveal_field_ids TEXT NOT NULL DEFAULT '[]',
  direction_key TEXT NOT NULL,
  state TEXT NOT NULL DEFAULT 'new'
    CHECK (state IN ('new', 'learning', 'review', 'relearning', 'leech')),
  difficulty REAL NOT NULL DEFAULT 5.0,
  stability REAL NOT NULL DEFAULT 0.2,
  scheduled_interval_days REAL NOT NULL DEFAULT 0.0,
  last_reviewed_at_utc TEXT,
  due_at_utc TEXT,
  lapses INTEGER NOT NULL DEFAULT 0,
  successful_reviews INTEGER NOT NULL DEFAULT 0,
  failed_reviews INTEGER NOT NULL DEFAULT 0,
  total_reviews INTEGER NOT NULL DEFAULT 0,
  same_day_reviews_count INTEGER NOT NULL DEFAULT 0,
  average_latency_ms REAL,
  last_latency_ms INTEGER,
  hint_used_last INTEGER NOT NULL DEFAULT 0,
  confidence_last REAL,
  suspended INTEGER NOT NULL DEFAULT 0,
  leech INTEGER NOT NULL DEFAULT 0,
  mastered INTEGER NOT NULL DEFAULT 0,
  learning_step_index INTEGER NOT NULL DEFAULT 0,
  relearning_step_index INTEGER NOT NULL DEFAULT 0,
  first_reviewed_at_utc TEXT,
  graduated_at_utc TEXT,
  mastered_at_utc TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  UNIQUE(card_id, prompt_field_id, direction_key)
);

CREATE TABLE IF NOT EXISTS review_logs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  review_unit_id INTEGER NOT NULL REFERENCES review_units(id) ON DELETE CASCADE,
  card_id INTEGER NOT NULL REFERENCES cards(id) ON DELETE CASCADE,
  deck_id INTEGER NOT NULL REFERENCES decks(id) ON DELETE CASCADE,
  session_id INTEGER REFERENCES study_sessions(id) ON DELETE SET NULL,
  reviewed_at_utc TEXT NOT NULL,
  rating TEXT NOT NULL CHECK (rating IN ('again', 'hard', 'good', 'easy')),
  was_correct INTEGER NOT NULL,
  state_before TEXT NOT NULL,
  state_after TEXT NOT NULL,
  retrievability_before REAL,
  difficulty_before REAL NOT NULL,
  difficulty_after REAL NOT NULL,
  stability_before REAL NOT NULL,
  stability_after REAL NOT NULL,
  interval_before_days REAL NOT NULL,
  interval_after_days REAL NOT NULL,
  scheduled_due_before_utc TEXT,
  scheduled_due_after_utc TEXT,
  elapsed_days REAL,
  latency_ms INTEGER,
  hint_used INTEGER NOT NULL DEFAULT 0,
  confidence REAL,
  leech_before INTEGER NOT NULL DEFAULT 0,
  leech_after INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_review_units_due
  ON review_units(deck_id, prompt_field_id, direction_key, due_at_utc, state, suspended);
CREATE INDEX IF NOT EXISTS idx_review_units_card ON review_units(card_id, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_review_logs_unit_reviewed
  ON review_logs(review_unit_id, reviewed_at_utc DESC);
CREATE INDEX IF NOT EXISTS idx_review_logs_deck_reviewed
  ON review_logs(deck_id, reviewed_at_utc DESC);
