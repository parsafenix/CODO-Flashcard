ALTER TABLE decks ADD COLUMN study_prompt_field_id INTEGER;
ALTER TABLE decks ADD COLUMN study_reveal_field_ids TEXT NOT NULL DEFAULT '[]';

ALTER TABLE study_sessions ADD COLUMN prompt_field_id INTEGER;
ALTER TABLE study_sessions ADD COLUMN reveal_field_ids TEXT NOT NULL DEFAULT '[]';

CREATE TABLE IF NOT EXISTS deck_fields (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  deck_id INTEGER NOT NULL REFERENCES decks(id) ON DELETE CASCADE,
  label TEXT NOT NULL,
  language_code TEXT,
  order_index INTEGER NOT NULL,
  required INTEGER NOT NULL DEFAULT 1,
  active INTEGER NOT NULL DEFAULT 1,
  field_type TEXT NOT NULL DEFAULT 'text',
  system_key TEXT
);

CREATE TABLE IF NOT EXISTS card_values (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  card_id INTEGER NOT NULL REFERENCES cards(id) ON DELETE CASCADE,
  field_id INTEGER NOT NULL REFERENCES deck_fields(id) ON DELETE CASCADE,
  raw_value TEXT NOT NULL,
  normalized_value TEXT NOT NULL DEFAULT '',
  compact_value TEXT NOT NULL DEFAULT '',
  UNIQUE(card_id, field_id)
);

CREATE INDEX IF NOT EXISTS idx_deck_fields_deck_order ON deck_fields(deck_id, active DESC, order_index ASC);
CREATE UNIQUE INDEX IF NOT EXISTS idx_deck_fields_system_key ON deck_fields(deck_id, system_key) WHERE system_key IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_card_values_card_field ON card_values(card_id, field_id);
CREATE INDEX IF NOT EXISTS idx_card_values_search ON card_values(field_id, normalized_value, compact_value);
