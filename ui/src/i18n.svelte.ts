/**
 * Interface text in English and Indonesian (PRD M9).
 *
 * `t(key)` reads the current language reactively, so text in templates
 * re-renders when the language changes. Messages that come from Rust (the
 * engine's hover text, the shortcut status, definitions problems) are English;
 * `translateMessage` translates the known ones and leaves the rest, such as
 * fend's own errors, in English.
 */

export type Language = "en" | "id";

const en = {
  "settings.title": "Settings",
  "settings.language": "Language",
  "settings.languageAuto": "Auto (system)",
  "settings.theme": "Theme",
  "settings.themeAuto": "Auto (Omarchy, else system)",
  "settings.themeLight": "Light",
  "settings.themeDark": "Dark",
  "settings.font": "Font",
  "settings.fontSize": "Font size",
  "settings.precision": "Decimal places",
  "settings.numberFormat": "Number format",
  "settings.cssPpi": "CSS px per inch",
  "settings.cssEm": "CSS px per em/rem",
  "settings.notesDir": "Notes folder",
  "settings.notesDirDefault": "Default (~/.local/share/notecal/notes)",
  "settings.shortcut": "Shortcut",
  "settings.alwaysOnTop": "Always on top (Ctrl+Shift+T; X11/macOS)",
  "settings.background": "Keep running in the background when closed",
  "settings.currencyRefresh": "Update exchange rates daily",
  "settings.definitionsBefore":
    "Your own variables, functions and units go in ~/.config/notecal/definitions.txt, for example",
  "settings.or": "or",
  "settings.definitionsAfter": "They apply to every note.",
  "settings.saved":
    "Saved to ~/.config/notecal/settings.toml as you change them. Exchange rates by Exchange Rate API (www.exchangerate-api.com, open.er-api.com).",
  "settings.escToClose": "Esc to close",

  "switcher.notes": "Notes",
  "switcher.placeholder": "Find or create a note",
  "switcher.newName": "New name for {name}",
  "switcher.open": "open",
  "switcher.rename": "Rename",
  "switcher.renameTitle": "Rename (F2)",
  "switcher.delete": "Delete",
  "switcher.deleteTitle": "Delete (Ctrl+Delete)",
  "switcher.create": "Create “{name}”",
  "switcher.confirmDelete": "Delete “{name}”? Press Enter to delete, any other key to keep.",
  "switcher.help": "Enter open · F2 rename · Ctrl+Delete delete · Esc close",
  "switcher.confirmDeleteShort": "Delete “{name}”? Enter to delete, any other key to keep",
  "switcher.empty": "Type a name to create a note",

  "app.alwaysOnTopOn": "Always on top: on",
  "app.alwaysOnTopOff": "Always on top: off",
  "app.ratesTitle": "Exchange rates by open.er-api.com",
  "app.rates": "Rates: {date}",
  "app.ratesNone": "none yet",
  "app.dismiss": "Dismiss",
  "app.copy": "Copy",

  "results.copied": "Copied",
  "results.copyFailed": "Could not copy the result",
  "results.none": "No result on this line",
  "results.noneBecause": "No result: {reason}",

  "session.unsaved": "“{name}” has unsaved edits that could not be saved.",
  "session.saveError": "Something went wrong while saving: {error}",
  "session.saveFailed": "Could not save “{name}”: {error}",
  "session.conflict":
    "“{name}” was changed outside notecal. Your version was saved as “{copy}”.",
} as const;

export type Key = keyof typeof en;

/** Every key must be translated: a missing one is a type error. */
const id: Record<Key, string> = {
  "settings.title": "Pengaturan",
  "settings.language": "Bahasa",
  "settings.languageAuto": "Otomatis (sistem)",
  "settings.theme": "Tema",
  "settings.themeAuto": "Otomatis (Omarchy, atau sistem)",
  "settings.themeLight": "Terang",
  "settings.themeDark": "Gelap",
  "settings.font": "Fon",
  "settings.fontSize": "Ukuran fon",
  "settings.precision": "Jumlah desimal",
  "settings.numberFormat": "Format angka",
  "settings.cssPpi": "px CSS per inci",
  "settings.cssEm": "px CSS per em/rem",
  "settings.notesDir": "Folder catatan",
  "settings.notesDirDefault": "Bawaan (~/.local/share/notecal/notes)",
  "settings.shortcut": "Pintasan",
  "settings.alwaysOnTop": "Selalu di atas (Ctrl+Shift+T; X11/macOS)",
  "settings.background": "Tetap berjalan di latar belakang saat ditutup",
  "settings.currencyRefresh": "Perbarui kurs setiap hari",
  "settings.definitionsBefore":
    "Variabel, fungsi, dan satuan Anda sendiri ditaruh di ~/.config/notecal/definitions.txt, misalnya",
  "settings.or": "atau",
  "settings.definitionsAfter": "Semuanya berlaku di setiap catatan.",
  "settings.saved":
    "Tersimpan ke ~/.config/notecal/settings.toml begitu diubah. Kurs dari Exchange Rate API (www.exchangerate-api.com, open.er-api.com).",
  "settings.escToClose": "Esc untuk menutup",

  "switcher.notes": "Catatan",
  "switcher.placeholder": "Cari atau buat catatan",
  "switcher.newName": "Nama baru untuk {name}",
  "switcher.open": "terbuka",
  "switcher.rename": "Ganti nama",
  "switcher.renameTitle": "Ganti nama (F2)",
  "switcher.delete": "Hapus",
  "switcher.deleteTitle": "Hapus (Ctrl+Delete)",
  "switcher.create": "Buat “{name}”",
  "switcher.confirmDelete":
    "Hapus “{name}”? Tekan Enter untuk menghapus, tombol lain untuk membatalkan.",
  "switcher.help": "Enter buka · F2 ganti nama · Ctrl+Delete hapus · Esc tutup",
  "switcher.confirmDeleteShort": "Hapus “{name}”? Enter untuk menghapus, tombol lain untuk batal",
  "switcher.empty": "Ketik nama untuk membuat catatan",

  "app.alwaysOnTopOn": "Selalu di atas: aktif",
  "app.alwaysOnTopOff": "Selalu di atas: nonaktif",
  "app.ratesTitle": "Kurs dari open.er-api.com",
  "app.rates": "Kurs: {date}",
  "app.ratesNone": "belum ada",
  "app.dismiss": "Tutup",
  "app.copy": "Salin",

  "results.copied": "Disalin",
  "results.copyFailed": "Hasil tidak dapat disalin",
  "results.none": "Tidak ada hasil di baris ini",
  "results.noneBecause": "Tidak ada hasil: {reason}",

  "session.unsaved": "“{name}” punya perubahan yang tidak dapat disimpan.",
  "session.saveError": "Terjadi kesalahan saat menyimpan: {error}",
  "session.saveFailed": "Gagal menyimpan “{name}”: {error}",
  "session.conflict":
    "“{name}” diubah di luar notecal. Versi Anda disimpan sebagai “{copy}”.",
};

const tables: Record<Language, Record<Key, string>> = { en, id };

const state = $state<{ language: Language }>({ language: "en" });

export function setLanguage(language: Language): void {
  state.language = language;
}

export function currentLanguage(): Language {
  return state.language;
}

/** The text for `key` in the current language, with `{name}` placeholders filled. */
export function t(key: Key, params: Record<string, string> = {}): string {
  const text = tables[state.language][key];
  return text.replace(/\{(\w+)\}/g, (match, name: string) => params[name] ?? match);
}

/**
 * Indonesian for messages produced in Rust, matched by their English text.
 * `$1`… are the pattern's groups. Unknown messages stay English.
 */
const MESSAGES_ID: [RegExp, string][] = [
  [/^No exchange rates yet$/, "Belum ada kurs"],
  [/^No exchange rate for (.+)$/, "Tidak ada kurs untuk $1"],
  [/^Internal error while evaluating this line$/, "Kesalahan internal saat menghitung baris ini"],
  [/^This calculation took too long$/, "Perhitungan ini terlalu lama"],
  [/^Unmatched '\)'$/, "Ada ')' tanpa pasangan"],
  [
    /^Only one assignment per line, at the start: name = …$/,
    "Hanya satu penetapan per baris, di awal: nama = …",
  ],
  [
    /^The lines above have units that cannot be added$/,
    "Baris di atas punya satuan yang tidak bisa dijumlahkan",
  ],
  [/^There is no earlier result for 'prev'$/, "Belum ada hasil sebelumnya untuk 'prev'"],
  [/^There is nothing above to '(.+)'$/, "Tidak ada baris di atas untuk '$1'"],
  [
    /^'(.+)' groups thousands, so write numbers like (.+)$/,
    "'$1' memisahkan ribuan, jadi tulis angka seperti $2",
  ],
  [/^'(.+)' is reserved and cannot be a variable$/, "'$1' sudah dipakai dan tidak bisa jadi variabel"],
  [/^'(.+)' is reserved and cannot be a unit$/, "'$1' sudah dipakai dan tidak bisa jadi satuan"],
  [
    /^'(.+)' is reserved: names starting with '_' are internal$/,
    "'$1' sudah dipakai: nama yang diawali '_' khusus untuk internal",
  ],
  [
    /^'(.+)' is already a unit, function or constant; pick another name$/,
    "'$1' sudah menjadi satuan, fungsi, atau konstanta; pilih nama lain",
  ],
  [
    /^A definition is 'name = value' or 'unit name = value'$/,
    "Definisi ditulis 'nama = nilai' atau 'unit nama = nilai'",
  ],
  [/^A unit is defined as 'unit name = value'$/, "Satuan ditulis 'unit nama = nilai'"],
  [/^Unknown time zone '(.+)'$/, "Zona waktu '$1' tidak dikenal"],
  [
    /^Add or subtract a duration, like '\+ 2 weeks'$/,
    "Tambah atau kurangi dengan durasi, misalnya '+ 2 weeks'",
  ],
  [/^Subtract a date or a duration$/, "Kurangi dengan tanggal atau durasi"],
  [
    /^Use whole numbers of years, months, weeks and days$/,
    "Gunakan bilangan bulat untuk tahun, bulan, minggu, dan hari",
  ],
  [/^That duration is too long$/, "Durasi itu terlalu panjang"],
  [/^Can't use '(.+)' with a date$/, "'$1' tidak bisa dipakai dengan tanggal"],
  [/^Can't use '(.+)' after a date difference$/, "'$1' tidak bisa dipakai setelah selisih tanggal"],
  [/^'(.+)' is not a date$/, "'$1' bukan tanggal"],
  [/^(.+) is not a time$/, "$1 bukan waktu"],
  [/^There is no (.+)$/, "Tanggal $1 tidak ada"],
  [
    /^On Wayland, bind `notecal --toggle` to a key in your compositor \(see docs\/linux-setup\.md\)\.$/,
    "Di Wayland, pasang `notecal --toggle` ke sebuah tombol di compositor Anda (lihat docs/linux-setup.md).",
  ],
  [/^(.+) shows and hides notecal\.$/, "$1 menampilkan dan menyembunyikan notecal."],
  [/^Could not register (.+?): (.*)$/, "Gagal mendaftarkan $1: $2"],
];

/** Translates a message produced in Rust into the current language. */
export function translateMessage(message: string): string {
  if (state.language === "en") return message;
  // `definitions.txt line 3: <engine message>`
  const definition = /^(definitions\.txt) line (\d+): (.*)$/.exec(message);
  if (definition) {
    return `${definition[1]} baris ${definition[2]}: ${translateMessage(definition[3] ?? "")}`;
  }
  for (const [pattern, replacement] of MESSAGES_ID) {
    if (pattern.test(message)) return message.replace(pattern, replacement);
  }
  return message;
}
