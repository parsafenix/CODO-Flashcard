use rand::seq::SliceRandom;
use tauri::State;

use crate::{
  db::{
    open_connection,
    repository::{card_repo, deck_repo, dynamic_repo, settings_repo, study_repo},
  },
  models::{
    error::AppError,
    types::{
      CompleteStudySessionInput, GradeCardInput, GradeCardResponse, SessionSummary, StudyCard, StudySessionOptions,
      StudySessionPayload, StudyMode,
    },
  },
  services::srs,
  AppState,
};

fn map_error(error: anyhow::Error) -> AppError {
  AppError::new("study_error", error.to_string())
}

fn prepare_cards(mut cards: Vec<StudyCard>, options: &StudySessionOptions) -> Vec<StudyCard> {
  if options.random_order {
    let mut rng = rand::thread_rng();
    match options.mode {
      StudyMode::Mixed => {
        let mut due_cards: Vec<StudyCard> = cards
          .iter()
          .filter(|card| card.status != crate::models::types::CardStatus::New)
          .cloned()
          .collect();
        let mut new_cards: Vec<StudyCard> = cards
          .into_iter()
          .filter(|card| card.status == crate::models::types::CardStatus::New)
          .collect();
        due_cards.shuffle(&mut rng);
        new_cards.shuffle(&mut rng);
        due_cards.extend(new_cards);
        cards = due_cards;
      }
      _ => cards.shuffle(&mut rng),
    }
  }

  cards.into_iter().take(options.cards_limit.max(1)).collect()
}

#[tauri::command]
pub fn start_study_session(
  state: State<'_, AppState>,
  options: StudySessionOptions,
) -> Result<StudySessionPayload, AppError> {
  let connection = open_connection(&state.db_path).map_err(AppError::from)?;
  let sanitized_options = StudySessionOptions {
    cards_limit: options.cards_limit.clamp(1, 200),
    ..options
  };

  let deck = deck_repo::get_deck(&connection, sanitized_options.deck_id)
    .map_err(map_error)?
    .ok_or_else(|| AppError::new("not_found", "Deck not found."))?;

  dynamic_repo::ensure_field_belongs_to_deck(&connection, sanitized_options.deck_id, sanitized_options.prompt_field_id)
    .map_err(map_error)?;
  for field_id in &sanitized_options.reveal_field_ids {
    dynamic_repo::ensure_field_belongs_to_deck(&connection, sanitized_options.deck_id, *field_id).map_err(map_error)?;
  }

  let cards = card_repo::get_cards_for_study(&connection, sanitized_options.deck_id, sanitized_options.mode)
    .map_err(map_error)?;
  let prepared_cards = prepare_cards(cards, &sanitized_options);
  let session_id = if prepared_cards.is_empty() {
    0
  } else {
    study_repo::start_session(&connection, &sanitized_options).map_err(map_error)?
  };

  Ok(StudySessionPayload {
    session_id,
    deck,
    options: sanitized_options,
    cards: prepared_cards,
  })
}

#[tauri::command]
pub fn grade_card(state: State<'_, AppState>, input: GradeCardInput) -> Result<GradeCardResponse, AppError> {
  let connection = open_connection(&state.db_path).map_err(AppError::from)?;
  let session = study_repo::get_session_record(&connection, input.session_id).map_err(map_error)?;
  let record = card_repo::get_scheduling_record(&connection, input.card_id)
    .map_err(map_error)?
    .ok_or_else(|| AppError::new("not_found", "Card not found."))?;
  if session.deck_id != record.deck_id {
    return Err(AppError::new("study_error", "This card does not belong to the current study session."));
  }

  let update = srs::schedule_review(&record, input.knew_it);
  study_repo::apply_scheduling_update(&connection, input.card_id, &update).map_err(map_error)?;

  let response = GradeCardResponse {
    card_id: input.card_id,
    status: update.status,
    next_review_at: update.next_review_at.clone(),
    current_interval_minutes: update.current_interval_minutes,
    ease_factor: update.ease_factor,
    mastery_score: update.mastery_score,
    newly_mastered: update.newly_mastered,
  };

  study_repo::record_review_history(&connection, input.session_id, &response, &record, input.knew_it)
    .map_err(map_error)?;

  Ok(response)
}

#[tauri::command]
pub fn complete_study_session(
  state: State<'_, AppState>,
  input: CompleteStudySessionInput,
) -> Result<SessionSummary, AppError> {
  let connection = open_connection(&state.db_path).map_err(AppError::from)?;
  let session = study_repo::get_session_record(&connection, input.session_id).map_err(map_error)?;
  if session.deck_id != input.deck_id {
    return Err(AppError::new("study_error", "This study session does not match the selected deck."));
  }
  let settings = settings_repo::get_settings(&connection).map_err(map_error)?;
  study_repo::complete_session(&connection, &input, settings.ui_language).map_err(map_error)
}
