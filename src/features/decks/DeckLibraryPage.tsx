import { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { useAppContext } from "../../app/AppContext";
import { Button } from "../../components/ui/Button";
import { ConfirmDialog } from "../../components/ui/ConfirmDialog";
import { EmptyState } from "../../components/ui/EmptyState";
import { StatCard } from "../../components/ui/StatCard";
import { useToast } from "../../components/ui/ToastProvider";
import { api } from "../../lib/api";
import { formatRelativeDate } from "../../lib/format";
import type { DeckLibrarySort, DeckSummary } from "../../lib/types";
import { ImportWizard } from "../import/ImportWizard";
import { DeckFormModal } from "./DeckFormModal";
import { sortDecks } from "./sorting";

export function DeckLibraryPage() {
  const navigate = useNavigate();
  const { settings } = useAppContext();
  const { notify } = useToast();
  const [decks, setDecks] = useState<DeckSummary[]>([]);
  const [search, setSearch] = useState("");
  const [loading, setLoading] = useState(true);
  const [editingDeck, setEditingDeck] = useState<DeckSummary | null>(null);
  const [deleteDeck, setDeleteDeck] = useState<DeckSummary | null>(null);
  const [showCreate, setShowCreate] = useState(false);
  const [showImport, setShowImport] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [sort, setSort] = useState<DeckLibrarySort>("due_desc");

  async function loadDecks() {
    setLoading(true);
    try {
      const response = await api.listDecks(search);
      setDecks(response);
      setLoadError(null);
    } catch (err) {
      const message = typeof err === "object" && err && "message" in err ? String(err.message) : "Unable to load decks.";
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
      notify("Deck updated.", "success");
    } else {
      await api.createDeck(input);
      notify("Deck created.", "success");
    }
    await loadDecks();
  }

  async function handleDeleteDeck() {
    if (!deleteDeck) {
      return;
    }

    await api.deleteDeck(deleteDeck.id);
    notify("Deck deleted.", "success");
    setDeleteDeck(null);
    await loadDecks();
  }

  async function handleDuplicateDeck(deckId: number) {
    await api.duplicateDeck(deckId);
    notify("Deck duplicated.", "success");
    await loadDecks();
  }

  if (!loading && decks.length === 0 && !search) {
    return (
      <>
        <section className="page-header">
          <div>
            <p className="eyebrow">Library</p>
            <h1>Your decks</h1>
            <p>Build a calm local study library and keep every card offline on your machine.</p>
          </div>
        </section>
        <EmptyState
          title="No decks yet"
          description="Create a deck manually or import a UTF-8 text file to get your first study set ready."
          actions={
            <>
              <Button onClick={() => setShowCreate(true)}>Create deck</Button>
              <Button variant="secondary" onClick={() => setShowImport(true)}>
                Import vocabulary
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
            notify("Import complete.", "success");
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
          <p className="eyebrow">Library</p>
          <h1>Your decks</h1>
          <p>Search, organize, and launch lightweight study sessions without leaving the desktop app.</p>
        </div>
        <div className="page-header__actions">
          <Button variant="secondary" onClick={() => setShowImport(true)}>
            Import vocabulary
          </Button>
          <Button onClick={() => setShowCreate(true)}>Create deck</Button>
        </div>
      </section>

      <section className="stat-grid">
        <StatCard label="Decks" value={decks.length} />
        <StatCard label="Cards" value={totalCards} />
        <StatCard label="Due now" value={dueCards} hint="Scheduled reviews only" />
        <StatCard label="New cards" value={newCards} hint="Tracked separately from due" />
      </section>

      <section className="toolbar toolbar--compact">
        <label className="field field--grow">
          <span className="sr-only">Search decks</span>
          <input dir="auto" placeholder="Search decks" value={search} onChange={(event) => setSearch(event.target.value)} />
        </label>
        <label className="field toolbar__select">
          <span>Sort decks</span>
          <select value={sort} onChange={(event) => setSort(event.target.value as DeckLibrarySort)}>
            <option value="due_desc">Most due cards</option>
            <option value="recent_studied">Most recently studied</option>
            <option value="new_desc">Most new cards</option>
            <option value="total_desc">Most cards total</option>
            <option value="mastered_desc">Most mastered cards</option>
            <option value="created_desc">Recently created</option>
            <option value="name_asc">Name (A-Z)</option>
            <option value="name_desc">Name (Z-A)</option>
          </select>
        </label>
      </section>

      {loadError ? (
        <div className="inline-error">
          {loadError}
          <div className="dialog-actions dialog-actions--start">
            <Button variant="secondary" onClick={() => void loadDecks()}>
              Retry
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
                <p>{deck.description || "No description yet."}</p>
              </div>
              <div className="deck-card__badges">
                <span className="pill">{deck.total_cards} cards</span>
                <span className="pill">{deck.due_cards} due</span>
              </div>
            </div>

            <dl className="deck-card__stats">
              <div>
                <dt>New</dt>
                <dd>{deck.new_cards}</dd>
              </div>
              <div>
                <dt>Mastered</dt>
                <dd>{deck.mastered_cards}</dd>
              </div>
              <div>
                <dt>Last studied</dt>
                <dd>{formatRelativeDate(deck.last_studied_at)}</dd>
              </div>
            </dl>

            <div className="deck-card__actions">
              <Button variant="secondary" onClick={() => navigate(`/study/${deck.id}`)}>
                Study
              </Button>
              <Button variant="ghost" onClick={() => navigate(`/decks/${deck.id}`)}>
                Open
              </Button>
              <Button variant="ghost" onClick={() => setEditingDeck(deck)}>
                Rename
              </Button>
              <Button variant="ghost" onClick={() => void handleDuplicateDeck(deck.id)}>
                Duplicate
              </Button>
              <Button variant="danger" onClick={() => setDeleteDeck(deck)}>
                Delete
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
          notify("Import complete.", "success");
          await loadDecks();
          navigate(`/decks/${deckId}`);
        }}
      />

      <ConfirmDialog
        open={Boolean(deleteDeck)}
        title="Delete deck"
        description={`Delete "${deleteDeck?.name ?? ""}" and every card inside it? This cannot be undone.`}
        confirmLabel="Delete deck"
        onCancel={() => setDeleteDeck(null)}
        onConfirm={() => void handleDeleteDeck()}
      />
    </>
  );
}
