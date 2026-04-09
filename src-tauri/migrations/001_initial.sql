CREATE TABLE IF NOT EXISTS decks (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL,
  description TEXT,
  language_1_label TEXT NOT NULL DEFAULT 'Language 1',
  language_2_label TEXT NOT NULL DEFAULT 'Language 2',
  language_3_label TEXT NOT NULL DEFAULT 'Language 3',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  last_studied_at TEXT
);

CREATE TABLE IF NOT EXISTS cards (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  deck_id INTEGER NOT NULL REFERENCES decks(id) ON DELETE CASCADE,
  language_1 TEXT NOT NULL,
  language_2 TEXT NOT NULL,
  language_3 TEXT NOT NULL,
  note TEXT,
  example_sentence TEXT,
  tag TEXT,
  language_1_normalized TEXT NOT NULL,
  language_2_normalized TEXT NOT NULL,
  language_3_normalized TEXT NOT NULL,
  language_1_compact TEXT NOT NULL,
  language_2_compact TEXT NOT NULL,
  language_3_compact TEXT NOT NULL,
  dedupe_key TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  last_reviewed_at TEXT,
  next_review_at TEXT,
  review_count INTEGER NOT NULL DEFAULT 0,
  correct_count INTEGER NOT NULL DEFAULT 0,
  wrong_count INTEGER NOT NULL DEFAULT 0,
  current_interval_minutes INTEGER NOT NULL DEFAULT 0,
  ease_factor REAL NOT NULL DEFAULT 2.2,
  mastery_score INTEGER NOT NULL DEFAULT 0,
  status TEXT NOT NULL DEFAULT 'new'
    CHECK (status IN ('new', 'learning', 'review', 'mastered')),
  UNIQUE(deck_id, dedupe_key)
);

CREATE TABLE IF NOT EXISTS review_history (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  card_id INTEGER NOT NULL REFERENCES cards(id) ON DELETE CASCADE,
  deck_id INTEGER NOT NULL REFERENCES decks(id) ON DELETE CASCADE,
  session_id INTEGER REFERENCES study_sessions(id) ON DELETE SET NULL,
  reviewed_at TEXT NOT NULL,
  knew_it INTEGER NOT NULL,
  previous_status TEXT NOT NULL,
  new_status TEXT NOT NULL,
  previous_interval_minutes INTEGER NOT NULL,
  new_interval_minutes INTEGER NOT NULL,
  previous_ease_factor REAL NOT NULL,
  new_ease_factor REAL NOT NULL,
  previous_mastery_score INTEGER NOT NULL,
  new_mastery_score INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS study_sessions (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  deck_id INTEGER NOT NULL REFERENCES decks(id) ON DELETE CASCADE,
  started_at TEXT NOT NULL,
  completed_at TEXT,
  mode TEXT NOT NULL,
  prompt_language TEXT NOT NULL,
  studied_count INTEGER NOT NULL DEFAULT 0,
  correct_count INTEGER NOT NULL DEFAULT 0,
  wrong_count INTEGER NOT NULL DEFAULT 0,
  newly_mastered_count INTEGER NOT NULL DEFAULT 0,
  accuracy_percent INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS app_settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_cards_deck_next_review ON cards(deck_id, next_review_at, status);
CREATE INDEX IF NOT EXISTS idx_cards_deck_status ON cards(deck_id, status);
CREATE INDEX IF NOT EXISTS idx_cards_search_normalized ON cards(deck_id, language_1_normalized, language_2_normalized, language_3_normalized);
CREATE INDEX IF NOT EXISTS idx_review_history_card ON review_history(card_id, reviewed_at DESC);
CREATE INDEX IF NOT EXISTS idx_sessions_deck ON study_sessions(deck_id, started_at DESC);

