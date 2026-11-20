<script lang="ts">
  import type { BranchInfo, WorktreeInfo } from "../types/git";
  import * as gitService from "../services/git";
  import { t } from "../i18n/index.svelte";
  import { pickDirectory } from "../utils/path";

  interface Props {
    repoPath: string;
    onClose: () => void;
    onRefreshRepo: () => Promise<void> | void;
    onOpenWorktreeRepo: (path: string) => Promise<void> | void;
    onStatus?: (msg: string, isError?: boolean) => void;
  }

  let {
    repoPath,
    onClose,
    onRefreshRepo,
    onOpenWorktreeRepo,
    onStatus,
  }: Props = $props();

  let worktrees = $state<WorktreeInfo[]>([]);
  let localBranches = $state<BranchInfo[]>([]);
  let loading = $state(false);
  let actionLoading = $state(false);
  let errorMsg = $state("");

  let showAddCard = $state(false);
  let newWtPath = $state("");
  let newWtBranch = $state("");
  let newWtCreateBranch = $state(false);
  let newWtCustomBranch = $state("");

  let lockModalTarget = $state<WorktreeInfo | null>(null);
  let lockReasonInput = $state("");

  let deleteModalTarget = $state<WorktreeInfo | null>(null);
  let forceDeleteCheckbox = $state(false);

  let copiedPath = $state<string | null>(null);

  let branchesInUse = $derived(
    new Set(worktrees.map((w) => w.branch).filter(Boolean) as string[])
  );

  $effect(() => {
    if (repoPath) {
      void loadData();
    }
  });

  function normalizePath(p: string): string {
    return p.replace(/\\/g, "/").replace(/\/+$/, "").toLowerCase();
  }

  function isCurrentWorkspace(wtPath: string): boolean {
    return normalizePath(repoPath) === normalizePath(wtPath);
  }

  async function loadData() {
    loading = true;
    errorMsg = "";
    try {
      const [wts, brs] = await Promise.all([
        gitService.listWorktrees(repoPath),
        gitService.listBranches(repoPath).catch(() => [] as BranchInfo[]),
      ]);
      worktrees = wts;
      localBranches = brs.filter((b) => !b.is_remote);

      const usedBranches = new Set(wts.map((w) => w.branch).filter(Boolean) as string[]);
      const firstAvailable = localBranches.find((b) => !usedBranches.has(b.name));
      if (firstAvailable && (!newWtBranch || usedBranches.has(newWtBranch))) {
        newWtBranch = firstAvailable.name;
        newWtCreateBranch = false;
      } else if (!firstAvailable && localBranches.length > 0) {
        newWtBranch = localBranches[0].name;
        newWtCreateBranch = true;
      }

      if (showAddCard && !newWtPath) {
        suggestPath(newWtCreateBranch ? (newWtCustomBranch || "feature") : newWtBranch);
      }
    } catch (err) {
      errorMsg = String(err);
    } finally {
      loading = false;
    }
  }

  function toggleAddCard() {
    showAddCard = !showAddCard;
    if (showAddCard && !newWtPath) {
      const branchToSuggest = newWtCreateBranch
        ? (newWtCustomBranch.trim() || "feature")
        : (newWtBranch || "worktree");
      suggestPath(branchToSuggest);
    }
  }

  async function handleBrowseFolder() {
    try {
      const dir = await pickDirectory({
        title: t("tools.worktreeBrowseBtn"),
        canCreateDirectories: true,
        defaultPath: repoPath,
      });
      if (dir) {
        newWtPath = dir;
      }
    } catch (err) {
      onStatus?.(String(err), true);
    }
  }

  function suggestPath(branch: string) {
    if (!branch.trim()) return;
    const cleanBranch = branch.trim().replace(/[^a-zA-Z0-9._-]/g, "-");
    const sep = repoPath.includes("\\") ? "\\" : "/";
    const parts = repoPath.split(/[\\/]/).filter(Boolean);
    const parent = parts.slice(0, -1).join(sep) || (sep === "\\" ? "C:\\" : "/");
    const repoName = parts[parts.length - 1] || "worktree";
    newWtPath = `${parent}${sep}${repoName}-${cleanBranch}`;
  }

  async function handleAddWorktree() {
    const branchToUse = newWtCreateBranch ? newWtCustomBranch.trim() : newWtBranch.trim();
    const targetPath = newWtPath.trim();
    if (!targetPath) {
      errorMsg = t("tools.worktreePathPlaceholder");
      return;
    }
    if (!branchToUse) {
      errorMsg = t("tools.worktreeBranchPlaceholder");
      return;
    }

    actionLoading = true;
    errorMsg = "";
    try {
      await gitService.addWorktree(repoPath, targetPath, branchToUse, newWtCreateBranch);
      onStatus?.(t("tools.worktreeAdded"));
      showAddCard = false;
      newWtPath = "";
      newWtCustomBranch = "";
      await loadData();
      await onRefreshRepo();
    } catch (err) {
      errorMsg = String(err);
      onStatus?.(String(err), true);
    } finally {
      actionLoading = false;
    }
  }

  async function handlePruneWorktrees() {
    actionLoading = true;
    errorMsg = "";
    try {
      await gitService.pruneWorktrees(repoPath);
      onStatus?.(t("tools.worktreePruned"));
      await loadData();
      await onRefreshRepo();
    } catch (err) {
      errorMsg = String(err);
      onStatus?.(String(err), true);
    } finally {
      actionLoading = false;
    }
  }

  async function handleReveal(path: string) {
    try {
      await gitService.openInFileManager(path);
    } catch (err) {
      errorMsg = String(err);
      onStatus?.(String(err), true);
    }
  }

  async function handleOpenTerminal(path: string) {
    try {
      await gitService.openInTerminal(path);
    } catch (err) {
      errorMsg = String(err);
      onStatus?.(String(err), true);
    }
  }

  async function submitLock() {
    if (!lockModalTarget) return;
    actionLoading = true;
    try {
      await gitService.lockWorktree(repoPath, lockModalTarget.path, lockReasonInput.trim());
      onStatus?.(t("tools.worktreeLockedMsg"));
      lockModalTarget = null;
      lockReasonInput = "";
      await loadData();
    } catch (err) {
      errorMsg = String(err);
      onStatus?.(String(err), true);
    } finally {
      actionLoading = false;
    }
  }

  async function handleUnlock(wt: WorktreeInfo) {
    actionLoading = true;
    try {
      await gitService.unlockWorktree(repoPath, wt.path);
      onStatus?.(t("tools.worktreeUnlockedMsg"));
      await loadData();
    } catch (err) {
      errorMsg = String(err);
      onStatus?.(String(err), true);
    } finally {
      actionLoading = false;
    }
  }

  async function submitDelete() {
    if (!deleteModalTarget) return;
    actionLoading = true;
    try {
      await gitService.removeWorktree(repoPath, deleteModalTarget.path, forceDeleteCheckbox);
      onStatus?.(t("tools.worktreeRemoved"));
      deleteModalTarget = null;
      forceDeleteCheckbox = false;
      await loadData();
      await onRefreshRepo();
    } catch (err) {
      errorMsg = String(err);
      onStatus?.(String(err), true);
    } finally {
      actionLoading = false;
    }
  }

  async function copyPath(path: string) {
    try {
      await navigator.clipboard.writeText(path);
      copiedPath = path;
      setTimeout(() => {
        if (copiedPath === path) copiedPath = null;
      }, 1500);
    } catch {
      // ignore
    }
  }
</script>

<div
  class="modal-overlay"
  role="presentation"
  tabindex="-1"
  onclick={(e) => {
    if (e.target === e.currentTarget && !actionLoading) onClose();
  }}
  onkeydown={(e) => {
    if (e.key === "Escape" && !actionLoading) {
      if (lockModalTarget) lockModalTarget = null;
      else if (deleteModalTarget) deleteModalTarget = null;
      else onClose();
    }
  }}
>
  <div class="modal-dialog modal-lg worktree-modal" role="dialog" aria-modal="true" aria-label={t("tools.worktreeTitle")}>
    <div class="modal-header">
      <div class="worktree-header-info">
        <div class="worktree-title-row">
          <!-- Semantic Git Worktree icon (branch fork) instead of hamburger menu -->
          <svg class="worktree-title-icon" width="16" height="16" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
            <path d="M11.75 2.5a.75.75 0 1 0 0 1.5.75.75 0 0 0 0-1.5Zm-2.25.75a2.25 2.25 0 1 1 3 2.122V6A2.5 2.5 0 0 1 10 8.5H6a1 1 0 0 0-1 1v1.128a2.251 2.251 0 1 1-1.5 0V5.372a2.25 2.25 0 1 1 1.5 0v1.836A2.493 2.493 0 0 1 6 7h4a1 1 0 0 0 1-1v-.628A2.25 2.25 0 0 1 9.5 3.25ZM4.25 12a.75.75 0 1 0 0 1.5.75.75 0 0 0 0-1.5ZM3.5 3.25a.75.75 0 1 0 1.5 0 .75.75 0 0 0-1.5 0Z" />
          </svg>
          <h3>{t("tools.worktreeTitle")}</h3>
          <span class="worktree-count-badge tabular-nums">
            {worktrees.length}
          </span>
        </div>
        <p class="worktree-subtitle">
          {t("tools.worktreeSubtitle", { count: worktrees.length })}
        </p>
      </div>

      <div class="worktree-header-actions">
        <button
          type="button"
          class="btn-primary btn-xs"
          disabled={loading || actionLoading}
          onclick={toggleAddCard}
        >
          <svg width="11" height="11" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <line x1="8" y1="3" x2="8" y2="13" />
            <line x1="3" y1="8" x2="13" y2="8" />
          </svg>
          <span>{t("tools.worktreeAddBtn")}</span>
        </button>

        <button
          type="button"
          class="btn-secondary btn-xs worktree-refresh-btn {loading ? 'is-spinning' : ''}"
          disabled={loading || actionLoading}
          onclick={() => void loadData()}
          title={t("tools.worktreeRefresh")}
          aria-label={t("tools.worktreeRefresh")}
        >
          <svg width="11" height="11" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
            <path d="M1.705 8.005a.75.75 0 0 1 .834.656 5.5 5.5 0 0 0 9.592 2.97l-1.204-1.204a.25.25 0 0 1 .177-.427h3.646a.25.25 0 0 1 .25.25v3.646a.25.25 0 0 1-.427.177l-1.38-1.38A7 7 0 0 1 1.05 8.84a.75.75 0 0 1 .656-.834ZM8 2.5a5.487 5.487 0 0 0-4.131 1.869l1.204 1.204A.25.25 0 0 1 4.896 6H1.25A.25.25 0 0 1 1 5.75V2.104a.25.25 0 0 1 .427-.177l1.38 1.38A7 7 0 0 1 14.95 7.16a.75.75 0 0 1-1.49.178A5.5 5.5 0 0 0 8 2.5Z" />
          </svg>
          <span>{t("tools.worktreeRefresh")}</span>
        </button>

        <button
          type="button"
          class="btn-secondary btn-xs"
          disabled={loading || actionLoading}
          onclick={handlePruneWorktrees}
          title={t("tools.pruneBtn")}
        >
          <!-- Fixed crisp Octicon trashcan icon -->
          <svg width="11" height="11" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
            <path d="M11 1.75V3h2.25a.75.75 0 0 1 0 1.5H2.75a.75.75 0 0 1 0-1.5H5V1.75C5 .784 5.784 0 6.75 0h2.5C10.216 0 11 .784 11 1.75ZM4.496 6.675l.66 6.6A1.75 1.75 0 0 0 6.898 15h2.204a1.75 1.75 0 0 0 1.742-1.725l.66-6.6a.75.75 0 0 0-1.492-.15l-.66 6.6a.25.25 0 0 1-.249.225H6.898a.25.25 0 0 1-.249-.225l-.66-6.6a.75.75 0 0 0-1.493.15ZM6.5 1.5h3a.25.25 0 0 0 .25-.25V1.75a.25.25 0 0 0-.25-.25h-3a.25.25 0 0 0-.25.25v.25c0 .138.112.25.25.25Z" />
          </svg>
          <span>{t("tools.pruneBtn")}</span>
        </button>

        <button
          type="button"
          class="modal-close-btn"
          onclick={onClose}
          aria-label={t("common.close")}
          disabled={actionLoading}
        >
          <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
            <path d="M3.72 3.72a.75.75 0 0 1 1.06 0L8 6.94l3.22-3.22a.749.749 0 0 1 1.275.326.749.749 0 0 1-.215.734L9.06 8l3.22 3.22a.749.749 0 0 1-.326 1.275.749.749 0 0 1-.734-.215L8 9.06l-3.22 3.22a.751.751 0 0 1-1.042-.018.751.751 0 0 1-.018-1.042L6.94 8 3.72 4.78a.75.75 0 0 1 0-1.06Z" />
          </svg>
        </button>
      </div>
    </div>

    {#if errorMsg}
      <div class="modal-banner is-error">
        <span>{errorMsg}</span>
        <button type="button" class="banner-close" onclick={() => (errorMsg = "")} aria-label={t("common.close")}>✕</button>
      </div>
    {/if}

    <div class="modal-body worktree-modal-body">
      {#if showAddCard}
        <div class="worktree-add-card">
          <div class="add-card-header">
            <h4>{t("tools.worktreeAddBtn")}</h4>
            <button
              type="button"
              class="btn-text btn-xs"
              onclick={() => (showAddCard = false)}
              aria-label={t("common.close")}
            >
              ✕
            </button>
          </div>

          <div class="add-card-form">
            <div class="form-row">
              <label for="wt-branch-source" class="form-label">{t("tools.worktreeBranchSelect")}</label>
              {#if newWtCreateBranch}
                <div class="input-with-action">
                  <input
                    id="wt-branch-source"
                    type="text"
                    class="form-input"
                    bind:value={newWtCustomBranch}
                    placeholder={t("tools.worktreeBranchPlaceholder")}
                    oninput={() => suggestPath(newWtCustomBranch)}
                  />
                </div>
              {:else}
                <select
                  id="wt-branch-source"
                  class="form-select"
                  bind:value={newWtBranch}
                  onchange={() => {
                    if (branchesInUse.has(newWtBranch)) {
                      newWtCreateBranch = true;
                      newWtCustomBranch = `${newWtBranch}-worktree`;
                      suggestPath(newWtCustomBranch);
                    } else {
                      suggestPath(newWtBranch);
                    }
                  }}
                >
                  {#each localBranches as br (br.name)}
                    {@const inUse = branchesInUse.has(br.name)}
                    <option value={br.name}>
                      {inUse ? t("tools.worktreeBranchInUse", { branch: br.name }) : br.name}
                    </option>
                  {/each}
                </select>
              {/if}
              <label class="checkbox-label" style="margin-top: 4px;">
                <input
                  type="checkbox"
                  bind:checked={newWtCreateBranch}
                  onchange={() => {
                    if (newWtCreateBranch) suggestPath(newWtCustomBranch || "feature");
                    else suggestPath(newWtBranch);
                  }}
                />
                <span>{t("tools.worktreeNewBranch")}</span>
              </label>
            </div>

            <div class="form-row">
              <label for="wt-path-input" class="form-label">{t("tools.worktreePathPlaceholder")}</label>
              <div class="input-browse-wrap">
                <input
                  id="wt-path-input"
                  type="text"
                  class="form-input"
                  bind:value={newWtPath}
                  placeholder={t("tools.worktreePathPlaceholder")}
                />
                <button
                  type="button"
                  class="btn-secondary btn-sm"
                  onclick={handleBrowseFolder}
                  disabled={actionLoading}
                >
                  {t("tools.worktreeBrowseBtn")}
                </button>
              </div>
            </div>

            <div class="add-card-footer">
              <button
                type="button"
                class="btn-secondary btn-sm"
                onclick={() => (showAddCard = false)}
                disabled={actionLoading}
              >
                {t("common.cancel")}
              </button>
              <button
                type="button"
                class="btn-primary btn-sm"
                disabled={actionLoading || !newWtPath.trim() || (!newWtCreateBranch && !newWtBranch.trim()) || (newWtCreateBranch && !newWtCustomBranch.trim())}
                onclick={handleAddWorktree}
              >
                {#if actionLoading}
                  <span class="btn-spinner"></span>
                {/if}
                <span>{t("tools.worktreeAddBtn")}</span>
              </button>
            </div>
          </div>
        </div>
      {/if}

      {#if loading && worktrees.length === 0}
        <div class="worktree-empty-state">
          <div class="spinner-ring"></div>
          <span>{t("common.loading")}</span>
        </div>
      {:else if worktrees.length === 0}
        <div class="worktree-empty-state">
          <!-- Semantic Git Worktree icon for empty state -->
          <svg width="32" height="32" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
            <path d="M11.75 2.5a.75.75 0 1 0 0 1.5.75.75 0 0 0 0-1.5Zm-2.25.75a2.25 2.25 0 1 1 3 2.122V6A2.5 2.5 0 0 1 10 8.5H6a1 1 0 0 0-1 1v1.128a2.251 2.251 0 1 1-1.5 0V5.372a2.25 2.25 0 1 1 1.5 0v1.836A2.493 2.493 0 0 1 6 7h4a1 1 0 0 0 1-1v-.628A2.25 2.25 0 0 1 9.5 3.25ZM4.25 12a.75.75 0 1 0 0 1.5.75.75 0 0 0 0-1.5ZM3.5 3.25a.75.75 0 1 0 1.5 0 .75.75 0 0 0-1.5 0Z" />
          </svg>
          <p>{t("tools.worktreesEmpty")}</p>
        </div>
      {:else}
        <div class="worktree-list">
          {#each worktrees as wt (wt.path)}
            {@const isCurrent = isCurrentWorkspace(wt.path)}
            <div class="worktree-card {isCurrent ? 'is-active-workspace' : ''}">
              <div class="worktree-card-main">
                <div class="worktree-card-title-row">
                  <span class="worktree-branch-name" title={wt.branch || "HEAD"}>
                    <svg class="branch-icon" width="13" height="13" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
                      <path d="M5 3.25a.75.75 0 1 1-1.5 0 .75.75 0 0 1 1.5 0Zm0 2.122a2.25 2.25 0 1 0-1.5 0v5.256a2.25 2.25 0 1 0 1.5 0V5.372Zm5.5 5.378a.75.75 0 1 1-1.5 0 .75.75 0 0 1 1.5 0Zm0 2.122a2.25 2.25 0 1 0-1.5 0V8.25a.75.75 0 0 0-.75-.75h-1.5a.75.75 0 0 0-.75.75v.068a2.25 2.25 0 0 0 0 4.304V12.75a.75.75 0 0 0 .75-.75h1.5a2.25 2.25 0 0 1 2.25 2.25v.122Z" />
                    </svg>
                    <span>{wt.branch || (wt.is_detached ? "DETACHED HEAD" : "bare")}</span>
                  </span>

                  {#if isCurrent}
                    <span class="badge-status badge-active" title={t("tools.worktreeActiveWorkspaceTitle")}>
                      {t("tools.worktreeActive")}
                    </span>
                  {/if}

                  {#if wt.is_main}
                    <span class="badge-status badge-main" title={t("tools.worktreeMainRepoTitle")}>
                      {t("tools.worktreeMain")}
                    </span>
                  {/if}

                  {#if wt.is_locked}
                    <span
                      class="badge-status badge-locked"
                      title={wt.lock_reason ? `${t("tools.worktreeLocked")}: ${wt.lock_reason}` : t("tools.worktreeLocked")}
                    >
                      <svg width="10" height="10" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
                        <path d="M4 4a4 4 0 0 1 8 0v2h.25c.966 0 1.75.784 1.75 1.75v5.5A1.75 1.75 0 0 1 12.25 15h-8.5A1.75 1.75 0 0 1 2 13.25v-5.5C2 6.784 2.784 6 3.75 6H4V4Zm2 2h4V4a2 2 0 1 0-4 0v2Z" />
                      </svg>
                      <span>{t("tools.worktreeLocked")}</span>
                    </span>
                  {/if}

                  {#if wt.is_prunable}
                    <span
                      class="badge-status badge-prunable"
                      title={wt.prunable_reason ? `${t("tools.worktreePrunable")}: ${wt.prunable_reason}` : t("tools.worktreePrunable")}
                    >
                      <svg width="10" height="10" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
                        <path d="M6.457 1.047c.659-1.234 2.427-1.234 3.086 0l6.082 11.378A1.75 1.75 0 0 1 14.082 15H1.918a1.75 1.75 0 0 1-1.543-2.575Zm1.763.707a.25.25 0 0 0-.44 0L1.698 13.132a.25.25 0 0 0 .22.368h12.164a.25.25 0 0 0 .22-.368Zm.53 3.996v2.5a.75.75 0 0 1-1.5 0v-2.5a.75.75 0 0 1 1.5 0ZM9 11a1 1 0 1 1-2 0 1 1 0 0 1 2 0Z" />
                      </svg>
                      <span>{t("tools.worktreePrunable")}</span>
                    </span>
                  {/if}

                  <span class="worktree-sha tabular-nums" title={wt.head}>
                    {wt.head.slice(0, 7)}
                  </span>
                </div>

                <div class="worktree-path-row">
                  <span class="worktree-path-text" title={wt.path}>{wt.path}</span>
                  <button
                    type="button"
                    class="btn-icon-copy"
                    onclick={() => copyPath(wt.path)}
                    title={copiedPath === wt.path ? t("tools.worktreePathCopied") : t("tools.worktreeCopyPath")}
                    aria-label={t("tools.worktreeCopyPath")}
                  >
                    {#if copiedPath === wt.path}
                      <svg width="11" height="11" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
                        <path d="M13.78 4.22a.75.75 0 0 1 0 1.06l-7.25 7.25a.75.75 0 0 1-1.06 0L2.22 9.28a.751.751 0 0 1 .018-1.042.751.751 0 0 1 1.042-.018L6 10.94l6.72-6.72a.75.75 0 0 1 1.06 0Z" />
                      </svg>
                    {:else}
                      <svg width="11" height="11" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
                        <path d="M0 6.75C0 5.784.784 5 1.75 5h1.5a.75.75 0 0 1 0 1.5h-1.5a.25.25 0 0 0-.25.25v7.5c0 .138.112.25.25.25h7.5a.25.25 0 0 0 .25-.25v-1.5a.75.75 0 0 1 1.5 0v1.5A1.75 1.75 0 0 1 9.25 16h-7.5A1.75 1.75 0 0 1 0 14.25Z" />
                        <path d="M5 1.75C5 .784 5.784 0 6.75 0h7.5C15.216 0 16 .784 16 1.75v7.5A1.75 1.75 0 0 1 14.25 11h-7.5A1.75 1.75 0 0 1 5 9.25Zm1.75-.25a.25.25 0 0 0-.25.25v7.5c0 .138.112.25.25.25h7.5a.25.25 0 0 0 .25-.25v-7.5a.25.25 0 0 0-.25-.25Z" />
                      </svg>
                    {/if}
                  </button>
                </div>
              </div>

              <div class="worktree-card-actions">
                {#if !isCurrent && !wt.is_bare}
                  <button
                    type="button"
                    class="btn-primary btn-xs"
                    disabled={actionLoading || wt.is_prunable}
                    onclick={async () => {
                      onClose();
                      await onOpenWorktreeRepo(wt.path);
                    }}
                    title={wt.is_prunable ? t("tools.worktreeFolderMissing") : t("tools.worktreeOpenWorkspace")}
                  >
                    <svg width="11" height="11" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
                      <path d="M8.22 2.97a.75.75 0 0 1 1.06 0l4.25 4.25a.75.75 0 0 1 0 1.06l-4.25 4.25a.751.751 0 0 1-1.042-.018.751.751 0 0 1-.018-1.042L11.44 9H2.75a.75.75 0 0 1 0-1.5h8.69L8.22 4.03a.75.75 0 0 1 0-1.06Z" />
                    </svg>
                    <span>{t("tools.worktreeOpenWorkspace")}</span>
                  </button>
                {/if}

                <button
                  type="button"
                  class="btn-secondary btn-xs"
                  disabled={actionLoading || wt.is_prunable}
                  onclick={() => handleReveal(wt.path)}
                  title={wt.is_prunable ? t("tools.worktreeFolderMissing") : t("tools.worktreeReveal")}
                  aria-label={t("tools.worktreeReveal")}
                >
                  <svg width="11" height="11" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
                    <path d="M1.75 1A1.75 1.75 0 0 0 0 2.75v10.5C0 14.216.784 15 1.75 15h12.5A1.75 1.75 0 0 0 16 13.25v-8.5A1.75 1.75 0 0 0 14.25 3H7.5a.25.25 0 0 1-.2-.1l-.9-1.2C6.07 1.26 5.55 1 5 1H1.75Z" />
                  </svg>
                  <span>{t("tools.worktreeReveal")}</span>
                </button>

                <button
                  type="button"
                  class="btn-secondary btn-xs"
                  disabled={actionLoading || wt.is_prunable}
                  onclick={() => handleOpenTerminal(wt.path)}
                  title={wt.is_prunable ? t("tools.worktreeFolderMissing") : t("tools.worktreeTerminal")}
                  aria-label={t("tools.worktreeTerminal")}
                >
                  <svg width="11" height="11" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
                    <path d="M0 2.75C0 1.784.784 1 1.75 1h12.5c.966 0 1.75.784 1.75 1.75v10.5A1.75 1.75 0 0 1 14.25 15H1.75A1.75 1.75 0 0 1 0 13.25Zm1.75-.25a.25.25 0 0 0-.25.25v10.5c0 .138.112.25.25.25h12.5a.25.25 0 0 0 .25-.25V2.75a.25.25 0 0 0-.25-.25ZM7.28 7.78 4.03 4.53a.75.75 0 0 0-1.06 1.06L5.69 8.5 2.97 11.22a.75.75 0 1 0 1.06 1.06l3.25-3.25a.75.75 0 0 0 0-1.06ZM8.75 11h4.5a.75.75 0 0 1 0 1.5h-4.5a.75.75 0 0 1 0-1.5Z" />
                  </svg>
                  <span>{t("tools.worktreeTerminal")}</span>
                </button>

                {#if !wt.is_main}
                  {#if wt.is_locked}
                    <button
                      type="button"
                      class="btn-secondary btn-xs"
                      disabled={actionLoading}
                      onclick={() => handleUnlock(wt)}
                      title={t("tools.worktreeUnlockBtn")}
                    >
                      <svg width="11" height="11" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
                        <path d="M10 2a3 3 0 0 0-3 3v2h5.25c.966 0 1.75.784 1.75 1.75v5.5A1.75 1.75 0 0 1 12.25 15h-8.5A1.75 1.75 0 0 1 2 13.25v-5.5C2 6.784 2.784 6 3.75 6H5.5V5a4.5 4.5 0 0 1 9 0 .75.75 0 0 1-1.5 0 3 3 0 0 0-3-3Z" />
                      </svg>
                      <span>{t("tools.worktreeUnlockBtn")}</span>
                    </button>
                  {:else}
                    <button
                      type="button"
                      class="btn-secondary btn-xs"
                      disabled={actionLoading}
                      onclick={() => {
                        lockModalTarget = wt;
                        lockReasonInput = "";
                      }}
                      title={t("tools.worktreeLockBtn")}
                    >
                      <svg width="11" height="11" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
                        <path d="M4 4a4 4 0 0 1 8 0v2h.25c.966 0 1.75.784 1.75 1.75v5.5A1.75 1.75 0 0 1 12.25 15h-8.5A1.75 1.75 0 0 1 2 13.25v-5.5C2 6.784 2.784 6 3.75 6H4V4Zm2 2h4V4a2 2 0 1 0-4 0v2Z" />
                      </svg>
                      <span>{t("tools.worktreeLockBtn")}</span>
                    </button>
                  {/if}

                  <button
                    type="button"
                    class="btn-secondary btn-xs btn-danger"
                    disabled={actionLoading}
                    onclick={() => {
                      deleteModalTarget = wt;
                      forceDeleteCheckbox = false;
                    }}
                    title={t("common.delete")}
                  >
                    <svg width="11" height="11" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
                      <path d="M11 1.75V3h2.25a.75.75 0 0 1 0 1.5H2.75a.75.75 0 0 1 0-1.5H5V1.75C5 .784 5.784 0 6.75 0h2.5C10.216 0 11 .784 11 1.75ZM4.496 6.675l.66 6.6A1.75 1.75 0 0 0 6.898 15h2.204a1.75 1.75 0 0 0 1.742-1.725l.66-6.6a.75.75 0 0 0-1.492-.15l-.66 6.6a.25.25 0 0 1-.249.225H6.898a.25.25 0 0 1-.249-.225l-.66-6.6a.75.75 0 0 0-1.493.15ZM6.5 1.5h3a.25.25 0 0 0 .25-.25V1.75a.25.25 0 0 0-.25-.25h-3a.25.25 0 0 0-.25.25v.25c0 .138.112.25.25.25Z" />
                    </svg>
                    <span>{t("common.delete")}</span>
                  </button>
                {/if}
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  </div>
</div>

{#if lockModalTarget}
  <div
    class="modal-overlay modal-sub-overlay"
    role="presentation"
    onclick={(e) => {
      if (e.target === e.currentTarget && !actionLoading) lockModalTarget = null;
    }}
  >
    <div class="modal-dialog modal-sm" role="dialog" aria-modal="true" aria-label={t("tools.worktreeLockBtn")}>
      <div class="modal-header">
        <h4>{t("tools.worktreeLockBtn")}</h4>
        <button type="button" class="modal-close-btn" onclick={() => (lockModalTarget = null)} aria-label={t("common.close")}>✕</button>
      </div>
      <div class="modal-body" style="display: flex; flex-direction: column; gap: 10px;">
        <p style="font-size: 11px; color: var(--text-muted);">
          {lockModalTarget.path}
        </p>
        <input
          type="text"
          class="form-input"
          placeholder={t("tools.worktreeLockReasonPlaceholder")}
          bind:value={lockReasonInput}
        />
        <div style="display: flex; justify-content: flex-end; gap: 6px; margin-top: 6px;">
          <button type="button" class="btn-secondary btn-xs" onclick={() => (lockModalTarget = null)}>
            {t("common.cancel")}
          </button>
          <button type="button" class="btn-primary btn-xs" disabled={actionLoading} onclick={submitLock}>
            {t("tools.worktreeLockBtn")}
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}

{#if deleteModalTarget}
  <div
    class="modal-overlay modal-sub-overlay"
    role="presentation"
    onclick={(e) => {
      if (e.target === e.currentTarget && !actionLoading) deleteModalTarget = null;
    }}
  >
    <div class="modal-dialog modal-sm" role="dialog" aria-modal="true" aria-label={t("common.delete")}>
      <div class="modal-header">
        <h4>{t("common.delete")}</h4>
        <button type="button" class="modal-close-btn" onclick={() => (deleteModalTarget = null)} aria-label={t("common.close")}>✕</button>
      </div>
      <div class="modal-body" style="display: flex; flex-direction: column; gap: 10px;">
        <p style="font-size: 11.5px; color: var(--text-primary);">
          {t("tools.worktreeConfirmDelete", { path: deleteModalTarget.path })}
        </p>
        <label class="checkbox-label">
          <input type="checkbox" bind:checked={forceDeleteCheckbox} />
          <span>{t("tools.worktreeForceDelete")}</span>
        </label>
        <div style="display: flex; justify-content: flex-end; gap: 6px; margin-top: 6px;">
          <button type="button" class="btn-secondary btn-xs" onclick={() => (deleteModalTarget = null)}>
            {t("common.cancel")}
          </button>
          <button type="button" class="btn-primary btn-xs btn-danger" disabled={actionLoading} onclick={submitDelete}>
            {t("common.delete")}
          </button>
        </div>
      </div>
    </div>
  </div>
{/if}