function collapseWhitespace(value: string): string {
  let output = "";
  let previousWasSpace = false;

  for (const rawCharacter of value) {
    const character = rawCharacter === "\u200C" || rawCharacter === "\u200D" ? " " : rawCharacter;
    const isSpaceLike = /\s/u.test(character) || character === "\u00A0" || character === "\u202F";

    if (isSpaceLike) {
      if (!previousWasSpace) {
        output += " ";
      }
      previousWasSpace = true;
      continue;
    }

    output += character;
    previousWasSpace = false;
  }

  return output.trim();
}

function mapPersianArabicCharacters(value: string): string {
  let output = "";
  for (const character of value) {
    if (character === "\u0640") {
      continue;
    }

    if (character === "\u064A" || character === "\u0649") {
      output += "\u06CC";
      continue;
    }

    if (character === "\u0643") {
      output += "\u06A9";
      continue;
    }

    output += character;
  }

  return output;
}

export function normalizeSearchText(value: string): string {
  return mapPersianArabicCharacters(collapseWhitespace(value).normalize("NFKC")).toLowerCase();
}

export function compactSearchText(value: string): string {
  return normalizeSearchText(value).replace(/\s+/gu, "");
}

export function isLikelyRtl(value: string): boolean {
  return /[\u0590-\u08FF]/u.test(value);
}
