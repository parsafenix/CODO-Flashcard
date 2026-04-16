import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { useAppContext } from "../../app/AppContext";
import { Button } from "../../components/ui/Button";
import { ConfirmDialog } from "../../components/ui/ConfirmDialog";
import { EmptyState } from "../../components/ui/EmptyState";
import { HiddenPanelsBar } from "../../components/ui/HiddenPanelsBar";
import { StatCard } from "../../components/ui/StatCard";
import { useToast } from "../../components/ui/ToastProvider";
import { api } from "../../lib/api";
import { formatRelativeDate } from "../../lib/format";
import { useI18n } from "../../lib/i18n";
import { localizeAppMessage } from "../../lib/messages";
import { usePanelVisibility } from "../../lib/usePanelVisibility";
import type { DeckLibrarySort, DeckSummary } from "../../lib/types";
import { DailyCoachPanel } from "./DailyCoachPanel";
import { ImportWizard } from "../import/ImportWizard";
import { DeckFormModal } from "./DeckFormModal";
import { sortDecks } from "./sorting";

export function DeckLibraryPage() {
  const navigate = useNavigate();
  const { settings } = useAppContext();
  const { notify } = useToast();
  const { t } = useI18n();
  const [decks, setDecks] = useState<DeckSummary[]>([]);
  const [search, setSearch] = useState("");
  const [loading, setLoading] = useState(true);
  const [editingDeck, setEditingDeck] = useState<DeckSummary | null>(null);
  const [deleteDeck, setDeleteDeck] = useState<DeckSummary | null>(null);
  const [showCreate, setShowCreate] = useState(false);
  const [showImport, setShowImport] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [sort, setSort] = useState<DeckLibrarySort>("due_desc");
  const coachPanels = [{ id: "daily-coach", label: t("coach.title") }];
  const { visiblePanels, hiddenPanels, hidePanel, showPanel } = usePanelVisibility("library", coachPanels);

  async function loadDecks() {
    setLoading(true);
    try {
      const response = await api.listDecks(search);
      setDecks(response);
      setLoadError(null);
    } catch (err) {
      const message = localizeAppMessage(
        typeof err === "object" && err && "message" in err ? String(err.message) : t("library.loadError"),
        t
      );
      setLoadError(message);
      notify(message, "error");
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void loadDecks();
  }, [search]);

  const totalCards = decks.reduce((sum, deck) => sum + deck.total_cards, 0);
  const dueCards = decks.reduce((sum, deck) => sum + deck.due_cards, 0);
  const newCards = decks.reduce((sum, deck) => sum + deck.new_cards, 0);
  const sortedDecks = sortDecks(decks, sort);

  async function handleSaveDeck(input: any) {
    if ("id" in input) {
      await api.updateDeck(input);
      notify(t("library.deckUpdated"), "success");
    } else {
      await api.createDeck(input);
      notify(t("library.deckCreated"), "success");
    }
    await loadDecks();
  }

  async function handleDeleteDeck() {
    if (!deleteDeck) {
      return;
    }

    await api.deleteDeck(deleteDeck.id);
    notify(t("library.deckDeleted"), "success");
    setDeleteDeck(null);
    await loadDecks();
  }

  async function handleDuplicateDeck(deckId: number) {
    await api.duplicateDeck(deckId);
    notify(t("library.deckDuplicated"), "success");
    await loadDecks();
  }

  if (!loading && decks.length === 0 && !search) {
    return (
      <>
        <section className="page-header">
          <div>
            <p className="eyebrow">{t("library.eyebrow")}</p>
            <h1>{t("library.title")}</h1>
            <p>{t("library.description")}</p>
          </div>
        </section>
        <EmptyState
          title={t("library.noDecks")}
          description={t("library.noDecksDescription")}
          actions={
            <>
              <Button onClick={() => setShowCreate(true)}>{t("library.createDeck")}</Button>
              <Button variant="secondary" onClick={() => setShowImport(true)}>
                {t("library.importVocabulary")}
              </Button>
            </>
          }
        />
        <DeckFormModal open={showCreate} onClose={() => setShowCreate(false)} onSubmit={handleSaveDeck} />
        <ImportWizard
          open={showImport}
          decks={decks}
          defaultDelimiter={settings.import_delimiter}
          onClose={() => setShowImport(false)}
          onImported={async (deckId) => {
            notify(t("library.importComplete"), "success");
            await loadDecks();
            navigate(`/decks/${deckId}`);
          }}
        />
      </>
    );
  }

  return (
    <>
      <section className="page-header">
        <div>
          <p className="eyebrow">{t("library.eyebrow")}</p>
          <h1>{t("library.title")}</h1>
          <p>{t("library.description")}</p>
        </div>
        <div className="page-header__actions">
          <Button variant="secondary" onClick={() => setShowImport(true)}>
            {t("library.importVocabulary")}
          </Button>
          <Button onClick={() => setShowCreate(true)}>{t("library.createDeck")}</Button>
        </div>
      </section>

      <section className="stat-grid">
        <StatCard label={t("library.stats.decks")} value={decks.length} />
        <StatCard label={t("library.stats.cards")} value={totalCards} />
        <StatCard label={t("library.stats.due")} value={dueCards} hint={t("library.stats.dueHint")} />
        <StatCard label={t("library.stats.new")} value={newCards} hint={t("library.stats.newHint")} />
      </section>

      <HiddenPanelsBar panels={hiddenPanels} onShow={(panelId) => void showPanel(panelId)} />

      {visiblePanels.some((panel) => panel.id === "daily-coach") ? (
        <DailyCoachPanel onHide={() => void hidePanel("daily-coach")} />
      ) : null}

      <section className="toolbar toolbar--compact">
        <label className="field field--grow">
          <span className="sr-only">{t("library.searchDecks")}</span>
          <input dir="auto" placeholder={t("library.searchDecks")} value={search} onChange={(event) => setSearch(event.target.value)} />
        </label>
        <label className="field toolbar__select">
          <span>{t("library.sortDecks")}</span>
          <select value={sort} onChange={(event) => setSort(event.target.value as DeckLibrarySort)}>
            <option value="due_desc">{t("library.sort.due")}</option>
            <option value="recent_studied">{t("library.sort.recent")}</option>
            <option value="new_desc">{t("library.sort.new")}</option>
            <option value="total_desc">{t("library.sort.total")}</option>
            <option value="mastered_desc">{t("library.sort.mastered")}</option>
            <option value="created_desc">{t("library.sort.created")}</option>
            <option value="name_asc">{t("library.sort.nameAsc")}</option>
            <option value="name_desc">{t("library.sort.nameDesc")}</option>
          </select>
        </label>
      </section>

      {loadError ? (
        <div className="inline-error">
          {loadError}
          <div className="dialog-actions dialog-actions--start">
            <Button variant="secondary" onClick={() => void loadDecks()}>
              {t("common.retry")}
            </Button>
          </div>
        </div>
      ) : null}

      <section className="deck-grid">
        {sortedDecks.map((deck) => (
          <article key={deck.id} className="deck-card">
            <div className="deck-card__head">
              <div>
                <h2>{deck.name}</h2>
                <p>{deck.description || "-"}</p>
              </div>
              <div className="deck-card__badges">
                <span className="pill">
                  {deck.total_cards} {t("library.cardsSuffix")}
                </span>
                <span className="pill">
                  {deck.due_cards} {t("library.dueSuffix")}
                </span>
              </div>
            </div>

            <dl className="deck-card__stats">
              <div>
                <dt>{t("library.new")}</dt>
                <dd>{deck.new_cards}</dd>
              </div>
              <div>
                <dt>{t("library.mastered")}</dt>
                <dd>{deck.mastered_cards}</dd>
              </div>
              <div>
                <dt>{t("library.lastStudied")}</dt>
                <dd>{formatRelativeDate(deck.last_studied_at)}</dd>
              </div>
            </dl>

            <div className="deck-card__actions">
              <Button variant="secondary" onClick={() => navigate(`/study/${deck.id}`)}>
                {t("library.study")}
              </Button>
              <Button variant="ghost" onClick={() => navigate(`/decks/${deck.id}`)}>
                {t("library.open")}
              </Button>
              <Button variant="ghost" onClick={() => setEditingDeck(deck)}>
                {t("library.editDeck")}
              </Button>
              <Button variant="ghost" onClick={() => void handleDuplicateDeck(deck.id)}>
                {t("library.duplicate")}
              </Button>
              <Button variant="danger" onClick={() => setDeleteDeck(deck)}>
                {t("common.delete")}
              </Button>
            </div>
          </article>
        ))}
      </section>

      <DeckFormModal
        open={showCreate || Boolean(editingDeck)}
        initialDeck={editingDeck}
        onClose={() => {
          setShowCreate(false);
          setEditingDeck(null);
        }}
        onSubmit={handleSaveDeck}
      />

      <ImportWizard
        open={showImport}
        decks={decks}
        defaultDelimiter={settings.import_delimiter}
        onClose={() => setShowImport(false)}
        onImported={async (deckId) => {
          notify(t("library.importComplete"), "success");
          await loadDecks();
          navigate(`/decks/${deckId}`);
        }}
      />

      <ConfirmDialog
        open={Boolean(deleteDeck)}
        title={t("library.deleteDeckTitle")}
        description={t("library.deleteDeckDescription", { name: deleteDeck?.name ?? "" })}
        confirmLabel={t("library.deleteDeckConfirm")}
        onCancel={() => setDeleteDeck(null)}
        onConfirm={() => void handleDeleteDeck()}
      />
    </>
  );
}
