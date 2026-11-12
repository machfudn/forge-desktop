<script lang="ts">
  import type { CommitChangedFile, CommitDetail, CommitInfo } from "../types/git";
  import * as gitService from "../services/git";
  import Diff from "./Diff.svelte";
  import { t } from "../i18n/index.svelte";
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
    githubRepoUrl?: string | null;
    commits: CommitInfo[];
    searchQuery?: string;
    allBranches?: boolean;
    checkRunsMap?: Record<string, { status: string; conclusion?: string }>;
    onToggleAllBranches?: () => void;
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
    githubRepoUrl = null,
    commits,
    searchQuery = "",
    allBranches = true,
    checkRunsMap = {},
    onToggleAllBranches,
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
  let indexedCommits = $derived(
    commits
      .map((commit, index) => ({ commit, index }))
      .filter(({ commit }) => {
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

  const LANE_W = 14;
  const ROW_H = 34;

  let graphLayouts = $derived(
    computeCommitGraph(indexedCommits.map((x) => x.commit))
  );
  let maxGraphCols = $derived(
    Math.min(10, Math.max(1, ...graphLayouts.map((g) => g.maxLanes)))
  );
  let graphColWidth = $derived(Math.max(54, maxGraphCols * LANE_W + 14));

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
    if (selectedSha && !commits.some((c) => c.sha === selectedSha)) {
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
    if (githubRepoUrl) {
      items.push({
        label: t("ctx.history.viewOnGithub"),
        onSelect: () => {
          void openExternalUrl(`${githubRepoUrl}/commit/${commit.sha}`);
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
      <span class="history-toolbar-count tabular-nums">
        {t("app.historySidebar.title", { count: indexedCommits.length })}
      </span>
      {#if onToggleAllBranches}
        <button
          type="button"
          class="graph-scope-btn {allBranches ? 'active' : ''}"
          onclick={onToggleAllBranches}
          title={t("history.toggleAllBranchesTitle")}
        >
          {allBranches ? t("history.allBranchesShort") : t("history.currentBranchShort")}
        </button>
      {/if}
    </div>
    <div class="history-container {selectedSha ? 'has-detail' : ''}">
    <div class="history-header-bar history-header-with-graph" style="--graph-col-w: {graphColWidth}px;">
      <span class="history-graph-col-header">{t("history.colGraph")}</span>
      <span>{t("history.colSha")}</span>
      <span>{t("history.colCommit")}</span>
      <span class="commit-status-col">{t("history.colStatus")}</span>
      <span class="commit-author-col">{t("history.colAuthor")}</span>
      <span class="commit-date-col">{t("history.colDate")}</span>
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
        {@const nodeY = ROW_H / 2}
        {@const nodeColor = GRAPH_COLORS[(layout?.colorIndex ?? 0) % GRAPH_COLORS.length]}
        {@const isHead = commit.refs?.some((r) => r.is_current) ?? false}
        <button
          type="button"
          draggable="true"
          class="commit-row commit-row-with-graph {isSelected ? 'selected' : ''} {dropOverSha === commit.sha ? 'is-drop-target' : ''}"
          style="--graph-col-w: {graphColWidth}px;"
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
          <span class="commit-graph-cell" aria-hidden="true">
            <svg class="commit-graph-svg" width={graphColWidth} height={ROW_H} viewBox="0 0 {graphColWidth} {ROW_H}">
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
                    <line x1={x1} y1={nodeY} x2={x2} y2={ROW_H} stroke={segColor} stroke-width="2" />
                  {:else if seg.kind === "branch-in"}
                    <path
                      d="M {x1} 0 C {x1} {nodeY * 0.65}, {x2} {nodeY * 0.35}, {x2} {nodeY}"
                      fill="none"
                      stroke={segColor}
                      stroke-width="2"
                    />
                  {:else}
                    <path
                      d="M {x1} {nodeY} C {x1} {nodeY + (ROW_H - nodeY) * 0.55}, {x2} {nodeY + (ROW_H - nodeY) * 0.45}, {x2} {ROW_H}"
                      fill="none"
                      stroke={segColor}
                      stroke-width="2"
                    />
                  {/if}
                {/each}
              {/if}
              {#if isHead}
                <circle cx={nodeX} cy={nodeY} r="6.5" fill="var(--bg-app)" stroke={nodeColor} stroke-width="2" />
              {/if}
              <circle cx={nodeX} cy={nodeY} r={isHead ? 3.8 : 4.2} fill={nodeColor} />
            </svg>
          </span>
          <span class="commit-sha">
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
            {commit.short_sha}
          </span>
          <span class="commit-summary-cell">
            {#if commit.refs?.length}
              <span class="commit-ref-list">
                {#each commit.refs as refItem (refItem.kind + ":" + refItem.name)}
                  {#if refItem.kind === "tag"}
                    <span class="tag-badge commit-tag-badge" data-tag={refItem.name} title={refItem.name}>
                      <span class="tag-badge-label">{refItem.name}</span>
                    </span>
                  {:else}
                    <!-- svelte-ignore a11y_no_static_element_interactions -->
                    <span
                      class="graph-ref-badge kind-{refItem.kind} {refItem.is_current ? 'is-current' : ''}"
                      draggable="true"
                      ondragstart={(e) => {
                        e.stopPropagation();
                        handleCommitDragStart(e, commit, refItem.name);
                      }}
                      title={refItem.name}
                    >
                      {refItem.is_current ? `HEAD → ${refItem.name}` : refItem.name}
                    </span>
                  {/if}
                {/each}
              </span>
            {:else if commit.tags?.length}
              <span class="commit-tag-list">
                {#each commit.tags as tagName (tagName)}
                  <span class="tag-badge commit-tag-badge" data-tag={tagName} title={tagName}>
                    <span class="tag-badge-label">{tagName}</span>
                  </span>
                {/each}
              </span>
            {/if}
            <span class="commit-summary" title={commit.summary}>{commit.summary}</span>
          </span>
          <span
            class="commit-push-badge {commit.is_pushed ? 'is-pushed' : 'is-local'}"
            title={commit.is_pushed
              ? t("history.pushedTitle")
              : t("history.localTitle")}
          >
            {#if commit.is_pushed}
              <svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                <path d="M4.5 12.75h7.1a2.65 2.65 0 0 0 .45-5.26A3.85 3.85 0 0 0 4.55 6.4 3.1 3.1 0 0 0 4.5 12.75Z" />
                <path d="M6.4 9.55 7.7 10.85l2.6-2.7" />
              </svg>
              <span>{t("history.badgeRemote")}</span>
            {:else}
              <svg width="11" height="11" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                <path d="M8 10V2.5" />
                <path d="M4.75 5.5 8 2.25l3.25 3.25" />
                <path d="M2.5 10.5v2a1 1 0 0 0 1 1h9a1 1 0 0 0 1-1v-2" />
              </svg>
              <span>{t("history.badgeLocal")}</span>
            {/if}
          </span>
          <span class="commit-author">{commit.author}</span>
          <span class="commit-date">{commit.date}</span>
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
