use std::{fs, path::Path};

use anyhow::{Context, Result};
use rusqlite::Connection;
use serde_json::json;

use crate::{
  db::repository::{card_repo, deck_repo},
  models::types::{ExportDeckInput, ExportFormat},
};

fn join_columns(columns: &[String], delimiter: &str) -> String {
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
      sort: Some(crate::models::types::CardSort::PrimaryFieldAsc),
    },
  )?;
  let output_path = Path::new(&input.output_path);

  match input.format {
    ExportFormat::Txt => {
      let delimiter = input.delimiter.clone().unwrap_or_else(|| "|".to_string());
      let mut lines = Vec::new();
      let active_fields = deck.fields.iter().filter(|field| field.active).collect::<Vec<_>>();
      if input.include_header.unwrap_or(true) {
        lines.push(join_columns(
          &active_fields.iter().map(|field| field.label.clone()).collect::<Vec<_>>(),
          &delimiter,
        ));
      }
      for card in cards {
        let values_by_field = card
          .values
          .iter()
          .map(|value| (value.field_id, value.value.clone()))
          .collect::<std::collections::HashMap<_, _>>();
        lines.push(join_columns(
          &active_fields
            .iter()
            .map(|field| values_by_field.get(&field.id).cloned().unwrap_or_default())
            .collect::<Vec<_>>(),
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
