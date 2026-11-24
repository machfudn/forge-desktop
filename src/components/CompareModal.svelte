<script lang="ts">
  import type { BranchInfo, CommitInfo, CompareFileItem, CompareSummary } from "../types/git";
  import * as gitService from "../services/git";
  import Diff from "./Diff.svelte";
  import { t } from "../i18n/index.svelte";
  import { formatFullTooltip, formatRelativeDate } from "../utils/dateFormat";
  import { copyTextToClipboard } from "../lib/contextMenu.svelte";

  interface Props {
    repoPath: string;
    initialBase?: string;
    initialTarget?: string;
    branches?: BranchInfo[];
    tags?: string[];
    externalEditorLabel?: string;
    onClose: () => void;
    onInspectFile?: (filePath: string, mode: "blame" | "history") => void;
    onRevealFile?: (filePath: string) => void;
    onOpenInEditor?: (filePath: string) => void;
    onStatus?: (msg: string, isError?: boolean) => void;
  }

  let {
    repoPath,
    initialBase = "",
    initialTarget = "HEAD",
    branches = [],
    tags = [],
    externalEditorLabel = "Editor",
    onClose,
    onInspectFile,
    onRevealFile,
    onOpenInEditor,
    onStatus,
  }: Props = $props();

  // Pick sensible defaults if initialBase is empty
  function getDefaultBase(): string {
    if (initialBase) return initialBase;
    const defaultBranch = branches.find((b) => b.name === "main" || b.name === "master");
    if (defaultBranch && defaultBranch.name !== initialTarget) {
      return defaultBranch.name;
    }
    const other = branches.find((b) => b.name !== initialTarget && !b.is_remote);
    if (other) return other.name;
    return "HEAD~1";
  }

  let baseRef = $state<string>("");
  let targetRef = $state<string>("");
  let threeDot = $state<boolean>(true);

  $effect(() => {
    if (!baseRef) {
      baseRef = initialBase || getDefaultBase();
    }
    if (!targetRef) {
      targetRef = initialTarget || "HEAD";
    }
  });

  let loading = $state<boolean>(false);
  let errorMsg = $state<string>("");
  let summary = $state<CompareSummary | null>(null);

  let activeTab = $state<"files" | "commits">("files");
  let filterQuery = $state<string>("");

  let selectedFile = $state<CompareFileItem | null>(null);
  let fileDiff = $state<string>("");
  let fileDiffLoading = $state<boolean>(false);
  let ignoreWhitespace = $state<boolean>(false);

  // Reference picker state
  let pickingFor = $state<"base" | "target" | null>(null);
  let refPickerTab = $state<"branch" | "tag" | "commit" | "custom">("branch");
  let refSearchQuery = $state<string>("");
  let recentCommits = $state<CommitInfo[]>([]);
  let loadingCommits = $state<boolean>(false);
  let customRefInput = $state<string>("");

  let compareSeq = 0;

  async function loadComparison() {
    if (!repoPath || !baseRef.trim() || !targetRef.trim()) return;
    const currentSeq = ++compareSeq;
    loading = true;
    errorMsg = "";
    selectedFile = null;
    fileDiff = "";

    try {
      const res = await gitService.compareRevisions(
        repoPath,
        baseRef.trim(),
        targetRef.trim(),
        threeDot
      );
      if (currentSeq === compareSeq) {
        summary = res;
        // Auto-select first file if available
        if (res.files.length > 0) {
          void selectFile(res.files[0]);
        }
      }
    } catch (err) {
      if (currentSeq === compareSeq) {
        errorMsg = String(err);
        summary = null;
      }
    } finally {
      if (currentSeq === compareSeq) {
        loading = false;
      }
    }
  }

  async function selectFile(file: CompareFileItem) {
    selectedFile = file;
    fileDiffLoading = true;
    try {
      const diff = await gitService.getCompareFileDiff(
        repoPath,
        baseRef.trim(),
        targetRef.trim(),
        file.path,
        file.old_path,
        threeDot,
        ignoreWhitespace
      );
      if (selectedFile?.path === file.path) {
        fileDiff = diff;
      }
    } catch (err) {
      if (selectedFile?.path === file.path) {
        fileDiff = "";
        onStatus?.(t("compareModal.failedToLoadDiff", { path: file.path, err: String(err) }), true);
      }
    } finally {
      if (selectedFile?.path === file.path) {
        fileDiffLoading = false;
      }
    }
  }

  function handleSwap() {
    const temp = baseRef;
    baseRef = targetRef;
    targetRef = temp;
    void loadComparison();
  }

  function handleToggleMode(newThreeDot: boolean) {
    if (threeDot === newThreeDot) return;
    threeDot = newThreeDot;
    void loadComparison();
  }

  function openRefPicker(target: "base" | "target") {
    pickingFor = target;
    refSearchQuery = "";
    customRefInput = target === "base" ? baseRef : targetRef;
    if (recentCommits.length === 0 && !loadingCommits) {
      loadingCommits = true;
      gitService
        .getCommitHistory(repoPath, 50, true)
        .then((c) => {
          recentCommits = c;
        })
        .catch(() => {})
        .finally(() => {
          loadingCommits = false;
        });
    }
  }

  function selectRef(ref: string) {
    if (pickingFor === "base") {
      baseRef = ref;
    } else if (pickingFor === "target") {
      targetRef = ref;
    }
    pickingFor = null;
    void loadComparison();
  }

  function applyCustomRef() {
    const clean = customRefInput.trim();
    if (clean) {
      selectRef(clean);
    }
  }

  function handleToggleWhitespace() {
    ignoreWhitespace = !ignoreWhitespace;
    if (selectedFile) {
      void selectFile(selectedFile);
    }
  }

  // Filtered files & commits
  const filteredFiles = $derived(
    summary
      ? summary.files.filter((f) =>
          f.path.toLowerCase().includes(filterQuery.trim().toLowerCase())
        )
      : []
  );

  const filteredCommits = $derived(
    summary
      ? summary.commits.filter((c) => {
          const q = filterQuery.trim().toLowerCase();
          return (
            c.summary.toLowerCase().includes(q) ||
            c.author.toLowerCase().includes(q) ||
            c.short_sha.toLowerCase().includes(q)
          );
        })
      : []
  );

  // Picker filters
  const filteredPickerBranches = $derived(
    branches.filter((b) => b.name.toLowerCase().includes(refSearchQuery.trim().toLowerCase()))
  );

  const filteredPickerTags = $derived(
    tags.filter((tag) => tag.toLowerCase().includes(refSearchQuery.trim().toLowerCase()))
  );

  const filteredPickerCommits = $derived(
    recentCommits.filter((c) => {
      const q = refSearchQuery.trim().toLowerCase();
      return (
        c.summary.toLowerCase().includes(q) ||
        c.author.toLowerCase().includes(q) ||
        c.short_sha.toLowerCase().includes(q)
      );
    })
  );

  function getStatusBadgeClass(status: string): string {
    const s = status.toUpperCase();
    if (s.startsWith("A")) return "badge-added";
    if (s.startsWith("D")) return "badge-deleted";
    if (s.startsWith("M")) return "badge-modified";
    if (s.startsWith("R")) return "badge-renamed";
    return "badge-other";
  }

  function getStatusLabel(status: string): string {
    const s = status.toUpperCase();
    if (s.startsWith("A")) return "A";
    if (s.startsWith("D")) return "D";
    if (s.startsWith("M")) return "M";
    if (s.startsWith("R")) return "R";
    return s.slice(0, 1) || "M";
  }

  // Initialize comparison on mount
  $effect(() => {
    void loadComparison();
  });
</script>

<div
  class="modal-overlay"
  role="presentation"
  tabindex="-1"
  onclick={(e) => {
    if (e.target === e.currentTarget && !pickingFor) onClose();
  }}
  onkeydown={(e) => {
    if (e.key === "Escape") {
      if (pickingFor) {
        pickingFor = null;
      } else {
        onClose();
      }
    }
  }}
>
  <div
    class="modal-dialog compare-modal-dialog"
    role="dialog"
    aria-modal="true"
    aria-label={t("compareModal.title")}
  >
    <!-- Modal Header -->
    <div class="modal-header compare-header">
      <div class="compare-title-section">
        <svg class="compare-title-icon" width="18" height="18" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <circle cx="5" cy="5" r="2.5" />
          <circle cx="11" cy="11" r="2.5" />
          <path d="M5 7.5v3a2 2 0 0 0 2 2h1.5" />
        </svg>
        <div class="compare-title-text">
          <h3 class="compare-heading">{t("compareModal.title")}</h3>
          <span class="compare-subheading">
            {baseRef} {threeDot ? "..." : ".."} {targetRef}
          </span>
        </div>
      </div>

      <button
        type="button"
        class="btn-secondary btn-xs"
        onclick={onClose}
        aria-label={t("compareModal.close")}
        title={t("compareModal.close")}
      >
        {t("compareModal.close")}
      </button>
    </div>

    <!-- Revision Selector Control Bar -->
    <div class="compare-control-bar">
      <!-- Base Ref Selector -->
      <div class="ref-box">
        <span class="ref-label">{t("compareModal.base")}</span>
        <button
          type="button"
          class="ref-pill-btn"
          onclick={() => openRefPicker("base")}
          title={t("compareModal.selectRefTitle")}
        >
          <span class="ref-pill-text">{baseRef}</span>
          <svg class="ref-pill-chevron" width="12" height="12" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
            <path d="M4.427 6.427l3.396 3.396a.25.25 0 0 0 .354 0l3.396-3.396A.25.25 0 0 0 11.396 6H4.604a.25.25 0 0 0-.177.427z" />
          </svg>
        </button>
      </div>

      <!-- Swap Button -->
      <button
        type="button"
        class="swap-btn"
        onclick={handleSwap}
        title={t("compareModal.swap")}
        aria-label={t("compareModal.swap")}
      >
        <svg width="15" height="15" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
          <path fill-rule="evenodd" d="M1 11.5a.5.5 0 0 0 .5.5h11.793l-3.147 3.146a.5.5 0 0 0 .708.708l4-4a.5.5 0 0 0 0-.708l-4-4a.5.5 0 0 0-.708.708L13.293 11H1.5a.5.5 0 0 0-.5.5zm14-7a.5.5 0 0 1-.5.5H2.707l3.147 3.146a.5.5 0 1 1-.708.708l-4-4a.5.5 0 0 1 0-.708l4-4a.5.5 0 1 1 .708.708L2.707 4H14.5a.5.5 0 0 1 .5.5z"/>
        </svg>
      </button>

      <!-- Target Ref Selector -->
      <div class="ref-box">
        <span class="ref-label">{t("compareModal.target")}</span>
        <button
          type="button"
          class="ref-pill-btn"
          onclick={() => openRefPicker("target")}
          title={t("compareModal.selectRefTitle")}
        >
          <span class="ref-pill-text">{targetRef}</span>
          <svg class="ref-pill-chevron" width="12" height="12" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
            <path d="M4.427 6.427l3.396 3.396a.25.25 0 0 0 .354 0l3.396-3.396A.25.25 0 0 0 11.396 6H4.604a.25.25 0 0 0-.177.427z" />
          </svg>
        </button>
      </div>

      <!-- Mode Switcher: 3-dot (...) vs 2-dot (..) -->
      <div class="seg-group mode-seg-group" role="radiogroup" aria-label="Comparison Mode">
        <button
          type="button"
          class="seg-btn {threeDot ? 'active' : ''}"
          onclick={() => handleToggleMode(true)}
          title={t("compareModal.modeThreeDotTitle")}
        >
          {t("compareModal.modeThreeDot")}
        </button>
        <button
          type="button"
          class="seg-btn {!threeDot ? 'active' : ''}"
          onclick={() => handleToggleMode(false)}
          title={t("compareModal.modeTwoDotTitle")}
        >
          {t("compareModal.modeTwoDot")}
        </button>
      </div>

      <!-- Stats Summary Chips -->
      {#if summary}
        <div class="summary-chips">
          {#if summary.total_files === 0 && summary.commits.length === 0}
            <span class="chip-identical">
              ✓ {t("compareModal.identical")}
            </span>
          {:else}
            {#if summary.ahead > 0 || summary.behind > 0}
              <span class="chip-divergence tabular-nums">
                {t("compareModal.aheadBehind", { ahead: summary.ahead, behind: summary.behind })}
              </span>
            {/if}
            <span class="chip-stat chip-ins tabular-nums">
              +{summary.total_insertions}
            </span>
            <span class="chip-stat chip-del tabular-nums">
              -{summary.total_deletions}
            </span>
          {/if}
        </div>
      {/if}
    </div>

    <!-- Error Banner -->
    {#if errorMsg}
      <div class="alert-banner">
        <span>{errorMsg}</span>
      </div>
    {/if}

    <!-- Main Comparison Body (Split Master-Detail) -->
    <div class="compare-body">
      <!-- Left Sidebar: Commits & Changed Files -->
      <div class="compare-sidebar">
        <!-- Sidebar Tabs -->
        <div class="sidebar-tabs" role="tablist">
          <button
            type="button"
            role="tab"
            aria-selected={activeTab === "files"}
            class="sidebar-tab {activeTab === 'files' ? 'active' : ''}"
            onclick={() => (activeTab = "files")}
          >
            {t("compareModal.tabFiles", { count: summary?.files.length ?? 0 })}
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={activeTab === "commits"}
            class="sidebar-tab {activeTab === 'commits' ? 'active' : ''}"
            onclick={() => (activeTab = "commits")}
          >
            {t("compareModal.tabCommits", { count: summary?.commits.length ?? 0 })}
          </button>
        </div>

        <!-- Filter Input -->
        <div class="sidebar-search">
          <input
            type="text"
            class="form-input form-input-sm"
            placeholder={t("compareModal.filterPlaceholder")}
            bind:value={filterQuery}
          />
        </div>

        <!-- Tab Content -->
        <div class="sidebar-list">
          {#if loading}
            <div class="empty-state">
              <span>{t("compareModal.loading")}</span>
            </div>
          {:else if activeTab === "files"}
            {#if filteredFiles.length === 0}
              <div class="empty-state">
                <span>{filterQuery ? t("compareModal.emptySearch") : t("compareModal.identical")}</span>
              </div>
            {:else}
              {#each filteredFiles as file (file.path)}
                <button
                  type="button"
                  class="compare-file-item {selectedFile?.path === file.path ? 'is-selected' : ''}"
                  onclick={() => void selectFile(file)}
                  title={file.path}
                >
                  <span class="file-status-badge {getStatusBadgeClass(file.status_code)}">
                    {getStatusLabel(file.status_code)}
                  </span>
                  <span class="file-path-text" title={file.path}>
                    {file.path}
                  </span>
                  {#if file.is_binary}
                    <span class="badge-neutral" style="font-size: 10px;">bin</span>
                  {:else if file.insertions > 0 || file.deletions > 0}
                    <span class="file-diff-stat tabular-nums">
                      {#if file.insertions > 0}
                        <span class="stat-ins">+{file.insertions}</span>
                      {/if}
                      {#if file.deletions > 0}
                        <span class="stat-del">-{file.deletions}</span>
                      {/if}
                    </span>
                  {/if}
                </button>
              {/each}
            {/if}
          {:else if activeTab === "commits"}
            {#if filteredCommits.length === 0}
              <div class="empty-state">
                <span>{filterQuery ? t("compareModal.emptySearch") : t("compareModal.identical")}</span>
              </div>
            {:else}
              {#each filteredCommits as commit (commit.sha)}
                <div class="compare-commit-item">
                  <div class="commit-header-row">
                    <button
                      type="button"
                      class="commit-sha-btn tabular-nums"
                      onclick={() => copyTextToClipboard(commit.sha, onStatus, `SHA: ${commit.short_sha}`)}
                      title={commit.sha}
                    >
                      {commit.short_sha}
                    </button>
                    <span class="commit-date" title={formatFullTooltip(commit.date)}>
                      {formatRelativeDate(commit.date)}
                    </span>
                  </div>
                  <div class="commit-summary" title={commit.summary}>
                    {commit.summary}
                  </div>
                  <div class="commit-author">
                    {commit.author}
                  </div>
                </div>
              {/each}
            {/if}
          {/if}
        </div>
      </div>

      <!-- Right Panel: Diff View or Overview Card -->
      <div class="compare-detail-pane">
        {#if fileDiffLoading}
          <div class="empty-state">
            <span>{t("common.loading")}</span>
          </div>
        {:else if selectedFile}
          <div class="diff-wrapper">
            <Diff
              {repoPath}
              {externalEditorLabel}
              filePath={selectedFile.path}
              diffText={fileDiff}
              {ignoreWhitespace}
              onToggleWhitespace={handleToggleWhitespace}
              {onInspectFile}
              {onRevealFile}
              {onOpenInEditor}
              {onStatus}
            />
          </div>
        {:else if summary}
          <!-- Comparison Range Overview Card -->
          <div class="overview-container">
            <div class="overview-card">
              <h4 class="overview-title">{t("compareModal.overviewTitle")}</h4>

              <div class="overview-stats-grid">
                <div class="overview-stat-box">
                  <span class="stat-val tabular-nums">{summary.commits.length}</span>
                  <span class="stat-lbl">{t("compareModal.tabCommits", { count: summary.commits.length })}</span>
                </div>
                <div class="overview-stat-box">
                  <span class="stat-val tabular-nums">{summary.total_files}</span>
                  <span class="stat-lbl">{t("compareModal.tabFiles", { count: summary.total_files })}</span>
                </div>
                <div class="overview-stat-box">
                  <span class="stat-val stat-ins tabular-nums">+{summary.total_insertions}</span>
                  <span class="stat-lbl">{t("compareModal.insertionsSummary", { count: summary.total_insertions })}</span>
                </div>
                <div class="overview-stat-box">
                  <span class="stat-val stat-del tabular-nums">-{summary.total_deletions}</span>
                  <span class="stat-lbl">{t("compareModal.deletionsSummary", { count: summary.total_deletions })}</span>
                </div>
              </div>

              {#if summary.merge_base}
                <div class="merge-base-info">
                  <span class="info-label">{t("compareModal.mergeBase")}:</span>
                  <code class="info-sha tabular-nums">{summary.merge_base.slice(0, 10)}</code>
                </div>
              {:else if threeDot}
                <div class="alert-banner" style="margin-top: 12px;">
                  <span>{t("compareModal.noCommonAncestor")}</span>
                </div>
              {/if}

              <div class="overview-prompt">
                {#if summary.files.length > 0}
                  <button
                    type="button"
                    class="btn-primary"
                    onclick={() => void selectFile(summary!.files[0])}
                  >
                    {t("compareModal.selectFilePrompt")}
                  </button>
                {:else}
                  <p class="overview-identical-text">
                    {t("compareModal.identical")}
                  </p>
                {/if}
              </div>
            </div>
          </div>
        {:else}
          <div class="empty-state">
            <span>{t("compareModal.selectFilePrompt")}</span>
          </div>
        {/if}
      </div>
    </div>
  </div>

  <!-- Reference Selector Dialog / Flyout -->
  {#if pickingFor}
    <div
      class="ref-picker-overlay"
      role="presentation"
      onclick={(e) => {
        if (e.target === e.currentTarget) pickingFor = null;
      }}
    >
      <div class="ref-picker-card" role="dialog" aria-modal="true" aria-label={t("compareModal.selectRefTitle")}>
        <div class="ref-picker-header">
          <span class="ref-picker-title">
            {t("compareModal.selectRefTitle")} ({pickingFor === "base" ? t("compareModal.base") : t("compareModal.target")})
          </span>
          <button type="button" class="btn-secondary btn-xs" onclick={() => (pickingFor = null)}>
            {t("common.close")}
          </button>
        </div>

        <!-- Ref Picker Tabs -->
        <div class="ref-picker-tabs">
          <button
            type="button"
            class="ref-tab-btn {refPickerTab === 'branch' ? 'active' : ''}"
            onclick={() => (refPickerTab = "branch")}
          >
            {t("compareModal.tabBranch")}
          </button>
          <button
            type="button"
            class="ref-tab-btn {refPickerTab === 'tag' ? 'active' : ''}"
            onclick={() => (refPickerTab = "tag")}
          >
            {t("compareModal.tabTag")}
          </button>
          <button
            type="button"
            class="ref-tab-btn {refPickerTab === 'commit' ? 'active' : ''}"
            onclick={() => (refPickerTab = "commit")}
          >
            {t("compareModal.tabCommit")}
          </button>
          <button
            type="button"
            class="ref-tab-btn {refPickerTab === 'custom' ? 'active' : ''}"
            onclick={() => (refPickerTab = "custom")}
          >
            Custom SHA / Ref
          </button>
        </div>

        {#if refPickerTab !== "custom"}
          <div class="ref-picker-search">
            <!-- svelte-ignore a11y_autofocus -->
            <input
              type="text"
              class="form-input form-input-sm"
              placeholder={t("compareModal.searchRefPlaceholder")}
              bind:value={refSearchQuery}
              autofocus
            />
          </div>
        {/if}

        <div class="ref-picker-list">
          {#if refPickerTab === "branch"}
            {#if filteredPickerBranches.length === 0}
              <div class="empty-state"><span>{t("common.none")}</span></div>
            {:else}
              {#each filteredPickerBranches as b (b.name)}
                <button
                  type="button"
                  class="ref-picker-row {b.is_current ? 'is-current' : ''}"
                  onclick={() => selectRef(b.name)}
                >
                  <span class="ref-name">{b.name}</span>
                  {#if b.is_remote}
                    <span class="badge-neutral" style="font-size: 10px;">remote</span>
                  {:else if b.is_current}
                    <span class="badge-accent" style="font-size: 10px;">HEAD</span>
                  {/if}
                </button>
              {/each}
            {/if}
          {:else if refPickerTab === "tag"}
            {#if filteredPickerTags.length === 0}
              <div class="empty-state"><span>{t("common.none")}</span></div>
            {:else}
              {#each filteredPickerTags as tag (tag)}
                <button
                  type="button"
                  class="ref-picker-row"
                  onclick={() => selectRef(tag)}
                >
                  <span class="ref-name">{tag}</span>
                  <span class="badge-neutral" style="font-size: 10px;">tag</span>
                </button>
              {/each}
            {/if}
          {:else if refPickerTab === "commit"}
            {#if loadingCommits}
              <div class="empty-state"><span>{t("common.loading")}</span></div>
            {:else if filteredPickerCommits.length === 0}
              <div class="empty-state"><span>{t("common.none")}</span></div>
            {:else}
              {#each filteredPickerCommits as c (c.sha)}
                <button
                  type="button"
                  class="ref-picker-row commit-picker-row"
                  onclick={() => selectRef(c.sha)}
                >
                  <span class="ref-sha tabular-nums">{c.short_sha}</span>
                  <span class="ref-summary" title={c.summary}>{c.summary}</span>
                  <span class="ref-date">{formatRelativeDate(c.date)}</span>
                </button>
              {/each}
            {/if}
          {:else if refPickerTab === "custom"}
            <div class="custom-ref-panel">
              <label class="form-label" for="custom-ref-input">
                Enter branch, tag, full SHA, or revision expression (e.g. HEAD~2):
              </label>
              <div style="display: flex; gap: 8px;">
                <!-- svelte-ignore a11y_autofocus -->
                <input
                  id="custom-ref-input"
                  type="text"
                  class="form-input"
                  placeholder="e.g. main, v1.0.0, a1b2c3d, HEAD~1"
                  bind:value={customRefInput}
                  onkeydown={(e) => {
                    if (e.key === "Enter") applyCustomRef();
                  }}
                  autofocus
                />
                <button type="button" class="btn-primary" onclick={applyCustomRef}>
                  Apply
                </button>
              </div>
            </div>
          {/if}
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .compare-modal-dialog {
    width: min(1200px, calc(100vw - 32px));
    height: min(850px, calc(100vh - 36px));
    max-height: calc(100vh - 36px);
    display: flex;
    flex-direction: column;
    padding: 16px 20px;
    gap: 12px;
  }

  .compare-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-bottom: 8px;
    border-bottom: 1px solid var(--border-default);
    flex-shrink: 0;
  }

  .compare-title-section {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }

  .compare-title-icon {
    color: var(--text-accent);
    flex-shrink: 0;
  }

  .compare-heading {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .compare-subheading {
    font-size: 12px;
    color: var(--text-secondary);
    font-family: var(--font-mono, monospace);
  }

  /* Control Bar */
  .compare-control-bar {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
    background: var(--bg-subtle);
    padding: 8px 12px;
    border-radius: var(--radius-md);
    border: 1px solid var(--border-default);
    flex-shrink: 0;
  }

  .ref-box {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .ref-label {
    font-size: 11px;
    font-weight: 500;
    color: var(--text-muted, var(--text-secondary));
  }

  .ref-pill-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    background: var(--bg-panel);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-sm);
    padding: 4px 10px;
    font-size: 12px;
    font-family: var(--font-mono, monospace);
    font-weight: 500;
    color: var(--text-primary);
    cursor: pointer;
    transition: border-color 0.15s, background-color 0.15s;
    max-width: 200px;
  }

  .ref-pill-btn:hover {
    border-color: var(--border-hover, var(--text-accent));
    background: var(--bg-card);
  }

  .ref-pill-text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .ref-pill-chevron {
    color: var(--text-muted, var(--text-secondary));
    flex-shrink: 0;
  }

  .swap-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border-radius: var(--radius-sm);
    background: var(--bg-panel);
    border: 1px solid var(--border-default);
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.15s;
  }

  .swap-btn:hover {
    background: var(--bg-card);
    color: var(--text-primary);
    border-color: var(--text-accent);
  }

  .mode-seg-group {
    display: inline-flex;
    margin-left: auto;
  }

  .summary-chips {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .chip-identical {
    font-size: 11px;
    font-weight: 500;
    color: var(--diff-add-text, #4ade80);
    background: var(--diff-add-bg, rgba(74, 222, 128, 0.12));
    padding: 2px 8px;
    border-radius: var(--radius-sm);
  }

  .chip-divergence {
    font-size: 11px;
    font-weight: 500;
    color: var(--text-secondary);
    background: var(--bg-panel);
    padding: 2px 8px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-default);
  }

  .chip-stat {
    font-size: 11px;
    font-weight: 600;
    padding: 2px 6px;
    border-radius: var(--radius-sm);
  }

  .chip-ins {
    color: var(--diff-add-text, #4ade80);
    background: var(--diff-add-bg, rgba(74, 222, 128, 0.12));
  }

  .chip-del {
    color: var(--diff-del-text, #f87171);
    background: var(--diff-del-bg, rgba(248, 113, 113, 0.12));
  }

  /* Compare Body Split */
  .compare-body {
    display: flex;
    flex: 1;
    min-height: 0;
    gap: 12px;
    border-radius: var(--radius-md);
    overflow: hidden;
  }

  .compare-sidebar {
    width: 330px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    background: var(--bg-panel);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-md);
    overflow: hidden;
  }

  .sidebar-tabs {
    display: flex;
    border-bottom: 1px solid var(--border-default);
    background: var(--bg-subtle);
  }

  .sidebar-tab {
    flex: 1;
    padding: 8px 10px;
    background: transparent;
    border: none;
    font-size: 11px;
    font-weight: 500;
    color: var(--text-secondary);
    cursor: pointer;
    border-bottom: 2px solid transparent;
    transition: all 0.15s;
    text-align: center;
  }

  .sidebar-tab.active {
    color: var(--text-primary);
    font-weight: 600;
    border-bottom-color: var(--text-accent);
    background: var(--bg-panel);
  }

  .sidebar-search {
    padding: 8px 10px;
    border-bottom: 1px solid var(--border-default);
  }

  .sidebar-list {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
  }

  .compare-file-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    border: none;
    border-bottom: 1px solid var(--border-subtle, rgba(255, 255, 255, 0.04));
    background: transparent;
    color: var(--text-primary);
    text-align: left;
    cursor: pointer;
    font-size: 11px;
    transition: background 0.12s;
  }

  .compare-file-item:hover {
    background: var(--bg-subtle);
  }

  .compare-file-item.is-selected {
    background: var(--bg-card);
    box-shadow: inset 3px 0 0 var(--text-accent);
  }

  .file-status-badge {
    width: 18px;
    height: 18px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 3px;
    font-size: 10px;
    font-weight: 700;
    font-family: var(--font-mono, monospace);
    flex-shrink: 0;
  }

  .badge-added {
    color: var(--diff-add-text, #4ade80);
    background: var(--diff-add-bg, rgba(74, 222, 128, 0.15));
  }

  .badge-deleted {
    color: var(--diff-del-text, #f87171);
    background: var(--diff-del-bg, rgba(248, 113, 113, 0.15));
  }

  .badge-modified {
    color: var(--diff-mod-text, #38bdf8);
    background: var(--diff-mod-bg, rgba(56, 189, 248, 0.15));
  }

  .badge-renamed {
    color: #a78bfa;
    background: rgba(167, 139, 250, 0.15);
  }

  .badge-other {
    color: var(--text-secondary);
    background: var(--bg-subtle);
  }

  .file-path-text {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    direction: rtl;
    text-align: left;
  }

  .file-diff-stat {
    display: inline-flex;
    gap: 4px;
    font-size: 10px;
    flex-shrink: 0;
  }

  .stat-ins {
    color: var(--diff-add-text, #4ade80);
  }

  .stat-del {
    color: var(--diff-del-text, #f87171);
  }

  .compare-commit-item {
    padding: 8px 10px;
    border-bottom: 1px solid var(--border-subtle, rgba(255, 255, 255, 0.04));
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .commit-header-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .commit-sha-btn {
    font-size: 10px;
    font-family: var(--font-mono, monospace);
    background: var(--bg-subtle);
    border: 1px solid var(--border-default);
    border-radius: 3px;
    padding: 1px 4px;
    color: var(--text-accent);
    cursor: pointer;
  }

  .commit-date {
    font-size: 10px;
    color: var(--text-muted, var(--text-secondary));
  }

  .commit-summary {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .commit-author {
    font-size: 10px;
    color: var(--text-secondary);
  }

  /* Right Pane: Diff View / Overview */
  .compare-detail-pane {
    flex: 1;
    min-width: 0;
    background: var(--bg-panel);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-md);
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .diff-wrapper {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .overview-container {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
  }

  .overview-card {
    max-width: 500px;
    width: 100%;
    background: var(--bg-subtle);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-lg);
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 16px;
    text-align: center;
  }

  .overview-title {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .overview-stats-grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 10px;
  }

  .overview-stat-box {
    background: var(--bg-panel);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-md);
    padding: 12px 8px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .stat-val {
    font-size: 16px;
    font-weight: 700;
  }

  .stat-lbl {
    font-size: 10px;
    color: var(--text-secondary);
  }

  .merge-base-info {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    font-size: 11px;
    color: var(--text-secondary);
  }

  .info-sha {
    font-family: var(--font-mono, monospace);
    background: var(--bg-panel);
    padding: 2px 6px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-default);
    color: var(--text-accent);
  }

  .overview-prompt {
    margin-top: 8px;
  }

  .overview-identical-text {
    font-size: 12px;
    color: var(--text-secondary);
  }

  /* Ref Picker Dialog */
  .ref-picker-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.45);
    display: grid;
    place-items: center;
    z-index: 120;
    animation: fade-in 0.08s ease-out;
  }

  .ref-picker-card {
    background: var(--bg-panel);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-lg);
    width: min(440px, calc(100vw - 32px));
    max-height: 480px;
    box-shadow: var(--shadow-modal);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .ref-picker-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 14px;
    border-bottom: 1px solid var(--border-default);
  }

  .ref-picker-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .ref-picker-tabs {
    display: flex;
    border-bottom: 1px solid var(--border-default);
    background: var(--bg-subtle);
  }

  .ref-tab-btn {
    flex: 1;
    padding: 6px 8px;
    font-size: 11px;
    font-weight: 500;
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    color: var(--text-secondary);
    cursor: pointer;
  }

  .ref-tab-btn.active {
    color: var(--text-primary);
    font-weight: 600;
    border-bottom-color: var(--text-accent);
    background: var(--bg-panel);
  }

  .ref-picker-search {
    padding: 8px 12px;
    border-bottom: 1px solid var(--border-default);
  }

  .ref-picker-list {
    flex: 1;
    overflow-y: auto;
    max-height: 300px;
    display: flex;
    flex-direction: column;
  }

  .ref-picker-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 8px 12px;
    background: transparent;
    border: none;
    border-bottom: 1px solid var(--border-subtle, rgba(255, 255, 255, 0.04));
    cursor: pointer;
    text-align: left;
    transition: background 0.1s;
  }

  .ref-picker-row:hover {
    background: var(--bg-subtle);
  }

  .ref-picker-row.is-current {
    font-weight: 600;
  }

  .ref-name {
    font-size: 12px;
    font-family: var(--font-mono, monospace);
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .commit-picker-row {
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
  }

  .ref-sha {
    font-size: 11px;
    font-family: var(--font-mono, monospace);
    color: var(--text-accent);
  }

  .ref-summary {
    font-size: 11px;
    color: var(--text-primary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    width: 100%;
  }

  .ref-date {
    font-size: 10px;
    color: var(--text-muted, var(--text-secondary));
  }

  .custom-ref-panel {
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
</style>
