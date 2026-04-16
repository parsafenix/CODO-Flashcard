use rand::seq::SliceRandom;
use tauri::State;

use crate::{
  db::{
    open_connection,
    repository::{deck_repo, dynamic_repo, review_unit_repo, scheduler_repo, settings_repo, study_repo},
  },
  models::{
    error::AppError,
    types::{
      CompleteStudySessionInput, GradeCardInput, GradeCardResponse, SessionSummary, StudyCard, StudyMode, StudySessionOptions,
      StudySessionPayload,
    },
  },
  services::srs,
  AppState,
};

fn map_error(error: anyhow::Error) -> AppError {
  AppError::new("study_error", error.to_string())
}

fn prompt_value(card: &StudyCard, prompt_field_id: i64) -> String {
  card
    .values
    .iter()
    .find(|value| value.field_id == prompt_field_id)
    .map(|value| value.normalized_value.clone())
    .unwrap_or_default()
}

fn spread_prompt_collisions(cards: Vec<StudyCard>, prompt_field_id: i64) -> Vec<StudyCard> {
  let mut queue = cards;
  for index in 1..queue.len() {
    let current_prompt = prompt_value(&queue[index], prompt_field_id);
    let previous_prompt = prompt_value(&queue[index - 1], prompt_field_id);
    if !current_prompt.is_empty() && current_prompt == previous_prompt {
      if let Some(swap_index) = ((index + 1)..queue.len()).find(|candidate_index| {
        prompt_value(&queue[*candidate_index], prompt_field_id) != previous_prompt
      }) {
        queue.swap(index, swap_index);
      }
    }
  }
  queue
}

fn prepare_cards(mut cards: Vec<StudyCard>, options: &StudySessionOptions) -> Vec<StudyCard> {
  if options.random_order {
    let mut rng = rand::thread_rng();
    match options.mode {
      StudyMode::Mixed => {
        let mut relearning = cards
          .iter()
          .filter(|card| matches!(card.review_state, crate::models::types::ReviewUnitState::Leech | crate::models::types::ReviewUnitState::Relearning | crate::models::types::ReviewUnitState::Learning))
          .cloned()
          .collect::<Vec<_>>();
        let mut review = cards
          .iter()
          .filter(|card| matches!(card.review_state, crate::models::types::ReviewUnitState::Review))
          .cloned()
          .collect::<Vec<_>>();
        let mut new_cards = cards
          .into_iter()
          .filter(|card| matches!(card.review_state, crate::models::types::ReviewUnitState::New))
          .collect::<Vec<_>>();
        relearning.shuffle(&mut rng);
        review.shuffle(&mut rng);
        new_cards.shuffle(&mut rng);

        let mut mixed = Vec::with_capacity(relearning.len() + review.len() + new_cards.len());
        while !relearning.is_empty() || !review.is_empty() || !new_cards.is_empty() {
          if let Some(card) = relearning.pop() {
            mixed.push(card);
          }
          if let Some(card) = review.pop() {
            mixed.push(card);
          }
          if mixed.len() % 3 == 2 {
            if let Some(card) = new_cards.pop() {
              mixed.push(card);
            }
          }
          if relearning.is_empty() && review.is_empty() {
            while let Some(card) = new_cards.pop() {
              mixed.push(card);
            }
          }
        }
        cards = mixed;
      }
      _ => cards.shuffle(&mut rng),
    }
  }

  spread_prompt_collisions(cards, options.prompt_field_id)
    .into_iter()
    .take(options.cards_limit.max(1))
    .collect()
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

  let cards = review_unit_repo::list_study_cards(
    &connection,
    sanitized_options.deck_id,
    sanitized_options.prompt_field_id,
    &sanitized_options.reveal_field_ids,
    sanitized_options.mode,
  )
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
  let review_unit = review_unit_repo::get_review_unit(&connection, input.review_unit_id)
    .map_err(map_error)?
    .ok_or_else(|| AppError::new("not_found", "Review unit not found."))?;
  if session.deck_id != review_unit.deck_id || review_unit.card_id != input.card_id {
    return Err(AppError::new("study_error", "This review item does not belong to the current study session."));
  }
  if session.prompt_field_id != Some(review_unit.prompt_field_id)
    || review_unit_repo::canonical_reveal_field_ids(&session.reveal_field_ids)
      != review_unit_repo::canonical_reveal_field_ids(&review_unit.reveal_field_ids)
  {
    return Err(AppError::new("study_error", "This review direction does not match the current study session."));
  }

  let settings = settings_repo::get_settings(&connection).map_err(map_error)?;
  let scheduler_parameters = scheduler_repo::get_active_parameters(&connection).unwrap_or_default();
  let recent_again_count = review_unit_repo::count_recent_again(&connection, review_unit.id, 6).map_err(map_error)?;
  let scheduling_input = crate::models::types::SchedulerReviewInput {
    rating: input.rating,
    reviewed_at_utc: crate::models::types::utc_now().to_rfc3339(),
    latency_ms: input.latency_ms,
    hint_used: input.hint_used,
    confidence: input.confidence,
    desired_retention: settings.desired_retention,
    learning_steps_minutes: settings.learning_steps_minutes.clone(),
    relearning_steps_minutes: settings.relearning_steps_minutes.clone(),
    recent_again_count,
    leech_lapse_threshold: settings.leech_lapse_threshold as i64,
  };
  let update = srs::schedule_review_with_parameters(&review_unit, &scheduling_input, &scheduler_parameters);
  review_unit_repo::apply_review_update(&connection, review_unit.id, &update).map_err(map_error)?;
  review_unit_repo::record_review_log(
    &connection,
    &review_unit,
    input.session_id,
    input.rating,
    &update,
    input.latency_ms,
    input.hint_used,
    input.confidence,
  )
  .map_err(map_error)?;
  study_repo::record_review_history(&connection, input.session_id, &review_unit, input.rating, &update).map_err(map_error)?;
  review_unit_repo::sync_card_cache(&connection, review_unit.card_id).map_err(map_error)?;

  Ok(GradeCardResponse {
    card_id: input.card_id,
    review_unit_id: review_unit.id,
    review_state: update.state,
    due_at_utc: update.due_at_utc.clone(),
    scheduled_interval_days: update.scheduled_interval_days,
    retrievability_before: update.retrievability_before,
    difficulty: update.difficulty,
    stability_days: update.stability,
    mastered: update.mastered,
    leech: update.leech,
    newly_mastered: update.newly_mastered,
  })
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
