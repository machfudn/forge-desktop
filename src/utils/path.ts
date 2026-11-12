import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";

export function pathBaseName(rawPath: string): string {
  const cleaned = rawPath.replace(/[\\/]+$/, "");
  if (!cleaned) return rawPath;
  const parts = cleaned.split(/[\\/]+/);
  return parts[parts.length - 1] || cleaned;
}

export function joinRepoPath(repoPath: string, filePath: string): string {
  const base = repoPath.replace(/[\\/]+$/, "");
  const sep = base.includes("\\") && !base.includes("/") ? "\\" : "/";
  const rel = filePath.replace(/^[\\/]+/, "").replace(/[\\/]+/g, sep);
  return `${base}${sep}${rel}`;
}

export async function openExternalUrl(url: string): Promise<void> {
  if (!url) return;
  try {
    if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
      await openUrl(url);
      return;
    }
  } catch {
    // Fallback to browser window.open outside Tauri WebView
  }
  if (typeof window !== "undefined") {
    window.open(url, "_blank", "noopener,noreferrer");
  }
}


export function deriveRepoNameFromUrl(rawUrl: string): string {
  const trimmed = rawUrl.trim().replace(/[\\/]+$/, "");
  if (!trimmed) return "";
  const noGit = trimmed.replace(/\.git$/i, "");
  const parts = noGit.split(/[/:]+/).filter(Boolean);
  const last = parts[parts.length - 1] || "";
  return /^[A-Za-z0-9._-]+$/.test(last) ? last : "";
}

export async function pickDirectory(options?: {
  title?: string;
  defaultPath?: string;
  canCreateDirectories?: boolean;
}): Promise<string | null> {
  if (typeof window === "undefined" || !("__TAURI_INTERNALS__" in window)) {
    throw new Error("Pemilih folder native hanya tersedia di aplikasi desktop Tauri.");
  }
  const selected = await openDialog({
    directory: true,
    multiple: false,
    title: options?.title,
    defaultPath: options?.defaultPath,
    canCreateDirectories: options?.canCreateDirectories,
  });
  return typeof selected === "string" ? selected : null;
}

export async function pickFile(options?: {
  title?: string;
  defaultPath?: string;
}): Promise<string | null> {
  if (typeof window === "undefined" || !("__TAURI_INTERNALS__" in window)) {
    throw new Error("Pemilih file native hanya tersedia di aplikasi desktop Tauri.");
  }
  const selected = await openDialog({
    directory: false,
    multiple: false,
    title: options?.title,
    defaultPath: options?.defaultPath,
  });
  return typeof selected === "string" ? selected : null;
}