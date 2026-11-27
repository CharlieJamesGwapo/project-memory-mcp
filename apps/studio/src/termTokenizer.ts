import termTable from "./term-ranges.json";

// Pin character classification to the server's Rust Unicode data. System
// WebKit and Node can recognize different Unicode versions, so property-escape
// regexes are unsuitable for validating or modeling the native contract.
function isAlphanumeric(point: number): boolean {
  let low = 0;
  let high = termTable.ranges.length - 1;
  while (low <= high) {
    const middle = low + Math.floor((high - low) / 2);
    const [start, end] = termTable.ranges[middle];
    if (point < start) high = middle - 1;
    else if (point > end) low = middle + 1;
    else return true;
  }
  return false;
}

export function taskTerms(query: string): string[] {
  const terms = new Set<string>();
  let term = "";
  for (const character of query) {
    const point = character.codePointAt(0)!;
    if (isAlphanumeric(point)) {
      term += point >= 0x41 && point <= 0x5a
        ? String.fromCharCode(point + 0x20)
        : character;
    } else if (term) {
      terms.add(term);
      term = "";
    }
  }
  if (term) terms.add(term);
  return [...terms];
}
