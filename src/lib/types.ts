export type Theme = "light" | "dark";
export type UiLanguage = "en" | "fa" | "it";
export type CardStatus = "new" | "learning" | "review" | "mastered";
export type ReviewUnitState = "new" | "learning" | "review" | "relearning" | "leech";
export type ReviewRating = "again" | "hard" | "good" | "easy";
export type CardFilter = "all" | "new" | "due" | "mastered" | "weak";
export type CardSort = "updated_desc" | "created_desc" | "next_review_asc" | "primary_field_asc";
export type StudyMode = "due" | "new" | "mixed";
export type ExportFormat = "txt" | "json";
export type DeckLibrarySort =
  | "due_desc"
  | "recent_studied"
  | "name_asc"
  | "name_desc"
  | "new_desc"
  | "total_desc"
  | "mastered_desc"
  | "created_desc";
export type FieldPresetKind = "language" | "custom";

export interface FieldPreset {
  id: string;
  label: string;
  kind: FieldPresetKind;
}

export interface DeckField {
  id: number;
  deck_id: number;
  label: string;
  language_code: string | null;
  order_index: number;
  required: boolean;
  active: boolean;
  field_type: string;
  system_key: string | null;
}

export interface DeckFieldInput {
  id?: number;
  label: string;
  language_code?: string | null;
  order_index: number;
  required: boolean;
  active: boolean;
  field_type?: string | null;
}

export interface DeckSummary {
  id: number;
  name: string;
  description: string | null;
  language_1_label: string;
  language_2_label: string;
  language_3_label: string;
  created_at: string;
  updated_at: string;
  last_studied_at: string | null;
  total_cards: number;
  due_cards: number;
  new_cards: number;
  mastered_cards: number;
  study_prompt_field_id: number | null;
  study_reveal_field_ids: number[];
  fields: DeckField[];
}

export interface DeckDetail extends DeckSummary {}

export interface CreateDeckInput {
  name: string;
  description?: string;
  fields: DeckFieldInput[];
}

export interface UpdateDeckInput extends CreateDeckInput {
  id: number;
  deleted_field_ids?: number[];
}

export interface CardValueRecord {
  id: number;
  field_id: number;
  label: string;
  language_code: string | null;
  order_index: number;
  required: boolean;
  active: boolean;
  value: string;
  normalized_value: string;
  compact_value: string;
}

export interface CardRecord {
  id: number;
  deck_id: number;
  language_1: string;
  language_2: string;
  language_3: string;
  note: string | null;
  example_sentence: string | null;
  tag: string | null;
  values: CardValueRecord[];
  created_at: string;
  updated_at: string;
  last_reviewed_at: string | null;
  next_review_at: string | null;
  review_count: number;
  correct_count: number;
  wrong_count: number;
  current_interval_minutes: number;
  ease_factor: number;
  mastery_score: number;
  status: CardStatus;
}

export interface CardValueInput {
  field_id: number;
  value: string;
}

export interface CreateCardInput {
  deck_id: number;
  values: CardValueInput[];
}

export interface UpdateCardInput extends CreateCardInput {
  id: number;
}

export interface CardListQuery {
  deck_id: number;
  search?: string;
  filter?: CardFilter;
  sort?: CardSort;
}

export interface InvalidImportLine {
  line_number: number;
  raw: string;
  reason: string;
}

export interface ImportDetectedColumn {
  column_index: number;
  label: string;
}

export interface ImportColumnMapping {
  column_index: number;
  field_id?: number | null;
  label?: string | null;
  language_code?: string | null;
  required?: boolean | null;
  active?: boolean | null;
}

export interface ImportPreviewRow {
  line_number: number;
  columns: string[];
  duplicate: boolean;
  duplicate_reason: string | null;
  missing_required_fields: string[];
}

export interface ImportPreviewSummary {
  total_parsed: number;
  valid: number;
  invalid: number;
  duplicates: number;
  missing_required: number;
  importable: number;
}

export interface ImportPreviewResponse {
  detected_columns: ImportDetectedColumn[];
  rows: ImportPreviewRow[];
  invalid_lines: InvalidImportLine[];
  summary: ImportPreviewSummary;
  suggested_new_fields: DeckFieldInput[];
  unmapped_required_fields: string[];
  ready_for_commit: boolean;
}

export type ImportTarget =
  | {
      mode: "existing";
      deck_id: number;
    }
  | {
      mode: "new";
      name: string;
      description?: string;
    };

export interface ImportPreviewRequest {
  file_path: string;
  delimiter: string;
  has_header: boolean;
  target: ImportTarget;
  create_fields_from_header: boolean;
  mappings: ImportColumnMapping[];
}

export interface CommitImportRequest extends ImportPreviewRequest {}

export interface ImportCommitResponse {
  deck_id: number;
  total_parsed: number;
  imported: number;
  skipped: number;
  invalid: number;
  duplicates: number;
}

export interface StudySessionOptions {
  deck_id: number;
  prompt_field_id: number;
  reveal_field_ids: number[];
  mode: StudyMode;
  random_order: boolean;
  reverse_mode: boolean;
  cards_limit: number;
}

export interface StudyCard {
  id: number;
  deck_id: number;
  review_unit_id: number;
  language_1: string;
  language_2: string;
  language_3: string;
  note: string | null;
  example_sentence: string | null;
  tag: string | null;
  values: CardValueRecord[];
  status: CardStatus;
  next_review_at: string | null;
  review_state: ReviewUnitState;
  due_at_utc: string | null;
  mastered: boolean;
  leech: boolean;
  suspended: boolean;
  difficulty: number;
  stability_days: number;
}

export interface StudySessionPayload {
  session_id: number;
  deck: DeckSummary;
  options: StudySessionOptions;
  cards: StudyCard[];
}

export interface GradeCardInput {
  session_id: number;
  card_id: number;
  review_unit_id: number;
  rating: ReviewRating;
  latency_ms?: number | null;
  hint_used?: boolean;
  confidence?: number | null;
}

export interface GradeCardResponse {
  card_id: number;
  review_unit_id: number;
  review_state: ReviewUnitState;
  due_at_utc: string | null;
  scheduled_interval_days: number;
  retrievability_before: number;
  difficulty: number;
  stability_days: number;
  mastered: boolean;
  leech: boolean;
  newly_mastered: boolean;
}

export interface CompleteStudySessionInput {
  session_id: number;
  deck_id: number;
  studied_count: number;
  correct_count: number;
  wrong_count: number;
  newly_mastered_count: number;
}

export interface SessionSummary {
  session_id: number;
  studied_count: number;
  correct_count: number;
  wrong_count: number;
  accuracy_percent: number;
  newly_mastered_count: number;
  remaining_due_cards: number;
  suggestion: string;
}

export interface AppSettings {
  theme: Theme;
  ui_language: UiLanguage;
  cards_per_session: number;
  default_study_mode: StudyMode;
  random_order: boolean;
  reverse_mode: boolean;
  reveal_all_on_flip: boolean;
  daily_review_goal: number;
  desired_retention: number;
  reminder_enabled: boolean;
  reminder_time: string;
  reminder_last_acknowledged_date: string | null;
  import_delimiter: string;
  last_backup_directory: string | null;
  learning_steps_minutes: number[];
  relearning_steps_minutes: number[];
  leech_lapse_threshold: number;
  calibration_use_recency_weighting: boolean;
  calibration_recency_half_life_days: number;
  field_presets: FieldPreset[];
}

export interface UiPreferences {
  daily_coach_last_dismissed_utc_date: string | null;
  daily_coach_last_shown_utc_date: string | null;
  hidden_panels: Record<string, string[]>;
}

export interface AnalyticsRequest {
  period_days: number;
}

export interface OverviewMetrics {
  total_cards: number;
  new_cards: number;
  due_cards: number;
  mastered_cards: number;
  total_reviews_completed: number;
  review_accuracy_percent: number;
  retention_score_percent: number;
}

export interface ProgressPoint {
  utc_date: string;
  reviews_completed: number;
  accuracy_percent: number;
  new_cards_learned: number;
}

export interface WeakCardAnalytics {
  review_unit_id: number;
  card_id: number;
  deck_id: number;
  deck_name: string;
  language_1: string;
  language_2: string;
  language_3: string;
  preview_fields: Array<{
    label: string;
    value: string;
    is_context: boolean;
  }>;
  status: CardStatus;
  review_state: ReviewUnitState;
  wrong_count: number;
  mastery_score: number;
  relearn_count: number;
  recent_success_rate_percent: number;
  difficulty_score: number;
  difficulty: number;
  stability_days: number;
  leech: boolean;
  needs_attention: boolean;
}

export interface LearningOutcomeAnalytics {
  first_pass_success_rate_percent: number;
  recognition_accuracy_percent: number;
  production_accuracy_percent: number;
  retention_7d_percent: number;
  retention_30d_percent: number;
  average_time_to_graduation_days: number;
  average_time_to_mastery_days: number;
  lapse_rate_percent: number;
  leech_rate_percent: number;
  review_burden_per_retained_item: number;
}

export interface RetentionForecastPoint {
  desired_retention: number;
  estimated_due_next_30_days: number;
}

export interface SchedulerHealthAnalytics {
  predicted_recall_percent: number;
  actual_recall_percent: number;
  average_stability_days: number;
  average_difficulty: number;
  successful_stability_growth_percent: number;
  review_lapse_rate_percent: number;
  overdue_success_percent: number;
  on_time_success_percent: number;
  due_forecast_7d: number;
  due_forecast_30d: number;
  workload_forecast_per_day_7d: number;
  workload_forecast_per_day_30d: number;
  retention_sensitivity: RetentionForecastPoint[];
}

export interface ContentQualityAnalytics {
  hardest_direction_count: number;
  repeated_again_count: number;
  leech_count: number;
  contextual_support_count: number;
}

export interface LearningBalance {
  new_card_reviews: number;
  review_card_reviews: number;
  new_card_percent: number;
  review_card_percent: number;
}

export interface StreakStats {
  current_streak: number;
  longest_streak: number;
  studied_today: boolean;
}

export interface DailyGoalProgress {
  daily_review_goal: number;
  completed_today: number;
  remaining_today: number;
  percent_complete: number;
  today_utc_date: string;
}

export interface ReminderState {
  enabled: boolean;
  reminder_time: string;
  due_cards: number;
  should_show: boolean;
  today_utc_date: string;
}

export interface AnalyticsResponse {
  period_days: number;
  overview: OverviewMetrics;
  outcomes: LearningOutcomeAnalytics;
  scheduler_health: SchedulerHealthAnalytics;
  calibration: SchedulerCalibrationStatus;
  content_quality: ContentQualityAnalytics;
  progress: ProgressPoint[];
  weak_cards: WeakCardAnalytics[];
  learning_balance: LearningBalance;
  streak: StreakStats;
  daily_goal: DailyGoalProgress;
  insights: string[];
  reminder: ReminderState;
}

export interface DailyCoachRecommendation {
  deck_id: number;
  deck_name: string;
  urgency_score: number;
  priority_label: string;
  due_cards: number;
  overdue_cards: number;
  new_cards: number;
  weak_direction_count: number;
  upcoming_due_7d: number;
  days_since_last_study: number;
  last_studied_at: string | null;
  reason_text: string;
  supporting_reasons: string[];
}

export interface DailyCoachResponse {
  today_utc_date: string;
  studied_today: boolean;
  dismissed_today: boolean;
  should_prompt: boolean;
  daily_goal_remaining: number;
  recommendations: DailyCoachRecommendation[];
}

export interface SchedulerParameterValue {
  name: string;
  label: string;
  value: number;
  default_value: number;
  min: number;
  max: number;
}

export interface CalibrationMetrics {
  event_count: number;
  log_loss: number;
  rmse_bins: number;
  auc: number;
  brier_score: number;
  calibration_slope: number | null;
  calibration_intercept: number | null;
}

export interface CalibrationSplitMetrics {
  training: CalibrationMetrics;
  validation: CalibrationMetrics;
  test: CalibrationMetrics;
}

export interface CalibrationCurvePoint {
  bin_index: number;
  label: string;
  average_predicted: number;
  actual_rate: number;
  event_count: number;
}

export interface CalibrationBreakdownRow {
  label: string;
  event_count: number;
  average_predicted: number;
  actual_rate: number;
  log_loss: number;
  brier_score: number;
}

export interface CalibrationDiagnostics {
  curve: CalibrationCurvePoint[];
  error_by_state: CalibrationBreakdownRow[];
  error_by_rating: CalibrationBreakdownRow[];
  retention_by_elapsed_band: CalibrationBreakdownRow[];
}

export interface CalibrationWorkloadForecast {
  due_next_7d: number;
  due_next_30d: number;
  expected_recall_at_due_percent: number;
}

export interface CalibrationWorkloadComparison {
  active: CalibrationWorkloadForecast;
  candidate: CalibrationWorkloadForecast;
  workload_change_percent_7d: number;
  workload_change_percent_30d: number;
}

export interface CalibrationDataSufficiency {
  enough_data: boolean;
  minimum_usable_events: number;
  minimum_distinct_review_units: number;
  minimum_mature_review_events: number;
  minimum_failure_events: number;
  total_events: number;
  usable_events: number;
  filtered_events: number;
  distinct_review_units: number;
  deck_coverage_count: number;
  mature_review_events: number;
  failure_events: number;
}

export interface SchedulerCalibrationProfile {
  id: number;
  profile_key: string;
  profile_version: string;
  label: string;
  source: string;
  is_active: boolean;
  created_at: string;
  activated_at: string | null;
  metrics: CalibrationSplitMetrics | null;
  parameters: SchedulerParameterValue[];
  notes: string | null;
}

export interface SchedulerCalibrationRunSummary {
  id: number;
  status: string;
  accepted: boolean;
  started_at: string;
  completed_at: string | null;
  used_recency_weighting: boolean;
  recency_half_life_days: number | null;
  total_events: number;
  usable_events: number;
  filtered_events: number;
  distinct_review_units: number;
  deck_coverage_count: number;
  mature_review_events: number;
  failure_events: number;
  train_events: number;
  validation_events: number;
  test_events: number;
  split_train_end_utc: string | null;
  split_validation_end_utc: string | null;
  baseline_metrics: CalibrationSplitMetrics;
  candidate_metrics: CalibrationSplitMetrics | null;
  diagnostics: CalibrationDiagnostics | null;
  workload: CalibrationWorkloadComparison | null;
  reason: string | null;
}

export interface SchedulerCalibrationStatus {
  active_profile: SchedulerCalibrationProfile;
  latest_run: SchedulerCalibrationRunSummary | null;
  sufficiency: CalibrationDataSufficiency;
}

export interface RunCalibrationRequest {}

export interface ExportDeckInput {
  deck_id: number;
  output_path: string;
  format: ExportFormat;
  delimiter?: string;
  include_header?: boolean;
}

export interface BackupResult {
  output_path: string;
  manifest_path: string;
}

export interface ApiErrorPayload {
  code: string;
  message: string;
  field?: string | null;
}
