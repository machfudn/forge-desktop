<script lang="ts">
  import type { BlameLine, CommitInfo } from "../types/git";
  import * as gitService from "../services/git";
  import Diff from "./Diff.svelte";
  import { t } from "../i18n/index.svelte";

  interface Props {
    repoPath: string;
    filePath: string;
    initialMode?: "blame" | "history";
    externalEditorLabel?: string;
    onClose: () => void;
    onRevealFile?: (filePath: string) => void;
    onOpenInEditor?: (filePath: string) => void;
    onAiExplainDiff?: (filePath: string, diffText: string) => void;
    onStatus?: (msg: string, isError?: boolean) => void;
  }

  let {
    repoPath,
    filePath,
    initialMode = "blame",
    externalEditorLabel = "Editor",
    onClose,
    onRevealFile,
    onOpenInEditor,
    onAiExplainDiff,
    onStatus,
  }: Props = $props();

  let activeTab = $state<"blame" | "history">("blame");
  let blameLines = $state<BlameLine[]>([]);
  let blameLoading = $state(false);
  let blameError = $state("");

  let historyCommits = $state<CommitInfo[]>([]);
  let historyLoading = $state(false);
  let historyError = $state("");
  let selectedSha = $state<string | null>(null);
  let selectedDiff = $state("");
  let diffLoading = $state(false);

  $effect(() => {
    activeTab = initialMode;
  });

  $effect(() => {
    if (!repoPath || !filePath) return;
    if (activeTab === "blame") {
      void loadBlame();
    } else {
      void loadFileHistory();
    }
  });

  async function loadBlame() {
    blameLoading = true;
    blameError = "";
    try {
      blameLines = await gitService.getFileBlame(repoPath, filePath);
    } catch (err) {
      blameError = String(err);
      blameLines = [];
    } finally {
      blameLoading = false;
    }
  }

  async function loadFileHistory() {
    historyLoading = true;
    historyError = "";
    try {
      const items = await gitService.getFileHistory(repoPath, filePath, 100);
      historyCommits = items;
      if (items.length > 0 && !selectedSha) {
        await selectCommit(items[0].sha);
      }
    } catch (err) {
      historyError = String(err);
      historyCommits = [];
    } finally {
      historyLoading = false;
    }
  }

  async function selectCommit(sha: string) {
    selectedSha = sha;
    diffLoading = true;
    try {
      selectedDiff = (await gitService.getCommitFileDiff(repoPath, sha, filePath, null, false)) ?? "";
    } catch (err) {
      selectedDiff = "";
      onStatus?.(String(err), true);
    } finally {
      diffLoading = false;
    }
  }

  async function jumpToCommitHistory(sha: string) {
    activeTab = "history";
    await loadFileHistory();
    await selectCommit(sha);
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="modal-overlay"
  onclick={(e) => {
    if (e.target === e.currentTarget) onClose();
  }}
  onkeydown={(e) => {
    if (e.key === "Escape") onClose();
  }}
>
  <div class="modal-dialog modal-xl inspect-modal" role="dialog" aria-modal="true" aria-label={filePath}>
    <div class="modal-header">
      <div style="display: flex; align-items: center; gap: 10px; min-width: 0;">
        <h3 style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap;" title={filePath}>
          {filePath}
        </h3>
        <div class="seg-group">
          <button
            type="button"
            class="seg-btn {activeTab === 'blame' ? 'active' : ''}"
            onclick={() => (activeTab = "blame")}
          >
            {t("inspect.tabBlame")}
          </button>
          <button
            type="button"
            class="seg-btn {activeTab === 'history' ? 'active' : ''}"
            onclick={() => (activeTab = "history")}
          >
            {t("inspect.tabFileHistory")}
          </button>
        </div>
      </div>
      <button type="button" class="btn-secondary btn-xs" onclick={onClose}>{t("common.close")}</button>
    </div>

    <div class="inspect-body">
      {#if activeTab === "blame"}
        {#if blameLoading}
          <div class="empty-state"><span>{t("inspect.loadingBlame")}</span></div>
        {:else if blameError}
          <div class="alert-banner"><span>{blameError}</span></div>
        {:else if blameLines.length === 0}
          <div class="empty-state"><span>{t("inspect.emptyBlame")}</span></div>
        {:else}
          <div class="blame-table">
            {#each blameLines as row (row.line_no)}
              <div class="blame-row">
                <button
                  type="button"
                  class="blame-gutter"
                  onclick={() => jumpToCommitHistory(row.sha)}
                  title={`${row.short_sha} · ${row.author} (${row.date})\n${row.summary}`}
                >
                  <span class="blame-sha">{row.short_sha}</span>
                  <span class="blame-author">{row.author}</span>
                  <span class="blame-date">{row.date}</span>
                </button>
                <span class="blame-ln tabular-nums">{row.line_no}</span>
                <span class="blame-code">{row.content}</span>
              </div>
            {/each}
          </div>
        {/if}
      {:else}
        <div class="file-history-split">
          <div class="file-history-list">
            {#if historyLoading}
              <div class="empty-state"><span>{t("inspect.loadingHistory")}</span></div>
            {:else if historyError}
              <div class="alert-banner"><span>{historyError}</span></div>
            {:else if historyCommits.length === 0}
              <div class="empty-state"><span>{t("inspect.emptyHistory")}</span></div>
            {:else}
              {#each historyCommits as c (c.sha)}
                <button
                  type="button"
                  class="file-history-item {selectedSha === c.sha ? 'active' : ''}"
                  onclick={() => selectCommit(c.sha)}
                >
                  <div class="file-history-top">
                    <span class="commit-sha">{c.short_sha}</span>
                    <span class="commit-date">{c.date}</span>
                  </div>
                  <div class="file-history-summary" title={c.summary}>{c.summary}</div>
                  <div class="file-history-author">{c.author}</div>
                </button>
              {/each}
            {/if}
          </div>
          <div class="file-history-diff">
            {#if diffLoading}
              <div class="empty-state"><span>{t("history.loadingDiff")}</span></div>
            {:else if selectedSha}
              <Diff
                {repoPath}
                {externalEditorLabel}
                {filePath}
                diffText={selectedDiff}
                {onRevealFile}
                {onOpenInEditor}
                {onAiExplainDiff}
                {onStatus}
              />
            {:else}
              <div class="empty-state"><span>{t("inspect.selectCommit")}</span></div>
            {/if}
          </div>
        </div>
      {/if}
    </div>
  </div>
</div>
