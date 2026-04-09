import { save } from "@tauri-apps/plugin-dialog";
import { useEffect, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { useAppContext } from "../../app/AppContext";
import { Button } from "../../components/ui/Button";
import { ConfirmDialog } from "../../components/ui/ConfirmDialog";
import { EmptyState } from "../../components/ui/EmptyState";
import { FieldText } from "../../components/ui/FieldText";
import { useToast } from "../../components/ui/ToastProvider";
import { api } from "../../lib/api";
import { formatRelativeDate } from "../../lib/format";
import type { CardFilter, CardRecord, CardSort, DeckSummary, ExportFormat } from "../../lib/types";
import { CardFormModal } from "../cards/CardFormModal";
import { ImportWizard } from "../import/ImportWizard";
import { DeckFormModal } from "./DeckFormModal";

export function DeckDetailPage() {
  const params = useParams();
  const navigate = useNavigate();
  const { notify } = useToast();
  const { settings } = useAppContext();
  const deckId = Number(params.deckId);
  const [deck, setDeck] = useState<DeckSummary | null>(null);
  const [cards, setCards] = useState<CardRecord[]>([]);
  const [search, setSearch] = useState("");
  const [filter, setFilter] = useState<CardFilter>("all");
  const [sort, setSort] = useState<CardSort>("updated_desc");
  const [loading, setLoading] = useState(true);
  const [showDeckModal, setShowDeckModal] = useState(false);
  const [showImport, setShowImport] = useState(false);
  const [showCardModal, setShowCardModal] = useState(false);
  const [editingCard, setEditingCard] = useState<CardRecord | null>(null);
  const [deleteCard, setDeleteCard] = useState<CardRecord | null>(null);
  const [deleteDeckOpen, setDeleteDeckOpen] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);

  async function loadDeck() {
    try {
      const [deckResponse, cardsResponse] = await Promise.all([
        api.getDeck(deckId),
        api.listCards({ deck_id: deckId, search, filter, sort })
      ]);
      setDeck(deckResponse);
      setCards(cardsResponse);
      setLoadError(null);
    } catch (err) {
      const message = typeof err === "object" && err && "message" in err ? String(err.message) : "Unable to load this deck.";
      setLoadError(message);
      notify(message, "error");
    }
  }

  useEffect(() => {
    setLoading(true);
    void loadDeck().finally(() => setLoading(false));
  }, [deckId, filter, search, sort]);

  async function handleSaveCard(input: any) {
    if (input.id) {
      await api.updateCard(input);
      notify("Card updated.", "success");
    } else {
      await api.createCard(input);
      notify("Card added.", "success");
    }
    await loadDeck();
  }

  async function handleDeleteCard() {
    if (!deleteCard) {
      return;
    }
    await api.deleteCard(deleteCard.id);
    setDeleteCard(null);
    notify("Card deleted.", "success");
    await loadDeck();
  }

  async function handleDeleteDeck() {
    await api.deleteDeck(deckId);
    notify("Deck deleted.", "success");
    navigate("/");
  }

  async function handleExport(format: ExportFormat) {
    if (!deck) {
      return;
    }

    const path = await save({
      defaultPath: `${deck.name.replace(/\s+/g, "-").toLowerCase()}.${format === "txt" ? "txt" : "json"}`
    });
    if (!path) {
      return;
    }

    await api.exportDeck({
      deck_id: deck.id,
      output_path: path,
      format,
      delimiter: settings.import_delimiter,
      include_header: true
    });
    notify(`Deck exported as ${format.toUpperCase()}.`, "success");
  }

  if (loading && !deck) {
    return (
      <div className="boot-screen">
        <p>Loading deck...</p>
      </div>
    );
  }

  if (!deck) {
    return (
      <EmptyState
        title="Deck not found"
        description="This deck may have been deleted. Return to the library to choose another deck."
        actions={<Button onClick={() => navigate("/")}>Back to library</Button>}
      />
    );
  }

  return (
    <>
      <section className="page-header">
        <div>
          <p className="eyebrow">Deck</p>
          <h1>{deck.name}</h1>
          <p>{deck.description || "No description yet."}</p>
        </div>
        <div className="page-header__actions">
          <Button variant="secondary" onClick={() => navigate(`/study/${deck.id}`)}>
            Study now
          </Button>
          <Button variant="ghost" onClick={() => setShowImport(true)}>
            Import more
          </Button>
          <Button variant="ghost" onClick={() => setShowDeckModal(true)}>
            Edit deck
          </Button>
        </div>
      </section>

      <section className="detail-summary">
        <div className="surface-muted">
          <div className="surface-muted__label">Languages</div>
          <div className="field-grid field-grid--triple">
            <FieldText value={deck.language_1_label} />
            <FieldText value={deck.language_2_label} />
            <FieldText value={deck.language_3_label} />
          </div>
        </div>
        <div className="surface-muted">
          <div className="surface-muted__label">Study state</div>
          <div className="detail-inline-stats">
            <span>{deck.total_cards} total</span>
            <span>{deck.due_cards} due</span>
            <span>{deck.new_cards} new</span>
            <span>{deck.mastered_cards} mastered</span>
            <span>Last studied {formatRelativeDate(deck.last_studied_at)}</span>
          </div>
        </div>
      </section>

      <section className="toolbar toolbar--wrap">
        <label className="field field--grow">
          <span className="sr-only">Search cards</span>
          <input
            dir="auto"
            placeholder="Search across all three language fields"
            value={search}
            onChange={(event) => setSearch(event.target.value)}
          />
        </label>
        <label className="field">
          <span>Filter</span>
          <select value={filter} onChange={(event) => setFilter(event.target.value as CardFilter)}>
            <option value="all">All cards</option>
            <option value="new">New</option>
            <option value="due">Due</option>
            <option value="mastered">Mastered</option>
            <option value="weak">Weak</option>
          </select>
        </label>
        <label className="field">
          <span>Sort</span>
          <select value={sort} onChange={(event) => setSort(event.target.value as CardSort)}>
            <option value="updated_desc">Recently updated</option>
            <option value="created_desc">Recently created</option>
            <option value="next_review_asc">Next review</option>
            <option value="language_1_asc">{deck.language_1_label} A-Z</option>
          </select>
        </label>
        <Button onClick={() => {
          setEditingCard(null);
          setShowCardModal(true);
        }}>
          Add card
        </Button>
        <Button variant="secondary" onClick={() => void handleExport("txt")}>
          Export TXT
        </Button>
        <Button variant="secondary" onClick={() => void handleExport("json")}>
          Export JSON
        </Button>
        <Button variant="danger" onClick={() => setDeleteDeckOpen(true)}>
          Delete deck
        </Button>
      </section>

      {loadError ? (
        <div className="inline-error">
          {loadError}
          <div className="dialog-actions dialog-actions--start">
            <Button variant="secondary" onClick={() => void loadDeck()}>
              Retry
            </Button>
          </div>
        </div>
      ) : null}

      {cards.length === 0 ? (
        <EmptyState
          title="No cards in this deck"
          description="Add cards manually or import a text file to start studying."
          actions={
            <>
              <Button
                onClick={() => {
                  setEditingCard(null);
                  setShowCardModal(true);
                }}
              >
                Add card
              </Button>
              <Button variant="secondary" onClick={() => setShowImport(true)}>
                Import vocabulary
              </Button>
            </>
          }
        />
      ) : (
        <section className="table-shell">
          <table className="data-table">
            <thead>
              <tr>
                <th>{deck.language_1_label}</th>
                <th>{deck.language_2_label}</th>
                <th>{deck.language_3_label}</th>
                <th>Status</th>
                <th>Next review</th>
                <th>Accuracy</th>
                <th>Actions</th>
              </tr>
            </thead>
            <tbody>
              {cards.map((card) => (
                <tr key={card.id}>
                  <td><FieldText value={card.language_1} /></td>
                  <td><FieldText value={card.language_2} /></td>
                  <td><FieldText value={card.language_3} /></td>
                  <td><span className="pill">{card.status}</span></td>
                  <td>{formatRelativeDate(card.next_review_at)}</td>
                  <td>{card.review_count === 0 ? "-" : `${Math.round((card.correct_count / card.review_count) * 100)}%`}</td>
                  <td>
                    <div className="inline-actions">
                      <Button variant="ghost" onClick={() => {
                        setEditingCard(card);
                        setShowCardModal(true);
                      }}>
                        Edit
                      </Button>
                      <Button variant="danger" onClick={() => setDeleteCard(card)}>
                        Delete
                      </Button>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>
      )}

      <CardFormModal
        open={showCardModal}
        deck={deck}
        initialCard={editingCard}
        onClose={() => {
          setShowCardModal(false);
          setEditingCard(null);
        }}
        onSubmit={handleSaveCard}
      />

      <DeckFormModal
        open={showDeckModal}
        initialDeck={deck}
        onClose={() => setShowDeckModal(false)}
        onSubmit={async (input) => {
          await api.updateDeck(input as any);
          notify("Deck updated.", "success");
          await loadDeck();
          setShowDeckModal(false);
        }}
      />

      <ImportWizard
        open={showImport}
        decks={[deck]}
        fixedDeck={deck}
        defaultDelimiter={settings.import_delimiter}
        onClose={() => setShowImport(false)}
        onImported={async () => {
          notify("Import complete.", "success");
          await loadDeck();
        }}
      />

      <ConfirmDialog
        open={Boolean(deleteCard)}
        title="Delete card"
        description="Delete this card permanently?"
        confirmLabel="Delete card"
        onCancel={() => setDeleteCard(null)}
        onConfirm={() => void handleDeleteCard()}
      />

      <ConfirmDialog
        open={deleteDeckOpen}
        title="Delete deck"
        description={`Delete "${deck.name}" and all of its cards? This action cannot be undone.`}
        confirmLabel="Delete deck"
        onCancel={() => setDeleteDeckOpen(false)}
        onConfirm={() => void handleDeleteDeck()}
      />
    </>
  );
}
