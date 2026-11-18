import { afterEach, describe, expect, it } from "vitest";
import { setLanguage, t, translateMessage } from "./i18n.svelte";

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
