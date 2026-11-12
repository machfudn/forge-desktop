export interface AppErrorLogEntry {
  id: number;
  timestamp: string;
  source: string;
  message: string;
  detail?: string;
}

const MAX_LOG_ENTRIES = 200;

let entries = $state<AppErrorLogEntry[]>([]);
let nextId = 1;
let listenersInstalled = false;

export function sanitizeLogText(raw: string): string {
  return raw
    .replace(/\b(gh[pous]_[A-Za-z0-9_]{10,}|github_pat_[A-Za-z0-9_]{10,})\b/g, "[REDACTED_TOKEN]")
    .replace(/\b(sk-[A-Za-z0-9_-]{12,})\b/g, "[REDACTED_KEY]")
    .replace(/(Bearer\s+)[A-Za-z0-9._-]+/gi, "$1[REDACTED]")
    .replace(/(https?:\/\/)([^:@\s]+):([^@\s]+)@/gi, "$1$2:[REDACTED]@");
}

function normalizeError(error: unknown, fallbackMessage?: string): { message: string; detail?: string } {
  if (error instanceof Error) {
    const message = sanitizeLogText((error.message || fallbackMessage || String(error)).trim());
    const stack = error.stack ? sanitizeLogText(error.stack.trim()) : undefined;
    const detail = stack && stack !== message ? stack : undefined;
    return { message, detail };
  }
  if (typeof error === "string") {
    return { message: sanitizeLogText(error.trim() || fallbackMessage || "Unknown error") };
  }
  if (error !== undefined && error !== null) {
    try {
      const json = JSON.stringify(error);
      return { message: sanitizeLogText(fallbackMessage || json) };
    } catch {
      return { message: sanitizeLogText(fallbackMessage || String(error)) };
    }
  }
  return { message: sanitizeLogText((fallbackMessage || "").trim()) };
}

export function logAppError(input: {
  source: string;
  message?: string;
  detail?: string;
  error?: unknown;
}): void {
  const normalized = normalizeError(input.error, input.message);
  const message = sanitizeLogText((input.message ?? normalized.message).trim());
  if (!message) return;

  const rawDetail = input.detail ? sanitizeLogText(input.detail.trim()) : normalized.detail;
  const detail = rawDetail && rawDetail !== message ? rawDetail : undefined;
  const now = Date.now();
  const latest = entries[0];
  if (latest) {
    const latestMs = Date.parse(latest.timestamp);
    if (Number.isFinite(latestMs) && now - latestMs < 1500) {
      if (latest.message === message) return;
      if (
        input.source === "ui" &&
        (message.includes(latest.message) || latest.message.includes(message))
      ) {
        return;
      }
    }
  }

  const next: AppErrorLogEntry = {
    id: nextId++,
    timestamp: new Date(now).toISOString(),
    source: input.source,
    message,
    detail,
  };
  entries = [next, ...entries.slice(0, MAX_LOG_ENTRIES - 1)];
}

export function getAppErrorLogs(): AppErrorLogEntry[] {
  return entries;
}

export function clearAppErrorLogs(): void {
  entries = [];
}

export function formatAppErrorLogsText(appVersion?: string): string {
  const header = appVersion ? `Forge Desktop v${appVersion} — Application Error Log` : "Forge Desktop — Application Error Log";
  if (entries.length === 0) return `${header}\n(No errors recorded)`;
  const lines = entries.map((e) => {
    const base = `[${e.timestamp}] [${e.source}] ${e.message}`;
    return e.detail ? `${base}\n${e.detail}` : base;
  });
  return `${header}\nTotal: ${entries.length}\n\n${lines.join("\n\n")}`;
}

export function installGlobalErrorListeners(): void {
  if (listenersInstalled || typeof window === "undefined") return;
  listenersInstalled = true;

  window.addEventListener("error", (event) => {
    const loc = event.filename ? `${event.filename}:${event.lineno}:${event.colno}` : "";
    const msg = event.message || (event.error instanceof Error ? event.error.message : "Unhandled runtime error");
    const stack = event.error instanceof Error ? event.error.stack : undefined;
    const detail = [loc, stack].filter(Boolean).join("\n");
    logAppError({
      source: "runtime",
      message: msg,
      detail: detail || undefined,
      error: event.error,
    });
  });

  window.addEventListener("unhandledrejection", (event) => {
    logAppError({
      source: "promise",
      error: event.reason,
      message: event.reason instanceof Error ? event.reason.message : String(event.reason || "Unhandled promise rejection"),
    });
  });
}
