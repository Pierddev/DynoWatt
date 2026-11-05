import { createI18n } from "vue-i18n";
import en from "./locales/en.json";

export type MessageSchema = typeof en;

export const i18n = createI18n<[MessageSchema], "en">({
  legacy: false,
  locale: "en",
  fallbackLocale: "en",
  messages: { en },
  datetimeFormats: {
    en: {
      // Activity start times are wall-clock times where the ride happened,
      // passed as UTC instants and formatted in UTC: the time zone of the
      // machine never shifts them (see `toRow`).
      date: { weekday: "short", day: "numeric", month: "short", year: "numeric", timeZone: "UTC" },
      time: { hour: "2-digit", minute: "2-digit", hourCycle: "h23", timeZone: "UTC" },
    },
  },
  numberFormats: {
    en: {
      km: { minimumFractionDigits: 1, maximumFractionDigits: 1 },
      whole: { maximumFractionDigits: 0 },
      tenths: { minimumFractionDigits: 1, maximumFractionDigits: 1 },
    },
  },
});
