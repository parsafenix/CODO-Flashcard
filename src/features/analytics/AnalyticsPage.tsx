import { useEffect, useState } from "react";
import { FieldText } from "../../components/ui/FieldText";
import { StatCard } from "../../components/ui/StatCard";
import { useToast } from "../../components/ui/ToastProvider";
import { api } from "../../lib/api";
import { formatUtcDateLabel } from "../../lib/format";
import { useI18n } from "../../lib/i18n";
import type { AnalyticsResponse } from "../../lib/types";

export function AnalyticsPage() {
  const { notify } = useToast();
  const { t } = useI18n();
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
      const message = typeof err === "object" && err && "message" in err ? String(err.message) : t("analytics.loadError");
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
          <p className="eyebrow">{t("nav.analytics")}</p>
          <h1>{t("analytics.title")}</h1>
          <p>{t("analytics.description")}</p>
        </div>
        <div className="page-header__actions">
          <div className="segmented-control" role="group" aria-label={t("nav.analytics")}>
            <button
              className={`segmented-control__button ${periodDays === 7 ? "segmented-control__button--active" : ""}`}
              onClick={() => setPeriodDays(7)}
            >
              {t("analytics.last7")}
            </button>
            <button
              className={`segmented-control__button ${periodDays === 30 ? "segmented-control__button--active" : ""}`}
              onClick={() => setPeriodDays(30)}
            >
              {t("analytics.last30")}
            </button>
          </div>
        </div>
      </section>

      {error ? <div className="inline-error">{error}</div> : null}

      <section className="stat-grid analytics-overview-grid">
        <StatCard label={t("analytics.overview.totalCards")} value={analytics?.overview.total_cards ?? (loading ? "..." : 0)} />
        <StatCard label={t("analytics.overview.newCards")} value={analytics?.overview.new_cards ?? (loading ? "..." : 0)} />
        <StatCard label={t("analytics.overview.dueCards")} value={analytics?.overview.due_cards ?? (loading ? "..." : 0)} />
        <StatCard label={t("analytics.overview.masteredCards")} value={analytics?.overview.mastered_cards ?? (loading ? "..." : 0)} />
        <StatCard label={t("analytics.overview.reviews")} value={analytics?.overview.total_reviews_completed ?? (loading ? "..." : 0)} />
        <StatCard
          label={t("analytics.overview.accuracy")}
          value={`${analytics?.overview.review_accuracy_percent ?? 0}%`}
          hint={t("analytics.overview.accuracyHint")}
        />
        <StatCard
          label={t("analytics.overview.retention")}
          value={`${analytics?.overview.retention_score_percent ?? 0}%`}
          hint={t("analytics.overview.retentionHint")}
        />
        <StatCard
          label={t("analytics.overview.dailyGoal")}
          value={analytics ? `${analytics.daily_goal.completed_today} / ${analytics.daily_goal.daily_review_goal}` : loading ? "..." : "0 / 0"}
          hint={analytics ? t("analytics.overview.dailyGoalHint", { percent: analytics.daily_goal.percent_complete }) : undefined}
        />
      </section>

      <section className="analytics-grid">
        <div className="surface-panel">
          <h2>{t("analytics.progressTitle")}</h2>
          <div className="table-shell">
            <table className="data-table">
              <thead>
                <tr>
                  <th>{t("analytics.progress.date")}</th>
                  <th>{t("analytics.progress.reviews")}</th>
                  <th>{t("analytics.progress.accuracy")}</th>
                  <th>{t("analytics.progress.newLearned")}</th>
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
          <h2>{t("analytics.streakTitle")}</h2>
          <div className="stat-grid stat-grid--compact analytics-mini-stats">
            <StatCard label={t("analytics.streak.current")} value={analytics?.streak.current_streak ?? (loading ? "..." : 0)} />
            <StatCard label={t("analytics.streak.longest")} value={analytics?.streak.longest_streak ?? (loading ? "..." : 0)} />
          </div>
          <div className="surface-muted">
            <div className="surface-muted__label">{t("common.today")}</div>
            <p>{analytics?.streak.studied_today ? t("analytics.streakActive") : t("analytics.streakInactive")}</p>
          </div>
        </div>

        <div className="surface-panel">
          <h2>{t("analytics.learningBalanceTitle")}</h2>
          <div className="analytics-balance">
            <div className="analytics-balance__bar">
              <div
                className="analytics-balance__bar-fill"
                style={{ width: `${analytics?.learning_balance.new_card_percent ?? 0}%` }}
              />
            </div>
            <div className="detail-inline-stats">
              <span>{t("analytics.learningBalanceNew", { count: analytics?.learning_balance.new_card_reviews ?? 0 })}</span>
              <span>{t("analytics.learningBalanceReview", { count: analytics?.learning_balance.review_card_reviews ?? 0 })}</span>
            </div>
            <p>
              {t("analytics.learningBalanceSplit", {
                newPercent: analytics?.learning_balance.new_card_percent ?? 0,
                reviewPercent: analytics?.learning_balance.review_card_percent ?? 0,
              })}
            </p>
          </div>
        </div>

        <div className="surface-panel">
          <h2>{t("analytics.insightsTitle")}</h2>
          {analytics?.insights.length ? (
            <ul className="simple-list">
              {analytics.insights.map((insight) => (
                <li key={insight}>{insight}</li>
              ))}
            </ul>
          ) : (
            <p>{t("analytics.insightsEmpty")}</p>
          )}
        </div>
      </section>

      <section className="surface-panel">
        <h2>{t("analytics.weakCardsTitle")}</h2>
        <p>{t("analytics.weakCardsDescription")}</p>
        <div className="table-shell">
          <table className="data-table">
            <thead>
              <tr>
                <th>{t("analytics.weakCards.deck")}</th>
                <th>{t("analytics.weakCards.card")}</th>
                <th>{t("analytics.weakCards.difficulty")}</th>
                <th>{t("analytics.weakCards.wrong")}</th>
                <th>{t("analytics.weakCards.mastery")}</th>
                <th>{t("analytics.weakCards.recentSuccess")}</th>
              </tr>
            </thead>
            <tbody>
              {(analytics?.weak_cards ?? []).map((card) => (
                <tr key={card.card_id}>
                  <td>{card.deck_name}</td>
                  <td>
                    <div className="analytics-card-preview">
                      {(card.preview_fields.length > 0
                        ? card.preview_fields
                        : [
                            { label: "1", value: card.language_1, is_context: false },
                            { label: "2", value: card.language_2, is_context: false },
                            { label: "3", value: card.language_3, is_context: false },
                          ]
                      )
                        .filter((field) => field.value)
                        .map((field) => (
                          <div
                            key={`${card.card_id}-${field.label}`}
                            className={`analytics-card-preview__row ${field.is_context ? "analytics-card-preview__row--context" : ""}`}
                          >
                            <span className="flashcard__answer-label">{field.label}</span>
                            <FieldText value={field.value} />
                          </div>
                        ))}
                      {card.needs_attention ? <span className="pill pill--danger">{t("analytics.weakCards.needsAttention")}</span> : null}
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
