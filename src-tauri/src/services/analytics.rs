use anyhow::Result;
use chrono::{Days, Local, NaiveDate, Timelike, Utc};
use rusqlite::Connection;

use crate::{
  db::repository::{analytics_repo, scheduler_repo},
  models::types::{
    AnalyticsRequest, AnalyticsResponse, AppSettings, DailyGoalProgress, OverviewMetrics, ReminderState, StreakStats, UiLanguage,
  },
  services::calibration,
};

fn normalize_period_days(request: &AnalyticsRequest) -> i64 {
  match request.period_days {
    30 => 30,
    _ => 7,
  }
}

fn round_percent(numerator: i64, denominator: i64) -> i64 {
  if denominator <= 0 {
    0
  } else {
    ((numerator as f64 / denominator as f64) * 100.0).round() as i64
  }
}

pub fn reminder_time_passed_local(reminder_time: &str) -> bool {
  let mut parts = reminder_time.split(':');
  let Some(hours) = parts.next().and_then(|value| value.parse::<u32>().ok()) else {
    return false;
  };
  let Some(minutes) = parts.next().and_then(|value| value.parse::<u32>().ok()) else {
    return false;
  };
  if hours > 23 || minutes > 59 {
    return false;
  }

  let now = Local::now();
  let current_minutes = now.hour() * 60 + now.minute();
  let reminder_minutes = hours * 60 + minutes;
  current_minutes >= reminder_minutes
}

pub fn calculate_streak(review_dates: &[NaiveDate], today: NaiveDate) -> StreakStats {
  if review_dates.is_empty() {
    return StreakStats {
      current_streak: 0,
      longest_streak: 0,
      studied_today: false,
    };
  }

  let studied_today = review_dates.last().copied() == Some(today);

  let mut longest_streak = 1_i64;
  let mut current_run = 1_i64;
  for pair in review_dates.windows(2) {
    if pair[1].signed_duration_since(pair[0]).num_days() == 1 {
      current_run += 1;
      longest_streak = longest_streak.max(current_run);
    } else {
      current_run = 1;
    }
  }

  let mut current_streak = 0_i64;
  if studied_today {
    current_streak = 1;
    let mut cursor = today;
    for date in review_dates.iter().rev().skip(1) {
      let expected = cursor.checked_sub_days(Days::new(1)).unwrap_or(cursor);
      if *date == expected {
        current_streak += 1;
        cursor = *date;
      } else {
        break;
      }
    }
  }

  StreakStats {
    current_streak,
    longest_streak,
    studied_today,
  }
}

fn build_daily_goal(settings: &AppSettings, completed_today: i64, today_utc_date: String) -> DailyGoalProgress {
  let goal = settings.daily_review_goal as i64;
  DailyGoalProgress {
    daily_review_goal: goal,
    completed_today,
    remaining_today: (goal - completed_today).max(0),
    percent_complete: round_percent(completed_today.min(goal), goal),
    today_utc_date,
  }
}

fn build_reminder(settings: &AppSettings, overview: &OverviewMetrics, today_utc_date: String) -> ReminderState {
  let dismissed_today = settings.reminder_last_acknowledged_date.as_deref() == Some(today_utc_date.as_str());
  ReminderState {
    enabled: settings.reminder_enabled,
    reminder_time: settings.reminder_time.clone(),
    due_cards: overview.due_cards,
    should_show: settings.reminder_enabled
      && overview.due_cards > 0
      && !dismissed_today
      && reminder_time_passed_local(&settings.reminder_time),
    today_utc_date,
  }
}

fn build_insights(
  language: UiLanguage,
  overview: &OverviewMetrics,
  weak_cards_len: usize,
  streak: &StreakStats,
  daily_goal: &DailyGoalProgress,
  progress: &[crate::models::types::ProgressPoint],
) -> Vec<String> {
  let mut insights = Vec::new();

  if overview.due_cards > 0 {
    insights.push(match language {
      UiLanguage::Fa => format!("{} کارت مرورِ موعدگذشته منتظر شما هستند.", overview.due_cards),
      _ => format!("You have {} overdue review cards waiting.", overview.due_cards),
    });
  }

  if weak_cards_len > 0 {
    insights.push(match language {
      UiLanguage::Fa => format!("روی {} کارت هنوز نیاز به تمرکز بیشتری دارید.", weak_cards_len),
      _ => format!("You are struggling with {} cards.", weak_cards_len),
    });
  }

  if streak.current_streak >= 2 {
    insights.push(match language {
      UiLanguage::Fa => format!("{} روز پیاپی مطالعه کرده‌اید.", streak.current_streak),
      _ => format!("You studied {} days in a row.", streak.current_streak),
    });
  }

  let recent_non_zero = progress
    .iter()
    .rev()
    .filter(|point| point.reviews_completed > 0)
    .take(6)
    .cloned()
    .collect::<Vec<_>>();
  if recent_non_zero.len() >= 4 {
    let midpoint = recent_non_zero.len() / 2;
    let latest = &recent_non_zero[..midpoint];
    let earlier = &recent_non_zero[midpoint..];
    let latest_accuracy = latest.iter().map(|point| point.accuracy_percent).sum::<i64>() / latest.len() as i64;
    let earlier_accuracy = earlier.iter().map(|point| point.accuracy_percent).sum::<i64>() / earlier.len() as i64;
    if latest_accuracy > earlier_accuracy {
      insights.push(match language {
        UiLanguage::Fa => "دقت مرور شما این هفته بهتر شده است.".to_string(),
        _ => "Your review accuracy improved this week.".to_string(),
      });
    }
  }

  if daily_goal.remaining_today > 0 && overview.total_reviews_completed > 0 {
    insights.push(match language {
      UiLanguage::Fa => format!("تا هدف امروز فقط {} مرور فاصله دارید.", daily_goal.remaining_today),
      _ => format!("You are {} reviews away from today's goal.", daily_goal.remaining_today),
    });
  }

  insights.truncate(4);
  insights
}

pub fn get_analytics(connection: &Connection, settings: &AppSettings, request: &AnalyticsRequest) -> Result<AnalyticsResponse> {
  let period_days = normalize_period_days(request);
  let active_parameters = scheduler_repo::get_active_parameters(connection).unwrap_or_default();
  let overview = analytics_repo::get_overview_metrics(connection)?;
  let outcomes = analytics_repo::get_learning_outcomes(connection)?;
  let progress = analytics_repo::get_progress_points(connection, period_days)?;
  let weak_cards = analytics_repo::get_weak_cards(connection, 12)?;
  let scheduler_health = analytics_repo::get_scheduler_health(connection, settings.desired_retention, &active_parameters)?;
  let calibration = calibration::get_calibration_status(connection)?;
  let content_quality = analytics_repo::get_content_quality(connection, &weak_cards)?;
  let learning_balance = analytics_repo::get_learning_balance(connection, period_days)?;
  let review_dates = analytics_repo::get_review_dates(connection)?;
  let today_utc = Utc::now().date_naive();
  let streak = calculate_streak(&review_dates, today_utc);
  let daily_goal = build_daily_goal(settings, analytics_repo::get_today_review_count(connection)?, today_utc.to_string());
  let reminder = build_reminder(settings, &overview, today_utc.to_string());
  let insights = build_insights(
    settings.ui_language,
    &overview,
    weak_cards.iter().filter(|card| card.needs_attention).count(),
    &streak,
    &daily_goal,
    &progress,
  );

  Ok(AnalyticsResponse {
    period_days,
    overview,
    outcomes,
    scheduler_health,
    calibration,
    content_quality,
    progress,
    weak_cards,
    learning_balance,
    streak,
    daily_goal,
    insights,
    reminder,
  })
}

#[cfg(test)]
mod tests {
  use chrono::{Days, NaiveDate};

  use crate::models::types::AppSettings;

  use super::{build_daily_goal, calculate_streak, reminder_time_passed_local};

  #[test]
  fn calculates_current_and_longest_streak() {
    let today = NaiveDate::from_ymd_opt(2026, 4, 9).unwrap();
    let dates = vec![
      today.checked_sub_days(Days::new(5)).unwrap(),
      today.checked_sub_days(Days::new(4)).unwrap(),
      today.checked_sub_days(Days::new(2)).unwrap(),
      today.checked_sub_days(Days::new(1)).unwrap(),
      today,
    ];

    let streak = calculate_streak(&dates, today);
    assert_eq!(streak.current_streak, 3);
    assert_eq!(streak.longest_streak, 3);
    assert!(streak.studied_today);
  }

  #[test]
  fn validates_reminder_time_strings() {
    assert!(reminder_time_passed_local("00:00"));
    assert!(!reminder_time_passed_local("25:00"));
    assert!(!reminder_time_passed_local("bad"));
  }

  #[test]
  fn daily_goal_progress_uses_settings_goal() {
    let goal = build_daily_goal(&AppSettings::default(), 15, "2026-04-09".to_string());
    assert_eq!(goal.daily_review_goal, 20);
    assert_eq!(goal.completed_today, 15);
    assert_eq!(goal.remaining_today, 5);
    assert_eq!(goal.percent_complete, 75);
  }
}
