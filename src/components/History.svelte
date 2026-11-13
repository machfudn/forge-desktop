<script lang="ts">
  import type { AccountProfile, BranchInfo, CommitChangedFile, CommitDetail, CommitInfo, HistoryColumnId } from "../types/git";
  import * as gitService from "../services/git";
  import Diff from "./Diff.svelte";
  import { getLocale, t } from "../i18n/index.svelte";
  import { joinRepoPath, openExternalUrl } from "../utils/path";
  import { computeCommitGraph, GRAPH_COLORS } from "../utils/commitGraph";
  import {
    copyTextToClipboard,
    openContextMenu,
    type ContextMenuItem,
  } from "../lib/contextMenu.svelte";

  interface Props {
    repoPath?: string;
    externalEditorLabel?: string;
    remoteWebUrl?: string | null;
    /** @deprecated use remoteWebUrl */
    githubRepoUrl?: string | null;
    accounts?: AccountProfile[];
    commits: CommitInfo[];
    searchQuery?: string;
    allBranches?: boolean;
    branches?: BranchInfo[];
    currentBranch?: string;
    selectedBranches?: string[];
    onFilterBranches?: (filter: { all: boolean; selected: string[] }) => void;
    tags?: string[];
    onCreateTag?: () => void;
    onInteractiveRebase?: () => void;
    onSearchChange?: (q: string) => void;
    onTagContextMenu?: (e: MouseEvent, tagName: string) => void;
    checkRunsMap?: Record<string, { status: string; conclusion?: string }>;
    onCherryPick?: (sha: string) => void;
    onRevert?: (sha: string) => void;
    onTag?: (sha: string) => void;
    onDeleteTag?: (tagName: string) => void;
    onReset?: (sha: string) => void;
    onSquash?: (commitIndex: number, sha: string) => void;
    onMergeBranch?: (branchName: string) => void;
    onRebaseBranch?: (baseRef: string) => void;
    onRevealFile?: (filePath: string) => void;
    onOpenInEditor?: (filePath: string) => void;
    onInspectFile?: (filePath: string, mode: "blame" | "history") => void;
    onAiExplainCommit?: (sha: string, summary: string) => void;
    onStatus?: (msg: string, isError?: boolean) => void;
  }

  let {
    repoPath = "",
    externalEditorLabel = "Editor",
    remoteWebUrl = null,
    githubRepoUrl = null,
    accounts = [],
    commits,
    searchQuery = "",
    allBranches = true,
    branches = [],
    currentBranch = "",
    selectedBranches = [],
    onFilterBranches,
    tags = [],
    onCreateTag,
    onInteractiveRebase,
    onSearchChange,
    onTagContextMenu,
    checkRunsMap = {},
    onCherryPick,
    onRevert,
    onTag,
    onDeleteTag,
    onReset,
    onSquash,
    onMergeBranch,
    onRebaseBranch,
    onRevealFile,
    onOpenInEditor,
    onInspectFile,
    onAiExplainCommit,
    onStatus,
  }: Props = $props();
  let selectedSha = $state<string | null>(null);
  let selectedDetail = $state<CommitDetail | null>(null);
  let detailLoading = $state(false);
  let detailError = $state("");
  let selectedCommitFile = $state<CommitChangedFile | null>(null);
  let commitFileDiff = $state("");
  let diffLoading = $state(false);
  let commitIgnoreWhitespace = $state(false);
  let detailRequestSeq = 0;
  let diffRequestSeq = 0;
  let lastRepoPath = "";

  const DETAIL_PCT_DEFAULT = 58;
  const DETAIL_PCT_MIN = 24;
  const DETAIL_PCT_MAX = 84;
  const DESC_H_MIN = 56;
  const FILES_W_DEFAULT = 220;
  const FILES_W_MIN = 140;
  const FILES_W_MAX = 480;
  const FILES_H_DEFAULT = 136;
  const FILES_H_MIN = 56;
  const FILES_H_MAX = 440;

  function readSavedNumber(key: string): number | null {
    const raw = localStorage.getItem(key);
    if (!raw) return null;
    const n = Number(raw);
    return Number.isFinite(n) ? n : null;
  }

  function clamp(val: number, min: number, max: number): number {
    return Math.min(max, Math.max(min, Math.round(val)));
  }

  let detailHeightPct = $state(
    clamp(readSavedNumber("forge_history_detail_pct") ?? DETAIL_PCT_DEFAULT, DETAIL_PCT_MIN, DETAIL_PCT_MAX)
  );
  let descHeaderHeight = $state<number | null>(readSavedNumber("forge_history_desc_h"));
  let filesWidthPx = $state(
    clamp(readSavedNumber("forge_history_files_w") ?? FILES_W_DEFAULT, FILES_W_MIN, FILES_W_MAX)
  );
  let filesHeightPx = $state(
    clamp(readSavedNumber("forge_history_files_h") ?? FILES_H_DEFAULT, FILES_H_MIN, FILES_H_MAX)
  );
  let filesLayout = $state<"horizontal" | "vertical">(
    localStorage.getItem("forge_history_files_layout") === "vertical" ? "vertical" : "horizontal"
  );

  let historyShellEl = $state<HTMLDivElement | null>(null);
  let detailPaneEl = $state<HTMLElement | null>(null);
  let detailHeaderEl = $state<HTMLDivElement | null>(null);
  let isCompactWidth = $state(false);
  let activeResizer = $state<"detail" | "desc" | "files" | null>(null);

  let isFilesStacked = $derived(isCompactWidth || filesLayout === "vertical");

  $effect(() => {
    const el = historyShellEl;
    if (!el) return;
    const updateWidth = () => {
      isCompactWidth = el.clientWidth > 0 && el.clientWidth <= 680;
    };
    updateWidth();
    const observer = new ResizeObserver(updateWidth);
    observer.observe(el);
    return () => observer.disconnect();
  });

  function updateDetailPct(next: number) {
    const clamped = clamp(next, DETAIL_PCT_MIN, DETAIL_PCT_MAX);
    detailHeightPct = clamped;
    localStorage.setItem("forge_history_detail_pct", String(clamped));
  }

  function updateDescHeaderHeight(next: number) {
    const maxH = Math.max(140, Math.round((detailPaneEl?.clientHeight ?? 520) * 0.72));
    const clamped = clamp(next, DESC_H_MIN, maxH);
    descHeaderHeight = clamped;
    localStorage.setItem("forge_history_desc_h", String(clamped));
  }

  function resetDescHeaderHeight() {
    descHeaderHeight = null;
    localStorage.removeItem("forge_history_desc_h");
  }

  function updateFilesWidth(next: number) {
    const maxW = Math.min(FILES_W_MAX, Math.max(FILES_W_MIN, Math.round((detailPaneEl?.clientWidth ?? 700) * 0.65)));
    const clamped = clamp(next, FILES_W_MIN, maxW);
    filesWidthPx = clamped;
    localStorage.setItem("forge_history_files_w", String(clamped));
  }

  function updateFilesHeight(next: number) {
    const maxH = Math.min(FILES_H_MAX, Math.max(120, Math.round((detailPaneEl?.clientHeight ?? 520) * 0.65)));
    const clamped = clamp(next, FILES_H_MIN, maxH);
    filesHeightPx = clamped;
    localStorage.setItem("forge_history_files_h", String(clamped));
  }

  function toggleFilesLayout() {
    filesLayout = filesLayout === "horizontal" ? "vertical" : "horizontal";
    localStorage.setItem("forge_history_files_layout", filesLayout);
  }

  function startDrag(
    kind: "detail" | "desc" | "files",
    axis: "x" | "y",
    e: PointerEvent,
    onMoveDelta: (delta: number) => void
  ) {
    e.preventDefault();
    const startPos = axis === "y" ? e.clientY : e.clientX;
    activeResizer = kind;

    const onMove = (ev: PointerEvent) => {
      const pos = axis === "y" ? ev.clientY : ev.clientX;
      onMoveDelta(pos - startPos);
    };
    const onUp = () => {
      activeResizer = null;
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
      window.removeEventListener("pointercancel", onUp);
    };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
    window.addEventListener("pointercancel", onUp);
  }

  function startDetailResize(e: PointerEvent) {
    const shellH = historyShellEl?.clientHeight || 500;
    const startPct = detailHeightPct;
    startDrag("detail", "y", e, (dy) => {
      updateDetailPct(startPct - (dy / shellH) * 100);
    });
  }

  function handleDetailResizerKeydown(e: KeyboardEvent) {
    const step = e.shiftKey ? 8 : 4;
    if (e.key === "ArrowUp") {
      e.preventDefault();
      updateDetailPct(detailHeightPct + step);
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      updateDetailPct(detailHeightPct - step);
    }
  }

  function startDescResize(e: PointerEvent) {
    const startH = detailHeaderEl?.getBoundingClientRect().height ?? 110;
    startDrag("desc", "y", e, (dy) => {
      updateDescHeaderHeight(startH + dy);
    });
  }

  function handleDescResizerKeydown(e: KeyboardEvent) {
    const current = descHeaderHeight ?? (detailHeaderEl?.getBoundingClientRect().height ?? 110);
    const step = e.shiftKey ? 28 : 14;
    if (e.key === "ArrowDown") {
      e.preventDefault();
      updateDescHeaderHeight(current + step);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      updateDescHeaderHeight(current - step);
    }
  }

  function startFilesResize(e: PointerEvent) {
    if (isFilesStacked) {
      const startH = filesHeightPx;
      startDrag("files", "y", e, (dy) => {
        updateFilesHeight(startH + dy);
      });
    } else {
      const startW = filesWidthPx;
      startDrag("files", "x", e, (dx) => {
        updateFilesWidth(startW + dx);
      });
    }
  }

  function handleFilesResizerKeydown(e: KeyboardEvent) {
    const step = e.shiftKey ? 24 : 12;
    if (isFilesStacked) {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        updateFilesHeight(filesHeightPx + step);
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        updateFilesHeight(filesHeightPx - step);
      }
    } else {
      if (e.key === "ArrowRight") {
        e.preventDefault();
        updateFilesWidth(filesWidthPx + step);
      } else if (e.key === "ArrowLeft") {
        e.preventDefault();
        updateFilesWidth(filesWidthPx - step);
      }
    }
  }
  let selectedTag = $state<string | null>(null);
  let tagFilterOpen = $state(false);
  let tagSearchQuery = $state("");

  let filteredTags = $derived.by(() => {
    const q = tagSearchQuery.trim().toLowerCase();
    if (!q) return tags;
    return tags.filter((t) => t.toLowerCase().includes(q));
  });

  let indexedCommits = $derived(
    commits
      .map((commit, index) => ({ commit, index }))
      .filter(({ commit }) => {
        if (selectedTag) {
          const hasTag =
            (commit.tags?.includes(selectedTag) ?? false) ||
            (commit.refs?.some((r) => r.kind === "tag" && r.name === selectedTag) ?? false);
          if (!hasTag) return false;
        }
        const q = searchQuery.trim().toLowerCase();
        if (!q) return true;
        return (
          commit.summary.toLowerCase().includes(q) ||
          commit.author.toLowerCase().includes(q) ||
          commit.short_sha.toLowerCase().includes(q) ||
          commit.sha.toLowerCase().includes(q) ||
          (commit.tags?.some((tag) => tag.toLowerCase().includes(q)) ?? false) ||
          (commit.refs?.some((r) => r.name.toLowerCase().includes(q)) ?? false)
        );
      })
  );

  let failedAvatars = $state<Record<string, boolean>>({});

  const LANE_W = 16;

  interface ColumnDef {
    id: HistoryColumnId;
    labelKey: string;
    width: string;
    canHide: boolean;
  }

  const ALL_COLUMNS: ColumnDef[] = [
    { id: "graph", labelKey: "history.colGraph", width: "var(--graph-col-w, 54px)", canHide: true },
    { id: "commit", labelKey: "history.colCommit", width: "minmax(200px, 1fr)", canHide: false },
    { id: "changes", labelKey: "history.colChanges", width: "105px", canHide: true },
    { id: "author", labelKey: "history.colAuthor", width: "110px", canHide: true },
    { id: "date", labelKey: "history.colDateTime", width: "125px", canHide: true },
    { id: "sha", labelKey: "history.colSha", width: "85px", canHide: true },
  ];

  const DEFAULT_ORDER: HistoryColumnId[] = ["graph", "commit", "changes", "author", "date", "sha"];
  const DEFAULT_VISIBILITY: Record<HistoryColumnId, boolean> = {
    graph: true,
    commit: true,
    changes: true,
    author: true,
    date: true,
    sha: true,
  };

  function loadSavedOrder(): HistoryColumnId[] {
    try {
      const raw = localStorage.getItem("forge_history_column_order");
      if (raw) {
        const parsed = JSON.parse(raw);
        if (Array.isArray(parsed)) {
          const valid = parsed.filter((id): id is HistoryColumnId => DEFAULT_ORDER.includes(id as HistoryColumnId));
          for (const id of DEFAULT_ORDER) {
            if (!valid.includes(id)) valid.push(id);
          }
          return valid;
        }
      }
    } catch {}
    return [...DEFAULT_ORDER];
  }

  function loadSavedVisibility(): Record<HistoryColumnId, boolean> {
    try {
      const raw = localStorage.getItem("forge_history_column_visibility");
      if (raw) {
        const parsed = JSON.parse(raw);
        if (parsed && typeof parsed === "object") {
          return {
            ...DEFAULT_VISIBILITY,
            ...parsed,
            commit: true,
          };
        }
      }
    } catch {}
    return { ...DEFAULT_VISIBILITY };
  }

  let columnOrder = $state<HistoryColumnId[]>(loadSavedOrder());
  let columnVisibility = $state<Record<HistoryColumnId, boolean>>(loadSavedVisibility());
  let rowDensity = $state<"comfortable" | "compact">(
    (typeof localStorage !== "undefined" && (localStorage.getItem("forge_history_density") as "comfortable" | "compact")) || "comfortable"
  );
  let dateFormat = $state<"relative" | "absolute">(
    (typeof localStorage !== "undefined" && (localStorage.getItem("forge_history_date_format") as "relative" | "absolute")) || "relative"
  );

  let isColumnSettingsOpen = $state(false);
  let columnSettingsEl = $state<HTMLDivElement | null>(null);

  let draggedColId = $state<HistoryColumnId | null>(null);
  let dragOverColId = $state<HistoryColumnId | null>(null);
  let dropPosition = $state<"before" | "after">("before");
  let isDraggingCol = $state(false);
  let headerElements: Record<string, HTMLElement> = {};
  const visibleColumns = $derived(
    columnOrder.filter((id) => columnVisibility[id] !== false)
  );

  const gridTemplate = $derived.by(() => {
    const colWidths = visibleColumns.map((id) => {
      const col = ALL_COLUMNS.find((c) => c.id === id);
      return col ? col.width : "100px";
    });
    return `${colWidths.join(" ")} 28px`;
  });

  const rowHeight = $derived(rowDensity === "compact" ? 28 : 34);

  function saveColumnOrder(newOrder: HistoryColumnId[]) {
    columnOrder = newOrder;
    localStorage.setItem("forge_history_column_order", JSON.stringify(newOrder));
  }

  function toggleColumnVisibility(colId: HistoryColumnId) {
    if (colId === "commit") return;
    columnVisibility = {
      ...columnVisibility,
      [colId]: !columnVisibility[colId],
    };
    localStorage.setItem("forge_history_column_visibility", JSON.stringify(columnVisibility));
  }

  function setRowDensity(density: "comfortable" | "compact") {
    rowDensity = density;
    localStorage.setItem("forge_history_density", density);
  }

  function setDateFormat(format: "relative" | "absolute") {
    dateFormat = format;
    localStorage.setItem("forge_history_date_format", format);
  }

  function resetColumnLayout() {
    columnOrder = [...DEFAULT_ORDER];
    columnVisibility = { ...DEFAULT_VISIBILITY };
    rowDensity = "comfortable";
    dateFormat = "relative";
    localStorage.removeItem("forge_history_column_order");
    localStorage.removeItem("forge_history_column_visibility");
    localStorage.removeItem("forge_history_density");
    localStorage.removeItem("forge_history_date_format");
  }

  function moveColumn(colId: HistoryColumnId, delta: -1 | 1) {
    const visibleIndex = visibleColumns.indexOf(colId);
    if (visibleIndex < 0) return;
    const targetVisibleIndex = visibleIndex + delta;
    if (targetVisibleIndex < 0 || targetVisibleIndex >= visibleColumns.length) return;
    const targetColId = visibleColumns[targetVisibleIndex];
    const next = [...columnOrder];
    const fromIdx = next.indexOf(colId);
    next.splice(fromIdx, 1);
    next.splice(next.indexOf(targetColId), 0, colId);
    saveColumnOrder(next);
  }

  function handleHeaderPointerDown(e: PointerEvent, colId: HistoryColumnId) {
    if (e.button !== 0) return;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    e.preventDefault();
    const startX = e.clientX;
    draggedColId = colId;
    isDraggingCol = false;

    function onPointerMove(moveEvent: PointerEvent) {
      if (!draggedColId) return;
      const deltaX = Math.abs(moveEvent.clientX - startX);
      if (!isDraggingCol && deltaX > 3) isDraggingCol = true;
      if (!isDraggingCol) return;
      const cols = visibleColumns;
      if (cols.length === 0) return;
      let foundCol: HistoryColumnId | null = null;
      let foundPos: "before" | "after" = "before";
      const firstEl = headerElements[cols[0]];
      const lastEl = headerElements[cols[cols.length - 1]];
      if (firstEl && moveEvent.clientX < firstEl.getBoundingClientRect().left) {
        foundCol = cols[0];
      } else if (lastEl && moveEvent.clientX > lastEl.getBoundingClientRect().right) {
        foundCol = cols[cols.length - 1];
        foundPos = "after";
      } else {
        for (const id of cols) {
          const el = headerElements[id];
          if (!el) continue;
          const rect = el.getBoundingClientRect();
          if (moveEvent.clientX >= rect.left && moveEvent.clientX <= rect.right) {
            foundCol = id;
            foundPos = moveEvent.clientX < rect.left + rect.width / 2 ? "before" : "after";
            break;
          }
        }
      }
      dragOverColId = foundCol && foundCol !== draggedColId ? foundCol : null;
      dropPosition = foundPos;
    }

    function onPointerUp() {
      window.removeEventListener("pointermove", onPointerMove);
      window.removeEventListener("pointerup", onPointerUp);
      window.removeEventListener("pointercancel", onPointerUp);
      if (isDraggingCol && draggedColId && dragOverColId && draggedColId !== dragOverColId) {
        const next = [...columnOrder];
        const fromIdx = next.indexOf(draggedColId);
        next.splice(fromIdx, 1);
        const toIdx = next.indexOf(dragOverColId);
        next.splice(dropPosition === "before" ? toIdx : toIdx + 1, 0, draggedColId);
        saveColumnOrder(next);
      }
      draggedColId = null;
      dragOverColId = null;
      isDraggingCol = false;
    }
    window.addEventListener("pointermove", onPointerMove);
    window.addEventListener("pointerup", onPointerUp);
    window.addEventListener("pointercancel", onPointerUp);
  }


  function handleHeaderKeyDown(e: KeyboardEvent, colId: HistoryColumnId) {
    if (e.key === "ArrowLeft") {
      e.preventDefault();
      moveColumn(colId, -1);
    } else if (e.key === "ArrowRight") {
      e.preventDefault();
      moveColumn(colId, 1);
    }
  }

  function formatCommitDate(rawDate: string, format: "relative" | "absolute"): string {
    if (format === "absolute" || !rawDate) return rawDate;
    try {
      const cleanDate = rawDate.includes("T") ? rawDate : rawDate.replace(" ", "T");
      const parsed = Date.parse(cleanDate);
      if (isNaN(parsed)) return rawDate;
      const diffSec = Math.round((parsed - Date.now()) / 1000);
      const absSec = Math.abs(diffSec);
      const lang = getLocale();
      if (absSec < 45) return t("history.justNow");
      const rtf = new Intl.RelativeTimeFormat(lang === "en" ? "en-US" : "id-ID", { numeric: "auto" });
      if (absSec < 3600) return rtf.format(Math.round(diffSec / 60), "minute");
      if (absSec < 86400) return rtf.format(Math.round(diffSec / 3600), "hour");
      if (absSec < 2592000) return rtf.format(Math.round(diffSec / 86400), "day");
      if (absSec < 31536000) return rtf.format(Math.round(diffSec / 2592000), "month");
      return rtf.format(Math.round(diffSec / 31536000), "year");
    } catch {
      return rawDate;
    }
  }

  function getColumnTitle(colId: HistoryColumnId): string {
    switch (colId) {
      case "graph": return t("history.colGraph");
      case "commit": return t("history.colCommit");
      case "changes": return t("history.colChanges");
      case "author": return t("history.colAuthor");
      case "date": return t("history.colDateTime");
      case "sha": return t("history.colSha");
    }
  }

  $effect(() => {
    if (!isColumnSettingsOpen) return;
    requestAnimationFrame(() => {
      const firstFocusable = columnSettingsEl?.querySelector<HTMLElement>(
        '.history-col-settings-menu input:not([disabled]), .history-col-settings-menu button:not([disabled])'
      );
      firstFocusable?.focus();
    });

    function handleDown(e: MouseEvent) {
      if (columnSettingsEl && !columnSettingsEl.contains(e.target as Node)) {
        isColumnSettingsOpen = false;
      }
    }
    function handleKey(e: KeyboardEvent) {
      if (e.key === "Escape") {
        isColumnSettingsOpen = false;
        const toggleBtn = columnSettingsEl?.querySelector<HTMLButtonElement>(".history-col-settings-btn");
        toggleBtn?.focus();
        return;
      }

      if (!columnSettingsEl) return;
      const focusables = Array.from(
        columnSettingsEl.querySelectorAll<HTMLElement>(
          'button:not([disabled]), input:not([disabled]), [tabindex="0"]'
        )
      ).filter((el) => !el.classList.contains("history-col-settings-btn"));

      if (focusables.length === 0) return;
      const activeIdx = focusables.indexOf(document.activeElement as HTMLElement);

      if (e.key === "ArrowDown") {
        e.preventDefault();
        const nextIdx = activeIdx < 0 || activeIdx >= focusables.length - 1 ? 0 : activeIdx + 1;
        focusables[nextIdx]?.focus();
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        const prevIdx = activeIdx <= 0 ? focusables.length - 1 : activeIdx - 1;
        focusables[prevIdx]?.focus();
      } else if (e.key === "Tab") {
        if (e.shiftKey) {
          if (activeIdx <= 0) {
            e.preventDefault();
            focusables[focusables.length - 1]?.focus();
          }
        } else {
          if (activeIdx >= focusables.length - 1) {
            e.preventDefault();
            focusables[0]?.focus();
          }
        }
      }
    }
    window.addEventListener("mousedown", handleDown);
    window.addEventListener("keydown", handleKey);
    return () => {
      window.removeEventListener("mousedown", handleDown);
      window.removeEventListener("keydown", handleKey);
    };
  });
  let branchFilterOpen = $state(false);
  let branchSearchQuery = $state("");

  let availableBranches = $derived.by(() => {
    if (branches && branches.length > 0) return branches;
    const seen = new Set<string>();
    const list: BranchInfo[] = [];
    for (const c of commits) {
      for (const r of c.refs || []) {
        if ((r.kind === "branch" || r.kind === "remote") && !seen.has(r.name)) {
          seen.add(r.name);
          list.push({
            name: r.name,
            is_current: r.is_current,
            is_remote: r.kind === "remote",
            is_pushed: r.kind !== "remote",
            upstream: null,
            ahead: 0,
            behind: 0,
            upstream_gone: false,
          });
        }
      }
    }
    return list;
  });

  let filteredBranchesList = $derived.by(() => {
    const q = branchSearchQuery.trim().toLowerCase();
    if (!q) return availableBranches;
    return availableBranches.filter((b) => b.name.toLowerCase().includes(q));
  });

  function applyFilterMode(mode: "all" | "current" | "clear") {
    if (mode === "all") {
      onFilterBranches?.({ all: true, selected: [] });
    } else if (mode === "current") {
      const active = currentBranch || availableBranches.find((b) => b.is_current)?.name || "main";
      onFilterBranches?.({ all: false, selected: [active] });
    } else if (mode === "clear") {
      onFilterBranches?.({ all: true, selected: [] });
    }
  }

  function toggleBranchSelection(branchName: string) {
    let next: string[];
    if (selectedBranches.includes(branchName)) {
      next = selectedBranches.filter((b) => b !== branchName);
    } else {
      next = [...selectedBranches, branchName];
    }
    if (next.length === 0) {
      onFilterBranches?.({ all: true, selected: [] });
    } else {
      onFilterBranches?.({ all: false, selected: next });
    }
  }

  function getCommitAvatarUrl(commit: CommitInfo): string {
    if (accounts && accounts.length > 0) {
      const aLower = commit.author.trim().toLowerCase();
      const eLower = (commit.author_email || "").trim().toLowerCase();
      const match = accounts.find((acc) => {
        const accEmail = (acc.email || "").toLowerCase();
        const accLogin = acc.login.toLowerCase();
        const accName = (acc.name || "").toLowerCase();
        return (
          (accEmail && eLower === accEmail) ||
          (accLogin && (aLower === accLogin || eLower.includes(accLogin))) ||
          (accName && aLower === accName)
        );
      });
      if (match?.avatar_url) return match.avatar_url;
    }
    if (commit.avatar_url) return commit.avatar_url;
    const trimmed = commit.author.trim();
    if (trimmed && !trimmed.includes(" ")) {
      return `https://github.com/${trimmed}.png?size=48`;
    }
    return "";
  }


  let graphLayouts = $derived(
    computeCommitGraph(indexedCommits.map((x) => x.commit))
  );
  let maxGraphCols = $derived(
    Math.min(10, Math.max(1, ...graphLayouts.map((g) => g.maxLanes)))
  );
  let graphColWidth = $derived(Math.max(54, maxGraphCols * LANE_W + 16));
  let selectedCommitLayout = $derived(
    selectedSha ? graphLayouts[indexedCommits.findIndex((c) => c.commit.sha === selectedSha)] : null
  );
  interface DragCommitPayload {
    sha: string;
    shortSha: string;
    branchName: string | null;
    label: string;
  }

  let dragSource = $state<DragCommitPayload | null>(null);
  let dropOverSha = $state<string | null>(null);
  let dropActionPrompt = $state<{
    source: DragCommitPayload;
    target: CommitInfo;
    targetBranch: string | null;
  } | null>(null);

  function handleCommitDragStart(e: DragEvent, commit: CommitInfo, explicitBranch?: string | null) {
    const branchRef =
      explicitBranch ??
      commit.refs?.find((r) => r.kind === "branch" || r.kind === "remote")?.name ??
      null;
    dragSource = {
      sha: commit.sha,
      shortSha: commit.short_sha,
      branchName: branchRef,
      label: branchRef || commit.short_sha,
    };
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = "move";
      e.dataTransfer.setData("text/plain", commit.sha);
    }
  }

  function handleCommitDragOver(e: DragEvent, targetSha: string) {
    if (!dragSource || dragSource.sha === targetSha) return;
    e.preventDefault();
    dropOverSha = targetSha;
  }

  function handleCommitDrop(e: DragEvent, targetCommit: CommitInfo) {
    e.preventDefault();
    dropOverSha = null;
    if (!dragSource || dragSource.sha === targetCommit.sha) {
      dragSource = null;
      return;
    }
    const targetBranch =
      targetCommit.refs?.find((r) => r.kind === "branch" || r.kind === "remote")?.name ?? null;
    dropActionPrompt = {
      source: dragSource,
      target: targetCommit,
      targetBranch,
    };
    dragSource = null;
  }

  let selectedCommitSummary = $derived(
    commits.find((c) => c.sha === selectedSha) ?? null
  );

  function clearSelection() {
    detailRequestSeq++;
    diffRequestSeq++;
    selectedSha = null;
    selectedDetail = null;
    detailLoading = false;
    detailError = "";
    selectedCommitFile = null;
    commitFileDiff = "";
    diffLoading = false;
  }

  $effect(() => {
    if (repoPath !== lastRepoPath) {
      lastRepoPath = repoPath;
      if (selectedSha) clearSelection();
      return;
    }
    if (selectedSha && !indexedCommits.some((c) => c.commit.sha === selectedSha)) {
      clearSelection();
    }
  });

  async function loadFileDiff(
    sha: string,
    file: CommitChangedFile,
    ignoreWs = commitIgnoreWhitespace
  ) {
    if (!repoPath) return;
    const reqId = ++diffRequestSeq;
    selectedCommitFile = file;
    diffLoading = true;
    try {
      const diff = await gitService.getCommitFileDiff(
        repoPath,
        sha,
        file.path,
        file.old_path,
        ignoreWs
      );
      if (reqId !== diffRequestSeq) return;
      commitFileDiff = diff;
    } catch (err) {
      if (reqId !== diffRequestSeq) return;
      commitFileDiff = "";
      onStatus?.(String(err), true);
    } finally {
      if (reqId === diffRequestSeq) {
        diffLoading = false;
      }
    }
  }

  async function handleSelectCommit(sha: string) {
    if (selectedSha === sha) {
      clearSelection();
      return;
    }
    selectedSha = sha;
    if (!repoPath) return;

    const reqId = ++detailRequestSeq;
    diffRequestSeq++;
    detailLoading = true;
    detailError = "";
    selectedDetail = null;
    selectedCommitFile = null;
    commitFileDiff = "";
    diffLoading = false;

    try {
      const detail = await gitService.getCommitDetail(repoPath, sha);
      if (reqId !== detailRequestSeq) return;
      selectedDetail = detail;
      if (detail.files.length > 0) {
        await loadFileDiff(detail.sha, detail.files[0]);
      }
    } catch (err) {
      if (reqId !== detailRequestSeq) return;
      detailError = String(err);
      onStatus?.(t("history.loadDetailFailed", { err: String(err) }), true);
    } finally {
      if (reqId === detailRequestSeq) {
        detailLoading = false;
      }
    }
  }

  async function handleToggleWhitespace() {
    commitIgnoreWhitespace = !commitIgnoreWhitespace;
    if (selectedSha && selectedCommitFile) {
      await loadFileDiff(selectedSha, selectedCommitFile, commitIgnoreWhitespace);
    }
  }

  function openCommitContextMenu(e: MouseEvent, commit: CommitInfo, index: number) {
    const clickedTag =
      e.target instanceof HTMLElement
        ? e.target.closest<HTMLElement>(".commit-tag-badge")?.dataset.tag ?? null
        : null;
    const items: ContextMenuItem[] = [];

    if (clickedTag) {
      items.push({
        label: t("ctx.tag.copyName"),
        onSelect: () => {
          void copyTextToClipboard(
            clickedTag,
            onStatus,
            t("ctx.status.copiedTag", { tag: clickedTag })
          );
        },
      });
      if (onDeleteTag) {
        items.push({
          label: t("ctx.tag.deleteTag", { tag: clickedTag }),
          danger: true,
          onSelect: () => onDeleteTag(clickedTag),
        });
      }
      items.push({ type: "divider" });
    }

    if (onCherryPick) {
      items.push({
        label: t("ctx.history.cherryPick"),
        onSelect: () => onCherryPick(commit.sha),
      });
    }
    if (onRevert) {
      items.push({
        label: t("ctx.history.revertCommit"),
        onSelect: () => onRevert(commit.sha),
      });
    }
    if (onSquash && index > 0 && !searchQuery.trim()) {
      items.push({
        label: t("ctx.history.squashCommits", { count: index + 1 }),
        onSelect: () => onSquash(index, commit.sha),
      });
    }
    if (onTag) {
      items.push({
        label: t("ctx.history.createTag"),
        onSelect: () => onTag(commit.sha),
      });
    }
    if (onReset) {
      items.push({
        label: t("ctx.history.resetHere"),
        danger: true,
        onSelect: () => onReset(commit.sha),
      });
    }

    items.push({ type: "divider" });
    items.push({
      label: t("ctx.history.copySha"),
      onSelect: () => {
        void copyTextToClipboard(
          commit.sha,
          onStatus,
          t("ctx.status.copiedSha", { sha: commit.short_sha })
        );
      },
    });
    items.push({
      label: t("ctx.history.copyMessage"),
      onSelect: () => {
        void copyTextToClipboard(commit.summary, onStatus, t("ctx.status.copiedCommitMsg"));
      },
    });
    const effectiveWebUrl = remoteWebUrl || githubRepoUrl;
    if (effectiveWebUrl) {
      let commitUrl = `${effectiveWebUrl}/commit/${commit.sha}`;
      if (effectiveWebUrl.includes("gitlab")) {
        commitUrl = `${effectiveWebUrl}/-/commit/${commit.sha}`;
      } else if (effectiveWebUrl.includes("bitbucket")) {
        commitUrl = `${effectiveWebUrl}/commits/${commit.sha}`;
      }
      items.push({
        label: t("ctx.history.viewOnGithub"),
        onSelect: () => {
          void openExternalUrl(commitUrl);
        },
      });
    }

    openContextMenu(e, items);
  }

  function openCommitFileContextMenu(e: MouseEvent, file: CommitChangedFile) {
    const isDeleted = file.status_code.trim().startsWith("D");
    const fullPath = repoPath ? joinRepoPath(repoPath, file.path) : file.path;
    const items: ContextMenuItem[] = [];

    if (repoPath) {
      items.push({
        label: t("ctx.common.copyFilePath"),
        onSelect: () => {
          void copyTextToClipboard(fullPath, onStatus, t("ctx.status.copiedPath"));
        },
      });
    }
    items.push({
      label: t("ctx.common.copyRelativePath"),
      onSelect: () => {
        void copyTextToClipboard(file.path, onStatus, t("ctx.status.copiedPath"));
      },
    });

    if (onInspectFile) {
      items.push({ type: "divider" });
      items.push({
        label: t("ctx.common.fileBlame"),
        disabled: isDeleted,
        onSelect: () => onInspectFile(file.path, "blame"),
      });
      items.push({
        label: t("ctx.common.fileHistory"),
        onSelect: () => onInspectFile(file.path, "history"),
      });
    }

    if (onRevealFile || onOpenInEditor) {
      items.push({ type: "divider" });
      if (onRevealFile) {
        items.push({
          label: t("ctx.common.revealInFileManager"),
          onSelect: () => onRevealFile(file.path),
        });
      }
      if (onOpenInEditor) {
        items.push({
          label: t("ctx.common.openInEditor", { editor: externalEditorLabel }),
          disabled: isDeleted,
          onSelect: () => onOpenInEditor(file.path),
        });
      }
    }

    openContextMenu(e, items);
  }
</script>

{#if commits.length === 0}
  <div class="empty-state">
    <span>{t("history.empty")}</span>
  </div>
{:else}
  <div
    bind:this={historyShellEl}
    class="history-shell {activeResizer ? 'is-resizing-any' : ''}"
    style="--detail-pane-pct: {detailHeightPct}%; --commit-files-w: {filesWidthPx}px; --commit-files-h: {filesHeightPx}px;"
  >
    <div class="history-toolbar">
      <div class="history-toolbar-left">
      <div class="history-branch-filter-wrap">
        <button
          type="button"
          class="branch-filter-trigger-btn {selectedBranches.length > 0 || !allBranches ? 'is-filtered' : ''}"
          onclick={() => (branchFilterOpen = !branchFilterOpen)}
          aria-expanded={branchFilterOpen}
          aria-haspopup="dialog"
          title={t("history.toggleAllBranchesTitle")}
        >
          <svg class="branch-filter-icon" width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <line x1="6" y1="3" x2="6" y2="13" />
            <circle cx="6" cy="3" r="2" />
            <circle cx="6" cy="13" r="2" />
            <circle cx="12" cy="6" r="2" />
            <path d="M6 9a3 3 0 0 0 3-3h3" />
          </svg>
          <span class="branch-filter-label">
            {#if selectedBranches.length > 0}
              {t("history.branchFilterSelectedCount", { count: selectedBranches.length })}
            {:else if allBranches}
              {t("history.allBranchesShort")}
            {:else}
              {currentBranch ? t("history.branchFilterActive", { name: currentBranch }) : t("history.currentBranchShort")}
            {/if}
          </span>
          <svg class="branch-filter-chevron {branchFilterOpen ? 'is-open' : ''}" width="10" height="10" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <polyline points="4 6 8 10 12 6" />
          </svg>
        </button>

        {#if branchFilterOpen}
          <div
            class="branch-filter-backdrop"
            onclick={() => (branchFilterOpen = false)}
            aria-hidden="true"
          ></div>

          <div
            class="branch-filter-popover"
            role="dialog"
            aria-label={t("history.branchFilterBtn")}
            tabindex="-1"
            onkeydown={(e) => {
              if (e.key === "Escape") {
                branchFilterOpen = false;
              }
            }}
          >
            <div class="branch-filter-search-box">
              <svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                <circle cx="7" cy="7" r="4.5" />
                <line x1="10.5" y1="10.5" x2="14" y2="14" />
              </svg>
              <input
                type="text"
                class="branch-filter-search-input"
                placeholder={t("history.branchFilterSearch")}
                bind:value={branchSearchQuery}
              />
              {#if branchSearchQuery}
                <button
                  type="button"
                  class="branch-search-clear"
                  onclick={() => (branchSearchQuery = "")}
                  title="Clear search"
                  aria-label="Clear search"
                >
                  ✕
                </button>
              {/if}
            </div>

            <div class="branch-filter-quick-bar">
              <button
                type="button"
                class="quick-pill {allBranches && selectedBranches.length === 0 ? 'selected' : ''}"
                onclick={() => applyFilterMode("all")}
              >
                {t("history.branchFilterAll")}
              </button>
              <button
                type="button"
                class="quick-pill {!allBranches && selectedBranches.length === 0 ? 'selected' : ''}"
                onclick={() => applyFilterMode("current")}
              >
                {t("history.branchFilterActiveOnly")}
              </button>
              {#if selectedBranches.length > 0}
                <button
                  type="button"
                  class="quick-pill btn-clear"
                  onclick={() => applyFilterMode("clear")}
                >
                  {t("history.branchFilterClear")}
                </button>
              {/if}
            </div>

            <div class="branch-filter-list" role="group">
              {#if filteredBranchesList.length === 0}
                <div class="branch-filter-empty">
                  {t("history.branchFilterNoMatch")}
                </div>
              {:else}
                {#each filteredBranchesList as b (b.name)}
                  {@const isChecked = selectedBranches.includes(b.name)}
                  {@const isCurrent = b.name === currentBranch || b.is_current}
                  <label class="branch-filter-item {isChecked ? 'is-checked' : ''} {isCurrent ? 'is-current' : ''}">
                    <input
                      type="checkbox"
                      checked={isChecked}
                      onchange={() => toggleBranchSelection(b.name)}
                      class="branch-checkbox"
                    />
                    <svg class="item-branch-icon" width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                      {#if b.is_remote}
                        <path d="M4.5 12.75h7.1a2.65 2.65 0 0 0 .45-5.26A3.85 3.85 0 0 0 4.55 6.4 3.1 3.1 0 0 0 4.5 12.75Z" />
                      {:else}
                        <line x1="6" y1="3" x2="6" y2="13" />
                        <circle cx="6" cy="3" r="2" />
                        <circle cx="6" cy="13" r="2" />
                        <circle cx="12" cy="6" r="2" />
                        <path d="M6 9a3 3 0 0 0 3-3h3" />
                      {/if}
                    </svg>
                    <span class="item-branch-name" title={b.name}>
                      {b.name}
                    </span>
                    {#if isCurrent}
                      <span class="branch-pill-badge is-active">aktif</span>
                    {/if}
                    {#if b.is_remote}
                      <span class="branch-pill-badge is-remote">remote</span>
                    {/if}
                  </label>
                {/each}
              {/if}
            </div>

            <div class="branch-filter-footer">
              <span class="footer-count">
                {#if selectedBranches.length > 0}
                  {t("history.branchFilterSelectedCount", { count: selectedBranches.length })}
                {:else if allBranches}
                  {t("history.branchFilterAll")}
                {:else}
                  {currentBranch ? t("history.branchFilterActive", { name: currentBranch }) : t("history.branchFilterActiveOnly")}
                {/if}
              </span>
              <button
                type="button"
                class="btn-primary btn-xs"
                onclick={() => (branchFilterOpen = false)}
              >
                {t("history.branchFilterDone")}
              </button>
            </div>
          </div>
        {/if}
      </div>

        <!-- Tag filter dropdown & management -->
        <div class="history-tag-filter-wrap">
          <button
            type="button"
            class="tag-filter-trigger-btn {selectedTag ? 'is-filtered' : ''}"
            onclick={() => (tagFilterOpen = !tagFilterOpen)}
            aria-expanded={tagFilterOpen}
            aria-haspopup="dialog"
            title="Filter Tag"
          >
            <svg class="tag-filter-icon" width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <path d="M2.5 8.5L8.5 2.5H13.5V7.5L7.5 13.5L2.5 8.5Z" />
              <circle cx="10.5" cy="5.5" r="1.5" fill="currentColor" />
            </svg>
            <span class="tag-filter-label">
              {#if selectedTag}
                {selectedTag}
              {:else}
                {t("app.historySidebar.tagsHeader", { count: tags.length })}
              {/if}
            </span>
            <svg class="tag-filter-chevron {tagFilterOpen ? 'is-open' : ''}" width="10" height="10" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <polyline points="4 6 8 10 12 6" />
            </svg>
          </button>

          {#if tagFilterOpen}
            <div
              class="tag-filter-backdrop"
              onclick={() => (tagFilterOpen = false)}
              aria-hidden="true"
            ></div>

            <div
              class="tag-filter-popover"
              role="dialog"
              aria-label="Filter Tag"
              tabindex="-1"
              onkeydown={(e) => {
                if (e.key === "Escape") tagFilterOpen = false;
              }}
            >
              <div class="tag-filter-header">
                <span class="tag-filter-title tabular-nums">
                  {t("app.historySidebar.tagsHeader", { count: tags.length })}
                </span>
                {#if onCreateTag}
                  <button
                    type="button"
                    class="btn-primary btn-xs"
                    onclick={() => {
                      tagFilterOpen = false;
                      onCreateTag();
                    }}
                  >
                    + {t("app.historySidebar.createTagBtn")}
                  </button>
                {/if}
              </div>

              {#if tags.length > 5}
                <div class="tag-filter-search-box">
                  <input
                    type="text"
                    class="tag-filter-search-input"
                    placeholder="Cari tag..."
                    bind:value={tagSearchQuery}
                  />
                </div>
              {/if}

              {#if selectedTag}
                <div class="tag-filter-clear-bar">
                  <button
                    type="button"
                    class="quick-pill btn-clear"
                    onclick={() => (selectedTag = null)}
                  >
                    Hapus filter tag
                  </button>
                </div>
              {/if}

              <div class="tag-filter-list">
                {#if filteredTags.length === 0}
                  <div class="tag-filter-empty">
                    {t("app.historySidebar.noTags")}
                  </div>
                {:else}
                  {#each filteredTags as tagName (tagName)}
                    {@const isCurrentTag = selectedTag === tagName}
                    <div class="tag-filter-item {isCurrentTag ? 'is-selected' : ''}">
                      <button
                        type="button"
                        class="tag-item-btn"
                        onclick={() => {
                          selectedTag = isCurrentTag ? null : tagName;
                          tagFilterOpen = false;
                        }}
                        oncontextmenu={(e) => onTagContextMenu?.(e, tagName)}
                        title={tagName}
                      >
                        <svg class="tag-item-icon" width="11" height="11" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                          <path d="M2.5 8.5L8.5 2.5H13.5V7.5L7.5 13.5L2.5 8.5Z" />
                          <circle cx="10.5" cy="5.5" r="1.5" fill="currentColor" />
                         </svg>
                        <span class="tag-item-name">{tagName}</span>
                      </button>
                      {#if onDeleteTag}
                        <button
                          type="button"
                          class="tag-item-del-btn"
                          onclick={() => onDeleteTag(tagName)}
                          title="Hapus tag {tagName}"
                          aria-label="Hapus tag {tagName}"
                        >
                          ✕
                        </button>
                      {/if}
                    </div>
                  {/each}
                {/if}
              </div>
            </div>
          {/if}
        </div>

        <!-- Search bar: Filter Riwayat -->
        <div class="history-toolbar-search">
          <svg class="search-icon" width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <circle cx="7" cy="7" r="4.5" />
            <line x1="10.5" y1="10.5" x2="14" y2="14" />
          </svg>
          <input
            type="search"
            class="history-search-input"
            placeholder={t("app.historySidebar.filterPlaceholder")}
            bind:value={searchQuery}
            oninput={(e) => onSearchChange?.((e.target as HTMLInputElement).value)}
          />
          {#if searchQuery}
            <button
              type="button"
              class="history-search-clear-btn"
              onclick={() => {
                searchQuery = "";
                onSearchChange?.("");
              }}
              title="Hapus filter"
              aria-label="Hapus filter"
            >
              ✕
            </button>
          {/if}
        </div>
      </div>

      <div class="history-toolbar-right">
        {#if onCreateTag}
          <button
            type="button"
            class="btn-secondary btn-xs"
            onclick={onCreateTag}
            title={t("app.historySidebar.createTagBtn")}
          >
            <svg width="11" height="11" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <line x1="8" y1="3" x2="8" y2="13" />
              <line x1="3" y1="8" x2="13" y2="8" />
            </svg>
            <span>{t("app.historySidebar.createTagBtn")}</span>
          </button>
        {/if}

        {#if onInteractiveRebase && commits.length > 1}
          <button
            type="button"
            class="btn-secondary btn-xs"
            onclick={onInteractiveRebase}
            title={t("app.historySidebar.interactiveRebase")}
          >
            <svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <circle cx="5" cy="4" r="2" />
              <circle cx="11" cy="4" r="2" />
              <circle cx="8" cy="12" r="2" />
              <path d="M5 6v2a3 3 0 0 0 3 3" />
              <path d="M11 6v2a3 3 0 0 1-3 3" />
            </svg>
            <span>{t("app.historySidebar.interactiveRebase")}</span>
          </button>
        {/if}

        <span class="history-toolbar-count tabular-nums">
          {t("app.historySidebar.title", { count: indexedCommits.length })}
        </span>
      </div>
    </div>
    <div class="history-container {selectedSha ? 'has-detail' : ''}">
    <div
      class="history-header-bar history-header-with-graph"
      style="--graph-col-w: {graphColWidth}px; --history-grid-template: {gridTemplate};"
    >
      {#each visibleColumns as colId (colId)}
        <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
        <div
          bind:this={headerElements[colId]}
          class="history-col-header history-{colId}-col-header {draggedColId === colId ? 'is-dragging' : ''} {dragOverColId === colId ? `drop-${dropPosition}` : ''}"
          role="columnheader"
          tabindex="0"
          title="{getColumnTitle(colId)} ({t('history.dragToReorder')})"
          onpointerdown={(e) => handleHeaderPointerDown(e, colId)}
          onkeydown={(e) => handleHeaderKeyDown(e, colId)}
        >
          <span class="header-col-drag-grip" aria-hidden="true">
            <svg width="6" height="10" viewBox="0 0 6 10" fill="currentColor">
              <circle cx="1.5" cy="2" r="1"/>
              <circle cx="4.5" cy="2" r="1"/>
              <circle cx="1.5" cy="5" r="1"/>
              <circle cx="4.5" cy="5" r="1"/>
              <circle cx="1.5" cy="8" r="1"/>
              <circle cx="4.5" cy="8" r="1"/>
            </svg>
          </span>
          <span class="header-col-title">{getColumnTitle(colId)}</span>
        </div>
      {/each}

      <div class="history-header-actions-col" bind:this={columnSettingsEl}>
        <button
          type="button"
          class="history-col-settings-btn {isColumnSettingsOpen ? 'is-active' : ''}"
          title={t("history.headerSettingsTooltip")}
          aria-label={t("history.headerSettingsTitle")}
          aria-expanded={isColumnSettingsOpen}
          onclick={() => (isColumnSettingsOpen = !isColumnSettingsOpen)}
        >
          <svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <circle cx="8" cy="8" r="2.5" />
            <path d="M13.8 10.3a1.4 1.4 0 0 0 .3 1.6l.1.1a1.2 1.2 0 1 1-1.7 1.7l-.1-.1a1.4 1.4 0 0 0-1.6-.3 1.4 1.4 0 0 0-.9 1.3v.2a1.2 1.2 0 1 1-2.4 0v-.2a1.4 1.4 0 0 0-.9-1.3 1.4 1.4 0 0 0-1.6.3l-.1.1a1.2 1.2 0 1 1-1.7-1.7l.1-.1a1.4 1.4 0 0 0 .3-1.6 1.4 1.4 0 0 0-1.3-.9H2.2a1.2 1.2 0 1 1 0-2.4h.2a1.4 1.4 0 0 0 1.3-.9 1.4 1.4 0 0 0-.3-1.6l-.1-.1a1.2 1.2 0 1 1 1.7-1.7l.1.1a1.4 1.4 0 0 0 1.6.3 1.4 1.4 0 0 0 .9-1.3V2.2a1.2 1.2 0 1 1 2.4 0v.2a1.4 1.4 0 0 0 .9 1.3 1.4 1.4 0 0 0 1.6-.3l.1-.1a1.2 1.2 0 1 1 1.7 1.7l-.1.1a1.4 1.4 0 0 0-.3 1.6 1.4 1.4 0 0 0 1.3.9h.2a1.2 1.2 0 1 1 0 2.4h-.2a1.4 1.4 0 0 0-1.3.9Z" />
          </svg>
        </button>

        {#if isColumnSettingsOpen}
          <div class="history-col-settings-menu" role="menu">
            <div class="settings-menu-header">{t("history.columnVisibility")}</div>
            <div class="settings-menu-list">
              {#each columnOrder as colId, idx (colId)}
                {@const isVisible = columnVisibility[colId] !== false}
                {@const isRequired = colId === "commit"}
                <div class="settings-menu-item">
                  <label class="settings-menu-label">
                    <input
                      type="checkbox"
                      class="settings-menu-checkbox"
                      checked={isVisible}
                      disabled={isRequired}
                      onchange={() => toggleColumnVisibility(colId)}
                    />
                    <span class="settings-menu-title" title={isRequired ? t("history.columnRequired") : undefined}>
                      {getColumnTitle(colId)}
                    </span>
                  </label>
                  <div class="settings-menu-item-actions">
                    <button
                      type="button"
                      class="btn-col-move"
                      disabled={idx === 0}
                      title={t("history.moveColumnLeft")}
                      aria-label="{t('history.moveColumnLeft')}: {getColumnTitle(colId)}"
                      onclick={() => moveColumn(colId, -1)}
                    >
                      <svg width="10" height="10" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <polyline points="10 13 5 8 10 3" />
                      </svg>
                    </button>
                    <button
                      type="button"
                      class="btn-col-move"
                      disabled={idx === columnOrder.length - 1}
                      title={t("history.moveColumnRight")}
                      aria-label="{t('history.moveColumnRight')}: {getColumnTitle(colId)}"
                      onclick={() => moveColumn(colId, 1)}
                    >
                      <svg width="10" height="10" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <polyline points="6 3 11 8 6 13" />
                      </svg>
                    </button>
                  </div>
                </div>
              {/each}
            </div>

            <div class="settings-menu-divider"></div>

            <div class="settings-menu-section">
              <div class="settings-menu-section-title">{t("history.rowDensity")}</div>
              <div class="segmented-group">
                <button
                  type="button"
                  class="segmented-btn {rowDensity === 'comfortable' ? 'is-active' : ''}"
                  onclick={() => setRowDensity('comfortable')}
                >
                  {t("history.densityComfortable")}
                </button>
                <button
                  type="button"
                  class="segmented-btn {rowDensity === 'compact' ? 'is-active' : ''}"
                  onclick={() => setRowDensity('compact')}
                >
                  {t("history.densityCompact")}
                </button>
              </div>
            </div>

            <div class="settings-menu-divider"></div>

            <div class="settings-menu-section">
              <div class="settings-menu-section-title">{t("history.dateFormat")}</div>
              <div class="segmented-group">
                <button
                  type="button"
                  class="segmented-btn {dateFormat === 'relative' ? 'is-active' : ''}"
                  onclick={() => setDateFormat('relative')}
                >
                  {t("history.dateRelative")}
                </button>
                <button
                  type="button"
                  class="segmented-btn {dateFormat === 'absolute' ? 'is-active' : ''}"
                  onclick={() => setDateFormat('absolute')}
                >
                  {t("history.dateAbsolute")}
                </button>
              </div>
            </div>

            <div class="settings-menu-divider"></div>

            <button
              type="button"
              class="settings-reset-btn"
              onclick={resetColumnLayout}
            >
              {t("history.resetColumns")}
            </button>
          </div>
        {/if}
      </div>
    </div>

    {#if dropActionPrompt}
      <div class="graph-drop-banner">
        <span class="graph-drop-title">
          {t("history.dropPromptTitle", {
            source: dropActionPrompt.source.label,
            target: dropActionPrompt.targetBranch || dropActionPrompt.target.short_sha,
          })}
        </span>
        <div class="graph-drop-actions">
          {#if onMergeBranch && dropActionPrompt.source.branchName}
            <button
              type="button"
              class="btn-primary btn-xs"
              onclick={() => {
                const b = dropActionPrompt!.source.branchName!;
                dropActionPrompt = null;
                onMergeBranch(b);
              }}
            >
              {t("history.dropMerge", { branch: dropActionPrompt.source.branchName })}
            </button>
          {/if}
          {#if onRebaseBranch}
            <button
              type="button"
              class="btn-secondary btn-xs"
              onclick={() => {
                const refName = dropActionPrompt!.targetBranch || dropActionPrompt!.target.sha;
                dropActionPrompt = null;
                onRebaseBranch(refName);
              }}
            >
              {t("history.dropRebase", { target: dropActionPrompt.targetBranch || dropActionPrompt.target.short_sha })}
            </button>
          {/if}
          {#if onCherryPick}
            <button
              type="button"
              class="btn-secondary btn-xs"
              onclick={() => {
                const sha = dropActionPrompt!.source.sha;
                dropActionPrompt = null;
                onCherryPick(sha);
              }}
            >
              {t("history.cherryPickBtn")} ({dropActionPrompt.source.shortSha})
            </button>
          {/if}
          {#if onReset}
            <button
              type="button"
              class="btn-secondary btn-xs btn-danger"
              onclick={() => {
                const sha = dropActionPrompt!.target.sha;
                dropActionPrompt = null;
                onReset(sha);
              }}
            >
              {t("history.resetHereBtn")}
            </button>
          {/if}
          <button type="button" class="btn-secondary btn-xs" onclick={() => (dropActionPrompt = null)}>
            {t("common.cancel")}
          </button>
        </div>
      </div>
    {/if}

    {#if indexedCommits.length === 0}
      <div class="empty-state">
        <span>{t("history.noSearchMatch", { query: searchQuery })}</span>
      </div>
    {:else}
      {#each indexedCommits as { commit, index }, rowIdx (commit.sha)}
        {@const isSelected = selectedSha === commit.sha}
        {@const layout = graphLayouts[rowIdx]}
        {@const nodeX = (layout?.lane ?? 0) * LANE_W + LANE_W / 2 + 4}
        {@const nodeY = rowHeight / 2}
        {@const nodeColor = GRAPH_COLORS[(layout?.colorIndex ?? 0) % GRAPH_COLORS.length]}
        {@const isHead = commit.refs?.some((r) => r.is_current) ?? false}
        {@const avatarUrl = getCommitAvatarUrl(commit)}
        <button
          type="button"
          draggable="true"
          class="commit-row commit-row-with-graph {rowDensity === 'compact' ? 'is-compact' : ''} {isSelected ? 'selected' : ''} {dropOverSha === commit.sha ? 'is-drop-target' : ''}"
          style="--graph-col-w: {graphColWidth}px; --history-grid-template: {gridTemplate}; min-height: {rowHeight}px; height: {rowHeight}px;"
          aria-expanded={isSelected}
          onclick={() => handleSelectCommit(commit.sha)}
          oncontextmenu={(e) => openCommitContextMenu(e, commit, index)}
          ondragstart={(e) => handleCommitDragStart(e, commit)}
          ondragover={(e) => handleCommitDragOver(e, commit.sha)}
          ondragleave={() => {
            if (dropOverSha === commit.sha) dropOverSha = null;
          }}
          ondrop={(e) => handleCommitDrop(e, commit)}
          ondragend={() => {
            dragSource = null;
            dropOverSha = null;
          }}
        >
          {#each visibleColumns as colId (colId)}
            {#if colId === "graph"}
              <span
                class="commit-graph-cell"
                aria-hidden="true"
                title={layout?.branchName ? `Cabang: ${layout.branchName}` : undefined}
              >
                <svg class="commit-graph-svg" width={graphColWidth} height={rowHeight} viewBox="0 0 {graphColWidth} {rowHeight}">
                  {#if layout}
                    {#each layout.incomingLanes as inLane}
                      {@const x = inLane * LANE_W + LANE_W / 2 + 4}
                      {@const c = GRAPH_COLORS[inLane % GRAPH_COLORS.length]}
                      <line x1={x} y1={0} x2={x} y2={nodeY} stroke={c} stroke-width="2" />
                    {/each}
                    {#each layout.segments as seg}
                      {@const x1 = seg.fromLane * LANE_W + LANE_W / 2 + 4}
                      {@const x2 = seg.toLane * LANE_W + LANE_W / 2 + 4}
                      {@const segColor = GRAPH_COLORS[seg.colorIndex % GRAPH_COLORS.length]}
                      {#if seg.fromLane === seg.toLane}
                        <line x1={x1} y1={nodeY} x2={x2} y2={rowHeight} stroke={segColor} stroke-width="2" />
                      {:else if seg.kind === "branch-in"}
                        <path
                          d="M {x1} 0 C {x1} {nodeY * 0.65}, {x2} {nodeY * 0.35}, {x2} {nodeY}"
                          fill="none"
                          stroke={segColor}
                          stroke-width="2"
                        />
                      {:else}
                        <path
                          d="M {x1} {nodeY} C {x1} {nodeY + (rowHeight - nodeY) * 0.55}, {x2} {nodeY + (rowHeight - nodeY) * 0.45}, {x2} {rowHeight}"
                          fill="none"
                          stroke={segColor}
                          stroke-width="2"
                        />
                      {/if}
                    {/each}
                  {/if}
                </svg>
                <div
                  class="graph-node-avatar {isHead ? 'is-head' : ''}"
                  style="left: {nodeX}px; top: {nodeY}px; --node-color: {nodeColor};"
                  title="{layout?.branchName ? `[${layout.branchName}] ` : ''}{commit.author} ({commit.author_email || ''})"
                >
                  {#if avatarUrl && !failedAvatars[commit.sha]}
                    <img
                      src={avatarUrl}
                      alt=""
                      class="graph-node-avatar-img"
                      onerror={() => (failedAvatars[commit.sha] = true)}
                    />
                  {:else}
                    <span class="graph-node-avatar-fallback">
                      {(commit.author || "?").slice(0, 1).toUpperCase()}
                    </span>
                  {/if}
                </div>
              </span>
            {:else if colId === "commit"}
              <span class="commit-summary-cell">
                {#if commit.refs?.length}
                  <span class="commit-inline-refs">
                    {#each commit.refs as refItem (refItem.kind + ":" + refItem.name)}
                      {#if refItem.kind === "tag"}
                        <span
                          class="graph-tag-badge"
                          style="--tag-color: {nodeColor};"
                          title={refItem.name}
                        >
                          <svg class="badge-icon" width="11" height="11" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                            <path d="M2.5 8.5L8.5 2.5H13.5V7.5L7.5 13.5L2.5 8.5Z" />
                            <circle cx="10.5" cy="5.5" r="1.5" fill="currentColor" />
                          </svg>
                          <span class="badge-name">{refItem.name}</span>
                        </span>
                      {:else}
                        <!-- svelte-ignore a11y_no_static_element_interactions -->
                        <span
                          class="graph-branch-badge {refItem.is_current ? 'is-current' : ''} kind-{refItem.kind}"
                          style="--branch-color: {nodeColor};"
                          draggable="true"
                          ondragstart={(e) => {
                            e.stopPropagation();
                            handleCommitDragStart(e, commit, refItem.name);
                          }}
                          title={refItem.name}
                        >
                          <svg class="badge-icon" width="10" height="10" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                            {#if refItem.kind === "remote"}
                              <path d="M4.5 12.75h7.1a2.65 2.65 0 0 0 .45-5.26A3.85 3.85 0 0 0 4.55 6.4 3.1 3.1 0 0 0 4.5 12.75Z" />
                            {:else}
                              <line x1="6" y1="3" x2="6" y2="13" />
                              <circle cx="6" cy="3" r="2" />
                              <circle cx="6" cy="13" r="2" />
                              <circle cx="12" cy="6" r="2" />
                              <path d="M6 9a3 3 0 0 0 3-3h3" />
                            {/if}
                          </svg>
                          <span class="badge-name">{refItem.is_current ? `HEAD → ${refItem.name}` : refItem.name}</span>
                        </span>
                      {/if}
                    {/each}
                  </span>
                {:else if commit.tags?.length}
                  <span class="commit-inline-refs">
                    {#each commit.tags as tagName (tagName)}
                      <span
                        class="graph-tag-badge"
                        style="--tag-color: {nodeColor};"
                        title={tagName}
                      >
                        <svg class="badge-icon" width="11" height="11" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                          <path d="M2.5 8.5L8.5 2.5H13.5V7.5L7.5 13.5L2.5 8.5Z" />
                          <circle cx="10.5" cy="5.5" r="1.5" fill="currentColor" />
                        </svg>
                        <span class="badge-name">{tagName}</span>
                      </span>
                    {/each}
                  </span>
                {/if}
                <span class="commit-summary" title="{layout?.branchName ? `[${layout.branchName}] ` : ''}{commit.summary}">{commit.summary}</span>
              </span>
             {:else if colId === "changes"}
              <span class="commit-changes-cell">
                {#if commit.files_changed}
                  <span class="commit-files-count" title="{commit.files_changed} {commit.files_changed === 1 ? 'file' : 'files'} changed">
                    <span>{commit.files_changed}</span>
                    <svg class="file-stat-icon" width="10" height="10" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                      <path d="M9 2H4a1.5 1.5 0 0 0-1.5 1.5v9A1.5 1.5 0 0 0 4 14h8a1.5 1.5 0 0 0 1.5-1.5V6.5L9 2Z" />
                      <path d="M9 2v4.5h4.5" />
                    </svg>
                  </span>
                  {#if commit.insertions}
                    <span class="stat-pill stat-add" title="+{commit.insertions} additions">+{commit.insertions}</span>
                  {/if}
                  {#if commit.deletions}
                    <span class="stat-pill stat-del" title="-{commit.deletions} deletions">-{commit.deletions}</span>
                  {/if}
                {:else}
                  <span class="commit-changes-empty">-</span>
                {/if}
              </span>
             {:else if colId === "author"}
              <span class="commit-author" title={commit.author}>{commit.author}</span>
             {:else if colId === "date"}
              <span class="commit-date" title={commit.date}>{formatCommitDate(commit.date, dateFormat)}</span>
             {:else if colId === "sha"}
              <span class="commit-sha">
                {#if !commit.is_pushed}
                  <span
                    class="unpushed-badge"
                    title={t("history.localTitle")}
                    aria-label={t("history.localTitle")}
                  >
                    <svg width="8" height="8" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                      <line x1="8" y1="13" x2="8" y2="3" />
                      <polyline points="4 7 8 3 12 7" />
                    </svg>
                  </span>
                {/if}
                {#if checkRunsMap[commit.sha]}
                  {@const ci = checkRunsMap[commit.sha]}
                  <span
                    class="ci-dot"
                    style="background-color: {ci.conclusion === 'success'
                      ? 'var(--diff-add-text)'
                      : ci.conclusion === 'failure'
                        ? 'var(--diff-del-text)'
                        : 'var(--warn-text)'};"
                    title="CI: {ci.conclusion || ci.status}"
                  ></span>
                {/if}
                <span class="commit-sha-text">{commit.short_sha}</span>
              </span>
            {/if}
          {/each}
          <span class="commit-row-end-spacer" aria-hidden="true"></span>
        </button>
        {#if isSelected}
          <div class="history-action-bar">
            <span class="badge-neutral">{commit.author}</span>
            {#if onSquash && index > 0 && !searchQuery.trim()}
              <button
                type="button"
                onclick={() => onSquash(index, commit.sha)}
                class="btn-secondary btn-xs"
                title={t("history.squashTitle", { count: index + 1 })}
              >
                {t("history.squashBtn", { count: index + 1 })}
              </button>
            {/if}
            {#if onCherryPick}
              <button
                type="button"
                onclick={() => onCherryPick(commit.sha)}
                class="btn-secondary btn-xs"
                title={t("history.cherryPickTitle")}
              >
                {t("history.cherryPickBtn")}
              </button>
            {/if}
            {#if onRevert}
              <button
                type="button"
                onclick={() => onRevert(commit.sha)}
                class="btn-secondary btn-xs"
                title={t("history.revertTitle")}
              >
                {t("history.revertBtn")}
              </button>
            {/if}
            {#if onTag}
              <button
                type="button"
                onclick={() => onTag(commit.sha)}
                class="btn-secondary btn-xs"
                title={t("history.createTagTitle")}
              >
                {t("history.createTagBtn")}
              </button>
            {/if}
            {#if onAiExplainCommit}
              <button
                type="button"
                onclick={() => onAiExplainCommit(commit.sha, commit.summary)}
                class="btn-secondary btn-xs"
                title={t("ai.explainCommitTitle")}
              >
                ✦ {t("ai.explainCommitBtn")}
              </button>
            {/if}
            {#if onReset}
              <button
                type="button"
                onclick={() => onReset(commit.sha)}
                class="btn-secondary btn-xs btn-danger history-reset-btn"
                title={t("history.resetHereTitle")}
              >
                {t("history.resetHereBtn")}
              </button>
            {/if}
          </div>
        {/if}
      {/each}
    {/if}
    </div>

    {#if selectedSha && repoPath}
      <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
      <div
        class="pane-resizer-h {activeResizer === 'detail' ? 'is-resizing' : ''}"
        role="separator"
        aria-orientation="horizontal"
        aria-label={t("history.detailResizerAria")}
        title={t("history.detailResizerTitle")}
        tabindex="0"
        onpointerdown={startDetailResize}
        ondblclick={() => updateDetailPct(DETAIL_PCT_DEFAULT)}
        onkeydown={handleDetailResizerKeydown}
      ></div>
      <section
        bind:this={detailPaneEl}
        class="commit-detail-pane"
        aria-label={t("history.detailPaneAria")}
      >
        <div
          bind:this={detailHeaderEl}
          class="commit-detail-header"
          style={descHeaderHeight !== null ? `height: ${descHeaderHeight}px; max-height: none;` : undefined}
        >
          <div class="commit-detail-top">
            <div class="commit-detail-title-group">
              <h3 class="commit-detail-summary">
                {selectedDetail?.summary || selectedCommitSummary?.summary || ""}
              </h3>
              <span class="commit-detail-sha" title={selectedSha}>
                {selectedDetail?.short_sha || selectedCommitSummary?.short_sha || selectedSha.slice(0, 7)}
              </span>
              {#if selectedCommitLayout?.branchName}
                <span class="commit-lane-badge" title="Cabang pada alur ini">
                  <svg width="11" height="11" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                    <line x1="6" y1="3" x2="6" y2="13" />
                    <circle cx="6" cy="3" r="2" />
                    <circle cx="6" cy="13" r="2" />
                    <circle cx="12" cy="6" r="2" />
                    <path d="M6 9a3 3 0 0 0 3-3h3" />
                  </svg>
                  <span>{selectedCommitLayout.branchName}</span>
                </span>
              {/if}
              {#if selectedCommitSummary}
                <span
                  class="commit-push-badge {selectedCommitSummary.is_pushed ? 'is-pushed' : 'is-local'}"
                  title={selectedCommitSummary.is_pushed
                    ? t("history.pushedTitle")
                    : t("history.localTitle")}
                >
                  {#if selectedCommitSummary.is_pushed}
                    <svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                      <path d="M4.5 12.75h7.1a2.65 2.65 0 0 0 .45-5.26A3.85 3.85 0 0 0 4.55 6.4 3.1 3.1 0 0 0 4.5 12.75Z" />
                      <path d="M6.4 9.55 7.7 10.85l2.6-2.7" />
                    </svg>
                    <span>{t("history.badgePushed")}</span>
                  {:else}
                    <svg width="11" height="11" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                      <path d="M8 10V2.5" />
                      <path d="M4.75 5.5 8 2.25l3.25 3.25" />
                      <path d="M2.5 10.5v2a1 1 0 0 0 1 1h9a1 1 0 0 0 1-1v-2" />
                    </svg>
                    <span>{t("history.badgeUnpushed")}</span>
                  {/if}
                </span>
              {/if}
            </div>
            <button
              type="button"
              class="btn-secondary btn-xs"
              onclick={clearSelection}
              title={t("history.closeDetailTitle")}
            >
              {t("common.close")}
            </button>
          </div>

          <div class="commit-detail-meta">
            <span>
              {selectedDetail?.author || selectedCommitSummary?.author}{#if selectedDetail?.author_email} &lt;{selectedDetail.author_email}&gt;{/if}
            </span>
            <span>·</span>
            <span>{selectedDetail?.date || selectedCommitSummary?.date}</span>
            {#if selectedDetail}
              <span>·</span>
              <span class="tabular-nums">
                {selectedDetail.files.length === 1
                  ? t("history.filesChangedCountOne", { count: selectedDetail.files.length })
                  : t("history.filesChangedCountMany", { count: selectedDetail.files.length })}
              </span>
            {/if}
          </div>

          {#if detailLoading}
            <div class="commit-detail-empty-desc">{t("history.loadingDetail")}</div>
          {:else if detailError}
            <div class="alert-banner" style="border-radius: var(--radius-sm); border: 1px solid var(--danger-border);">
              <span>{detailError}</span>
            </div>
          {:else if selectedDetail?.description}
            <pre class="commit-detail-desc">{selectedDetail.description}</pre>
          {:else if selectedDetail}
            <div class="commit-detail-empty-desc">{t("history.noDescription")}</div>
          {/if}
        </div>
        {#if !detailLoading && !detailError && selectedDetail}
          <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
          <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
          <div
            class="pane-resizer-h subtle {activeResizer === 'desc' ? 'is-resizing' : ''}"
            role="separator"
            aria-orientation="horizontal"
            aria-label={t("history.descResizerAria")}
            title={t("history.descResizerTitle")}
            tabindex="0"
            onpointerdown={startDescResize}
            ondblclick={resetDescHeaderHeight}
            onkeydown={handleDescResizerKeydown}
          ></div>
          {#if selectedDetail.files.length === 0}
            <div class="empty-state" style="padding: 18px;">
              <span>{t("history.noChangedFiles")}</span>
            </div>
          {:else}
            <div class="commit-detail-body {isFilesStacked ? 'is-stacked' : ''}">
              <div class="commit-files-pane">
                <div class="commit-files-header tabular-nums">
                  <span>{t("history.changedFilesHeader", { count: selectedDetail.files.length })}</span>
                  {#if !isCompactWidth}
                    <button
                      type="button"
                      class="commit-files-layout-btn"
                      onclick={toggleFilesLayout}
                      title={filesLayout === "vertical"
                        ? t("history.layoutToHorizontalTitle")
                        : t("history.layoutToVerticalTitle")}
                    >
                      {filesLayout === "vertical" ? t("history.layoutHorizontalLabel") : t("history.layoutVerticalLabel")}
                    </button>
                  {/if}
                </div>
                <ul class="commit-files-list">
                  {#each selectedDetail.files as file (file.path)}
                    {@const isFileSelected = selectedCommitFile?.path === file.path}
                    {@const slashIdx = Math.max(file.path.lastIndexOf("/"), file.path.lastIndexOf("\\"))}
                    <li>
                      <button
                        type="button"
                        class="commit-file-btn {isFileSelected ? 'selected' : ''}"
                        onclick={() => loadFileDiff(selectedDetail!.sha, file)}
                        oncontextmenu={(e) => openCommitFileContextMenu(e, file)}
                        title={file.old_path ? `${file.old_path} → ${file.path}` : file.path}
                      >
                        <span class="file-name">
                          {#if slashIdx >= 0}
                            <span class="file-dir">{file.path.slice(0, slashIdx + 1)}</span><span class="file-base">{file.path.slice(slashIdx + 1)}</span>
                          {:else}
                            <span class="file-base">{file.path}</span>
                          {/if}
                        </span>
                        <span class="file-status-badge status-{file.status_code}">
                          {#if file.status_code === "A"}
                            <svg width="10" height="10" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
                              <path d="M7.75 2a.75.75 0 0 1 .75.75V7h4.25a.75.75 0 0 1 0 1.5H8.5v4.25a.75.75 0 0 1-1.5 0V8.5H2.75a.75.75 0 0 1 0-1.5H7V2.75A.75.75 0 0 1 7.75 2Z" />
                            </svg>
                          {:else}
                            {file.status_code}
                          {/if}
                        </span>
                      </button>
                    </li>
                  {/each}
                </ul>
              </div>

              <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
              <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
              <div
                class="{isFilesStacked ? 'pane-resizer-h' : 'pane-resizer-v'} {activeResizer === 'files' ? 'is-resizing' : ''}"
                role="separator"
                aria-orientation={isFilesStacked ? "horizontal" : "vertical"}
                aria-label={isFilesStacked
                  ? t("history.filesResizerVerticalAria")
                  : t("history.filesResizerHorizontalAria")}
                title={isFilesStacked
                  ? t("history.filesResizerVerticalTitle")
                  : t("history.filesResizerHorizontalTitle")}
                tabindex="0"
                onpointerdown={startFilesResize}
                ondblclick={() =>
                  isFilesStacked
                    ? updateFilesHeight(FILES_H_DEFAULT)
                    : updateFilesWidth(FILES_W_DEFAULT)}
                onkeydown={handleFilesResizerKeydown}
              ></div>

              <div class="commit-diff-pane">
                {#if diffLoading}
                  <div class="empty-state">
                    <span>{t("history.loadingDiff")}</span>
                  </div>
                {:else if selectedCommitFile}
                  <Diff
                    {repoPath}
                    {externalEditorLabel}
                    filePath={selectedCommitFile.path}
                    diffText={commitFileDiff}
                    ignoreWhitespace={commitIgnoreWhitespace}
                    onToggleWhitespace={handleToggleWhitespace}
                    {onRevealFile}
                    {onOpenInEditor}
                    {onStatus}
                  />
                {/if}
              </div>
            </div>
          {/if}
        {/if}
      </section>
    {/if}
  </div>
{/if}
