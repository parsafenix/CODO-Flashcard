use std::{fs, path::Path};

use anyhow::{Context, Result};
use rusqlite::Connection;
use serde_json::json;

use crate::{
  db::repository::{card_repo, deck_repo},
  models::types::{ExportDeckInput, ExportFormat},
};

fn join_columns(columns: [&str; 3], delimiter: &str) -> String {
  columns.join(delimiter)
}

pub fn export_deck(connection: &Connection, input: &ExportDeckInput) -> Result<()> {
  let deck = deck_repo::get_deck(connection, input.deck_id)?.context("Deck not found")?;
  let cards = card_repo::list_cards(
    connection,
    &crate::models::types::CardListQuery {
      deck_id: input.deck_id,
      search: None,
      filter: Some(crate::models::types::CardFilter::All),
      sort: Some(crate::models::types::CardSort::Language1Asc),
    },
  )?;
  let output_path = Path::new(&input.output_path);

  match input.format {
    ExportFormat::Txt => {
      let delimiter = input.delimiter.clone().unwrap_or_else(|| "|".to_string());
      let mut lines = Vec::new();
      if input.include_header.unwrap_or(true) {
        lines.push(join_columns(
          [
            deck.language_1_label.as_str(),
            deck.language_2_label.as_str(),
            deck.language_3_label.as_str(),
          ],
          &delimiter,
        ));
      }
      for card in cards {
        lines.push(join_columns(
          [
            card.language_1.as_str(),
            card.language_2.as_str(),
            card.language_3.as_str(),
          ],
          &delimiter,
        ));
      }
      fs::write(output_path, lines.join("\n"))?;
    }
    ExportFormat::Json => {
      let payload = json!({
        "deck": deck,
        "cards": cards
      });
      fs::write(output_path, serde_json::to_string_pretty(&payload)?)?;
    }
  }

  Ok(())
}
