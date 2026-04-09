import { useEffect, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { useAppContext } from "../../app/AppContext";
import { Button } from "../../components/ui/Button";
import { EmptyState } from "../../components/ui/EmptyState";
import { FieldText } from "../../components/ui/FieldText";
import { StatCard } from "../../components/ui/StatCard";
import { useToast } from "../../components/ui/ToastProvider";
import { api } from "../../lib/api";
import { formatAccuracy } from "../../lib/format";
import { useKeyboardShortcut } from "../../lib/keyboard";
import type { DeckSummary, PromptLanguage, SessionSummary, StudyCard, StudyMode, StudySessionOptions, StudySessionPayload } from "../../lib/types";

function getFrontField(card: StudyCard, prompt: PromptLanguage, reverse: boolean): [string, string] {
  if (!reverse) {
    if (prompt === "language_1") return ["language_1", card.language_1];
    if (prompt === "language_2") return ["language_2", card.language_2];
    return ["language_3", card.language_3];
  }

  if (prompt === "language_1") return ["language_2", card.language_2];
  if (prompt === "language_2") return ["language_1", card.language_1];
  return ["language_1", card.language_1];
}

function getAnswerFields(card: StudyCard, frontField: string): Array<[string, string]> {
  const values: Array<[string, string]> = [
    ["language_1", card.language_1],
    ["language_2", card.language_2],
    ["language_3", card.language_3]
  ];
  return values.filter(([field]) => field !== frontField);
}

export function StudySessionPage() {
  const maxSessionRequeues = 2;
  const params = useParams();
  const navigate = useNavigate();
  const { settings } = useAppContext();
  const { notify } = useToast();
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
  const [options, setOptions] = useState<StudySessionOptions>({
    deck_id: deckId,
    prompt_language: settings.default_prompt_language,
    mode: settings.default_study_mode,
    random_order: settings.random_order,
    reverse_mode: settings.reverse_mode,
    cards_limit: settings.cards_per_session
  });

  useEffect(() => {
    void api
      .getDeck(deckId)
      .then((response) => {
        setDeck(response);
        setLoadError(null);
      })
      .catch((err) => {
        const message = typeof err === "object" && err && "message" in err ? String(err.message) : "Unable to load this deck.";
        setLoadError(message);
        notify(message, "error");
      });
  }, [deckId]);

  const currentCard = queue[0] ?? null;
  const totalSteps = stats.studied + queue.length;
  const [frontField, frontValue] = currentCard
    ? getFrontField(currentCard, options.prompt_language, options.reverse_mode)
    : ["language_1", ""];
  const answerFields = currentCard ? getAnswerFields(currentCard, frontField) : [];

  useKeyboardShortcut([" ", "Enter"], () => setRevealed(true), Boolean(currentCard && !revealed));
  useKeyboardShortcut(["ArrowRight", "k", "K"], () => void handleGrade(true), Boolean(currentCard && revealed));
  useKeyboardShortcut(["ArrowLeft", "d", "D"], () => void handleGrade(false), Boolean(currentCard && revealed));
  useKeyboardShortcut(["Escape"], () => navigate(`/decks/${deckId}`), true);

  async function startSession() {
    setStarting(true);
    setSummary(null);
    setStats({ studied: 0, correct: 0, wrong: 0, newlyMastered: 0 });
    setRequeueCounts({});
    try {
      const payload = await api.startStudySession(options);
      setSession(payload);
      setQueue(payload.cards);
      setRevealed(false);
      setLoadError(null);
      if (payload.cards.length === 0) {
        notify("No cards match this study mode right now.", "info");
      }
    } catch (err) {
      const message = typeof err === "object" && err && "message" in err ? String(err.message) : "Unable to start the study session.";
      setLoadError(message);
      notify(message, "error");
    } finally {
      setStarting(false);
    }
  }

  async function handleGrade(knewIt: boolean) {
    if (!session || !currentCard) {
      return;
    }

    let response;
    try {
      response = await api.gradeCard({
        session_id: session.session_id,
        card_id: currentCard.id,
        knew_it: knewIt
      });
    } catch (err) {
      const message = typeof err === "object" && err && "message" in err ? String(err.message) : "Unable to grade this card.";
      notify(message, "error");
      return;
    }

    const nextStudied = stats.studied + 1;
    const nextCorrect = stats.correct + (knewIt ? 1 : 0);
    const nextWrong = stats.wrong + (knewIt ? 0 : 1);
    const nextNewlyMastered = stats.newlyMastered + (response.newly_mastered ? 1 : 0);

    const remaining = queue.slice(1);
    if (!knewIt && (requeueCounts[currentCard.id] ?? 0) < maxSessionRequeues) {
      const insertIndex = Math.min(2, remaining.length);
      remaining.splice(insertIndex, 0, currentCard);
      setRequeueCounts((current) => ({
        ...current,
        [currentCard.id]: (current[currentCard.id] ?? 0) + 1
      }));
    }
    setQueue(remaining);
    setRevealed(false);
    setStats({
      studied: nextStudied,
      correct: nextCorrect,
      wrong: nextWrong,
      newlyMastered: nextNewlyMastered
    });

    if (remaining.length === 0) {
      try {
        const nextSummary = await api.completeStudySession({
          session_id: session.session_id,
          deck_id: session.deck.id,
          studied_count: nextStudied,
          correct_count: nextCorrect,
          wrong_count: nextWrong,
          newly_mastered_count: nextNewlyMastered
        });
        setSummary(nextSummary);
      } catch (err) {
        const message = typeof err === "object" && err && "message" in err ? String(err.message) : "Unable to complete this session.";
        notify(message, "error");
      }
    }
  }

  return (
    <div className="study-page">
      <section className="page-header">
        <div>
          <p className="eyebrow">Study</p>
          <h1>{deck?.name ?? "Study session"}</h1>
          <p>One card at a time, with keyboard shortcuts and session-local reinforcement for misses.</p>
        </div>
        <div className="page-header__actions">
          <Button variant="secondary" onClick={() => navigate(`/decks/${deckId}`)}>
            Back to deck
          </Button>
        </div>
      </section>

      {loadError ? <div className="inline-error">{loadError}</div> : null}

      {!session || summary ? (
        <section className="study-layout">
          <div className="study-config surface-panel">
            <h2>{summary ? "Session summary" : "Session options"}</h2>

            {summary ? (
              <div className="form-stack">
                <div className="stat-grid">
                  <StatCard label="Studied" value={summary.studied_count} />
                  <StatCard label="Accuracy" value={`${summary.accuracy_percent}%`} />
                  <StatCard label="Wrong" value={summary.wrong_count} />
                  <StatCard label="Newly mastered" value={summary.newly_mastered_count} />
                </div>
                <div className="surface-muted">
                  <div className="surface-muted__label">What next</div>
                  <p>{summary.suggestion}</p>
                  <div className="detail-inline-stats">
                    <span>{summary.remaining_due_cards} due cards remain</span>
                  </div>
                </div>
                <div className="dialog-actions dialog-actions--start">
                  <Button variant="secondary" onClick={() => navigate(`/decks/${deckId}`)}>
                    Back to deck
                  </Button>
                  <Button onClick={() => void startSession()}>Study again</Button>
                </div>
              </div>
            ) : (
              <div className="form-stack">
                <div className="field-grid field-grid--dual">
                  <label className="field">
                    <span>Prompt language</span>
                    <select
                      value={options.prompt_language}
                      onChange={(event) =>
                        setOptions((current) => ({
                          ...current,
                          prompt_language: event.target.value as PromptLanguage
                        }))
                      }
                    >
                      <option value="language_1">{deck?.language_1_label ?? "Language 1"}</option>
                      <option value="language_2">{deck?.language_2_label ?? "Language 2"}</option>
                      <option value="language_3">{deck?.language_3_label ?? "Language 3"}</option>
                    </select>
                  </label>
                  <label className="field">
                    <span>Study mode</span>
                    <select
                      value={options.mode}
                      onChange={(event) =>
                        setOptions((current) => ({
                          ...current,
                          mode: event.target.value as StudyMode
                        }))
                      }
                    >
                      <option value="mixed">Mixed</option>
                      <option value="due">Due only</option>
                      <option value="new">New only</option>
                    </select>
                  </label>
                </div>

                <div className="field-grid field-grid--triple">
                  <label className="field">
                    <span>Cards this session</span>
                    <input
                      type="number"
                      min={1}
                      max={200}
                      value={options.cards_limit}
                      onChange={(event) =>
                        setOptions((current) => ({
                          ...current,
                          cards_limit: Number(event.target.value)
                        }))
                      }
                    />
                  </label>
                  <label className="field field--checkbox">
                    <input
                      type="checkbox"
                      checked={options.random_order}
                      onChange={(event) => setOptions((current) => ({ ...current, random_order: event.target.checked }))}
                    />
                    <span>Random order</span>
                  </label>
                  <label className="field field--checkbox">
                    <input
                      type="checkbox"
                      checked={options.reverse_mode}
                      onChange={(event) => setOptions((current) => ({ ...current, reverse_mode: event.target.checked }))}
                    />
                    <span>Reverse mode</span>
                  </label>
                </div>

                <div className="surface-muted">
                  <div className="surface-muted__label">Keyboard</div>
                  <div className="detail-inline-stats">
                    <span>Reveal: Space / Enter</span>
                    <span>Knew it: K / Right Arrow</span>
                    <span>Didn't know: D / Left Arrow</span>
                  </div>
                </div>

                <Button onClick={() => void startSession()} disabled={starting}>
                  {starting ? "Preparing..." : "Start session"}
                </Button>
              </div>
            )}
          </div>

          {!summary ? (
            <div className="study-empty">
              <EmptyState
                title="Ready when you are"
                description="Choose the study direction and launch a local review session."
              />
            </div>
          ) : null}
        </section>
      ) : queue.length === 0 ? (
        <EmptyState
          title="No cards available"
          description="Try a different study mode or import more cards into this deck."
          actions={
            <>
              <Button variant="secondary" onClick={() => setSession(null)}>
                Change options
              </Button>
              <Button onClick={() => navigate(`/decks/${deckId}`)}>Back to deck</Button>
            </>
          }
        />
      ) : (
        <section className="study-layout">
          <div className="study-panel">
            <div className="study-progress">
              <span>{stats.studied + 1} / {totalSteps}</span>
              <span>{queue.length - 1} remaining</span>
            </div>

            <article className={`flashcard ${revealed ? "flashcard--revealed" : ""}`}>
              <div className="flashcard__label">
                {frontField === "language_1"
                  ? deck?.language_1_label
                  : frontField === "language_2"
                    ? deck?.language_2_label
                    : deck?.language_3_label}
              </div>
              <div className="flashcard__prompt">
                <FieldText value={frontValue} />
              </div>

              {revealed ? (
                <div className="flashcard__answers">
                  {answerFields.map(([field, value]) => (
                    <div key={field} className="flashcard__answer-row">
                      <span className="flashcard__answer-label">
                        {field === "language_1"
                          ? deck?.language_1_label
                          : field === "language_2"
                            ? deck?.language_2_label
                            : deck?.language_3_label}
                      </span>
                      <FieldText value={value} />
                    </div>
                  ))}
                  {currentCard.note ? (
                    <div className="flashcard__meta">
                      <span>Note</span>
                      <FieldText value={currentCard.note} />
                    </div>
                  ) : null}
                  {currentCard.example_sentence ? (
                    <div className="flashcard__meta">
                      <span>Example</span>
                      <FieldText value={currentCard.example_sentence} />
                    </div>
                  ) : null}
                </div>
              ) : (
                <button className="reveal-button" onClick={() => setRevealed(true)}>
                  Reveal answer
                </button>
              )}
            </article>

            <div className="study-actions">
              <Button variant="danger" disabled={!revealed} onClick={() => void handleGrade(false)}>
                Didn't know it
              </Button>
              <Button disabled={!revealed} onClick={() => void handleGrade(true)}>
                Knew it
              </Button>
            </div>
          </div>

          <aside className="study-sidebar">
            <StatCard label="Studied" value={stats.studied} />
            <StatCard label="Correct" value={stats.correct} />
            <StatCard label="Wrong" value={stats.wrong} />
            <StatCard label="Accuracy" value={formatAccuracy(stats.correct, stats.studied)} />
          </aside>
        </section>
      )}
    </div>
  );
}
