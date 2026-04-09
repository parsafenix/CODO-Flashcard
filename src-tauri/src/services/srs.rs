use chrono::{Duration, Utc};

use crate::models::types::{CardSchedulingRecord, CardStatus, SchedulingUpdate};

fn utc_string_from_now(minutes: i64) -> String {
  (Utc::now() + Duration::minutes(minutes)).to_rfc3339()
}

pub fn schedule_review(record: &CardSchedulingRecord, knew_it: bool) -> SchedulingUpdate {
  let previous_status = record.status;
  let previous_interval = record.current_interval_minutes.max(0);
  let previous_mastery = record.mastery_score;
  let previous_ease = record.ease_factor;

  let review_count = record.review_count + 1;
  let last_reviewed_at = Utc::now().to_rfc3339();

  if knew_it {
    let correct_count = record.correct_count + 1;
    let wrong_count = record.wrong_count;

    match previous_status {
      CardStatus::New => SchedulingUpdate {
        status: CardStatus::Learning,
        review_count,
        correct_count,
        wrong_count,
        current_interval_minutes: 10,
        ease_factor: 2.2_f64.max(previous_ease),
        mastery_score: (previous_mastery + 15).clamp(0, 100),
        last_reviewed_at,
        next_review_at: Some(utc_string_from_now(10)),
        newly_mastered: false,
      },
      // Any correct answer while the card is actively in the learning bucket
      // graduates it back to scheduled review with at least a one-day interval.
      CardStatus::Learning => {
        let next_interval = if previous_interval < 24 * 60 {
          24 * 60
        } else {
          ((previous_interval as f64) * previous_ease.clamp(1.3, 3.0)).round() as i64
        }
        .max(24 * 60);

        SchedulingUpdate {
          status: CardStatus::Review,
          review_count,
          correct_count,
          wrong_count,
          current_interval_minutes: next_interval,
          ease_factor: (previous_ease + 0.05).clamp(1.3, 3.0),
          mastery_score: (previous_mastery + 20).clamp(0, 100),
          last_reviewed_at,
          next_review_at: Some(utc_string_from_now(next_interval)),
          newly_mastered: false,
        }
      }
      _ => {
        let base_interval = if previous_interval <= 0 { 24 * 60 } else { previous_interval };
        let growth = (base_interval as f64 * previous_ease.clamp(1.3, 3.0)).round() as i64;
        let next_interval = growth.max(base_interval + 24 * 60);
        let next_mastery = (previous_mastery + 12).clamp(0, 100);
        let next_status = if next_mastery >= 85 && next_interval >= 30 * 24 * 60 {
          CardStatus::Mastered
        } else {
          CardStatus::Review
        };

        SchedulingUpdate {
          status: next_status,
          review_count,
          correct_count,
          wrong_count,
          current_interval_minutes: next_interval,
          ease_factor: (previous_ease + 0.1).clamp(1.3, 3.0),
          mastery_score: next_mastery,
          last_reviewed_at,
          next_review_at: Some(utc_string_from_now(next_interval)),
          newly_mastered: previous_status != CardStatus::Mastered && next_status == CardStatus::Mastered,
        }
      }
    }
  } else {
    let correct_count = record.correct_count;
    let wrong_count = record.wrong_count + 1;
    let next_interval = match previous_status {
      CardStatus::New => 5,
      CardStatus::Learning => 10,
      CardStatus::Review | CardStatus::Mastered => (previous_interval / 4).max(10),
    };

    SchedulingUpdate {
      status: CardStatus::Learning,
      review_count,
      correct_count,
      wrong_count,
      current_interval_minutes: next_interval,
      ease_factor: (previous_ease - 0.2).clamp(1.3, 3.0),
      mastery_score: (previous_mastery - 20).clamp(0, 100),
      last_reviewed_at,
      next_review_at: Some(utc_string_from_now(next_interval)),
      newly_mastered: false,
    }
  }
}

#[cfg(test)]
mod tests {
  use crate::models::types::{CardSchedulingRecord, CardStatus};

  use super::schedule_review;

  fn base_record(status: CardStatus) -> CardSchedulingRecord {
    CardSchedulingRecord {
      id: 1,
      deck_id: 1,
      status,
      review_count: 0,
      correct_count: 0,
      wrong_count: 0,
      current_interval_minutes: 0,
      ease_factor: 2.2,
      mastery_score: 0,
      last_reviewed_at: None,
      next_review_at: None,
    }
  }

  #[test]
  fn first_correct_moves_new_card_to_learning() {
    let updated = schedule_review(&base_record(CardStatus::New), true);
    assert_eq!(updated.status, CardStatus::Learning);
    assert_eq!(updated.current_interval_minutes, 10);
  }

  #[test]
  fn wrong_answer_drops_card_to_learning() {
    let mut record = base_record(CardStatus::Review);
    record.current_interval_minutes = 60 * 24 * 7;
    record.review_count = 3;
    record.correct_count = 3;
    record.mastery_score = 60;
    let updated = schedule_review(&record, false);
    assert_eq!(updated.status, CardStatus::Learning);
    assert!(updated.current_interval_minutes < record.current_interval_minutes);
  }

  #[test]
  fn correct_answer_in_learning_graduates_to_review() {
    let mut record = base_record(CardStatus::Learning);
    record.current_interval_minutes = 10;
    record.review_count = 1;
    record.correct_count = 1;
    let updated = schedule_review(&record, true);
    assert_eq!(updated.status, CardStatus::Review);
    assert_eq!(updated.current_interval_minutes, 24 * 60);
  }
}
