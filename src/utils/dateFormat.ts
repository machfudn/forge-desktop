import { getLocale, type Locale } from "../i18n/index.svelte";

export interface ParsedDateResult {
  date: Date;
  hasTime: boolean;
}

/**
 * Safely parses any date string, timestamp, or Date object into a valid Date and whether time was specified.
 * Supports:
 * - Date objects
 * - Unix timestamps (seconds or milliseconds)
 * - "YYYY-MM-DD" (parsed in local timezone to avoid UTC day-shift)
 * - "YYYY-MM-DD HH:MM" or "YYYY-MM-DD HH:MM:SS"
 * - "YYYY-MM-DD HH:MM:SS +0700" / Git tz strings
 * - Standard ISO-8601 strings
 */
export function parseDateInput(raw: string | number | Date | null | undefined): ParsedDateResult | null {
  if (raw == null || raw === "") return null;

  if (raw instanceof Date) {
    if (isNaN(raw.getTime())) return null;
    return { date: raw, hasTime: true };
  }

  if (typeof raw === "number") {
    // If < 1e11, it's Unix seconds (e.g. 1732430000); otherwise milliseconds
    const ms = raw < 1e11 ? raw * 1000 : raw;
    const d = new Date(ms);
    if (isNaN(d.getTime())) return null;
    return { date: d, hasTime: true };
  }

  const str = String(raw).trim();
  if (!str) return null;

  // Case 1: Simple date only "YYYY-MM-DD"
  const dateOnlyMatch = /^(\d{4})-(\d{2})-(\d{2})$/.exec(str);
  if (dateOnlyMatch) {
    const year = parseInt(dateOnlyMatch[1], 10);
    const month = parseInt(dateOnlyMatch[2], 10) - 1;
    const day = parseInt(dateOnlyMatch[3], 10);
    // Use noon to prevent daylight-savings or timezone shifts from changing the day
    const d = new Date(year, month, day, 12, 0, 0);
    return { date: d, hasTime: false };
  }

  // Case 2: "YYYY-MM-DD HH:MM" or "YYYY-MM-DD HH:MM:SS" (without timezone offset)
  const dateTimeNoTzMatch = /^(\d{4})-(\d{2})-(\d{2})[ T](\d{2}):(\d{2})(?::(\d{2}))?$/.exec(str);
  if (dateTimeNoTzMatch) {
    const year = parseInt(dateTimeNoTzMatch[1], 10);
    const month = parseInt(dateTimeNoTzMatch[2], 10) - 1;
    const day = parseInt(dateTimeNoTzMatch[3], 10);
    const hour = parseInt(dateTimeNoTzMatch[4], 10);
    const min = parseInt(dateTimeNoTzMatch[5], 10);
    const sec = dateTimeNoTzMatch[6] ? parseInt(dateTimeNoTzMatch[6], 10) : 0;
    const d = new Date(year, month, day, hour, min, sec);
    return { date: d, hasTime: true };
  }

  // Case 3: "YYYY-MM-DD HH:MM:SS +0700" / Git log output with offset
  const gitTzMatch = /^(\d{4})-(\d{2})-(\d{2})[ T](\d{2}):(\d{2}):(\d{2})\s*([+-]\d{2}):?(\d{2})?$/.exec(str);
  if (gitTzMatch) {
    const isoLike = `${gitTzMatch[1]}-${gitTzMatch[2]}-${gitTzMatch[3]}T${gitTzMatch[4]}:${gitTzMatch[5]}:${gitTzMatch[6]}${gitTzMatch[7]}:${gitTzMatch[8] || "00"}`;
    const parsed = Date.parse(isoLike);
    if (!isNaN(parsed)) {
      return { date: new Date(parsed), hasTime: true };
    }
  }

  // Case 4: Standard ISO / Date.parse fallback
  const cleanIso = str.includes("T") ? str : str.replace(" ", "T");
  const parsed = Date.parse(cleanIso);
  if (!isNaN(parsed)) {
    return { date: new Date(parsed), hasTime: !/^\d{4}-\d{2}-\d{2}$/.test(str) };
  }

  return null;
}

/**
 * Returns the Intl locale string ("id-ID" or "en-US") based on the active language.
 */
export function getIntlLocale(locale?: Locale): string {
  const lang = locale ?? getLocale();
  return lang === "en" ? "en-US" : "id-ID";
}

/**
 * Formats a date into a localized representation (Day Month Year [Hour:Minute]).
 * - Indonesian (id-ID): "24 Nov 2026" or "24 Nov 2026, 14:05" (24-hour)
 * - English (en-US): "Nov 24, 2026" or "Nov 24, 2026, 2:05 PM" (12-hour AM/PM)
 */
export function formatLocalDate(
  raw: string | number | Date | null | undefined,
  locale?: Locale
): string {
  const parsed = parseDateInput(raw);
  if (!parsed) return raw ? String(raw) : "";

  const intlLocale = getIntlLocale(locale);
  try {
    if (parsed.hasTime) {
      const isEn = intlLocale === "en-US";
      return new Intl.DateTimeFormat(intlLocale, {
        day: "numeric",
        month: "short",
        year: "numeric",
        hour: isEn ? "numeric" : "2-digit",
        minute: "2-digit",
        hour12: isEn,
      }).format(parsed.date);
    }

    return new Intl.DateTimeFormat(intlLocale, {
      day: "numeric",
      month: "short",
      year: "numeric",
    }).format(parsed.date);
  } catch {
    return raw ? String(raw) : "";
  }
}

/**
 * Formats a date into localized relative time ("5 menit yang lalu" / "5 minutes ago").
 */
export function formatRelativeDate(
  raw: string | number | Date | null | undefined,
  locale?: Locale
): string {
  const parsed = parseDateInput(raw);
  if (!parsed) return raw ? String(raw) : "";

  const lang = locale ?? getLocale();
  const diffSec = Math.round((parsed.date.getTime() - Date.now()) / 1000);
  const absSec = Math.abs(diffSec);

  if (absSec < 45) {
    return lang === "en" ? "just now" : "baru saja";
  }

  const intlLocale = getIntlLocale(lang);
  try {
    const rtf = new Intl.RelativeTimeFormat(intlLocale, { numeric: "auto" });
    if (absSec < 3600) return rtf.format(Math.round(diffSec / 60), "minute");
    if (absSec < 86400) return rtf.format(Math.round(diffSec / 3600), "hour");
    if (absSec < 2592000) return rtf.format(Math.round(diffSec / 86400), "day");
    if (absSec < 31536000) return rtf.format(Math.round(diffSec / 2592000), "month");
    return rtf.format(Math.round(diffSec / 31536000), "year");
  } catch {
    return formatLocalDate(raw, lang);
  }
}

/**
 * Formats a commit date according to the user's chosen mode: "relative" or "local" / "absolute".
 */
export function formatCommitDate(
  raw: string | number | Date | null | undefined,
  mode: "relative" | "absolute" | "local" = "relative",
  locale?: Locale
): string {
  if (mode === "relative") {
    return formatRelativeDate(raw, locale);
  }
  return formatLocalDate(raw, locale);
}

/**
 * Formats a full date and time for mouse hover tooltips (`title` attribute).
 * E.g.: "Selasa, 24 November 2026 14.05.00" or "Tuesday, November 24, 2026, 2:05:00 PM"
 */
export function formatFullTooltip(
  raw: string | number | Date | null | undefined,
  locale?: Locale
): string {
  const parsed = parseDateInput(raw);
  if (!parsed) return raw ? String(raw) : "";

  const intlLocale = getIntlLocale(locale);
  try {
    if (parsed.hasTime) {
      return new Intl.DateTimeFormat(intlLocale, {
        dateStyle: "full",
        timeStyle: "medium",
      }).format(parsed.date);
    }
    return new Intl.DateTimeFormat(intlLocale, {
      dateStyle: "full",
    }).format(parsed.date);
  } catch {
    return raw ? String(raw) : "";
  }
}
