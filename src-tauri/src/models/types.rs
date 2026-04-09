use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CardStatus {
  New,
  Learning,
  Review,
  Mastered,
}

impl CardStatus {
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::New => "new",
      Self::Learning => "learning",
      Self::Review => "review",
      Self::Mastered => "mastered",
    }
  }

  pub fn from_db(value: &str) -> Self {
    match value {
      "learning" => Self::Learning,
      "review" => Self::Review,
      "mastered" => Self::Mastered,
      _ => Self::New,
    }
  }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StudyMode {
  Due,
  New,
  Mixed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PromptLanguage {
  #[serde(rename = "language_1")]
  Language1,
  #[serde(rename = "language_2")]
  Language2,
  #[serde(rename = "language_3")]
  Language3,
}

impl PromptLanguage {
  pub fn as_db_field(&self) -> &'static str {
    match self {
      Self::Language1 => "language_1",
      Self::Language2 => "language_2",
      Self::Language3 => "language_3",
    }
  }
}

impl StudyMode {
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::Due => "due",
      Self::New => "new",
      Self::Mixed => "mixed",
    }
  }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
  Light,
  Dark,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckSummary {
  pub id: i64,
  pub name: String,
  pub description: Option<String>,
  pub language_1_label: String,
  pub language_2_label: String,
  pub language_3_label: String,
  pub created_at: String,
  pub updated_at: String,
  pub last_studied_at: Option<String>,
  pub total_cards: i64,
  pub due_cards: i64,
  pub new_cards: i64,
  pub mastered_cards: i64,
}

pub type DeckDetail = DeckSummary;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDeckInput {
  pub name: String,
  pub description: Option<String>,
  pub language_1_label: Option<String>,
  pub language_2_label: Option<String>,
  pub language_3_label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateDeckInput {
  pub id: i64,
  pub name: String,
  pub description: Option<String>,
  pub language_1_label: Option<String>,
  pub language_2_label: Option<String>,
  pub language_3_label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardRecord {
  pub id: i64,
  pub deck_id: i64,
  pub language_1: String,
  pub language_2: String,
  pub language_3: String,
  pub note: Option<String>,
  pub example_sentence: Option<String>,
  pub tag: Option<String>,
  pub created_at: String,
  pub updated_at: String,
  pub last_reviewed_at: Option<String>,
  pub next_review_at: Option<String>,
  pub review_count: i64,
  pub correct_count: i64,
  pub wrong_count: i64,
  pub current_interval_minutes: i64,
  pub ease_factor: f64,
  pub mastery_score: i64,
  pub status: CardStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCardInput {
  pub deck_id: i64,
  pub language_1: String,
  pub language_2: String,
  pub language_3: String,
  pub note: Option<String>,
  pub example_sentence: Option<String>,
  pub tag: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCardInput {
  pub id: i64,
  pub deck_id: i64,
  pub language_1: String,
  pub language_2: String,
  pub language_3: String,
  pub note: Option<String>,
  pub example_sentence: Option<String>,
  pub tag: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CardFilter {
  All,
  New,
  Due,
  Mastered,
  Weak,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CardSort {
  UpdatedDesc,
  CreatedDesc,
  NextReviewAsc,
  Language1Asc,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardListQuery {
  pub deck_id: i64,
  pub search: Option<String>,
  pub filter: Option<CardFilter>,
  pub sort: Option<CardSort>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvalidImportLine {
  pub line_number: usize,
  pub raw: String,
  pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportPreviewRow {
  pub line_number: usize,
  pub language_1: String,
  pub language_2: String,
  pub language_3: String,
  pub duplicate: bool,
  pub duplicate_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportPreviewSummary {
  pub total_parsed: usize,
  pub valid: usize,
  pub invalid: usize,
  pub duplicates: usize,
  pub importable: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportPreviewResponse {
  pub rows: Vec<ImportPreviewRow>,
  pub invalid_lines: Vec<InvalidImportLine>,
  pub summary: ImportPreviewSummary,
  pub header_labels: Option<[String; 3]>,
  pub can_update_existing_labels: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum ImportTarget {
  Existing { deck_id: i64 },
  New {
    name: String,
    description: Option<String>,
    language_1_label: Option<String>,
    language_2_label: Option<String>,
    language_3_label: Option<String>,
  },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportPreviewRequest {
  pub file_path: String,
  pub delimiter: String,
  pub has_header: bool,
  pub target: ImportTarget,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitImportRequest {
  pub file_path: String,
  pub delimiter: String,
  pub has_header: bool,
  pub target: ImportTarget,
  pub apply_header_labels_to_existing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportCommitResponse {
  pub deck_id: i64,
  pub total_parsed: usize,
  pub imported: usize,
  pub skipped: usize,
  pub invalid: usize,
  pub duplicates: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudySessionOptions {
  pub deck_id: i64,
  pub prompt_language: PromptLanguage,
  pub mode: StudyMode,
  pub random_order: bool,
  pub reverse_mode: bool,
  pub cards_limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudyCard {
  pub id: i64,
  pub deck_id: i64,
  pub language_1: String,
  pub language_2: String,
  pub language_3: String,
  pub note: Option<String>,
  pub example_sentence: Option<String>,
  pub tag: Option<String>,
  pub status: CardStatus,
  pub next_review_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudySessionPayload {
  pub session_id: i64,
  pub deck: DeckSummary,
  pub options: StudySessionOptions,
  pub cards: Vec<StudyCard>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradeCardInput {
  pub session_id: i64,
  pub card_id: i64,
  pub knew_it: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradeCardResponse {
  pub card_id: i64,
  pub status: CardStatus,
  pub next_review_at: Option<String>,
  pub current_interval_minutes: i64,
  pub ease_factor: f64,
  pub mastery_score: i64,
  pub newly_mastered: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteStudySessionInput {
  pub session_id: i64,
  pub deck_id: i64,
  pub studied_count: i64,
  pub correct_count: i64,
  pub wrong_count: i64,
  pub newly_mastered_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
  pub session_id: i64,
  pub studied_count: i64,
  pub correct_count: i64,
  pub wrong_count: i64,
  pub accuracy_percent: i64,
  pub newly_mastered_count: i64,
  pub remaining_due_cards: i64,
  pub suggestion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
  pub theme: Theme,
  pub default_prompt_language: PromptLanguage,
  pub cards_per_session: usize,
  pub default_study_mode: StudyMode,
  pub random_order: bool,
  pub reverse_mode: bool,
  pub reveal_all_on_flip: bool,
  pub daily_review_goal: usize,
  pub reminder_enabled: bool,
  pub reminder_time: String,
  pub reminder_last_acknowledged_date: Option<String>,
  pub import_delimiter: String,
  pub last_backup_directory: Option<String>,
}

impl Default for AppSettings {
  fn default() -> Self {
    Self {
      theme: Theme::Dark,
      default_prompt_language: PromptLanguage::Language1,
      cards_per_session: 20,
      default_study_mode: StudyMode::Mixed,
      random_order: true,
      reverse_mode: false,
      reveal_all_on_flip: true,
      daily_review_goal: 20,
      reminder_enabled: false,
      reminder_time: "18:00".to_string(),
      reminder_last_acknowledged_date: None,
      import_delimiter: "|".to_string(),
      last_backup_directory: None,
    }
  }
}

impl AppSettings {
  pub fn validate(mut self) -> Self {
    self.cards_per_session = self.cards_per_session.clamp(1, 200);
    self.daily_review_goal = self.daily_review_goal.clamp(1, 500);
    if self.import_delimiter.trim().is_empty() {
      self.import_delimiter = "|".to_string();
    }
    if self.import_delimiter.chars().count() > 3 {
      self.import_delimiter = "|".to_string();
    }
    if !valid_reminder_time(&self.reminder_time) {
      self.reminder_time = "18:00".to_string();
    }
    self
  }
}

fn valid_reminder_time(value: &str) -> bool {
  let mut parts = value.split(':');
  let Some(hours) = parts.next() else {
    return false;
  };
  let Some(minutes) = parts.next() else {
    return false;
  };
  if parts.next().is_some() {
    return false;
  }
  match (hours.parse::<u32>(), minutes.parse::<u32>()) {
    (Ok(hours), Ok(minutes)) => hours < 24 && minutes < 60,
    _ => false,
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportDeckInput {
  pub deck_id: i64,
  pub output_path: String,
  pub format: ExportFormat,
  pub delimiter: Option<String>,
  pub include_header: Option<bool>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
  Txt,
  Json,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupResult {
  pub output_path: String,
  pub manifest_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsRequest {
  pub period_days: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverviewMetrics {
  pub total_cards: i64,
  pub new_cards: i64,
  pub due_cards: i64,
  pub mastered_cards: i64,
  pub total_reviews_completed: i64,
  pub review_accuracy_percent: i64,
  pub retention_score_percent: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressPoint {
  pub utc_date: String,
  pub reviews_completed: i64,
  pub accuracy_percent: i64,
  pub new_cards_learned: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeakCardAnalytics {
  pub card_id: i64,
  pub deck_id: i64,
  pub deck_name: String,
  pub language_1: String,
  pub language_2: String,
  pub language_3: String,
  pub status: CardStatus,
  pub wrong_count: i64,
  pub mastery_score: i64,
  pub relearn_count: i64,
  pub recent_success_rate_percent: i64,
  pub difficulty_score: i64,
  pub needs_attention: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningBalance {
  pub new_card_reviews: i64,
  pub review_card_reviews: i64,
  pub new_card_percent: i64,
  pub review_card_percent: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreakStats {
  pub current_streak: i64,
  pub longest_streak: i64,
  pub studied_today: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyGoalProgress {
  pub daily_review_goal: i64,
  pub completed_today: i64,
  pub remaining_today: i64,
  pub percent_complete: i64,
  pub today_utc_date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReminderState {
  pub enabled: bool,
  pub reminder_time: String,
  pub due_cards: i64,
  pub should_show: bool,
  pub today_utc_date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsResponse {
  pub period_days: i64,
  pub overview: OverviewMetrics,
  pub progress: Vec<ProgressPoint>,
  pub weak_cards: Vec<WeakCardAnalytics>,
  pub learning_balance: LearningBalance,
  pub streak: StreakStats,
  pub daily_goal: DailyGoalProgress,
  pub insights: Vec<String>,
  pub reminder: ReminderState,
}

#[derive(Debug, Clone)]
pub struct CardSchedulingRecord {
  pub id: i64,
  pub deck_id: i64,
  pub status: CardStatus,
  pub review_count: i64,
  pub correct_count: i64,
  pub wrong_count: i64,
  pub current_interval_minutes: i64,
  pub ease_factor: f64,
  pub mastery_score: i64,
  pub last_reviewed_at: Option<String>,
  pub next_review_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SchedulingUpdate {
  pub status: CardStatus,
  pub review_count: i64,
  pub correct_count: i64,
  pub wrong_count: i64,
  pub current_interval_minutes: i64,
  pub ease_factor: f64,
  pub mastery_score: i64,
  pub last_reviewed_at: String,
  pub next_review_at: Option<String>,
  pub newly_mastered: bool,
}

#[derive(Debug, Clone)]
pub struct SessionRecord {
  pub id: i64,
  pub deck_id: i64,
}

#[derive(Debug, Clone)]
pub struct ParsedImportCard {
  pub line_number: usize,
  pub language_1: String,
  pub language_2: String,
  pub language_3: String,
  pub dedupe_key: String,
}

#[derive(Debug, Clone)]
pub struct ParsedImportDocument {
  pub header_labels: Option<[String; 3]>,
  pub cards: Vec<ParsedImportCard>,
  pub invalid_lines: Vec<InvalidImportLine>,
}

pub fn utc_now() -> DateTime<Utc> {
  Utc::now()
}
