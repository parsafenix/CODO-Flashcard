import { save } from "@tauri-apps/plugin-dialog";
import { useEffect, useMemo, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { useAppContext } from "../../app/AppContext";
import { Button } from "../../components/ui/Button";
import { ConfirmDialog } from "../../components/ui/ConfirmDialog";
import { EmptyState } from "../../components/ui/EmptyState";
import { FieldText } from "../../components/ui/FieldText";
import { useToast } from "../../components/ui/ToastProvider";
import { api } from "../../lib/api";
import { getActiveFields, getCardValue } from "../../lib/deckFields";
import { useI18n } from "../../lib/i18n";
import { localizeAppMessage, localizeCardStatus } from "../../lib/messages";
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
  const { t } = useI18n();
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
        api.listCards({ deck_id: deckId, search, filter, sort }),
      ]);
      setDeck(deckResponse);
      setCards(cardsResponse);
      setLoadError(null);
    } catch (err) {
      const message = localizeAppMessage(
        typeof err === "object" && err && "message" in err ? String(err.message) : t("deck.loadError"),
        t
      );
      setLoadError(message);
      notify(message, "error");
    }
  }

  useEffect(() => {
    setLoading(true);
    void loadDeck().finally(() => setLoading(false));
  }, [deckId, filter, search, sort]);

  const activeFields = useMemo(() => (deck ? getActiveFields(deck.fields) : []), [deck]);

  async function handleSaveCard(input: any) {
    if (input.id) {
      await api.updateCard(input);
      notify(t("deck.cardUpdated"), "success");
    } else {
      await api.createCard(input);
      notify(t("deck.cardAdded"), "success");
    }
    await loadDeck();
  }

  async function handleDeleteCard() {
    if (!deleteCard) {
      return;
    }
    await api.deleteCard(deleteCard.id);
    setDeleteCard(null);
    notify(t("deck.cardDeleted"), "success");
    await loadDeck();
  }

  async function handleDeleteDeck() {
    await api.deleteDeck(deckId);
    notify(t("library.deckDeleted"), "success");
    navigate("/");
  }

  async function handleExport(format: ExportFormat) {
    if (!deck) {
      return;
    }

    const path = await save({
      defaultPath: `${deck.name.replace(/\s+/g, "-").toLowerCase()}.${format === "txt" ? "txt" : "json"}`,
    });
    if (!path) {
      return;
    }

    await api.exportDeck({
      deck_id: deck.id,
      output_path: path,
      format,
      delimiter: settings.import_delimiter,
      include_header: true,
    });
    notify(t("deck.exported", { format: format.toUpperCase() }), "success");
  }

  if (loading && !deck) {
    return (
      <div className="boot-screen">
        <p>{t("common.loading")}</p>
      </div>
    );
  }

  if (!deck) {
    return (
      <EmptyState
        title={t("deck.notFound")}
        description={t("deck.notFoundDescription")}
        actions={<Button onClick={() => navigate("/")}>{t("deck.backToLibrary")}</Button>}
      />
    );
  }

  return (
    <>
      <section className="page-header">
        <div>
          <p className="eyebrow">{t("deck.eyebrow")}</p>
          <h1>{deck.name}</h1>
          <p>{deck.description || t("deck.noDescription")}</p>
        </div>
        <div className="page-header__actions">
          <Button variant="secondary" onClick={() => navigate(`/study/${deck.id}`)}>
            {t("deck.studyNow")}
          </Button>
          <Button variant="ghost" onClick={() => setShowImport(true)}>
            {t("deck.importMore")}
          </Button>
          <Button variant="ghost" onClick={() => setShowDeckModal(true)}>
            {t("deck.editDeck")}
          </Button>
        </div>
      </section>

      <section className="detail-summary">
        <div className="surface-muted">
          <div className="surface-muted__label">{t("deck.fields")}</div>
          <div className="detail-inline-stats detail-inline-stats--wrap">
            {activeFields.map((field) => (
              <span key={field.id}>{field.label}</span>
            ))}
          </div>
        </div>
        <div className="surface-muted">
          <div className="surface-muted__label">{t("deck.studyState")}</div>
          <div className="detail-inline-stats detail-inline-stats--wrap">
            <span>{deck.total_cards} {t("deck.total")}</span>
            <span>{deck.due_cards} {t("deck.due")}</span>
            <span>{deck.new_cards} {t("deck.new")}</span>
            <span>{deck.mastered_cards} {t("deck.mastered")}</span>
            <span>
              {t("deck.lastStudied")} {formatRelativeDate(deck.last_studied_at)}
            </span>
          </div>
        </div>
      </section>

      <section className="toolbar toolbar--wrap">
        <label className="field field--grow">
          <span className="sr-only">{t("deck.searchCards")}</span>
          <input
            dir="auto"
            placeholder={t("deck.searchCards")}
            value={search}
            onChange={(event) => setSearch(event.target.value)}
          />
        </label>
        <label className="field">
          <span>{t("deck.filter")}</span>
          <select value={filter} onChange={(event) => setFilter(event.target.value as CardFilter)}>
            <option value="all">{t("deck.filter.all")}</option>
            <option value="new">{t("deck.filter.new")}</option>
            <option value="due">{t("deck.filter.due")}</option>
            <option value="mastered">{t("deck.filter.mastered")}</option>
            <option value="weak">{t("deck.filter.weak")}</option>
          </select>
        </label>
        <label className="field">
          <span>{t("deck.sort")}</span>
          <select value={sort} onChange={(event) => setSort(event.target.value as CardSort)}>
            <option value="updated_desc">{t("deck.sort.updated")}</option>
            <option value="created_desc">{t("deck.sort.created")}</option>
            <option value="next_review_asc">{t("deck.sort.nextReview")}</option>
            <option value="primary_field_asc">
              {activeFields[0]
                ? t("deck.sort.primaryField", { field: activeFields[0].label })
                : t("deck.sort.primaryFieldFallback")}
            </option>
          </select>
        </label>
        <Button
          onClick={() => {
            setEditingCard(null);
            setShowCardModal(true);
          }}
        >
          {t("deck.addCard")}
        </Button>
        <Button variant="secondary" onClick={() => void handleExport("txt")}>
          {t("deck.exportTxt")}
        </Button>
        <Button variant="secondary" onClick={() => void handleExport("json")}>
          {t("deck.exportJson")}
        </Button>
        <Button variant="danger" onClick={() => setDeleteDeckOpen(true)}>
          {t("deck.deleteDeck")}
        </Button>
      </section>

      {loadError ? (
        <div className="inline-error">
          {loadError}
          <div className="dialog-actions dialog-actions--start">
            <Button variant="secondary" onClick={() => void loadDeck()}>
              {t("common.retry")}
            </Button>
          </div>
        </div>
      ) : null}

      {cards.length === 0 ? (
        <EmptyState
          title={t("deck.noCards")}
          description={t("deck.noCardsDescription")}
          actions={
            <>
              <Button
                onClick={() => {
                  setEditingCard(null);
                  setShowCardModal(true);
                }}
              >
                {t("deck.addCard")}
              </Button>
              <Button variant="secondary" onClick={() => setShowImport(true)}>
                {t("library.importVocabulary")}
              </Button>
            </>
          }
        />
      ) : (
        <section className="table-shell">
          <table className="data-table">
            <thead>
              <tr>
                {activeFields.map((field) => (
                  <th key={field.id}>{field.label}</th>
                ))}
                <th>{t("deck.table.status")}</th>
                <th>{t("deck.table.nextReview")}</th>
                <th>{t("deck.table.accuracy")}</th>
                <th>{t("deck.table.actions")}</th>
              </tr>
            </thead>
            <tbody>
              {cards.map((card) => (
                <tr key={card.id}>
                  {activeFields.map((field) => (
                    <td key={field.id}>
                      <FieldText value={getCardValue(card.values, field.id)} />
                    </td>
                  ))}
                  <td>
                    <span className="pill">{localizeCardStatus(card.status, t)}</span>
                  </td>
                  <td>{formatRelativeDate(card.next_review_at)}</td>
                  <td>{card.review_count === 0 ? "-" : `${Math.round((card.correct_count / card.review_count) * 100)}%`}</td>
                  <td>
                    <div className="inline-actions">
                      <Button
                        variant="ghost"
                        onClick={() => {
                          setEditingCard(card);
                          setShowCardModal(true);
                        }}
                      >
                        {t("common.edit")}
                      </Button>
                      <Button variant="danger" onClick={() => setDeleteCard(card)}>
                        {t("common.delete")}
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
          notify(t("library.deckUpdated"), "success");
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
          notify(t("library.importComplete"), "success");
          await loadDeck();
        }}
      />

      <ConfirmDialog
        open={Boolean(deleteCard)}
        title={t("deck.deleteCardTitle")}
        description={t("deck.deleteCardDescription")}
        confirmLabel={t("common.delete")}
        onCancel={() => setDeleteCard(null)}
        onConfirm={() => void handleDeleteCard()}
      />

      <ConfirmDialog
        open={deleteDeckOpen}
        title={t("deck.deleteDeck")}
        description={t("deck.deleteDeckDescription", { name: deck.name })}
        confirmLabel={t("deck.deleteDeck")}
        onCancel={() => setDeleteDeckOpen(false)}
        onConfirm={() => void handleDeleteDeck()}
      />
    </>
  );
}
