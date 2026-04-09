use sha2::{Digest, Sha256};
use unicode_normalization::UnicodeNormalization;

#[derive(Debug, Clone)]
pub struct NormalizedCardFields {
  pub language_1_normalized: String,
  pub language_2_normalized: String,
  pub language_3_normalized: String,
  pub language_1_compact: String,
  pub language_2_compact: String,
  pub language_3_compact: String,
  pub dedupe_key: String,
}

fn collapse_whitespace(value: &str) -> String {
  let mut output = String::new();
  let mut previous_was_space = false;

  for character in value.chars() {
    let mut mapped = character;

    if matches!(mapped, '\u{200C}' | '\u{200D}') {
      mapped = ' ';
    }

    if mapped.is_whitespace() || matches!(mapped, '\u{00A0}' | '\u{202F}') {
      if !previous_was_space {
        output.push(' ');
      }
      previous_was_space = true;
    } else {
      output.push(mapped);
      previous_was_space = false;
    }
  }

  output.trim().to_string()
}

fn normalize_persian_arabic(value: &str) -> String {
  value
    .chars()
    .filter_map(|character| match character {
      '\u{0640}' => None,
      '\u{064A}' | '\u{0649}' => Some('\u{06CC}'),
      '\u{0643}' => Some('\u{06A9}'),
      other => Some(other),
    })
    .collect()
}

pub fn normalize_text(value: &str) -> String {
  let collapsed = collapse_whitespace(value);
  let normalized = collapsed.nfkc().collect::<String>();
  normalize_persian_arabic(&normalized).to_lowercase()
}

pub fn compact_text(value: &str) -> String {
  normalize_text(value).replace(' ', "")
}

pub fn normalize_card_fields(language_1: &str, language_2: &str, language_3: &str) -> NormalizedCardFields {
  let language_1_normalized = normalize_text(language_1);
  let language_2_normalized = normalize_text(language_2);
  let language_3_normalized = normalize_text(language_3);

  let language_1_compact = language_1_normalized.replace(' ', "");
  let language_2_compact = language_2_normalized.replace(' ', "");
  let language_3_compact = language_3_normalized.replace(' ', "");

  let mut hasher = Sha256::new();
  hasher.update(language_1_normalized.as_bytes());
  hasher.update([0x1F]);
  hasher.update(language_2_normalized.as_bytes());
  hasher.update([0x1F]);
  hasher.update(language_3_normalized.as_bytes());

  let dedupe_key = format!("{:x}", hasher.finalize());

  NormalizedCardFields {
    language_1_normalized,
    language_2_normalized,
    language_3_normalized,
    language_1_compact,
    language_2_compact,
    language_3_compact,
    dedupe_key,
  }
}

#[cfg(test)]
mod tests {
  use super::{compact_text, normalize_card_fields, normalize_text};

  #[test]
  fn normalizes_persian_characters_and_whitespace() {
    let value = "  \u{0643}\u{062A}\u{0627}\u{0628}\u{200c}\u{0647}\u{0627}  ";
    assert_eq!(normalize_text(value), "\u{06A9}\u{062A}\u{0627}\u{0628} \u{0647}\u{0627}");
    assert_eq!(compact_text(value), "\u{06A9}\u{062A}\u{0627}\u{0628}\u{0647}\u{0627}");
  }

  #[test]
  fn dedupe_is_stable_for_equivalent_variants() {
    let a = normalize_card_fields("\u{0633}\u{0644}\u{0627}\u{0645}", "Hello", "Ciao");
    let b = normalize_card_fields("\u{0633}\u{0644}\u{0627}\u{0645}", "  hello ", "Ciao");
    assert_eq!(a.dedupe_key, b.dedupe_key);
  }
}
