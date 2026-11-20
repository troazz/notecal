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
  "settings.alwaysOnTop": "Always on top (Ctrl+Shift+T; X11, macOS, Hyprland)",
  "settings.vimMode": "Vim keys in the editor",
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

  "cheatsheet.open": "Cheatsheet: calculations and functions",
  "cheatsheet.hint": "Type ? for the cheatsheet",
  "cheatsheet.help": "Enter insert into the note · type to filter · delete ? for notes · Esc close",
  "cheatsheet.footer":
    "Your own variables, functions and units for every note: definitions.txt (see Settings). Type ?keys for keyboard shortcuts.",
  "cheatsheet.none": "No example matches",
  "cheatsheet.arithmetic": "Arithmetic",
  "cheatsheet.percentages": "Percentages",
  "cheatsheet.variables": "Variables, prev and totals",
  "cheatsheet.units": "Units and conversion",
  "cheatsheet.currency": "Currency",
  "cheatsheet.dates": "Dates and time zones",
  "cheatsheet.css": "CSS units",
  "cheatsheet.bases": "Number bases",
  "cheatsheet.functions": "Functions",
  "cheatsheet.comments": "Comments and headers",
  "cheatsheet.keys": "Keyboard shortcuts",
  "cheat.precedence": "Multiplication before addition",
  "cheat.parentheses": "Parentheses first",
  "cheat.power": "Power (exponent)",
  "cheat.decimals": "Exact decimals, no 0.30000000000000004",
  "cheat.modulo": "Remainder of a division",
  "cheat.grouping": "Thousands separators are allowed",
  "cheat.percent-of": "A percentage of a number",
  "cheat.percent-add": "Add a percentage: tax, VAT, tip, markup",
  "cheat.percent-subtract": "Subtract a percentage: discount, sale",
  "cheat.as-percent": "What percent one number is of another",
  "cheat.assign": "Name a value and use it on later lines",
  "cheat.total": "Sum of the lines above (also sum)",
  "cheat.average": "Average of the lines above (also average)",
  "cheat.prev": "The previous line's result",
  "cheat.length": "Convert length and distance",
  "cheat.temperature": "Convert temperature",
  "cheat.data-size": "Convert file and data sizes",
  "cheat.speed": "Convert speed",
  "cheat.volume": "Convert cooking volumes",
  "cheat.duration": "Convert time spans",
  "cheat.currency-convert": "Convert money with daily exchange rates",
  "cheat.currency-to": "to works like in",
  "cheat.currency-large": "Large amounts with separators",
  "cheat.date-add": "A date from today: days, weeks, months, deadline",
  "cheat.date-diff": "Days between two dates, countdown",
  "cheat.date-diff-unit": "Time until a date in weeks",
  "cheat.month-end": "Adding a month clamps to the month's last day",
  "cheat.time-zone": "Current time in a city or time zone",
  "cheat.time-convert": "Convert a time between time zones, meeting time",
  "cheat.pt-to-px": "CSS points to pixels",
  "cheat.rem-to-px": "CSS rem to pixels (em size in Settings)",
  "cheat.px-to-rem": "Pixels to CSS rem",
  "cheat.to-hex": "Decimal to hexadecimal",
  "cheat.to-binary": "Hexadecimal to binary",
  "cheat.binary-math": "Calculate with binary numbers",
  "cheat.octal": "Octal to decimal",
  "cheat.sqrt": "Square root",
  "cheat.round": "Round to the nearest whole number",
  "cheat.abs": "Absolute value",
  "cheat.log": "Logarithm, base 10",
  "cheat.floor": "Round down",
  "cheat.pi": "Constants such as pi: area of a circle",
  "cheat.header": "A line starting with # is a header",
  "cheat.comment": "Text after // is a comment",
  "keys.notes": "Notes: open, create, rename, delete",
  "keys.cheatsheet": "This cheatsheet (type ? in Ctrl+P)",
  "keys.settings": "Settings",
  "keys.hide": "Hide the window; edits are saved",
  "keys.pin": "Always on top (pin)",
  "keys.copy": "Copy the cursor line's result",
  "keys.copyBare": "Copy the bare number (also Alt+click a result)",
  "keys.clickCopy": "Copy a result as shown",
  "keys.announce": "Read the result aloud (screen reader)",
  "keys.find": "Find and replace in the note",
  "keys.undo": "Undo, redo",
  "keys.stats": "Typing speed statistics",
  "keys.vimEsc": "Vim: insert or visual to normal mode; in normal mode, hide the window",
  "keys.vimWrite": "Vim: save now",
  "keys.vimQuit": "Vim: save and hide the window",
  "keys.vimClipboard": "Vim: yank to and put from the system clipboard",

  "app.alwaysOnTopOn": "Always on top: on",
  "app.alwaysOnTopOff": "Always on top: off",
  "app.ratesTitle": "Exchange rates by open.er-api.com",
  "app.rates": "Rates: {date}",
  "app.ratesNone": "none yet",
  "app.dismiss": "Dismiss",
  "app.copy": "Copy",

  "status.live": "live",
  "status.liveTitle": "Results with now or today update every minute",
  "status.localeTitle": "Number format (click to switch)",
  "status.languageTitle": "Interface language (click to switch)",
  "status.vimTitle": "Vim mode (Esc in normal mode hides the window)",
  "status.pinOn": "Always on top: on (Ctrl+Shift+T)",
  "status.pinOff": "Always on top: off (Ctrl+Shift+T)",
  "status.pinUnsupported": "This desktop doesn't let apps stay on top; use a window rule",

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
  "settings.alwaysOnTop": "Selalu di atas (Ctrl+Shift+T; X11, macOS, Hyprland)",
  "settings.vimMode": "Tombol Vim di editor",
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

  "cheatsheet.open": "Contekan: perhitungan dan fungsi",
  "cheatsheet.hint": "Ketik ? untuk contekan",
  "cheatsheet.help": "Enter sisipkan ke catatan · ketik untuk menyaring · hapus ? untuk catatan · Esc tutup",
  "cheatsheet.footer":
    "Variabel, fungsi, dan satuan Anda sendiri untuk semua catatan: definitions.txt (lihat Pengaturan). Ketik ?keys untuk pintasan keyboard.",
  "cheatsheet.none": "Tidak ada contoh yang cocok",
  "cheatsheet.arithmetic": "Aritmetika",
  "cheatsheet.percentages": "Persentase",
  "cheatsheet.variables": "Variabel, prev, dan total",
  "cheatsheet.units": "Satuan dan konversi",
  "cheatsheet.currency": "Mata uang",
  "cheatsheet.dates": "Tanggal dan zona waktu",
  "cheatsheet.css": "Satuan CSS",
  "cheatsheet.bases": "Basis bilangan",
  "cheatsheet.functions": "Fungsi",
  "cheatsheet.comments": "Komentar dan judul",
  "cheatsheet.keys": "Pintasan keyboard",
  "cheat.precedence": "Perkalian sebelum penjumlahan",
  "cheat.parentheses": "Tanda kurung lebih dulu",
  "cheat.power": "Pangkat (eksponen)",
  "cheat.decimals": "Desimal tepat, tanpa 0.30000000000000004",
  "cheat.modulo": "Sisa pembagian",
  "cheat.grouping": "Pemisah ribuan boleh dipakai",
  "cheat.percent-of": "Persentase dari sebuah angka",
  "cheat.percent-add": "Tambah persentase: pajak, PPN, tip, markup",
  "cheat.percent-subtract": "Kurangi persentase: diskon, potongan harga",
  "cheat.as-percent": "Berapa persen satu angka dari angka lain",
  "cheat.assign": "Beri nama nilai dan pakai di baris berikutnya",
  "cheat.total": "Jumlah baris di atas (juga sum)",
  "cheat.average": "Rata-rata baris di atas (juga average)",
  "cheat.prev": "Hasil baris sebelumnya",
  "cheat.length": "Konversi panjang dan jarak",
  "cheat.temperature": "Konversi suhu",
  "cheat.data-size": "Konversi ukuran file dan data",
  "cheat.speed": "Konversi kecepatan",
  "cheat.volume": "Konversi takaran masak",
  "cheat.duration": "Konversi rentang waktu",
  "cheat.currency-convert": "Konversi uang dengan kurs harian",
  "cheat.currency-to": "to sama dengan in",
  "cheat.currency-large": "Jumlah besar dengan pemisah ribuan",
  "cheat.date-add": "Tanggal dari hari ini: hari, minggu, bulan, tenggat",
  "cheat.date-diff": "Hari di antara dua tanggal, hitung mundur",
  "cheat.date-diff-unit": "Waktu sampai suatu tanggal dalam minggu",
  "cheat.month-end": "Menambah sebulan berhenti di hari terakhir bulan",
  "cheat.time-zone": "Waktu sekarang di kota atau zona waktu",
  "cheat.time-convert": "Konversi jam antar zona waktu, jadwal rapat",
  "cheat.pt-to-px": "Poin CSS ke piksel",
  "cheat.rem-to-px": "rem CSS ke piksel (ukuran em di Pengaturan)",
  "cheat.px-to-rem": "Piksel ke rem CSS",
  "cheat.to-hex": "Desimal ke heksadesimal",
  "cheat.to-binary": "Heksadesimal ke biner",
  "cheat.binary-math": "Berhitung dengan bilangan biner",
  "cheat.octal": "Oktal ke desimal",
  "cheat.sqrt": "Akar kuadrat",
  "cheat.round": "Bulatkan ke bilangan bulat terdekat",
  "cheat.abs": "Nilai mutlak",
  "cheat.log": "Logaritma basis 10",
  "cheat.floor": "Bulatkan ke bawah",
  "cheat.pi": "Konstanta seperti pi: luas lingkaran",
  "cheat.header": "Baris yang diawali # adalah judul",
  "cheat.comment": "Teks setelah // adalah komentar",
  "keys.notes": "Catatan: buka, buat, ganti nama, hapus",
  "keys.cheatsheet": "Contekan ini (ketik ? di Ctrl+P)",
  "keys.settings": "Pengaturan",
  "keys.hide": "Sembunyikan jendela; perubahan disimpan",
  "keys.pin": "Selalu di atas (sematkan)",
  "keys.copy": "Salin hasil baris kursor",
  "keys.copyBare": "Salin angka saja (juga Alt+klik hasil)",
  "keys.clickCopy": "Salin hasil seperti yang tampil",
  "keys.announce": "Bacakan hasil (pembaca layar)",
  "keys.find": "Cari dan ganti di catatan",
  "keys.undo": "Batalkan, ulangi",
  "keys.stats": "Statistik kecepatan mengetik",
  "keys.vimEsc": "Vim: dari insert atau visual ke mode normal; di mode normal, sembunyikan jendela",
  "keys.vimWrite": "Vim: simpan sekarang",
  "keys.vimQuit": "Vim: simpan dan sembunyikan jendela",
  "keys.vimClipboard": "Vim: salin ke dan tempel dari papan klip sistem",

  "app.alwaysOnTopOn": "Selalu di atas: aktif",
  "app.alwaysOnTopOff": "Selalu di atas: nonaktif",
  "app.ratesTitle": "Kurs dari open.er-api.com",
  "app.rates": "Kurs: {date}",
  "app.ratesNone": "belum ada",
  "app.dismiss": "Tutup",
  "app.copy": "Salin",

  "status.live": "langsung",
  "status.liveTitle": "Hasil dengan now atau today diperbarui setiap menit",
  "status.localeTitle": "Format angka (klik untuk mengganti)",
  "status.languageTitle": "Bahasa antarmuka (klik untuk mengganti)",
  "status.vimTitle": "Mode Vim (Esc di mode normal menyembunyikan jendela)",
  "status.pinOn": "Selalu di atas: aktif (Ctrl+Shift+T)",
  "status.pinOff": "Selalu di atas: nonaktif (Ctrl+Shift+T)",
  "status.pinUnsupported": "Desktop ini tidak mengizinkan aplikasi selalu di atas; gunakan aturan jendela",

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

/** A string in every language, for searches that should match either. */
export function inAllLanguages(key: Key): string[] {
  return [en[key], id[key]];
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
