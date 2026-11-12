import idMessages from "./id.json";
import enMessages from "./en.json";

export type Locale = "id" | "en";

export const SUPPORTED_LOCALES: ReadonlyArray<{ value: Locale; label: string }> = [
  { value: "id", label: "Bahasa Indonesia" },
  { value: "en", label: "English" },
];

type MessageSchema = typeof idMessages;

type DotKey<T, Prefix extends string = ""> = T extends Record<string, unknown>
  ? {
      [K in keyof T & string]: T[K] extends Record<string, unknown>
        ? DotKey<T[K], Prefix extends "" ? K : `${Prefix}.${K}`>
        : Prefix extends ""
          ? K
          : `${Prefix}.${K}`;
    }[keyof T & string]
  : never;

export type TranslationKey = DotKey<MessageSchema>;

const LOCALE_STORAGE_KEY = "forge_locale";

const dictionaries: Record<Locale, MessageSchema> = {
  id: idMessages,
  en: enMessages,
};
const _reverseParityCheck: typeof enMessages = idMessages;
void _reverseParityCheck;

function readStoredLocale(): Locale {
  try {
    const saved = localStorage.getItem(LOCALE_STORAGE_KEY);
    if (saved === "id" || saved === "en") return saved;
  } catch {
    // ignore storage access errors
  }
  return "id";
}

let currentLocale = $state<Locale>(readStoredLocale());

export function getLocale(): Locale {
  return currentLocale;
}

export function setLocale(next: Locale): void {
  const normalized: Locale = next === "en" ? "en" : "id";
  currentLocale = normalized;
  try {
    localStorage.setItem(LOCALE_STORAGE_KEY, normalized);
  } catch {
    // ignore storage errors
  }
  if (typeof document !== "undefined") {
    document.documentElement.lang = normalized;
  }
}

export function initLocale(): void {
  if (typeof document !== "undefined") {
    document.documentElement.lang = currentLocale;
  }
}

function lookupPath(dict: Record<string, unknown>, key: string): string | undefined {
  let node: unknown = dict;
  for (const part of key.split(".")) {
    if (node && typeof node === "object" && part in (node as Record<string, unknown>)) {
      node = (node as Record<string, unknown>)[part];
    } else {
      return undefined;
    }
  }
  return typeof node === "string" ? node : undefined;
}

export function t(key: TranslationKey, params?: Record<string, string | number>): string {
  const dict = dictionaries[currentLocale] as unknown as Record<string, unknown>;
  const template =
    lookupPath(dict, key) ??
    lookupPath(idMessages as unknown as Record<string, unknown>, key) ??
    key;
  if (!params) return template;
  return template.replace(/\{([a-zA-Z0-9_]+)\}/g, (_, name: string) =>
    Object.prototype.hasOwnProperty.call(params, name) ? String(params[name]) : `{${name}}`
  );
}

export function formatBranchDisplay(branch: string): string {
  return branch === "HEAD terlepas" ? t("common.detachedHead") : branch;
}
