import { inAllLanguages, t, type Key } from "../i18n.svelte";
import { SHORTCUTS } from "./shortcuts";

// Cheatsheet search (M12). Every word of the query must match, in either
// language and ignoring case and accents, one of:
// - a whole word of the section's title or keywords (`?date` shows the section;
//   `?pin` must not match every shortcut through "Pintasan"),
// - the start of a word of the row's explanation (`?pin` must not match "typing"), or
// - part of the example or the keys themselves (`?%`, `?0x`).

/** Extra words people search with, in English and Indonesian. */
export const SECTION_KEYWORDS: Record<string, string | undefined> = {
  arithmetic: "math add plus minus times multiply divide tambah kurang kali bagi hitung",
  percentages: "percent discount tax vat tip markup sale persen diskon pajak ppn potongan",
  variables: "variable name sum total average avg prev previous variabel nama jumlah rata",
  units: "unit convert conversion length weight distance satuan konversi berat jarak",
  currency: "money exchange rate fx usd idr eur uang kurs valuta",
  dates: "date dates time day week month year timezone tz calendar countdown deadline tanggal waktu hari minggu bulan tahun zona jam",
  css: "css px pt em rem pixel font web design",
  bases: "base hex hexadecimal binary octal decimal bit basis heksa biner oktal",
  functions: "function sqrt root round abs log floor ceil pi fungsi akar bulat",
  comments: "comment header heading note title komentar judul catatan",
  keys: "keyboard shortcut shortcuts key keys hotkey pintasan tombol",
};

/** Lowercase, without accents, so `é` matches `e`. */
export function fold(text: string): string {
  return text.normalize("NFD").replace(/\p{Diacritic}/gu, "").toLowerCase();
}

/** The query's words, folded. Empty for an empty query. */
export function queryWords(query: string): string[] {
  return fold(query).split(/\s+/).filter(Boolean);
}

/** The whole words of `texts`, folded. */
export function wordSet(texts: readonly string[]): Set<string> {
  return new Set(fold(texts.join(" ")).split(/[^\p{L}\p{N}]+/u).filter(Boolean));
}

/** True when every query word matches the section, the explanation or the code (see above). */
export function matchesAll(
  words: readonly string[],
  section: Set<string>,
  explanation: readonly string[],
  code: string,
): boolean {
  const prose = [...wordSet(explanation)];
  const folded = fold(code);
  return words.every(
    (word) => section.has(word) || folded.includes(word) || prose.some((p) => p.startsWith(word)),
  );
}

/** Mirrors `CheatSection` in src-tauri/src/evaluator.rs. */
export interface CheatSection {
  id: string;
  examples: { id: string; text: string; result: string | null }[];
}

export type CheatRow =
  | { kind: "example"; title: string; text: string; result: string | null; help: string; first: boolean }
  | { kind: "shortcut"; title: string; keys: string; help: string; first: boolean };

/** Cheatsheet rows matching `query`; the keyboard shortcuts come last. */
export function cheatRows(sheet: readonly CheatSection[], query: string, vimMode: boolean): CheatRow[] {
  const words = queryWords(query);
  const rows: CheatRow[] = [];
  for (const section of sheet) {
    const titleKey = `cheatsheet.${section.id}` as Key;
    const sectionWords = wordSet([...inAllLanguages(titleKey), section.id, SECTION_KEYWORDS[section.id] ?? ""]);
    let first = true;
    for (const example of section.examples) {
      const helpKey = `cheat.${example.id}` as Key;
      if (!matchesAll(words, sectionWords, inAllLanguages(helpKey), example.text)) continue;
      rows.push({ kind: "example", title: t(titleKey), ...example, help: t(helpKey), first });
      first = false;
    }
  }
  const keysWords = wordSet([...inAllLanguages("cheatsheet.keys"), SECTION_KEYWORDS.keys ?? ""]);
  let first = true;
  for (const shortcut of SHORTCUTS) {
    if (shortcut.vim && !vimMode) continue;
    if (!matchesAll(words, keysWords, inAllLanguages(shortcut.label), shortcut.keys)) continue;
    rows.push({ kind: "shortcut", title: t("cheatsheet.keys"), keys: shortcut.keys, help: t(shortcut.label), first });
    first = false;
  }
  return rows;
}
