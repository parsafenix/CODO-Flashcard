import { invoke } from "@tauri-apps/api/core";
import type {
  AnalyticsRequest,
  AnalyticsResponse,
  AppSettings,
  BackupResult,
  CardListQuery,
  CardRecord,
  CommitImportRequest,
  CompleteStudySessionInput,
  CreateCardInput,
  CreateDeckInput,
  DeckDetail,
  DeckSummary,
  ExportDeckInput,
  GradeCardInput,
  GradeCardResponse,
  DailyCoachResponse,
  ImportPreviewRequest,
  ImportPreviewResponse,
  ImportCommitResponse,
  RunCalibrationRequest,
  SchedulerCalibrationStatus,
  SessionSummary,
  StudySessionOptions,
  StudySessionPayload,
  UiPreferences,
  UpdateCardInput,
  UpdateDeckInput
} from "./types";

export interface NormalizedApiError {
  code: string;
  message: string;
  field?: string | null;
}

function normalizeError(error: unknown): NormalizedApiError {
  if (typeof error === "string") {
    return { code: "unknown", message: error };
  }

  if (typeof error === "object" && error !== null) {
    const maybe = error as Record<string, unknown>;
    if (typeof maybe.message === "string") {
      return {
        code: typeof maybe.code === "string" ? maybe.code : "unknown",
        message: maybe.message,
        field: typeof maybe.field === "string" ? maybe.field : null
      };
    }
  }

  return { code: "unknown", message: "Something went wrong." };
}

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw normalizeError(error);
  }
}

export const api = {
  listDecks: (search = "") => call<DeckSummary[]>("list_decks", { search }),
  getDeck: (deckId: number) => call<DeckDetail>("get_deck", { deckId }),
  createDeck: (input: CreateDeckInput) => call<DeckDetail>("create_deck", { input }),
  updateDeck: (input: UpdateDeckInput) => call<DeckDetail>("update_deck", { input }),
  deleteDeck: (deckId: number) => call<void>("delete_deck", { deckId }),
  duplicateDeck: (deckId: number) => call<DeckDetail>("duplicate_deck", { deckId }),
  listCards: (query: CardListQuery) => call<CardRecord[]>("list_cards", { query }),
  createCard: (input: CreateCardInput) => call<CardRecord>("create_card", { input }),
  updateCard: (input: UpdateCardInput) => call<CardRecord>("update_card", { input }),
  deleteCard: (cardId: number) => call<void>("delete_card", { cardId }),
  previewImport: (request: ImportPreviewRequest) =>
    call<ImportPreviewResponse>("preview_import", { request }),
  commitImport: (request: CommitImportRequest) =>
    call<ImportCommitResponse>("commit_import", { request }),
  startStudySession: (options: StudySessionOptions) =>
    call<StudySessionPayload>("start_study_session", { options }),
  gradeCard: (input: GradeCardInput) => call<GradeCardResponse>("grade_card", { input }),
  completeStudySession: (input: CompleteStudySessionInput) =>
    call<SessionSummary>("complete_study_session", { input }),
  getAnalytics: (request: AnalyticsRequest) => call<AnalyticsResponse>("get_analytics", { request }),
  getDailyCoach: () => call<DailyCoachResponse>("get_daily_coach"),
  getSchedulerCalibrationStatus: () => call<SchedulerCalibrationStatus>("get_scheduler_calibration_status"),
  runSchedulerCalibration: (request: RunCalibrationRequest = {}) =>
    call<SchedulerCalibrationStatus>("run_scheduler_calibration", { request }),
  getSettings: () => call<AppSettings>("get_settings"),
  updateSettings: (settings: AppSettings) => call<AppSettings>("update_settings", { settings }),
  getUiPreferences: () => call<UiPreferences>("get_ui_preferences"),
  updateUiPreferences: (preferences: UiPreferences) => call<UiPreferences>("update_ui_preferences", { preferences }),
  exportDeck: (input: ExportDeckInput) => call<void>("export_deck", { input }),
  createBackup: (directoryPath: string) =>
    call<BackupResult>("create_backup", { directoryPath }),
  resetAppData: () => call<void>("reset_app_data"),
  openDataFolder: () => call<string>("open_data_folder")
};
