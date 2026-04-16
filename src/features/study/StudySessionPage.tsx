import { useEffect, useMemo, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { useAppContext } from "../../app/AppContext";
import { Button } from "../../components/ui/Button";
import { EmptyState } from "../../components/ui/EmptyState";
import { FieldText } from "../../components/ui/FieldText";
import { StatCard } from "../../components/ui/StatCard";
import { useToast } from "../../components/ui/ToastProvider";
import { api } from "../../lib/api";
import { activeFieldLabel, defaultPromptFieldId, defaultRevealFieldIds, getActiveFields, getStudyFieldValue, isContextField } from "../../lib/deckFields";
import { formatAccuracy } from "../../lib/format";
import { useI18n } from "../../lib/i18n";
import { useKeyboardShortcut } from "../../lib/keyboard";
import { localizeAppMessage } from "../../lib/messages";
import type { DeckSummary, ReviewRating, SessionSummary, StudyCard, StudyMode, StudySessionOptions, StudySessionPayload } from "../../lib/types";
import { buildStudyDirectionPreview } from "./setupSummary";

function getVisibleFieldIds(options: StudySessionOptions) {
  if (!options.reverse_mode || options.reveal_field_ids.length === 0) {
    return {
      frontFieldId: options.prompt_field_id,
      answerFieldIds: options.reveal_field_ids,
    };
  }

  const [firstReveal, ...restReveal] = options.reveal_field_ids;
  return {
    frontFieldId: firstReveal,
    answerFieldIds: [options.prompt_field_id, ...restReveal],
  };
}

export function StudySessionPage() {
  const maxSessionRequeues = 2;
  const params = useParams();
  const navigate = useNavigate();
  const { settings } = useAppContext();
  const { notify } = useToast();
  const { t } = useI18n();
  const deckId = Number(params.deckId);
  const [deck, setDeck] = useState<DeckSummary | null>(null);
  const [session, setSession] = useState<StudySessionPayload | null>(null);
  const [queue, setQueue] = useState<StudyCard[]>([]);
  const [revealed, setRevealed] = useState(false);
  const [starting, setStarting] = useState(false);
  const [summary, setSummary] = useState<SessionSummary | null>(null);
  const [stats, setStats] = useState({ studied: 0, correct: 0, wrong: 0, newlyMastered: 0 });
  const [requeueCounts, setRequeueCounts] = useState<Record<number, number>>({});
  const [loadError, setLoadError] = useState<string | null>(null);
  const [cardShownAt, setCardShownAt] = useState<number>(() => Date.now());
  const [options, setOptions] = useState<StudySessionOptions>({
    deck_id: deckId,
    prompt_field_id: 0,
    reveal_field_ids: [],
    mode: settings.default_study_mode,
    random_order: settings.random_order,
    reverse_mode: settings.reverse_mode,
    cards_limit: settings.cards_per_session,
  });

  useEffect(() => {
    void api
      .getDeck(deckId)
      .then((response) => {
        setDeck(response);
        const activeFields = getActiveFields(response.fields);
        const promptFieldId = response.study_prompt_field_id ?? defaultPromptFieldId(activeFields);
        const revealFieldIds = response.study_reveal_field_ids.length
          ? response.study_reveal_field_ids
          : defaultRevealFieldIds(activeFields, promptFieldId);
        setOptions((current) => ({
          ...current,
          deck_id: response.id,
          prompt_field_id: promptFieldId,
          reveal_field_ids: revealFieldIds,
        }));
        setLoadError(null);
      })
      .catch((err) => {
        const message = localizeAppMessage(
          typeof err === "object" && err && "message" in err ? String(err.message) : t("deck.loadError"),
          t
        );
        setLoadError(message);
        notify(message, "error");
      });
  }, [deckId]);

  const currentCard = queue[0] ?? null;
  const totalSteps = stats.studied + queue.length;
  const activeFields = useMemo(() => (deck ? getActiveFields(deck.fields) : []), [deck]);
  const { frontFieldId, answerFieldIds } = getVisibleFieldIds(options);
  const frontValue = currentCard ? getStudyFieldValue(currentCard, frontFieldId) : "";
  const directionPreview = useMemo(
    () => buildStudyDirectionPreview(activeFields, options.prompt_field_id, options.reveal_field_ids, options.reverse_mode),
    [activeFields, options.prompt_field_id, options.reveal_field_ids, options.reverse_mode]
  );

  useKeyboardShortcut([" ", "Enter"], () => setRevealed(true), Boolean(currentCard && !revealed));
  useKeyboardShortcut(["1", "a", "A", "ArrowLeft"], () => void handleGrade("again"), Boolean(currentCard && revealed));
  useKeyboardShortcut(["2", "h", "H"], () => void handleGrade("hard"), Boolean(currentCard && revealed));
  useKeyboardShortcut(["3", "g", "G", "ArrowRight"], () => void handleGrade("good"), Boolean(currentCard && revealed));
  useKeyboardShortcut(["4", "e", "E", "ArrowUp"], () => void handleGrade("easy"), Boolean(currentCard && revealed));
  useKeyboardShortcut(["Escape"], () => navigate(`/decks/${deckId}`), true);

  useEffect(() => {
    if (currentCard) {
      setCardShownAt(Date.now());
    }
  }, [currentCard?.review_unit_id]);

  async function startSession() {
    if (!options.prompt_field_id || options.reveal_field_ids.length === 0) {
      notify(t("study.errorChooseFields"), "error");
      return;
    }

    setStarting(true);
    setSummary(null);
    setStats({ studied: 0, correct: 0, wrong: 0, newlyMastered: 0 });
    setRequeueCounts({});
    try {
      const payload = await api.startStudySession(options);
      setSession(payload);
      setQueue(payload.cards);
      setRevealed(false);
      setCardShownAt(Date.now());
      setLoadError(null);
      if (payload.cards.length === 0) {
        notify(t("study.errorNoCards"), "info");
      }
    } catch (err) {
      const message = localizeAppMessage(
        typeof err === "object" && err && "message" in err ? String(err.message) : t("study.errorStart"),
        t
      );
      setLoadError(message);
      notify(message, "error");
    } finally {
      setStarting(false);
    }
  }

  async function handleGrade(rating: ReviewRating) {
    if (!session || !currentCard) {
      return;
    }

    const latencyMs = Math.max(0, Date.now() - cardShownAt);
    let response;
    try {
      response = await api.gradeCard({
        session_id: session.session_id,
        card_id: currentCard.id,
        review_unit_id: currentCard.review_unit_id,
        rating,
        latency_ms: latencyMs,
        hint_used: false,
      });
    } catch (err) {
      const message = localizeAppMessage(
        typeof err === "object" && err && "message" in err ? String(err.message) : t("study.errorGrade"),
        t
      );
      notify(message, "error");
      return;
    }

    const wasCorrect = rating !== "again";
    const nextStudied = stats.studied + 1;
    const nextCorrect = stats.correct + (wasCorrect ? 1 : 0);
    const nextWrong = stats.wrong + (wasCorrect ? 0 : 1);
    const nextNewlyMastered = stats.newlyMastered + (response.newly_mastered ? 1 : 0);

    const remaining = queue.slice(1);
    if (rating === "again" && (requeueCounts[currentCard.review_unit_id] ?? 0) < maxSessionRequeues) {
      const insertIndex = Math.min(3, remaining.length);
      remaining.splice(insertIndex, 0, currentCard);
      setRequeueCounts((current) => ({
        ...current,
        [currentCard.review_unit_id]: (current[currentCard.review_unit_id] ?? 0) + 1,
      }));
    }
    setQueue(remaining);
    setRevealed(false);
    setStats({
      studied: nextStudied,
      correct: nextCorrect,
      wrong: nextWrong,
      newlyMastered: nextNewlyMastered,
    });

    if (remaining.length === 0) {
      try {
        const nextSummary = await api.completeStudySession({
          session_id: session.session_id,
          deck_id: session.deck.id,
          studied_count: nextStudied,
          correct_count: nextCorrect,
          wrong_count: nextWrong,
          newly_mastered_count: nextNewlyMastered,
        });
        setSummary(nextSummary);
      } catch (err) {
        const message = localizeAppMessage(
          typeof err === "object" && err && "message" in err ? String(err.message) : t("study.errorComplete"),
          t
        );
        notify(message, "error");
      }
    }
  }

  function toggleRevealField(fieldId: number) {
    setOptions((current) => ({
      ...current,
      reveal_field_ids: current.reveal_field_ids.includes(fieldId)
        ? current.reveal_field_ids.filter((id) => id !== fieldId)
        : [...current.reveal_field_ids, fieldId],
    }));
  }

  return (
    <div className="study-page">
      <section className="page-header">
        <div>
          <p className="eyebrow">{t("study.title")}</p>
          <h1>{deck?.name ?? t("study.title")}</h1>
          <p>{t("study.subtitle")}</p>
        </div>
        <div className="page-header__actions">
          <Button variant="secondary" onClick={() => navigate(`/decks/${deckId}`)}>
            {t("common.back")}
          </Button>
        </div>
      </section>

      {loadError ? <div className="inline-error">{loadError}</div> : null}

      {!session || summary ? (
        <section className="study-layout">
          <div className="study-config surface-panel">
            <h2>{summary ? t("study.summary") : t("study.options")}</h2>

            {summary ? (
              <div className="form-stack">
                <div className="stat-grid">
                  <StatCard label={t("study.summaryStudied")} value={summary.studied_count} />
                  <StatCard label={t("study.summaryAccuracy")} value={`${summary.accuracy_percent}%`} />
                  <StatCard label={t("study.summaryWrong")} value={summary.wrong_count} />
                  <StatCard label={t("study.summaryNewlyMastered")} value={summary.newly_mastered_count} />
                </div>
                <div className="surface-muted">
                  <div className="surface-muted__label">{t("study.summaryWhatNext")}</div>
                  <p>{summary.suggestion}</p>
                  <div className="detail-inline-stats">
                    <span>{t("study.summaryDueRemain", { count: summary.remaining_due_cards })}</span>
                  </div>
                </div>
                <div className="dialog-actions dialog-actions--start">
                  <Button variant="secondary" onClick={() => navigate(`/decks/${deckId}`)}>
                    {t("common.back")}
                  </Button>
                  <Button onClick={() => void startSession()}>{t("study.start")}</Button>
                </div>
              </div>
            ) : (
              <div className="form-stack">
                <label className="field">
                  <span>{t("study.promptField")}</span>
                  <select
                    value={options.prompt_field_id}
                    onChange={(event) =>
                      setOptions((current) => ({
                        ...current,
                        prompt_field_id: Number(event.target.value),
                        reveal_field_ids: current.reveal_field_ids.filter((fieldId) => fieldId !== Number(event.target.value)),
                      }))
                    }
                  >
                    {activeFields.map((field) => (
                      <option key={field.id} value={field.id}>
                        {field.label}
                      </option>
                    ))}
                  </select>
                </label>

                <div className="surface-muted">
                  <div className="surface-muted__label">{t("study.revealFields")}</div>
                  <div className="checkbox-grid">
                    {activeFields
                      .filter((field) => field.id !== options.prompt_field_id)
                      .map((field) => (
                        <label key={field.id} className="field field--checkbox">
                          <input
                            type="checkbox"
                            checked={options.reveal_field_ids.includes(field.id)}
                            onChange={() => toggleRevealField(field.id)}
                          />
                          <span>{field.label}</span>
                        </label>
                      ))}
                  </div>
                </div>

                <div className="field-grid field-grid--triple">
                  <label className="field">
                    <span>{t("study.mode")}</span>
                    <select
                      value={options.mode}
                      onChange={(event) =>
                        setOptions((current) => ({
                          ...current,
                          mode: event.target.value as StudyMode,
                        }))
                      }
                    >
                      <option value="mixed">{t("study.mode.mixed")}</option>
                      <option value="due">{t("study.mode.due")}</option>
                      <option value="new">{t("study.mode.new")}</option>
                    </select>
                  </label>
                  <label className="field">
                    <span>{t("study.cardsPerSession")}</span>
                    <input
                      type="number"
                      min={1}
                      max={200}
                      value={options.cards_limit}
                      onChange={(event) =>
                        setOptions((current) => ({
                          ...current,
                          cards_limit: Number(event.target.value),
                        }))
                      }
                    />
                  </label>
                  <label className="field field--checkbox">
                    <input
                      type="checkbox"
                      checked={options.reverse_mode}
                      onChange={(event) => setOptions((current) => ({ ...current, reverse_mode: event.target.checked }))}
                    />
                    <span>{t("study.reverseMode")}</span>
                  </label>
                </div>

                <label className="field field--checkbox">
                  <input
                    type="checkbox"
                    checked={options.random_order}
                    onChange={(event) => setOptions((current) => ({ ...current, random_order: event.target.checked }))}
                  />
                  <span>{t("study.randomOrder")}</span>
                </label>

                <div className="surface-muted">
                  <div className="surface-muted__label">{t("study.directionPreview")}</div>
                  <div className="detail-inline-stats detail-inline-stats--wrap">
                    <span>
                      {t("study.directionFront")}: {directionPreview.front}
                    </span>
                    <span>
                      {t("study.directionReveal")}: {directionPreview.reveal.join(", ")}
                    </span>
                  </div>
                  {options.reverse_mode ? <p>{t("study.reverseHelp")}</p> : null}
                </div>

                <div className="surface-muted">
                  <div className="surface-muted__label">{t("study.keyboard")}</div>
                  <div className="detail-inline-stats">
                    <span>{t("study.keyboardReveal")}</span>
                    <span>{t("study.keyboardAgain")}</span>
                    <span>{t("study.keyboardHard")}</span>
                    <span>{t("study.keyboardGood")}</span>
                    <span>{t("study.keyboardEasy")}</span>
                  </div>
                </div>

                <Button onClick={() => void startSession()} disabled={starting}>
                  {starting ? t("common.loading") : t("study.start")}
                </Button>
              </div>
            )}
          </div>

          {!summary ? (
            <div className="study-empty">
              <EmptyState title={t("study.readyTitle")} description={t("study.readyDescription")} />
            </div>
          ) : null}
        </section>
      ) : queue.length === 0 ? (
        <EmptyState
          title={t("study.noneTitle")}
          description={t("study.noneDescription")}
          actions={
            <>
              <Button variant="secondary" onClick={() => setSession(null)}>
                {t("study.changeOptions")}
              </Button>
              <Button onClick={() => navigate(`/decks/${deckId}`)}>{t("common.back")}</Button>
            </>
          }
        />
      ) : (
        <section className="study-layout">
          <div className="study-panel">
            <div className="study-progress">
              <span>
                {stats.studied + 1} / {totalSteps}
              </span>
              <span>
                {queue.length - 1} {t("study.remaining")}
              </span>
            </div>

            <article className={`flashcard ${revealed ? "flashcard--revealed" : ""}`}>
              <div className="flashcard__label">
                {activeFieldLabel(activeFields, frontFieldId)}
                <span className={`pill ${currentCard.leech ? "pill--danger" : ""}`}>
                  {currentCard.leech ? t("study.stateLeech") : t(`study.state.${currentCard.review_state}`)}
                </span>
              </div>
              <div className="flashcard__prompt">
                <FieldText value={frontValue} />
              </div>

              {revealed ? (
                <div className="flashcard__answers">
                  {answerFieldIds
                    .filter((fieldId) => !isContextField(activeFields.find((field) => field.id === fieldId) ?? { label: "", language_code: null }))
                    .map((fieldId) => (
                      <div key={fieldId} className="flashcard__answer-row">
                        <span className="flashcard__answer-label">{activeFieldLabel(activeFields, fieldId)}</span>
                        <FieldText value={getStudyFieldValue(currentCard, fieldId)} />
                      </div>
                    ))}
                  {answerFieldIds.some((fieldId) =>
                    isContextField(activeFields.find((field) => field.id === fieldId) ?? { label: "", language_code: null })
                  ) ? (
                    <div className="flashcard__context-group">
                      <div className="surface-muted__label">{t("study.answerContext")}</div>
                      {answerFieldIds
                        .filter((fieldId) =>
                          isContextField(activeFields.find((field) => field.id === fieldId) ?? { label: "", language_code: null })
                        )
                        .map((fieldId) => (
                          <div key={fieldId} className="flashcard__meta">
                            <span className="flashcard__answer-label">{activeFieldLabel(activeFields, fieldId)}</span>
                            <FieldText value={getStudyFieldValue(currentCard, fieldId)} />
                          </div>
                        ))}
                    </div>
                  ) : null}
                  <div className="flashcard__review-meta">
                    <span>{t("study.metaDifficulty", { value: currentCard.difficulty.toFixed(1) })}</span>
                    <span>{t("study.metaStability", { value: currentCard.stability_days.toFixed(1) })}</span>
                  </div>
                </div>
              ) : (
                <button className="reveal-button" onClick={() => setRevealed(true)}>
                  {t("study.reveal")}
                </button>
              )}
            </article>

            <div className="study-actions">
              <Button variant="danger" disabled={!revealed} onClick={() => void handleGrade("again")}>
                {t("study.again")}
              </Button>
              <Button variant="secondary" disabled={!revealed} onClick={() => void handleGrade("hard")}>
                {t("study.hard")}
              </Button>
              <Button disabled={!revealed} onClick={() => void handleGrade("good")}>
                {t("study.good")}
              </Button>
              <Button variant="ghost" disabled={!revealed} onClick={() => void handleGrade("easy")}>
                {t("study.easy")}
              </Button>
            </div>
          </div>

          <aside className="study-sidebar">
            <StatCard label={t("study.summaryStudied")} value={stats.studied} />
            <StatCard label={t("study.sidebarCorrect")} value={stats.correct} />
            <StatCard label={t("study.summaryWrong")} value={stats.wrong} />
            <StatCard label={t("study.summaryAccuracy")} value={formatAccuracy(stats.correct, stats.studied)} />
          </aside>
        </section>
      )}
    </div>
  );
}
