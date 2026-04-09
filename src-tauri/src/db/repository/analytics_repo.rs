use std::collections::HashMap;

use anyhow::Result;
use chrono::{Days, NaiveDate, Utc};
use rusqlite::{params, Connection};

use crate::models::types::{CardStatus, LearningBalance, OverviewMetrics, ProgressPoint, WeakCardAnalytics};

fn now_utc() -> String {
  Utc::now().to_rfc3339()
}

fn round_percent(numerator: i64, denominator: i64) -> i64 {
  if denominator <= 0 {
    0
  } else {
    ((numerator as f64 / denominator as f64) * 100.0).round() as i64
  }
}

pub fn get_overview_metrics(connection: &Connection) -> Result<OverviewMetrics> {
  let now = now_utc();
  let (total_cards, new_cards, due_cards, mastered_cards) = connection.query_row(
    "SELECT
      COUNT(*) AS total_cards,
      COALESCE(SUM(CASE WHEN status = 'new' THEN 1 ELSE 0 END), 0) AS new_cards,
      COALESCE(SUM(CASE WHEN status != 'new' AND next_review_at IS NOT NULL AND next_review_at <= ?1 THEN 1 ELSE 0 END), 0) AS due_cards,
      COALESCE(SUM(CASE WHEN status = 'mastered' THEN 1 ELSE 0 END), 0) AS mastered_cards
    FROM cards",
    params![now],
    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?, row.get::<_, i64>(3)?)),
  )?;

  let (total_reviews_completed, total_correct_reviews) = connection.query_row(
    "SELECT COUNT(*) AS total_reviews_completed, COALESCE(SUM(knew_it), 0) AS total_correct_reviews FROM review_history",
    [],
    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
  )?;

  let review_accuracy_percent = round_percent(total_correct_reviews, total_reviews_completed);

  Ok(OverviewMetrics {
    total_cards,
    new_cards,
    due_cards,
    mastered_cards,
    total_reviews_completed,
    review_accuracy_percent,
    retention_score_percent: review_accuracy_percent,
  })
}

pub fn get_progress_points(connection: &Connection, period_days: i64) -> Result<Vec<ProgressPoint>> {
  let today = Utc::now().date_naive();
  let start = today.checked_sub_days(Days::new((period_days.saturating_sub(1)) as u64)).unwrap_or(today);
  let end_exclusive = today.checked_add_days(Days::new(1)).unwrap_or(today);

  let mut statement = connection.prepare(
    "SELECT
      substr(reviewed_at, 1, 10) AS utc_date,
      COUNT(*) AS reviews_completed,
      COALESCE(CAST(ROUND(SUM(knew_it) * 100.0 / COUNT(*)) AS INTEGER), 0) AS accuracy_percent,
      COALESCE(SUM(CASE WHEN knew_it = 1 AND previous_status = 'new' THEN 1 ELSE 0 END), 0) AS new_cards_learned
    FROM review_history
    WHERE reviewed_at >= ?1 AND reviewed_at < ?2
    GROUP BY substr(reviewed_at, 1, 10)
    ORDER BY utc_date ASC",
  )?;

  let rows = statement.query_map(params![start.to_string(), end_exclusive.to_string()], |row| {
    Ok(ProgressPoint {
      utc_date: row.get("utc_date")?,
      reviews_completed: row.get("reviews_completed")?,
      accuracy_percent: row.get("accuracy_percent")?,
      new_cards_learned: row.get("new_cards_learned")?,
    })
  })?;

  let mut by_date = HashMap::new();
  for row in rows {
    let point = row?;
    by_date.insert(point.utc_date.clone(), point);
  }

  let mut points = Vec::new();
  let mut current = start;
  while current < end_exclusive {
    let date_key = current.to_string();
    points.push(by_date.remove(&date_key).unwrap_or(ProgressPoint {
      utc_date: date_key,
      reviews_completed: 0,
      accuracy_percent: 0,
      new_cards_learned: 0,
    }));
    current = current.checked_add_days(Days::new(1)).unwrap_or(end_exclusive);
  }

  Ok(points)
}

pub fn get_learning_balance(connection: &Connection, period_days: i64) -> Result<LearningBalance> {
  let today = Utc::now().date_naive();
  let start = today.checked_sub_days(Days::new((period_days.saturating_sub(1)) as u64)).unwrap_or(today);
  let end_exclusive = today.checked_add_days(Days::new(1)).unwrap_or(today);

  let (new_card_reviews, review_card_reviews) = connection.query_row(
    "SELECT
      COALESCE(SUM(CASE WHEN previous_status = 'new' THEN 1 ELSE 0 END), 0) AS new_card_reviews,
      COALESCE(SUM(CASE WHEN previous_status != 'new' THEN 1 ELSE 0 END), 0) AS review_card_reviews
    FROM review_history
    WHERE reviewed_at >= ?1 AND reviewed_at < ?2",
    params![start.to_string(), end_exclusive.to_string()],
    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
  )?;

  let total = new_card_reviews + review_card_reviews;
  Ok(LearningBalance {
    new_card_reviews,
    review_card_reviews,
    new_card_percent: round_percent(new_card_reviews, total),
    review_card_percent: round_percent(review_card_reviews, total),
  })
}

pub fn get_review_dates(connection: &Connection) -> Result<Vec<NaiveDate>> {
  let mut statement = connection.prepare(
    "SELECT DISTINCT substr(reviewed_at, 1, 10) AS utc_date
    FROM review_history
    ORDER BY utc_date ASC",
  )?;

  let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
  let mut dates = Vec::new();
  for row in rows {
    let value = row?;
    if let Ok(date) = NaiveDate::parse_from_str(&value, "%Y-%m-%d") {
      dates.push(date);
    }
  }
  Ok(dates)
}

pub fn get_today_review_count(connection: &Connection) -> Result<i64> {
  let today = Utc::now().date_naive().to_string();
  connection
    .query_row(
      "SELECT COUNT(*) FROM review_history WHERE substr(reviewed_at, 1, 10) = ?1",
      params![today],
      |row| row.get::<_, i64>(0),
    )
    .map_err(Into::into)
}

pub fn get_weak_cards(connection: &Connection, limit: usize) -> Result<Vec<WeakCardAnalytics>> {
  let mut statement = connection.prepare(
    "SELECT
      c.id AS card_id,
      c.deck_id,
      d.name AS deck_name,
      c.language_1,
      c.language_2,
      c.language_3,
      c.status,
      c.wrong_count,
      c.mastery_score,
      COALESCE((
        SELECT COUNT(*)
        FROM review_history rh
        WHERE rh.card_id = c.id
          AND rh.new_status = 'learning'
          AND rh.previous_status IN ('review', 'mastered')
      ), 0) AS relearn_count,
      COALESCE((
        SELECT CAST(ROUND(AVG(CASE WHEN recent.knew_it = 1 THEN 100.0 ELSE 0 END)) AS INTEGER)
        FROM (
          SELECT knew_it
          FROM review_history rh2
          WHERE rh2.card_id = c.id
          ORDER BY reviewed_at DESC
          LIMIT 10
        ) recent
      ), 0) AS recent_success_rate_percent
    FROM cards c
    INNER JOIN decks d ON d.id = c.deck_id
    WHERE c.review_count > 0 OR c.wrong_count > 0 OR c.status = 'learning' OR c.mastery_score < 70",
  )?;

  let rows = statement.query_map([], |row| {
    Ok(WeakCardAnalytics {
      card_id: row.get("card_id")?,
      deck_id: row.get("deck_id")?,
      deck_name: row.get("deck_name")?,
      language_1: row.get("language_1")?,
      language_2: row.get("language_2")?,
      language_3: row.get("language_3")?,
      status: CardStatus::from_db(&row.get::<_, String>("status")?),
      wrong_count: row.get("wrong_count")?,
      mastery_score: row.get("mastery_score")?,
      relearn_count: row.get("relearn_count")?,
      recent_success_rate_percent: row.get("recent_success_rate_percent")?,
      difficulty_score: 0,
      needs_attention: false,
    })
  })?;

  let mut cards = rows
    .collect::<rusqlite::Result<Vec<_>>>()?
    .into_iter()
    .map(|mut card| {
      let low_mastery_penalty = (100 - card.mastery_score).clamp(0, 100);
      let low_recent_success_penalty = (100 - card.recent_success_rate_percent).clamp(0, 100);
      let status_penalty = match card.status {
        CardStatus::Learning => 18,
        CardStatus::Review => 6,
        CardStatus::Mastered => 0,
        CardStatus::New => 0,
      };
      card.difficulty_score =
        card.wrong_count * 12 + card.relearn_count * 20 + low_mastery_penalty + (low_recent_success_penalty / 2) + status_penalty;
      card.needs_attention =
        card.wrong_count >= 2 || card.mastery_score < 50 || card.relearn_count >= 2 || card.recent_success_rate_percent < 60;
      card
    })
    .filter(|card| card.difficulty_score > 0)
    .collect::<Vec<_>>();

  cards.sort_by(|left, right| {
    right
      .difficulty_score
      .cmp(&left.difficulty_score)
      .then(right.wrong_count.cmp(&left.wrong_count))
      .then(left.mastery_score.cmp(&right.mastery_score))
  });
  cards.truncate(limit);
  Ok(cards)
}

#[cfg(test)]
mod tests {
  use std::time::{SystemTime, UNIX_EPOCH};

  use chrono::{Duration, Utc};
  use rusqlite::{params, Connection};

  use crate::db::{initialize_database, open_connection, repository::deck_repo};
  use crate::models::types::{CreateCardInput, CreateDeckInput};

  use super::{get_learning_balance, get_overview_metrics, get_progress_points, get_review_dates, get_today_review_count, get_weak_cards};

  fn temp_db_path() -> std::path::PathBuf {
    let unique = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .unwrap()
      .as_nanos();
    std::env::temp_dir().join(format!("flashcard-local-analytics-{unique}.sqlite"))
  }

  fn seed_review_history(connection: &Connection, card_id: i64, deck_id: i64, days_ago: i64, knew_it: bool, previous_status: &str, new_status: &str) {
    let reviewed_at = (Utc::now() - Duration::days(days_ago)).to_rfc3339();
    connection.execute(
      "INSERT INTO review_history (
        card_id, deck_id, session_id, reviewed_at, knew_it, previous_status, new_status,
        previous_interval_minutes, new_interval_minutes, previous_ease_factor, new_ease_factor,
        previous_mastery_score, new_mastery_score
      ) VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?6, 10, 10, 2.2, 2.2, 40, 60)",
      params![card_id, deck_id, reviewed_at, if knew_it { 1 } else { 0 }, previous_status, new_status],
    ).unwrap();
  }

  #[test]
  fn analytics_queries_cover_overview_progress_and_weak_cards() {
    let db_path = temp_db_path();
    initialize_database(&db_path).unwrap();
    let connection = open_connection(&db_path).unwrap();

    let deck = deck_repo::create_deck(
      &connection,
      &CreateDeckInput {
        name: "Analytics".into(),
        description: None,
        language_1_label: None,
        language_2_label: None,
        language_3_label: None,
      },
    )
    .unwrap();

    let new_card = crate::db::repository::card_repo::create_card(
      &connection,
      &CreateCardInput {
        deck_id: deck.id,
        language_1: "سلام".into(),
        language_2: "Hello".into(),
        language_3: "Ciao".into(),
        note: None,
        example_sentence: None,
        tag: None,
      },
    )
    .unwrap();

    let review_card = crate::db::repository::card_repo::create_card(
      &connection,
      &CreateCardInput {
        deck_id: deck.id,
        language_1: "کتاب".into(),
        language_2: "Book".into(),
        language_3: "Libro".into(),
        note: None,
        example_sentence: None,
        tag: None,
      },
    )
    .unwrap();

    connection.execute(
      "UPDATE cards SET status = 'review', next_review_at = ?1, wrong_count = 3, mastery_score = 25, review_count = 4 WHERE id = ?2",
      params![Utc::now().to_rfc3339(), review_card.id],
    ).unwrap();

    seed_review_history(&connection, new_card.id, deck.id, 0, true, "new", "learning");
    seed_review_history(&connection, review_card.id, deck.id, 0, false, "review", "learning");
    seed_review_history(&connection, review_card.id, deck.id, 1, true, "learning", "review");

    let overview = get_overview_metrics(&connection).unwrap();
    assert_eq!(overview.total_cards, 2);
    assert_eq!(overview.new_cards, 1);
    assert_eq!(overview.due_cards, 1);
    assert_eq!(overview.total_reviews_completed, 3);

    let progress = get_progress_points(&connection, 7).unwrap();
    assert_eq!(progress.len(), 7);
    assert!(progress.iter().any(|point| point.reviews_completed > 0));

    let balance = get_learning_balance(&connection, 7).unwrap();
    assert_eq!(balance.new_card_reviews, 1);
    assert_eq!(balance.review_card_reviews, 2);

    let weak_cards = get_weak_cards(&connection, 5).unwrap();
    assert_eq!(weak_cards[0].card_id, review_card.id);
    assert!(weak_cards[0].needs_attention);

    let review_dates = get_review_dates(&connection).unwrap();
    assert!(!review_dates.is_empty());
    assert_eq!(get_today_review_count(&connection).unwrap(), 2);

    let _ = std::fs::remove_file(db_path);
  }
}
