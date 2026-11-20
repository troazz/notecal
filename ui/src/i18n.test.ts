import cheatsheetSource from "../../crates/notecal-core/src/cheatsheet.rs?raw";
import { afterEach, describe, expect, it } from "vitest";
import { setLanguage, t, translateMessage, type Key } from "./i18n.svelte";

afterEach(() => setLanguage("en"));

describe("t", () => {
  it("fills placeholders in the current language", () => {
    expect(t("switcher.create", { name: "budget" })).toBe("Create “budget”");
    setLanguage("id");
    expect(t("switcher.create", { name: "budget" })).toBe("Buat “budget”");
    expect(t("settings.title")).toBe("Pengaturan");
  });

  it("leaves unknown placeholders visible", () => {
    expect(t("app.rates")).toBe("Rates: {date}");
  });
});

describe("translateMessage", () => {
  it("keeps English as is", () => {
    expect(translateMessage("No exchange rates yet")).toBe("No exchange rates yet");
  });

  it("translates engine, shortcut and definitions messages into Indonesian", () => {
    setLanguage("id");
    const cases: [string, string][] = [
      ["No exchange rates yet", "Belum ada kurs"],
      ["There is nothing above to 'total'", "Tidak ada baris di atas untuk 'total'"],
      [
        "'km' is already a unit, function or constant; pick another name",
        "'km' sudah menjadi satuan, fungsi, atau konstanta; pilih nama lain",
      ],
      ["Unknown time zone 'Atlantis'", "Zona waktu 'Atlantis' tidak dikenal"],
      ["There is no 30 feb 2026", "Tanggal 30 feb 2026 tidak ada"],
      ["Alt+Space shows and hides notecal.", "Alt+Space menampilkan dan menyembunyikan notecal."],
      [
        "definitions.txt line 3: A definition is 'name = value' or 'unit name = value'",
        "definitions.txt baris 3: Definisi ditulis 'nama = nilai' atau 'unit nama = nilai'",
      ],
    ];
    for (const [english, indonesian] of cases) expect(translateMessage(english)).toBe(indonesian);
  });

  it("leaves messages it does not know (fend's own) in English", () => {
    setLanguage("id");
    expect(translateMessage("unexpected end of input")).toBe("unexpected end of input");
  });
});

describe("cheatsheet", () => {
  it("has a title in both languages for every section the engine defines", () => {
    const ids = [...cheatsheetSource.matchAll(/Section \{\s*id: "(\w+)"/g)].map((m) => m[1]);
    expect(ids.length).toBeGreaterThan(5);
    for (const language of ["en", "id"] as const) {
      setLanguage(language);
      for (const id of ids) {
        const key = `cheatsheet.${id}` as Key;
        expect(t(key), `${language} ${key}`).not.toBe(key);
        expect(t(key)).toBeTruthy();
      }
    }
  });

  it("has an explanation in both languages for every example", () => {
    const ids = [...cheatsheetSource.matchAll(/ex\("([\w-]+)"/g)].map((m) => m[1]);
    expect(ids.length).toBeGreaterThan(40);
    for (const language of ["en", "id"] as const) {
      setLanguage(language);
      for (const id of ids) {
        const key = `cheat.${id}` as Key;
        expect(t(key), `${language} ${key}`).not.toBe(key);
      }
    }
  });
});
