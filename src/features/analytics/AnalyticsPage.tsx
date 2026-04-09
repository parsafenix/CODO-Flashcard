import { useEffect, useState } from "react";
import { FieldText } from "../../components/ui/FieldText";
import { StatCard } from "../../components/ui/StatCard";
import { useToast } from "../../components/ui/ToastProvider";
import { api } from "../../lib/api";
import { formatUtcDateLabel } from "../../lib/format";
import type { AnalyticsResponse } from "../../lib/types";

export function AnalyticsPage() {
  const { notify } = useToast();
  const [periodDays, setPeriodDays] = useState<7 | 30>(7);
  const [analytics, setAnalytics] = useState<AnalyticsResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  async function loadAnalytics(nextPeriodDays: 7 | 30 = periodDays) {
    setLoading(true);
    try {
      const response = await api.getAnalytics({ period_days: nextPeriodDays });
      setAnalytics(response);
      setError(null);
    } catch (err) {
      const message = typeof err === "object" && err && "message" in err ? String(err.message) : "Unable to load analytics.";
      setError(message);
      notify(message, "error");
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void loadAnalytics(periodDays);
  }, [periodDays]);

  return (
    <div className="analytics-page">
      <section className="page-header">
        <div>
          <p className="eyebrow">Analytics</p>
          <h1>Learning health</h1>
          <p>Track study volume, retention, weak cards, daily momentum, and the balance between new learning and review.</p>
        </div>
        <div className="page-header__actions">
          <div className="segmented-control" role="group" aria-label="Analytics period">
            <button
              className={`segmented-control__button ${periodDays === 7 ? "segmented-control__button--active" : ""}`}
              onClick={() => setPeriodDays(7)}
            >
              Last 7 days
            </button>
            <button
              className={`segmented-control__button ${periodDays === 30 ? "segmented-control__button--active" : ""}`}
              onClick={() => setPeriodDays(30)}
            >
              Last 30 days
            </button>
          </div>
        </div>
      </section>

      {error ? <div className="inline-error">{error}</div> : null}

      <section className="stat-grid analytics-overview-grid">
        <StatCard label="Total cards" value={analytics?.overview.total_cards ?? (loading ? "..." : 0)} />
        <StatCard label="New cards" value={analytics?.overview.new_cards ?? (loading ? "..." : 0)} />
        <StatCard label="Due cards" value={analytics?.overview.due_cards ?? (loading ? "..." : 0)} />
        <StatCard label="Mastered" value={analytics?.overview.mastered_cards ?? (loading ? "..." : 0)} />
        <StatCard label="Reviews completed" value={analytics?.overview.total_reviews_completed ?? (loading ? "..." : 0)} />
        <StatCard label="Accuracy" value={`${analytics?.overview.review_accuracy_percent ?? 0}%`} />
        <StatCard label="Retention" value={`${analytics?.overview.retention_score_percent ?? 0}%`} />
        <StatCard
          label="Daily goal"
          value={analytics ? `${analytics.daily_goal.completed_today} / ${analytics.daily_goal.daily_review_goal}` : loading ? "..." : "0 / 0"}
          hint={analytics ? `${analytics.daily_goal.percent_complete}% completed today` : undefined}
        />
      </section>

      <section className="analytics-grid">
        <div className="surface-panel">
          <h2>Progress over time</h2>
          <div className="table-shell">
            <table className="data-table">
              <thead>
                <tr>
                  <th>Date</th>
                  <th>Reviews</th>
                  <th>Accuracy</th>
                  <th>New learned</th>
                </tr>
              </thead>
              <tbody>
                {(analytics?.progress ?? []).map((point) => (
                  <tr key={point.utc_date}>
                    <td>{formatUtcDateLabel(point.utc_date)}</td>
                    <td>{point.reviews_completed}</td>
                    <td>{point.reviews_completed > 0 ? `${point.accuracy_percent}%` : "-"}</td>
                    <td>{point.new_cards_learned}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>

        <div className="surface-panel">
          <h2>Daily streak</h2>
          <div className="stat-grid stat-grid--compact analytics-mini-stats">
            <StatCard label="Current" value={analytics?.streak.current_streak ?? (loading ? "..." : 0)} />
            <StatCard label="Longest" value={analytics?.streak.longest_streak ?? (loading ? "..." : 0)} />
          </div>
          <div className="surface-muted">
            <div className="surface-muted__label">Today</div>
            <p>{analytics?.streak.studied_today ? "Today's streak is active." : "No graded review has been logged yet today."}</p>
          </div>
        </div>

        <div className="surface-panel">
          <h2>Learning balance</h2>
          <div className="analytics-balance">
            <div className="analytics-balance__bar">
              <div
                className="analytics-balance__bar-fill"
                style={{ width: `${analytics?.learning_balance.new_card_percent ?? 0}%` }}
              />
            </div>
            <div className="detail-inline-stats">
              <span>New-card reviews: {analytics?.learning_balance.new_card_reviews ?? 0}</span>
              <span>Review-card reviews: {analytics?.learning_balance.review_card_reviews ?? 0}</span>
            </div>
            <p>
              {analytics?.learning_balance.new_card_percent ?? 0}% new learning / {analytics?.learning_balance.review_card_percent ?? 0}% review
            </p>
          </div>
        </div>

        <div className="surface-panel">
          <h2>Smart insights</h2>
          {analytics?.insights.length ? (
            <ul className="simple-list">
              {analytics.insights.map((insight) => (
                <li key={insight}>{insight}</li>
              ))}
            </ul>
          ) : (
            <p>No insights yet. Study a few sessions to start seeing patterns.</p>
          )}
        </div>
      </section>

      <section className="surface-panel">
        <h2>Weak cards</h2>
        <p>The most difficult cards are ranked by wrong answers, relearning frequency, low mastery, and weak recent success.</p>
        <div className="table-shell">
          <table className="data-table">
            <thead>
              <tr>
                <th>Deck</th>
                <th>Card</th>
                <th>Difficulty</th>
                <th>Wrong</th>
                <th>Mastery</th>
                <th>Recent success</th>
              </tr>
            </thead>
            <tbody>
              {(analytics?.weak_cards ?? []).map((card) => (
                <tr key={card.card_id}>
                  <td>{card.deck_name}</td>
                  <td>
                    <div className="analytics-card-preview">
                      <FieldText value={card.language_1} />
                      <FieldText value={card.language_2} />
                      <FieldText value={card.language_3} />
                      {card.needs_attention ? <span className="pill pill--danger">Needs attention</span> : null}
                    </div>
                  </td>
                  <td>{card.difficulty_score}</td>
                  <td>{card.wrong_count}</td>
                  <td>{card.mastery_score}%</td>
                  <td>{card.recent_success_rate_percent}%</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>
    </div>
  );
}
