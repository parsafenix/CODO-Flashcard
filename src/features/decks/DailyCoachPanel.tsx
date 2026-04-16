import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { useAppContext } from "../../app/AppContext";
import { Button } from "../../components/ui/Button";
import { PanelCard } from "../../components/ui/PanelCard";
import { useToast } from "../../components/ui/ToastProvider";
import { api } from "../../lib/api";
import { formatRelativeDate } from "../../lib/format";
import { useI18n } from "../../lib/i18n";
import { localizeAppMessage } from "../../lib/messages";
import type { DailyCoachResponse } from "../../lib/types";

interface DailyCoachPanelProps {
  onHide: () => void;
}

export function DailyCoachPanel({ onHide }: DailyCoachPanelProps) {
  const navigate = useNavigate();
  const { uiPreferences, persistUiPreferences } = useAppContext();
  const { notify } = useToast();
  const { t } = useI18n();
  const [coach, setCoach] = useState<DailyCoachResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    async function loadCoach() {
      setLoading(true);
      try {
        const response = await api.getDailyCoach();
        if (cancelled) {
          return;
        }
        setCoach(response);
        setError(null);
        if (
          response.should_prompt &&
          uiPreferences.daily_coach_last_shown_utc_date !== response.today_utc_date
        ) {
          void persistUiPreferences({
            ...uiPreferences,
            daily_coach_last_shown_utc_date: response.today_utc_date,
          }).catch(() => undefined);
        }
      } catch (err) {
        if (cancelled) {
          return;
        }
        const message = localizeAppMessage(
          typeof err === "object" && err && "message" in err ? String(err.message) : t("coach.loadError"),
          t
        );
        setError(message);
        notify(message, "error");
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    }

    void loadCoach();
    return () => {
      cancelled = true;
    };
  }, [
    notify,
    persistUiPreferences,
    t,
    uiPreferences.daily_coach_last_dismissed_utc_date,
    uiPreferences.daily_coach_last_shown_utc_date,
  ]);

  async function dismissForToday() {
    if (!coach) {
      return;
    }

    try {
      const saved = await persistUiPreferences({
        ...uiPreferences,
        daily_coach_last_dismissed_utc_date: coach.today_utc_date,
      });
      setCoach((current) =>
        current
          ? {
              ...current,
              dismissed_today: saved.daily_coach_last_dismissed_utc_date === current.today_utc_date,
              should_prompt: false,
            }
          : current
      );
    } catch (err) {
      const message = localizeAppMessage(
        typeof err === "object" && err && "message" in err ? String(err.message) : t("coach.dismissError"),
        t
      );
      notify(message, "error");
    }
  }

  const topRecommendation = coach?.recommendations[0] ?? null;

  return (
    <PanelCard
      title={t("coach.title")}
      description={t("coach.description")}
      onHide={onHide}
      actions={
        coach?.should_prompt ? (
          <Button type="button" variant="secondary" onClick={() => void dismissForToday()}>
            {t("coach.snoozeToday")}
          </Button>
        ) : undefined
      }
    >
      {error ? <div className="inline-error">{error}</div> : null}

      {loading ? <p>{t("common.loading")}</p> : null}

      {!loading && !topRecommendation ? <p>{t("coach.empty")}</p> : null}

      {topRecommendation ? (
        <div className={`coach-card ${coach?.should_prompt ? "coach-card--prompt" : ""}`}>
          <div className="coach-card__head">
            <div>
              <div className="surface-muted__label">{t("coach.topRecommendation")}</div>
              <h3>{topRecommendation.deck_name}</h3>
            </div>
            <span className="pill">{topRecommendation.priority_label}</span>
          </div>
          <p>{topRecommendation.reason_text}</p>
          {topRecommendation.supporting_reasons.length > 0 ? (
            <ul className="simple-list">
              {topRecommendation.supporting_reasons.map((reason) => (
                <li key={reason}>{reason}</li>
              ))}
            </ul>
          ) : null}
          <div className="detail-inline-stats detail-inline-stats--wrap">
            <span>{t("coach.stats.due", { count: topRecommendation.due_cards })}</span>
            <span>{t("coach.stats.overdue", { count: topRecommendation.overdue_cards })}</span>
            <span>{t("coach.stats.weak", { count: topRecommendation.weak_direction_count })}</span>
            <span>{t("coach.stats.lastStudied", { value: formatRelativeDate(topRecommendation.last_studied_at) })}</span>
          </div>
          <div className="dialog-actions dialog-actions--start">
            <Button type="button" onClick={() => navigate(`/study/${topRecommendation.deck_id}`)}>
              {t("coach.startReview")}
            </Button>
            <Button type="button" variant="ghost" onClick={() => navigate(`/decks/${topRecommendation.deck_id}`)}>
              {t("coach.openDeck")}
            </Button>
            {coach?.should_prompt ? (
              <Button type="button" variant="secondary" onClick={() => void dismissForToday()}>
                {t("coach.snoozeToday")}
              </Button>
            ) : null}
          </div>
        </div>
      ) : null}

      {coach && coach.recommendations.length > 1 ? (
        <div className="coach-list">
          <div className="surface-muted__label">{t("coach.moreRecommendations")}</div>
          {coach.recommendations.slice(1).map((recommendation) => (
            <div key={recommendation.deck_id} className="coach-list__item">
              <div>
                <strong>{recommendation.deck_name}</strong>
                <p>{recommendation.reason_text}</p>
              </div>
              <div className="dialog-actions dialog-actions--start">
                <Button type="button" variant="ghost" onClick={() => navigate(`/study/${recommendation.deck_id}`)}>
                  {t("coach.startReview")}
                </Button>
                <Button type="button" variant="secondary" onClick={() => navigate(`/decks/${recommendation.deck_id}`)}>
                  {t("coach.openDeck")}
                </Button>
              </div>
            </div>
          ))}
        </div>
      ) : null}
    </PanelCard>
  );
}
