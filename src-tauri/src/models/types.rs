use std::collections::BTreeMap;

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
pub enum ReviewUnitState {
  New,
  Learning,
  Review,
  Relearning,
  Leech,
}

impl ReviewUnitState {
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::New => "new",
      Self::Learning => "learning",
      Self::Review => "review",
      Self::Relearning => "relearning",
      Self::Leech => "leech",
    }
  }

  pub fn from_db(value: &str) -> Self {
    match value {
      "learning" => Self::Learning,
      "review" => Self::Review,
      "relearning" => Self::Relearning,
      "leech" => Self::Leech,
      _ => Self::New,
    }
  }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReviewRating {
  Again,
  Hard,
  Good,
  Easy,
}

impl ReviewRating {
  pub fn as_str(&self) -> &'static str {
    match self {
      Self::Again => "again",
      Self::Hard => "hard",
      Self::Good => "good",
      Self::Easy => "easy",
    }
  }

  pub fn score(&self) -> i32 {
    match self {
      Self::Again => 1,
      Self::Hard => 2,
      Self::Good => 3,
      Self::Easy => 4,
    }
  }

  pub fn is_success(&self) -> bool {
    !matches!(self, Self::Again)
  }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StudyMode {
  Due,
  New,
  Mixed,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UiLanguage {
  En,
  Fa,
  It,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FieldPresetKind {
  Language,
  Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FieldPreset {
  pub id: String,
  pub label: String,
  pub kind: FieldPresetKind,
}

pub fn default_field_presets() -> Vec<FieldPreset> {
  vec![
    FieldPreset {
      id: "persian".to_string(),
      label: "Persian".to_string(),
      kind: FieldPresetKind::Language,
    },
    FieldPreset {
      id: "english".to_string(),
      label: "English".to_string(),
      kind: FieldPresetKind::Language,
    },
    FieldPreset {
      id: "german".to_string(),
      label: "German".to_string(),
      kind: FieldPresetKind::Language,
    },
    FieldPreset {
      id: "italian".to_string(),
      label: "Italian".to_string(),
      kind: FieldPresetKind::Language,
    },
    FieldPreset {
      id: "french".to_string(),
      label: "French".to_string(),
      kind: FieldPresetKind::Language,
    },
    FieldPreset {
      id: "definition".to_string(),
      label: "Definition".to_string(),
      kind: FieldPresetKind::Custom,
    },
    FieldPreset {
      id: "example".to_string(),
      label: "Example".to_string(),
      kind: FieldPresetKind::Custom,
    },
    FieldPreset {
      id: "notes".to_string(),
      label: "Notes".to_string(),
      kind: FieldPresetKind::Custom,
    },
  ]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckField {
  pub id: i64,
  pub deck_id: i64,
  pub label: String,
  pub language_code: Option<String>,
  pub order_index: i64,
  pub required: bool,
  pub active: bool,
  pub field_type: String,
  pub system_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckFieldInput {
  pub id: Option<i64>,
  pub label: String,
  pub language_code: Option<String>,
  pub order_index: i64,
  pub required: bool,
  pub active: bool,
  pub field_type: Option<String>,
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
  pub study_prompt_field_id: Option<i64>,
  pub study_reveal_field_ids: Vec<i64>,
  pub fields: Vec<DeckField>,
}

pub type DeckDetail = DeckSummary;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDeckInput {
  pub name: String,
  pub description: Option<String>,
  pub fields: Vec<DeckFieldInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateDeckInput {
  pub id: i64,
  pub name: String,
  pub description: Option<String>,
  pub fields: Vec<DeckFieldInput>,
  #[serde(default)]
  pub deleted_field_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardValueRecord {
  pub id: i64,
  pub field_id: i64,
  pub label: String,
  pub language_code: Option<String>,
  pub order_index: i64,
  pub required: bool,
  pub active: bool,
  pub value: String,
  pub normalized_value: String,
  pub compact_value: String,
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
  pub values: Vec<CardValueRecord>,
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
pub struct CardValueInput {
  pub field_id: i64,
  pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCardInput {
  pub deck_id: i64,
  pub values: Vec<CardValueInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCardInput {
  pub id: i64,
  pub deck_id: i64,
  pub values: Vec<CardValueInput>,
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
  PrimaryFieldAsc,
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
pub struct ImportDetectedColumn {
  pub column_index: usize,
  pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportColumnMapping {
  pub column_index: usize,
  pub field_id: Option<i64>,
  pub label: Option<String>,
  pub language_code: Option<String>,
  pub required: Option<bool>,
  pub active: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportPreviewRow {
  pub line_number: usize,
  pub columns: Vec<String>,
  pub duplicate: bool,
  pub duplicate_reason: Option<String>,
  pub missing_required_fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportPreviewSummary {
  pub total_parsed: usize,
  pub valid: usize,
  pub invalid: usize,
  pub duplicates: usize,
  pub missing_required: usize,
  pub importable: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportPreviewResponse {
  pub detected_columns: Vec<ImportDetectedColumn>,
  pub rows: Vec<ImportPreviewRow>,
  pub invalid_lines: Vec<InvalidImportLine>,
  pub summary: ImportPreviewSummary,
  pub suggested_new_fields: Vec<DeckFieldInput>,
  pub unmapped_required_fields: Vec<String>,
  pub ready_for_commit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum ImportTarget {
  Existing { deck_id: i64 },
  New {
    name: String,
    description: Option<String>,
  },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportPreviewRequest {
  pub file_path: String,
  pub delimiter: String,
  pub has_header: bool,
  pub target: ImportTarget,
  #[serde(default)]
  pub create_fields_from_header: bool,
  #[serde(default)]
  pub mappings: Vec<ImportColumnMapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitImportRequest {
  pub file_path: String,
  pub delimiter: String,
  pub has_header: bool,
  pub target: ImportTarget,
  #[serde(default)]
  pub create_fields_from_header: bool,
  #[serde(default)]
  pub mappings: Vec<ImportColumnMapping>,
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
  pub prompt_field_id: i64,
  pub reveal_field_ids: Vec<i64>,
  pub mode: StudyMode,
  pub random_order: bool,
  pub reverse_mode: bool,
  pub cards_limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudyCard {
  pub id: i64,
  pub deck_id: i64,
  pub review_unit_id: i64,
  pub language_1: String,
  pub language_2: String,
  pub language_3: String,
  pub note: Option<String>,
  pub example_sentence: Option<String>,
  pub tag: Option<String>,
  pub values: Vec<CardValueRecord>,
  pub status: CardStatus,
  pub next_review_at: Option<String>,
  pub review_state: ReviewUnitState,
  pub due_at_utc: Option<String>,
  pub mastered: bool,
  pub leech: bool,
  pub suspended: bool,
  pub difficulty: f64,
  pub stability_days: f64,
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
  pub review_unit_id: i64,
  pub rating: ReviewRating,
  pub latency_ms: Option<i64>,
  #[serde(default)]
  pub hint_used: bool,
  pub confidence: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradeCardResponse {
  pub card_id: i64,
  pub review_unit_id: i64,
  pub review_state: ReviewUnitState,
  pub due_at_utc: Option<String>,
  pub scheduled_interval_days: f64,
  pub retrievability_before: f64,
  pub difficulty: f64,
  pub stability_days: f64,
  pub mastered: bool,
  pub leech: bool,
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
  pub ui_language: UiLanguage,
  pub cards_per_session: usize,
  pub default_study_mode: StudyMode,
  pub random_order: bool,
  pub reverse_mode: bool,
  pub reveal_all_on_flip: bool,
  pub daily_review_goal: usize,
  pub desired_retention: f64,
  pub reminder_enabled: bool,
  pub reminder_time: String,
  pub reminder_last_acknowledged_date: Option<String>,
  pub import_delimiter: String,
  pub last_backup_directory: Option<String>,
  pub learning_steps_minutes: Vec<i64>,
  pub relearning_steps_minutes: Vec<i64>,
  pub leech_lapse_threshold: usize,
  pub calibration_use_recency_weighting: bool,
  pub calibration_recency_half_life_days: i64,
  pub field_presets: Vec<FieldPreset>,
}

impl Default for AppSettings {
  fn default() -> Self {
    Self {
      theme: Theme::Dark,
      ui_language: UiLanguage::En,
      cards_per_session: 20,
      default_study_mode: StudyMode::Mixed,
      random_order: true,
      reverse_mode: false,
      reveal_all_on_flip: true,
      daily_review_goal: 20,
      desired_retention: 0.90,
      reminder_enabled: false,
      reminder_time: "18:00".to_string(),
      reminder_last_acknowledged_date: None,
      import_delimiter: "|".to_string(),
      last_backup_directory: None,
      learning_steps_minutes: vec![10, 24 * 60, 3 * 24 * 60],
      relearning_steps_minutes: vec![10, 24 * 60],
      leech_lapse_threshold: 8,
      calibration_use_recency_weighting: false,
      calibration_recency_half_life_days: 180,
      field_presets: default_field_presets(),
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiPreferences {
  pub daily_coach_last_dismissed_utc_date: Option<String>,
  pub daily_coach_last_shown_utc_date: Option<String>,
  pub hidden_panels: BTreeMap<String, Vec<String>>,
}

impl Default for UiPreferences {
  fn default() -> Self {
    Self {
      daily_coach_last_dismissed_utc_date: None,
      daily_coach_last_shown_utc_date: None,
      hidden_panels: BTreeMap::new(),
    }
  }
}

impl UiPreferences {
  pub fn validate(mut self) -> Self {
    self.hidden_panels = self
      .hidden_panels
      .into_iter()
      .map(|(page, panel_ids)| {
        let mut cleaned = panel_ids
          .into_iter()
          .map(|value| value.trim().to_string())
          .filter(|value| !value.is_empty())
          .collect::<Vec<_>>();
        cleaned.sort();
        cleaned.dedup();
        (page.trim().to_string(), cleaned)
      })
      .filter(|(page, _)| !page.is_empty())
      .collect();
    self
  }
}

impl AppSettings {
  pub fn validate(mut self) -> Self {
    self.cards_per_session = self.cards_per_session.clamp(1, 200);
    self.daily_review_goal = self.daily_review_goal.clamp(1, 500);
    if self.desired_retention <= 0.0 {
      self.desired_retention = 0.90;
    }
    self.desired_retention = self.desired_retention.clamp(0.85, 0.95);
    if self.import_delimiter.trim().is_empty() {
      self.import_delimiter = "|".to_string();
    }
    if self.import_delimiter.chars().count() > 3 {
      self.import_delimiter = "|".to_string();
    }
    if !valid_reminder_time(&self.reminder_time) {
      self.reminder_time = "18:00".to_string();
    }
    if self.field_presets.is_empty() {
      self.field_presets = default_field_presets();
    } else {
      let mut seen = std::collections::HashSet::new();
      self.field_presets = self
        .field_presets
        .into_iter()
        .filter(|preset| !preset.label.trim().is_empty() && !preset.id.trim().is_empty())
        .filter(|preset| seen.insert(preset.id.clone()))
        .collect();
      if self.field_presets.is_empty() {
        self.field_presets = default_field_presets();
      }
    }
    self.learning_steps_minutes = sanitize_steps(&self.learning_steps_minutes, &[10, 24 * 60, 3 * 24 * 60]);
    self.relearning_steps_minutes = sanitize_steps(&self.relearning_steps_minutes, &[10, 24 * 60]);
    self.leech_lapse_threshold = self.leech_lapse_threshold.clamp(3, 20);
    self.calibration_recency_half_life_days = self.calibration_recency_half_life_days.clamp(14, 720);
    self
  }
}

fn sanitize_steps(value: &[i64], fallback: &[i64]) -> Vec<i64> {
  let mut cleaned = value
    .iter()
    .copied()
    .filter(|step| *step > 0)
    .collect::<Vec<_>>();
  cleaned.sort_unstable();
  cleaned.dedup();
  if cleaned.is_empty() {
    fallback.to_vec()
  } else {
    cleaned
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
pub struct WeakCardPreviewField {
  pub label: String,
  pub value: String,
  pub is_context: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeakCardAnalytics {
  pub card_id: i64,
  pub review_unit_id: i64,
  pub deck_id: i64,
  pub deck_name: String,
  pub language_1: String,
  pub language_2: String,
  pub language_3: String,
  pub preview_fields: Vec<WeakCardPreviewField>,
  pub status: CardStatus,
  pub review_state: ReviewUnitState,
  pub wrong_count: i64,
  pub mastery_score: i64,
  pub relearn_count: i64,
  pub recent_success_rate_percent: i64,
  pub difficulty_score: i64,
  pub difficulty: f64,
  pub stability_days: f64,
  pub leech: bool,
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
  pub outcomes: LearningOutcomeAnalytics,
  pub scheduler_health: SchedulerHealthAnalytics,
  pub calibration: SchedulerCalibrationStatus,
  pub content_quality: ContentQualityAnalytics,
  pub progress: Vec<ProgressPoint>,
  pub weak_cards: Vec<WeakCardAnalytics>,
  pub learning_balance: LearningBalance,
  pub streak: StreakStats,
  pub daily_goal: DailyGoalProgress,
  pub insights: Vec<String>,
  pub reminder: ReminderState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyCoachRecommendation {
  pub deck_id: i64,
  pub deck_name: String,
  pub urgency_score: f64,
  pub priority_label: String,
  pub due_cards: i64,
  pub overdue_cards: i64,
  pub new_cards: i64,
  pub weak_direction_count: i64,
  pub upcoming_due_7d: i64,
  pub days_since_last_study: i64,
  pub last_studied_at: Option<String>,
  pub reason_text: String,
  pub supporting_reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyCoachResponse {
  pub today_utc_date: String,
  pub studied_today: bool,
  pub dismissed_today: bool,
  pub should_prompt: bool,
  pub daily_goal_remaining: i64,
  pub recommendations: Vec<DailyCoachRecommendation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningOutcomeAnalytics {
  pub first_pass_success_rate_percent: i64,
  pub recognition_accuracy_percent: i64,
  pub production_accuracy_percent: i64,
  pub retention_7d_percent: i64,
  pub retention_30d_percent: i64,
  pub average_time_to_graduation_days: f64,
  pub average_time_to_mastery_days: f64,
  pub lapse_rate_percent: i64,
  pub leech_rate_percent: i64,
  pub review_burden_per_retained_item: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionForecastPoint {
  pub desired_retention: f64,
  pub estimated_due_next_30_days: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerHealthAnalytics {
  pub predicted_recall_percent: i64,
  pub actual_recall_percent: i64,
  pub average_stability_days: f64,
  pub average_difficulty: f64,
  pub successful_stability_growth_percent: i64,
  pub review_lapse_rate_percent: i64,
  pub overdue_success_percent: i64,
  pub on_time_success_percent: i64,
  pub due_forecast_7d: i64,
  pub due_forecast_30d: i64,
  pub workload_forecast_per_day_7d: f64,
  pub workload_forecast_per_day_30d: f64,
  pub retention_sensitivity: Vec<RetentionForecastPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentQualityAnalytics {
  pub hardest_direction_count: i64,
  pub repeated_again_count: i64,
  pub leech_count: i64,
  pub contextual_support_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerParameterValue {
  pub name: String,
  pub label: String,
  pub value: f64,
  pub default_value: f64,
  pub min: f64,
  pub max: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CalibrationMetrics {
  pub event_count: i64,
  pub log_loss: f64,
  pub rmse_bins: f64,
  pub auc: f64,
  pub brier_score: f64,
  pub calibration_slope: Option<f64>,
  pub calibration_intercept: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CalibrationSplitMetrics {
  pub training: CalibrationMetrics,
  pub validation: CalibrationMetrics,
  pub test: CalibrationMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationCurvePoint {
  pub bin_index: i64,
  pub label: String,
  pub average_predicted: f64,
  pub actual_rate: f64,
  pub event_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationBreakdownRow {
  pub label: String,
  pub event_count: i64,
  pub average_predicted: f64,
  pub actual_rate: f64,
  pub log_loss: f64,
  pub brier_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CalibrationDiagnostics {
  pub curve: Vec<CalibrationCurvePoint>,
  pub error_by_state: Vec<CalibrationBreakdownRow>,
  pub error_by_rating: Vec<CalibrationBreakdownRow>,
  pub retention_by_elapsed_band: Vec<CalibrationBreakdownRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CalibrationWorkloadForecast {
  pub due_next_7d: i64,
  pub due_next_30d: i64,
  pub expected_recall_at_due_percent: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CalibrationWorkloadComparison {
  pub active: CalibrationWorkloadForecast,
  pub candidate: CalibrationWorkloadForecast,
  pub workload_change_percent_7d: f64,
  pub workload_change_percent_30d: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CalibrationDataSufficiency {
  pub enough_data: bool,
  pub minimum_usable_events: i64,
  pub minimum_distinct_review_units: i64,
  pub minimum_mature_review_events: i64,
  pub minimum_failure_events: i64,
  pub total_events: i64,
  pub usable_events: i64,
  pub filtered_events: i64,
  pub distinct_review_units: i64,
  pub deck_coverage_count: i64,
  pub mature_review_events: i64,
  pub failure_events: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerCalibrationProfile {
  pub id: i64,
  pub profile_key: String,
  pub profile_version: String,
  pub label: String,
  pub source: String,
  pub is_active: bool,
  pub created_at: String,
  pub activated_at: Option<String>,
  pub metrics: Option<CalibrationSplitMetrics>,
  pub parameters: Vec<SchedulerParameterValue>,
  pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerCalibrationRunSummary {
  pub id: i64,
  pub status: String,
  pub accepted: bool,
  pub started_at: String,
  pub completed_at: Option<String>,
  pub used_recency_weighting: bool,
  pub recency_half_life_days: Option<f64>,
  pub total_events: i64,
  pub usable_events: i64,
  pub filtered_events: i64,
  pub distinct_review_units: i64,
  pub deck_coverage_count: i64,
  pub mature_review_events: i64,
  pub failure_events: i64,
  pub train_events: i64,
  pub validation_events: i64,
  pub test_events: i64,
  pub split_train_end_utc: Option<String>,
  pub split_validation_end_utc: Option<String>,
  pub baseline_metrics: CalibrationSplitMetrics,
  pub candidate_metrics: Option<CalibrationSplitMetrics>,
  pub diagnostics: Option<CalibrationDiagnostics>,
  pub workload: Option<CalibrationWorkloadComparison>,
  pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerCalibrationStatus {
  pub active_profile: SchedulerCalibrationProfile,
  pub latest_run: Option<SchedulerCalibrationRunSummary>,
  pub sufficiency: CalibrationDataSufficiency,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RunCalibrationRequest {}

#[derive(Debug, Clone)]
pub struct ReviewUnitRecord {
  pub id: i64,
  pub card_id: i64,
  pub deck_id: i64,
  pub prompt_field_id: i64,
  pub reveal_field_ids: Vec<i64>,
  pub direction_key: String,
  pub state: ReviewUnitState,
  pub difficulty: f64,
  pub stability: f64,
  pub scheduled_interval_days: f64,
  pub last_reviewed_at_utc: Option<String>,
  pub due_at_utc: Option<String>,
  pub lapses: i64,
  pub successful_reviews: i64,
  pub failed_reviews: i64,
  pub total_reviews: i64,
  pub same_day_reviews_count: i64,
  pub average_latency_ms: Option<f64>,
  pub last_latency_ms: Option<i64>,
  pub hint_used_last: bool,
  pub confidence_last: Option<f64>,
  pub suspended: bool,
  pub leech: bool,
  pub mastered: bool,
  pub learning_step_index: i64,
  pub relearning_step_index: i64,
  pub first_reviewed_at_utc: Option<String>,
  pub graduated_at_utc: Option<String>,
  pub mastered_at_utc: Option<String>,
  pub created_at: String,
  pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct SchedulerReviewInput {
  pub rating: ReviewRating,
  pub reviewed_at_utc: String,
  pub latency_ms: Option<i64>,
  pub hint_used: bool,
  pub confidence: Option<f64>,
  pub desired_retention: f64,
  pub learning_steps_minutes: Vec<i64>,
  pub relearning_steps_minutes: Vec<i64>,
  pub recent_again_count: i64,
  pub leech_lapse_threshold: i64,
}

#[derive(Debug, Clone)]
pub struct ReviewUnitUpdate {
  pub state: ReviewUnitState,
  pub difficulty: f64,
  pub stability: f64,
  pub scheduled_interval_days: f64,
  pub last_reviewed_at_utc: String,
  pub due_at_utc: Option<String>,
  pub lapses: i64,
  pub successful_reviews: i64,
  pub failed_reviews: i64,
  pub total_reviews: i64,
  pub same_day_reviews_count: i64,
  pub average_latency_ms: Option<f64>,
  pub last_latency_ms: Option<i64>,
  pub hint_used_last: bool,
  pub confidence_last: Option<f64>,
  pub suspended: bool,
  pub leech: bool,
  pub mastered: bool,
  pub learning_step_index: i64,
  pub relearning_step_index: i64,
  pub first_reviewed_at_utc: Option<String>,
  pub graduated_at_utc: Option<String>,
  pub mastered_at_utc: Option<String>,
  pub updated_at: String,
  pub retrievability_before: f64,
  pub newly_mastered: bool,
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
  pub prompt_field_id: Option<i64>,
  pub reveal_field_ids: Vec<i64>,
}

#[derive(Debug, Clone)]
pub struct ParsedImportRow {
  pub line_number: usize,
  pub columns: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ParsedImportDocument {
  pub header_labels: Option<Vec<String>>,
  pub column_count: usize,
  pub rows: Vec<ParsedImportRow>,
  pub invalid_lines: Vec<InvalidImportLine>,
}

pub fn utc_now() -> DateTime<Utc> {
  Utc::now()
}
