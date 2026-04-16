use anyhow::Result;
use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::Connection;

use crate::{
  db::repository::analytics_repo,
  models::types::{AppSettings, DailyCoachRecommendation, DailyCoachResponse, UiLanguage, UiPreferences},
};

fn localized_priority(language: UiLanguage, score: f64) -> String {
  match language {
    UiLanguage::Fa => {
      if score >= 48.0 {
        "اولویت بالا".to_string()
      } else if score >= 24.0 {
        "نیازمند توجه".to_string()
      } else {
        "پایدار".to_string()
      }
    }
    _ => {
      if score >= 48.0 {
        "High priority".to_string()
      } else if score >= 24.0 {
        "Needs attention".to_string()
      } else {
        "Steady".to_string()
      }
    }
  }
}

fn localized_reason(language: UiLanguage, key: &str, value: i64) -> String {
  match (language, key) {
    (UiLanguage::Fa, "overdue_pressure") => "این دک فشار مرورهای عقب‌افتاده دارد.".to_string(),
    (UiLanguage::Fa, "due_pressure") => "این دک همین حالا مرورهای موعددار دارد.".to_string(),
    (UiLanguage::Fa, "neglected") => format!("{} روز است به این دک سر نزده‌اید.", value),
    (UiLanguage::Fa, "never_started") => "این دک هنوز شروع نشده و آماده‌ی اولین مطالعه است.".to_string(),
    (UiLanguage::Fa, "weak_directions") => "این دک جهت‌های ضعیف و ناپایدار دارد.".to_string(),
    (UiLanguage::Fa, "daily_goal") => "این دک به رسیدن به هدف مرور امروز کمک می‌کند.".to_string(),
    (UiLanguage::Fa, "upcoming_load") => "اگر امروز رسیدگی نشود، فشار مرورهای آینده بیشتر می‌شود.".to_string(),
    (UiLanguage::Fa, "new_cards") => "این دک کارت‌های جدید آماده‌ی یادگیری دارد.".to_string(),
    (_, "overdue_pressure") => "This deck has overdue review pressure.".to_string(),
    (_, "due_pressure") => "This deck has due reviews waiting now.".to_string(),
    (_, "neglected") => format!("You have not visited this deck in {value} days."),
    (_, "never_started") => "This deck has not been started yet and is ready for first study.".to_string(),
    (_, "weak_directions") => "This deck contains weak directions.".to_string(),
    (_, "daily_goal") => "This deck supports your daily review goal.".to_string(),
    (_, "upcoming_load") => "This deck needs attention before upcoming due load increases.".to_string(),
    (_, "new_cards") => "This deck has new cards ready for learning.".to_string(),
    _ => String::new(),
  }
}

fn days_since_last_study(last_studied_at: &Option<String>, today: NaiveDate) -> i64 {
  let Some(value) = last_studied_at else {
    return 0;
  };

  let Ok(parsed) = DateTime::parse_from_rfc3339(value) else {
    return 0;
  };

  let last = parsed.with_timezone(&Utc).date_naive();
  today.signed_duration_since(last).num_days().max(0)
}

pub fn get_daily_coach(
  connection: &Connection,
  settings: &AppSettings,
  preferences: &UiPreferences,
) -> Result<DailyCoachResponse> {
  let today = Utc::now().date_naive();
  let today_utc_date = today.to_string();
  let completed_today = analytics_repo::get_today_review_count(connection)?;
  let studied_today = completed_today > 0;
  let dismissed_today =
    preferences.daily_coach_last_dismissed_utc_date.as_deref() == Some(today_utc_date.as_str());
  let daily_goal_remaining = (settings.daily_review_goal as i64 - completed_today).max(0);
  let signals = analytics_repo::get_daily_coach_signals(connection)?;

  let mut recommendations = signals
    .into_iter()
    .filter_map(|signal| {
      if signal.due_cards <= 0 && signal.new_cards <= 0 && signal.weak_direction_count <= 0 && signal.upcoming_due_7d <= 0 {
        return None;
      }

      let days_since = days_since_last_study(&signal.last_studied_at, today);
      let overdue_ratio = if signal.due_cards > 0 {
        signal.overdue_cards as f64 / signal.due_cards as f64
      } else {
        0.0
      };

      let mut contributions = Vec::<(f64, String)>::new();

      if signal.overdue_cards > 0 {
        contributions.push((
          signal.overdue_cards as f64 * 5.4 + overdue_ratio * 14.0,
          localized_reason(settings.ui_language, "overdue_pressure", signal.overdue_cards),
        ));
      }

      if signal.due_cards > 0 {
        contributions.push((
          signal.due_cards as f64 * 2.8,
          localized_reason(settings.ui_language, "due_pressure", signal.due_cards),
        ));
      }

      if days_since >= 3 {
        contributions.push((
          (days_since.min(12) as f64) * 1.8,
          localized_reason(settings.ui_language, "neglected", days_since),
        ));
      } else if signal.last_studied_at.is_none() && signal.new_cards > 0 {
        contributions.push((9.0, localized_reason(settings.ui_language, "never_started", 0)));
      }

      if signal.weak_direction_count > 0 {
        contributions.push((
          signal.weak_direction_count as f64 * 3.1,
          localized_reason(settings.ui_language, "weak_directions", signal.weak_direction_count),
        ));
      }

      if daily_goal_remaining > 0 && (signal.due_cards > 0 || signal.new_cards > 0) {
        contributions.push((7.5, localized_reason(settings.ui_language, "daily_goal", daily_goal_remaining)));
      }

      if signal.upcoming_due_7d > signal.due_cards {
        contributions.push((
          (signal.upcoming_due_7d - signal.due_cards) as f64 * 0.9,
          localized_reason(settings.ui_language, "upcoming_load", signal.upcoming_due_7d),
        ));
      }

      if signal.due_cards == 0 && signal.new_cards > 0 {
        contributions.push((
          signal.new_cards.min(12) as f64 * 0.75,
          localized_reason(settings.ui_language, "new_cards", signal.new_cards),
        ));
      }

      if contributions.is_empty() {
        return None;
      }

      contributions.sort_by(|left, right| right.0.total_cmp(&left.0));
      let urgency_score = contributions.iter().map(|(score, _)| *score).sum::<f64>();
      let reason_text = contributions.first().map(|(_, reason)| reason.clone()).unwrap_or_default();
      let supporting_reasons = contributions
        .into_iter()
        .skip(1)
        .map(|(_, reason)| reason)
        .filter(|reason| reason != &reason_text)
        .take(2)
        .collect::<Vec<_>>();

      Some(DailyCoachRecommendation {
        deck_id: signal.deck_id,
        deck_name: signal.deck_name,
        urgency_score: (urgency_score * 10.0).round() / 10.0,
        priority_label: localized_priority(settings.ui_language, urgency_score),
        due_cards: signal.due_cards,
        overdue_cards: signal.overdue_cards,
        new_cards: signal.new_cards,
        weak_direction_count: signal.weak_direction_count,
        upcoming_due_7d: signal.upcoming_due_7d,
        days_since_last_study: days_since,
        last_studied_at: signal.last_studied_at,
        reason_text,
        supporting_reasons,
      })
    })
    .collect::<Vec<_>>();

  recommendations.sort_by(|left, right| {
    right
      .urgency_score
      .total_cmp(&left.urgency_score)
      .then(right.overdue_cards.cmp(&left.overdue_cards))
      .then(right.due_cards.cmp(&left.due_cards))
      .then(left.deck_name.cmp(&right.deck_name))
  });
  recommendations.truncate(4);

  Ok(DailyCoachResponse {
    today_utc_date,
    studied_today,
    dismissed_today,
    should_prompt: !studied_today && !dismissed_today && !recommendations.is_empty(),
    daily_goal_remaining,
    recommendations,
  })
}

#[cfg(test)]
mod tests {
  use std::time::{SystemTime, UNIX_EPOCH};

  use chrono::{Duration, Utc};
  use rusqlite::{params, Connection};

  use crate::{
    db::{initialize_database, open_connection, repository::deck_repo},
    models::types::{AppSettings, CardValueInput, CreateCardInput, CreateDeckInput, DeckFieldInput, UiPreferences},
  };

  use super::get_daily_coach;

  fn temp_db_path() -> std::path::PathBuf {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    std::env::temp_dir().join(format!("codo-daily-coach-{unique}.sqlite"))
  }

  fn create_test_deck(connection: &Connection, name: &str) -> crate::models::types::DeckDetail {
    deck_repo::create_deck(
      connection,
      &CreateDeckInput {
        name: name.to_string(),
        description: None,
        fields: vec![
          DeckFieldInput {
            id: None,
            label: "Persian".into(),
            language_code: Some("persian".into()),
            order_index: 0,
            required: true,
            active: true,
            field_type: Some("text".into()),
          },
          DeckFieldInput {
            id: None,
            label: "English".into(),
            language_code: Some("english".into()),
            order_index: 1,
            required: true,
            active: true,
            field_type: Some("text".into()),
          },
        ],
      },
    )
    .unwrap()
  }

  #[test]
  fn ranks_overdue_and_weak_decks_first() {
    let db_path = temp_db_path();
    initialize_database(&db_path).unwrap();
    let connection = open_connection(&db_path).unwrap();

    let urgent = create_test_deck(&connection, "Urgent");
    let fresh = create_test_deck(&connection, "Fresh");

    let urgent_card = crate::db::repository::card_repo::create_card(
      &connection,
      &CreateCardInput {
        deck_id: urgent.id,
        values: vec![
          CardValueInput { field_id: urgent.fields[0].id, value: "سلام".into() },
          CardValueInput { field_id: urgent.fields[1].id, value: "Hello".into() },
        ],
      },
    )
    .unwrap();

    let fresh_card = crate::db::repository::card_repo::create_card(
      &connection,
      &CreateCardInput {
        deck_id: fresh.id,
        values: vec![
          CardValueInput { field_id: fresh.fields[0].id, value: "کتاب".into() },
          CardValueInput { field_id: fresh.fields[1].id, value: "Book".into() },
        ],
      },
    )
    .unwrap();

    let now = Utc::now();
    connection.execute(
      "UPDATE decks SET last_studied_at = ?1 WHERE id = ?2",
      params![(now - Duration::days(5)).to_rfc3339(), urgent.id],
    ).unwrap();

    connection.execute(
      "INSERT INTO review_units (
        card_id, deck_id, prompt_field_id, reveal_field_ids, direction_key, state, difficulty, stability,
        scheduled_interval_days, last_reviewed_at_utc, due_at_utc, lapses, successful_reviews, failed_reviews,
        total_reviews, same_day_reviews_count, hint_used_last, suspended, leech, mastered, learning_step_index,
        relearning_step_index, created_at, updated_at
      ) VALUES (?1, ?2, ?3, '[2]', 'field:1|2', 'relearning', 6.2, 1.4, 1.0, ?4, ?5, 2, 3, 3, 6, 0, 0, 0, 1, 0, 0, 1, ?4, ?4)",
      params![
        urgent_card.id,
        urgent.id,
        urgent.fields[0].id,
        (now - Duration::days(2)).to_rfc3339(),
        (now - Duration::days(2)).to_rfc3339()
      ],
    ).unwrap();

    connection.execute(
      "INSERT INTO review_units (
        card_id, deck_id, prompt_field_id, reveal_field_ids, direction_key, state, difficulty, stability,
        scheduled_interval_days, due_at_utc, lapses, successful_reviews, failed_reviews, total_reviews,
        same_day_reviews_count, hint_used_last, suspended, leech, mastered, learning_step_index,
        relearning_step_index, created_at, updated_at
      ) VALUES (?1, ?2, ?3, '[2]', 'field:1|2', 'new', 5.0, 0.4, 0.0, NULL, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, ?4, ?4)",
      params![fresh_card.id, fresh.id, fresh.fields[0].id, now.to_rfc3339()],
    ).unwrap();

    let response = get_daily_coach(&connection, &AppSettings::default(), &UiPreferences::default()).unwrap();

    assert!(!response.recommendations.is_empty());
    assert_eq!(response.recommendations[0].deck_name, "Urgent");
    assert!(response.recommendations[0].overdue_cards > 0);
    assert!(response.should_prompt);

    let _ = std::fs::remove_file(db_path);
  }

  #[test]
  fn respects_dismissal_for_today() {
    let db_path = temp_db_path();
    initialize_database(&db_path).unwrap();
    let connection = open_connection(&db_path).unwrap();
    let deck = create_test_deck(&connection, "Deck");

    let card = crate::db::repository::card_repo::create_card(
      &connection,
      &CreateCardInput {
        deck_id: deck.id,
        values: vec![
          CardValueInput { field_id: deck.fields[0].id, value: "خانه".into() },
          CardValueInput { field_id: deck.fields[1].id, value: "House".into() },
        ],
      },
    )
    .unwrap();
    let now = Utc::now();
    connection.execute(
      "INSERT INTO review_units (
        card_id, deck_id, prompt_field_id, reveal_field_ids, direction_key, state, difficulty, stability,
        scheduled_interval_days, due_at_utc, lapses, successful_reviews, failed_reviews, total_reviews,
        same_day_reviews_count, hint_used_last, suspended, leech, mastered, learning_step_index,
        relearning_step_index, created_at, updated_at
      ) VALUES (?1, ?2, ?3, '[2]', 'field:1|2', 'review', 5.4, 3.5, 3.0, ?4, 0, 2, 1, 3, 0, 0, 0, 0, 0, 0, 0, ?5, ?5)",
      params![card.id, deck.id, deck.fields[0].id, now.to_rfc3339(), now.to_rfc3339()],
    ).unwrap();

    let response = get_daily_coach(
      &connection,
      &AppSettings::default(),
      &UiPreferences {
        daily_coach_last_dismissed_utc_date: Some(Utc::now().date_naive().to_string()),
        ..UiPreferences::default()
      },
    )
    .unwrap();

    assert!(response.dismissed_today);
    assert!(!response.should_prompt);

    let _ = std::fs::remove_file(db_path);
  }
}
