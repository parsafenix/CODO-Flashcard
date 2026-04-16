import { useEffect, useState } from "react";
import { FieldText } from "../../components/ui/FieldText";
import { HiddenPanelsBar } from "../../components/ui/HiddenPanelsBar";
import { PanelCard } from "../../components/ui/PanelCard";
import { StatCard } from "../../components/ui/StatCard";
import { useToast } from "../../components/ui/ToastProvider";
import { api } from "../../lib/api";
import { formatUtcDateLabel } from "../../lib/format";
import { useI18n } from "../../lib/i18n";
import { localizeAppMessage, localizeCalibrationStatus } from "../../lib/messages";
import type { AnalyticsResponse } from "../../lib/types";
import { usePanelVisibility } from "../../lib/usePanelVisibility";

export function AnalyticsPage() {
  const { notify } = useToast();
  const { t } = useI18n();
  const [periodDays, setPeriodDays] = useState<7 | 30>(7);
  const [analytics, setAnalytics] = useState<AnalyticsResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const panelLabels = [
    { id: "outcomes", label: t("analytics.outcomesTitle") },
    { id: "progress", label: t("analytics.progressTitle") },
    { id: "streak", label: t("analytics.streakTitle") },
    { id: "balance", label: t("analytics.learningBalanceTitle") },
    { id: "insights", label: t("analytics.insightsTitle") },
    { id: "scheduler", label: t("analytics.schedulerTitle") },
    { id: "calibration", label: t("analytics.calibrationTitle") },
    { id: "weak-cards", label: t("analytics.weakCardsTitle") },
  ];
  const { visiblePanels, hiddenPanels, hidePanel, showPanel } = usePanelVisibility("analytics", panelLabels);

  async function loadAnalytics(nextPeriodDays: 7 | 30 = periodDays) {
    setLoading(true);
    try {
      const response = await api.getAnalytics({ period_days: nextPeriodDays });
      setAnalytics(response);
      setError(null);
    } catch (err) {
      const message = localizeAppMessage(
        typeof err === "object" && err && "message" in err ? String(err.message) : t("analytics.loadError"),
        t
      );
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

      <HiddenPanelsBar panels={hiddenPanels} onShow={(panelId) => void showPanel(panelId)} />

      <section className="panel-masonry">
        {visiblePanels.some((panel) => panel.id === "outcomes") ? (
          <div className="panel-masonry__item">
            <PanelCard title={t("analytics.outcomesTitle")} onHide={() => void hidePanel("outcomes")}>
              <div className="stat-grid stat-grid--compact analytics-mini-stats">
                <StatCard label={t("analytics.outcomes.firstPass")} value={`${analytics?.outcomes.first_pass_success_rate_percent ?? 0}%`} />
                <StatCard label={t("analytics.outcomes.retention7")} value={`${analytics?.outcomes.retention_7d_percent ?? 0}%`} />
                <StatCard label={t("analytics.outcomes.retention30")} value={`${analytics?.outcomes.retention_30d_percent ?? 0}%`} />
                <StatCard label={t("analytics.outcomes.lapseRate")} value={`${analytics?.outcomes.lapse_rate_percent ?? 0}%`} />
              </div>
              <div className="detail-inline-stats detail-inline-stats--wrap">
                <span>{t("analytics.outcomes.recognition", { percent: analytics?.outcomes.recognition_accuracy_percent ?? 0 })}</span>
                <span>{t("analytics.outcomes.production", { percent: analytics?.outcomes.production_accuracy_percent ?? 0 })}</span>
                <span>{t("analytics.outcomes.graduationTime", { days: analytics?.outcomes.average_time_to_graduation_days ?? 0 })}</span>
                <span>{t("analytics.outcomes.masteryTime", { days: analytics?.outcomes.average_time_to_mastery_days ?? 0 })}</span>
                <span>{t("analytics.outcomes.reviewBurden", { value: analytics?.outcomes.review_burden_per_retained_item ?? 0 })}</span>
              </div>
            </PanelCard>
          </div>
        ) : null}

        {visiblePanels.some((panel) => panel.id === "progress") ? (
          <div className="panel-masonry__item">
            <PanelCard title={t("analytics.progressTitle")} onHide={() => void hidePanel("progress")}>
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
            </PanelCard>
          </div>
        ) : null}

        {visiblePanels.some((panel) => panel.id === "streak") ? (
          <div className="panel-masonry__item">
            <PanelCard title={t("analytics.streakTitle")} onHide={() => void hidePanel("streak")}>
              <div className="stat-grid stat-grid--compact analytics-mini-stats">
                <StatCard label={t("analytics.streak.current")} value={analytics?.streak.current_streak ?? (loading ? "..." : 0)} />
                <StatCard label={t("analytics.streak.longest")} value={analytics?.streak.longest_streak ?? (loading ? "..." : 0)} />
              </div>
              <div className="surface-muted">
                <div className="surface-muted__label">{t("common.today")}</div>
                <p>{analytics?.streak.studied_today ? t("analytics.streakActive") : t("analytics.streakInactive")}</p>
              </div>
            </PanelCard>
          </div>
        ) : null}

        {visiblePanels.some((panel) => panel.id === "balance") ? (
          <div className="panel-masonry__item">
            <PanelCard title={t("analytics.learningBalanceTitle")} onHide={() => void hidePanel("balance")}>
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
            </PanelCard>
          </div>
        ) : null}

        {visiblePanels.some((panel) => panel.id === "insights") ? (
          <div className="panel-masonry__item">
            <PanelCard title={t("analytics.insightsTitle")} onHide={() => void hidePanel("insights")}>
              {analytics?.insights.length ? (
                <ul className="simple-list">
                  {analytics.insights.map((insight) => (
                    <li key={insight}>{insight}</li>
                  ))}
                </ul>
              ) : (
                <p>{t("analytics.insightsEmpty")}</p>
              )}
            </PanelCard>
          </div>
        ) : null}

        {visiblePanels.some((panel) => panel.id === "scheduler") ? (
          <div className="panel-masonry__item">
            <PanelCard title={t("analytics.schedulerTitle")} onHide={() => void hidePanel("scheduler")}>
              <div className="stat-grid stat-grid--compact analytics-mini-stats">
                <StatCard label={t("analytics.scheduler.predicted")} value={`${analytics?.scheduler_health.predicted_recall_percent ?? 0}%`} />
                <StatCard label={t("analytics.scheduler.actual")} value={`${analytics?.scheduler_health.actual_recall_percent ?? 0}%`} />
                <StatCard label={t("analytics.scheduler.stability")} value={analytics?.scheduler_health.average_stability_days ?? (loading ? "..." : 0)} />
                <StatCard label={t("analytics.scheduler.difficulty")} value={analytics?.scheduler_health.average_difficulty ?? (loading ? "..." : 0)} />
              </div>
              <div className="detail-inline-stats detail-inline-stats--wrap">
                <span>{t("analytics.scheduler.growth", { percent: analytics?.scheduler_health.successful_stability_growth_percent ?? 0 })}</span>
                <span>{t("analytics.scheduler.lapse", { percent: analytics?.scheduler_health.review_lapse_rate_percent ?? 0 })}</span>
                <span>{t("analytics.scheduler.overdue", { percent: analytics?.scheduler_health.overdue_success_percent ?? 0 })}</span>
                <span>{t("analytics.scheduler.onTime", { percent: analytics?.scheduler_health.on_time_success_percent ?? 0 })}</span>
                <span>{t("analytics.scheduler.forecast7", { count: analytics?.scheduler_health.due_forecast_7d ?? 0 })}</span>
                <span>{t("analytics.scheduler.forecast30", { count: analytics?.scheduler_health.due_forecast_30d ?? 0 })}</span>
              </div>
              <div className="surface-muted">
                <div className="surface-muted__label">{t("analytics.scheduler.sensitivity")}</div>
                <div className="detail-inline-stats detail-inline-stats--wrap">
                  {(analytics?.scheduler_health.retention_sensitivity ?? []).map((point) => (
                    <span key={point.desired_retention}>
                      {t("analytics.scheduler.sensitivityPoint", {
                        retention: Math.round(point.desired_retention * 100),
                        count: point.estimated_due_next_30_days,
                      })}
                    </span>
                  ))}
                </div>
              </div>
            </PanelCard>
          </div>
        ) : null}

        {visiblePanels.some((panel) => panel.id === "calibration") ? (
          <div className="panel-masonry__item">
            <PanelCard title={t("analytics.calibrationTitle")} onHide={() => void hidePanel("calibration")}>
              <div className="stat-grid stat-grid--compact analytics-mini-stats">
                <StatCard
                  label={t("analytics.calibration.profile")}
                  value={analytics?.calibration.active_profile.label ?? (loading ? "..." : "-")}
                />
                <StatCard
                  label={t("analytics.calibration.logLoss")}
                  value={
                    analytics?.calibration.latest_run?.candidate_metrics?.validation.log_loss ??
                    analytics?.calibration.latest_run?.baseline_metrics.validation.log_loss ??
                    (loading ? "..." : "-")
                  }
                />
                <StatCard
                  label={t("analytics.calibration.rmse")}
                  value={
                    analytics?.calibration.latest_run?.candidate_metrics?.validation.rmse_bins ??
                    analytics?.calibration.latest_run?.baseline_metrics.validation.rmse_bins ??
                    (loading ? "..." : "-")
                  }
                />
                <StatCard
                  label={t("analytics.calibration.auc")}
                  value={
                    analytics?.calibration.latest_run?.candidate_metrics?.validation.auc ??
                    analytics?.calibration.latest_run?.baseline_metrics.validation.auc ??
                    (loading ? "..." : "-")
                  }
                />
              </div>

              <div className="detail-inline-stats detail-inline-stats--wrap">
                <span>{t("analytics.calibration.usable", { count: analytics?.calibration.sufficiency.usable_events ?? 0 })}</span>
                <span>{t("analytics.calibration.mature", { count: analytics?.calibration.sufficiency.mature_review_events ?? 0 })}</span>
                <span>{t("analytics.calibration.failures", { count: analytics?.calibration.sufficiency.failure_events ?? 0 })}</span>
                <span>
                  {t("analytics.calibration.status", {
                    status: localizeCalibrationStatus(analytics?.calibration.latest_run?.status, t),
                  })}
                </span>
              </div>

              <div className="surface-muted">
                <div className="surface-muted__label">{t("settings.calibrationReadiness")}</div>
                <p>
                  {analytics?.calibration.sufficiency.enough_data
                    ? t("settings.calibrationReady")
                    : t("settings.calibrationNotReady", {
                        reviews: analytics?.calibration.sufficiency.minimum_usable_events ?? 0,
                        units: analytics?.calibration.sufficiency.minimum_distinct_review_units ?? 0,
                      })}
                </p>
                {analytics?.calibration.latest_run?.reason ? (
                  <p className="form-helper-text">{localizeAppMessage(analytics.calibration.latest_run.reason, t)}</p>
                ) : null}
              </div>

              {analytics?.calibration.latest_run?.workload ? (
                <div className="surface-muted">
                  <div className="surface-muted__label">{t("analytics.calibration.workload")}</div>
                  <div className="detail-inline-stats detail-inline-stats--wrap">
                    <span>
                      {t("analytics.calibration.workload7", {
                        from: analytics.calibration.latest_run.workload.active.due_next_7d,
                        to: analytics.calibration.latest_run.workload.candidate.due_next_7d,
                      })}
                    </span>
                    <span>
                      {t("analytics.calibration.workload30", {
                        from: analytics.calibration.latest_run.workload.active.due_next_30d,
                        to: analytics.calibration.latest_run.workload.candidate.due_next_30d,
                      })}
                    </span>
                  </div>
                </div>
              ) : null}

              {(analytics?.calibration.latest_run?.diagnostics?.curve ?? []).length ? (
                <div className="table-shell">
                  <table className="data-table">
                    <thead>
                      <tr>
                        <th>{t("analytics.calibration.bin")}</th>
                        <th>{t("analytics.calibration.predicted")}</th>
                        <th>{t("analytics.calibration.actual")}</th>
                        <th>{t("analytics.calibration.events")}</th>
                      </tr>
                    </thead>
                    <tbody>
                      {analytics?.calibration.latest_run?.diagnostics?.curve.map((point) => (
                        <tr key={point.bin_index}>
                          <td>{point.label}</td>
                          <td>{Math.round(point.average_predicted * 100)}%</td>
                          <td>{Math.round(point.actual_rate * 100)}%</td>
                          <td>{point.event_count}</td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              ) : (
                <p>{t("analytics.calibration.empty")}</p>
              )}
            </PanelCard>
          </div>
        ) : null}

        {visiblePanels.some((panel) => panel.id === "weak-cards") ? (
          <div className="panel-masonry__item">
            <PanelCard
              title={t("analytics.weakCardsTitle")}
              description={t("analytics.weakCardsDescription")}
              onHide={() => void hidePanel("weak-cards")}
            >
              <div className="table-shell">
                <table className="data-table">
                  <thead>
                    <tr>
                      <th>{t("analytics.weakCards.deck")}</th>
                      <th>{t("analytics.weakCards.card")}</th>
                      <th>{t("analytics.weakCards.difficulty")}</th>
                      <th>{t("analytics.weakCards.wrong")}</th>
                      <th>{t("analytics.weakCards.mastery")}</th>
                      <th>{t("analytics.weakCards.stability")}</th>
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
                        <td>{card.stability_days.toFixed(1)}</td>
                        <td>{card.recent_success_rate_percent}%</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
              <div className="surface-muted">
                <div className="surface-muted__label">{t("analytics.contentQualityTitle")}</div>
                <div className="detail-inline-stats detail-inline-stats--wrap">
                  <span>{t("analytics.contentQuality.hardest", { count: analytics?.content_quality.hardest_direction_count ?? 0 })}</span>
                  <span>{t("analytics.contentQuality.again", { count: analytics?.content_quality.repeated_again_count ?? 0 })}</span>
                  <span>{t("analytics.contentQuality.leech", { count: analytics?.content_quality.leech_count ?? 0 })}</span>
                  <span>{t("analytics.contentQuality.context", { count: analytics?.content_quality.contextual_support_count ?? 0 })}</span>
                </div>
              </div>
            </PanelCard>
          </div>
        ) : null}
      </section>
    </div>
  );
}
