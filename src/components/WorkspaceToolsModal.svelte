<script lang="ts">
  import type { ReflogEntry, SubmoduleInfo, WorktreeInfo } from "../types/git";
  import * as gitService from "../services/git";
  import { t } from "../i18n/index.svelte";

  interface Props {
    repoPath: string;
    initialTab?: "reflog" | "submodules" | "worktrees";
    onClose: () => void;
    onRefreshRepo: () => Promise<void> | void;
    onOpenWorktreeRepo: (path: string) => Promise<void> | void;
    onStatus?: (msg: string, isError?: boolean) => void;
  }

  let {
    repoPath,
    initialTab = "reflog",
    onClose,
    onRefreshRepo,
    onOpenWorktreeRepo,
    onStatus,
  }: Props = $props();

  let activeTab = $state<"reflog" | "submodules" | "worktrees">("reflog");
  let loading = $state(false);
  let errorMsg = $state("");

  $effect(() => {
    activeTab = initialTab;
  });

  let reflogEntries = $state<ReflogEntry[]>([]);
  let reflogSearch = $state("");
  let hardResetTarget = $state<ReflogEntry | null>(null);

  let submodules = $state<SubmoduleInfo[]>([]);
  let worktrees = $state<WorktreeInfo[]>([]);

  let newSubUrl = $state("");
  let newSubPath = $state("");

  let newWtPath = $state("");
  let newWtBranch = $state("");
  let newWtCreateBranch = $state(true);

  let filteredReflog = $derived.by(() => {
    const q = reflogSearch.trim().toLowerCase();
    if (!q) return reflogEntries;
    return reflogEntries.filter(
      (e) =>
        e.action.toLowerCase().includes(q) ||
        e.short_sha.toLowerCase().includes(q) ||
        e.selector.toLowerCase().includes(q) ||
        (e.author && e.author.toLowerCase().includes(q))
    );
  });

  function getActionBadgeInfo(action: string): { label: string; class: string } {
    const lower = action.toLowerCase();
    if (lower.startsWith("commit:") || lower.startsWith("commit (initial):")) {
      return { label: "commit", class: "badge-reflog-commit" };
    }
    if (lower.startsWith("checkout:")) {
      return { label: "checkout", class: "badge-reflog-checkout" };
    }
    if (lower.startsWith("reset:")) {
      return { label: "reset", class: "badge-reflog-reset" };
    }
    if (lower.startsWith("cherry-pick:")) {
      return { label: "cherry-pick", class: "badge-reflog-cherry-pick" };
    }
    if (lower.startsWith("merge")) {
      return { label: "merge", class: "badge-reflog-merge" };
    }
    if (lower.startsWith("rebase")) {
      return { label: "rebase", class: "badge-reflog-rebase" };
    }
    return { label: "other", class: "badge-reflog-other" };
  }

  $effect(() => {
    if (repoPath && activeTab) {
      void loadTab(activeTab);
    }
  });

  async function loadTab(tab: "reflog" | "submodules" | "worktrees") {
    loading = true;
    errorMsg = "";
    try {
      if (tab === "reflog") {
        reflogEntries = await gitService.listReflog(repoPath, 80);
      } else if (tab === "submodules") {
        submodules = await gitService.listSubmodules(repoPath);
      } else {
        worktrees = await gitService.listWorktrees(repoPath);
      }
    } catch (err) {
      errorMsg = String(err);
    } finally {
      loading = false;
    }
  }

  async function handleReflogReset(sha: string, hard: boolean) {
    loading = true;
    errorMsg = "";
    try {
      await gitService.resetToCommit(repoPath, sha, hard);
      onStatus?.(t("tools.reflogResetDone", { sha: sha.slice(0, 7) }));
      await onRefreshRepo();
      await loadTab("reflog");
    } catch (err) {
      errorMsg = String(err);
      onStatus?.(String(err), true);
    } finally {
      loading = false;
    }
  }

  async function confirmHardReset() {
    if (!hardResetTarget) return;
    const target = hardResetTarget;
    hardResetTarget = null;
    await handleReflogReset(target.sha, true);
  }

  async function handleReflogCherryPick(sha: string) {
    try {
      await gitService.cherryPick(repoPath, sha);
      onStatus?.(t("tools.reflogCherryPickDone", { sha: sha.slice(0, 7) }));
      await onRefreshRepo();
    } catch (err) {
      errorMsg = String(err);
      onStatus?.(String(err), true);
    }
  }

  async function handleAddSubmodule() {
    if (!newSubUrl.trim() || !newSubPath.trim()) return;
    loading = true;
    errorMsg = "";
    try {
      await gitService.addSubmodule(repoPath, newSubUrl.trim(), newSubPath.trim());
      newSubUrl = "";
      newSubPath = "";
      onStatus?.(t("tools.submoduleAdded"));
      await loadTab("submodules");
      await onRefreshRepo();
    } catch (err) {
      errorMsg = String(err);
      onStatus?.(String(err), true);
    } finally {
      loading = false;
    }
  }

  async function handleUpdateSubmodules() {
    loading = true;
    errorMsg = "";
    try {
      await gitService.updateSubmodules(repoPath, true, true);
      onStatus?.(t("tools.submodulesUpdated"));
      await loadTab("submodules");
      await onRefreshRepo();
    } catch (err) {
      errorMsg = String(err);
      onStatus?.(String(err), true);
    } finally {
      loading = false;
    }
  }

  async function handleSyncSubmodules() {
    loading = true;
    errorMsg = "";
    try {
      await gitService.syncSubmodules(repoPath);
      onStatus?.(t("tools.submodulesSynced"));
      await loadTab("submodules");
    } catch (err) {
      errorMsg = String(err);
      onStatus?.(String(err), true);
    } finally {
      loading = false;
    }
  }

  async function handleAddWorktree() {
    if (!newWtPath.trim() || !newWtBranch.trim()) return;
    loading = true;
    errorMsg = "";
    try {
      await gitService.addWorktree(repoPath, newWtPath.trim(), newWtBranch.trim(), newWtCreateBranch);
      newWtPath = "";
      newWtBranch = "";
      onStatus?.(t("tools.worktreeAdded"));
      await loadTab("worktrees");
    } catch (err) {
      errorMsg = String(err);
      onStatus?.(String(err), true);
    } finally {
      loading = false;
    }
  }

  function normalizePath(p: string): string {
    return p.replace(/\\/g, "/").replace(/\/+$/, "").toLowerCase();
  }

  function isCurrentWorkspace(wtPath: string): boolean {
    return normalizePath(repoPath) === normalizePath(wtPath);
  }

  async function handleRemoveWorktree(path: string, force = false) {
    if (typeof window !== "undefined" && !window.confirm(t("tools.worktreeConfirmDelete", { path }))) {
      return;
    }
    loading = true;
    errorMsg = "";
    try {
      await gitService.removeWorktree(repoPath, path, force);
      onStatus?.(t("tools.worktreeRemoved"));
      await loadTab("worktrees");
    } catch (err) {
      errorMsg = String(err);
      onStatus?.(String(err), true);
    } finally {
      loading = false;
    }
  }

  async function handlePruneWorktrees() {
    loading = true;
    errorMsg = "";
    try {
      await gitService.pruneWorktrees(repoPath);
      onStatus?.(t("tools.worktreePruned"));
      await loadTab("worktrees");
    } catch (err) {
      errorMsg = String(err);
      onStatus?.(String(err), true);
    } finally {
      loading = false;
    }
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
  <div class="modal-dialog modal-lg" role="dialog" aria-modal="true" aria-label={t("tools.title")}>
    <div class="modal-header">
      <div style="display: flex; align-items: center; gap: 10px;">
        <h3>{t("tools.title")}</h3>
        <div class="seg-group">
          <button
            type="button"
            class="seg-btn {activeTab === 'reflog' ? 'active' : ''}"
            onclick={() => {
              activeTab = "reflog";
            }}
          >
            {t("tools.tabReflog")}
          </button>
          <button
            type="button"
            class="seg-btn {activeTab === 'submodules' ? 'active' : ''}"
            onclick={() => {
              activeTab = "submodules";
            }}
          >
            {t("tools.tabSubmodules")}
          </button>
          <button
            type="button"
            class="seg-btn {activeTab === 'worktrees' ? 'active' : ''}"
            onclick={() => {
              activeTab = "worktrees";
            }}
          >
            {t("tools.tabWorktrees")}
          </button>
        </div>
      </div>
      <button type="button" class="btn-secondary btn-xs" onclick={onClose}>{t("common.close")}</button>
    </div>

    <div class="modal-body">
      {#if errorMsg}
        <div class="alert-banner" style="border-radius: var(--radius-sm);">
          <span>{errorMsg}</span>
        </div>
      {/if}

      {#if activeTab === "reflog"}
        <div style="font-size: 11px; color: var(--text-muted); margin-bottom: 8px;">
          {t("tools.reflogDesc")}
        </div>

        <div class="reflog-search-row">
          <input
            type="search"
            class="form-input"
            style="flex: 1;"
            bind:value={reflogSearch}
            placeholder={t("tools.reflogSearchPlaceholder")}
          />
          <button
            type="button"
            class="btn-secondary btn-xs"
            disabled={loading}
            onclick={() => loadTab("reflog")}
            title={t("common.refresh")}
            aria-label={t("common.refresh")}
            style="display: flex; align-items: center; gap: 4px; height: 32px; padding: 0 10px;"
          >
            <svg class={loading ? "spin" : ""} width="12" height="12" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
              <path d="M1.705 8.001a6.3 6.3 0 0 1 10.47-4.664.75.75 0 0 1-.99 1.127 4.8 4.8 0 1 0 1.25 5.37.75.75 0 1 1 1.42.5 6.3 6.3 0 0 1-12.15-2.333Z" />
              <path d="m13.25 1.5.02 3.75-3.75-.02a.75.75 0 0 1 0-1.5l2.25.01-.01-2.24a.75.75 0 0 1 1.49 0Z" />
            </svg>
            <span>{t("common.refresh")}</span>
          </button>
        </div>

        {#if loading && reflogEntries.length === 0}
          <div class="empty-state" style="padding: 32px 16px;">
            <div class="spinner-ring" style="width: 22px; height: 22px; border-width: 2px; margin-bottom: 8px;"></div>
            <span style="font-size: 12px; color: var(--text-muted);">{t("common.loading")}</span>
          </div>
        {:else if reflogEntries.length === 0}
          <div class="empty-state"><span>{t("tools.reflogEmpty")}</span></div>
        {:else if filteredReflog.length === 0}
          <div class="empty-state"><span>{t("tools.reflogNoMatch")}</span></div>
        {:else}
          <div class="list-box" style="max-height: 380px;">
            {#each filteredReflog as entry, i (entry.sha + '-' + entry.selector + '-' + i)}
              {@const badge = getActionBadgeInfo(entry.action)}
              <div class="list-row">
                <div style="min-width: 0; flex: 1;">
                  <div style="display: flex; align-items: center; gap: 8px; flex-wrap: wrap;">
                    <span class="commit-sha">{entry.short_sha}</span>
                    <span class="badge-neutral" style="font-family: var(--font-mono, monospace); font-size: 10px;">{entry.selector}</span>
                    <span class="badge-reflog {badge.class}">{badge.label}</span>
                    <span style="font-size: 11px; color: var(--text-muted);">{entry.date_relative}</span>
                    {#if entry.author}
                      <span style="font-size: 11px; color: var(--text-muted); opacity: 0.8;">• {entry.author}</span>
                    {/if}
                  </div>
                  <div style="font-size: 12px; color: var(--text-primary); margin-top: 3px; line-height: 1.4; word-break: break-word;">
                    {entry.action}
                  </div>
                </div>
                <div style="display: flex; gap: 4px; flex-shrink: 0; align-items: flex-start; margin-top: 2px;">
                  <button
                    type="button"
                    class="btn-secondary btn-xs"
                    disabled={loading}
                    onclick={() => handleReflogCherryPick(entry.sha)}
                  >
                    {t("history.cherryPickBtn")}
                  </button>
                  <button
                    type="button"
                    class="btn-secondary btn-xs"
                    disabled={loading}
                    onclick={() => handleReflogReset(entry.sha, false)}
                  >
                    {t("tools.softResetBtn")}
                  </button>
                  <button
                    type="button"
                    class="btn-secondary btn-xs btn-danger"
                    disabled={loading}
                    onclick={() => { hardResetTarget = entry; }}
                  >
                    {t("tools.hardResetBtn")}
                  </button>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      {:else if activeTab === "submodules"}
        <div style="display: flex; gap: 6px; align-items: center; flex-wrap: wrap;">
          <input
            type="text"
            class="form-input"
            style="flex: 2; min-width: 180px;"
            bind:value={newSubUrl}
            placeholder={t("tools.submoduleUrlPlaceholder")}
          />
          <input
            type="text"
            class="form-input"
            style="flex: 1; min-width: 120px;"
            bind:value={newSubPath}
            placeholder={t("tools.submodulePathPlaceholder")}
          />
          <button
            type="button"
            class="btn-primary btn-xs"
            disabled={loading || !newSubUrl.trim() || !newSubPath.trim()}
            onclick={handleAddSubmodule}
          >
            {t("tools.addBtn")}
          </button>
          <button type="button" class="btn-secondary btn-xs" disabled={loading} onclick={handleUpdateSubmodules}>
            {t("tools.updateSubmodulesBtn")}
          </button>
          <button type="button" class="btn-secondary btn-xs" disabled={loading} onclick={handleSyncSubmodules}>
            {t("tools.syncSubmodulesBtn")}
          </button>
        </div>

        {#if loading && submodules.length === 0}
          <div class="empty-state"><span>{t("common.loading")}</span></div>
        {:else if submodules.length === 0}
          <div class="empty-state"><span>{t("tools.submodulesEmpty")}</span></div>
        {:else}
          <div class="list-box" style="max-height: 320px;">
            {#each submodules as sub (sub.path)}
              <div class="list-row">
                <div style="min-width: 0; flex: 1;">
                  <div style="display: flex; align-items: center; gap: 8px;">
                    <span style="font-weight: 600; font-size: 12px;">{sub.path}</span>
                    <span class="badge-neutral">{sub.status}</span>
                    <span class="commit-sha">{sub.sha.slice(0, 7)}</span>
                  </div>
                  {#if sub.url}
                    <div style="font-size: 11px; color: var(--text-muted); margin-top: 2px;">{sub.url}</div>
                  {/if}
                </div>
              </div>
            {/each}
          </div>
        {/if}
      {:else}
        <div style="display: flex; flex-direction: column; gap: 8px;">
          <div style="display: flex; gap: 6px; align-items: center; flex-wrap: wrap;">
            <input
              type="text"
              class="form-input"
              style="flex: 2; min-width: 180px;"
              bind:value={newWtPath}
              placeholder={t("tools.worktreePathPlaceholder")}
            />
            <input
              type="text"
              class="form-input"
              style="flex: 1; min-width: 120px;"
              bind:value={newWtBranch}
              placeholder={t("tools.worktreeBranchPlaceholder")}
            />
            <label class="checkbox-label" style="white-space: nowrap;">
              <input type="checkbox" bind:checked={newWtCreateBranch} />
              <span>{t("tools.worktreeNewBranch")}</span>
            </label>
            <button
              type="button"
              class="btn-primary btn-xs"
              disabled={loading || !newWtPath.trim() || !newWtBranch.trim()}
              onclick={handleAddWorktree}
            >
              {t("tools.addBtn")}
            </button>
            <button type="button" class="btn-secondary btn-xs" disabled={loading} onclick={handlePruneWorktrees} title={t("tools.pruneBtn")}>
              <svg width="11" height="11" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
                <path d="M11 1.75V3h2.25a.75.75 0 0 1 0 1.5H2.75a.75.75 0 0 1 0-1.5H5V1.75C5 .784 5.784 0 6.75 0h2.5C10.216 0 11 .784 11 1.75ZM4.496 6.675l.66 6.6A1.75 1.75 0 0 0 6.898 15h2.204a1.75 1.75 0 0 0 1.742-1.725l.66-6.6a.75.75 0 0 0-1.492-.15l-.66 6.6a.25.25 0 0 1-.249.225H6.898a.25.25 0 0 1-.249-.225l-.66-6.6a.75.75 0 0 0-1.493.15ZM6.5 1.5h3a.25.25 0 0 0 .25-.25V1.75a.25.25 0 0 0-.25-.25h-3a.25.25 0 0 0-.25.25v.25c0 .138.112.25.25.25Z" />
              </svg>
              <span>{t("tools.pruneBtn")}</span>
            </button>
          </div>
        </div>

        {#if loading && worktrees.length === 0}
          <div class="empty-state"><span>{t("common.loading")}</span></div>
        {:else if worktrees.length === 0}
          <div class="empty-state"><span>{t("tools.worktreesEmpty")}</span></div>
        {:else}
          <div class="list-box" style="max-height: 320px;">
            {#each worktrees as wt (wt.path)}
              {@const isCurrent = isCurrentWorkspace(wt.path)}
              <div class="list-row {isCurrent ? 'is-active-workspace' : ''}">
                <div style="min-width: 0; flex: 1;">
                  <div style="display: flex; align-items: center; gap: 8px; flex-wrap: wrap;">
                    <span style="font-weight: 600; font-size: 12px;">{wt.branch || (wt.is_detached ? "DETACHED" : "bare")}</span>
                    {#if isCurrent}
                      <span class="badge-status badge-active" style="font-size: 9px; padding: 0 5px;">{t("tools.worktreeActive")}</span>
                    {/if}
                    {#if wt.is_main}
                      <span class="badge-neutral">{t("tools.worktreeMain")}</span>
                    {/if}
                    {#if wt.is_locked}
                      <span class="badge-neutral">{t("tools.worktreeLocked")}</span>
                    {/if}
                    {#if wt.is_prunable}
                      <span class="badge-status badge-prunable" style="font-size: 9px; padding: 0 5px;" title={wt.prunable_reason || t("tools.worktreePrunable")}>
                        {t("tools.worktreePrunable")}
                      </span>
                    {/if}
                    <span class="commit-sha">{wt.head.slice(0, 7)}</span>
                  </div>
                  <div style="font-size: 11px; color: var(--text-muted); margin-top: 2px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
                    {wt.path}
                  </div>
                </div>
                <div style="display: flex; gap: 4px; flex-shrink: 0; align-items: center;">
                  {#if !isCurrent && !wt.is_bare}
                    <button
                      type="button"
                      class="btn-secondary btn-xs"
                      disabled={wt.is_prunable}
                      title={wt.is_prunable ? t("tools.worktreeFolderMissing") : t("common.open")}
                      onclick={async () => {
                        onClose();
                        await onOpenWorktreeRepo(wt.path);
                      }}
                    >
                      {t("common.open")}
                    </button>
                  {/if}
                  {#if !wt.is_main}
                    <button
                      type="button"
                      class="btn-secondary btn-xs btn-danger"
                      onclick={() => handleRemoveWorktree(wt.path, false)}
                    >
                      {t("common.delete")}
                    </button>
                  {/if}
                </div>
              </div>
            {/each}
          </div>
        {/if}
      {/if}
    </div>
  </div>
</div>

{#if hardResetTarget}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="modal-overlay"
    style="z-index: 1100; background: rgba(0, 0, 0, 0.65);"
    onclick={(e) => {
      if (e.target === e.currentTarget) hardResetTarget = null;
    }}
    onkeydown={(e) => {
      if (e.key === "Escape") hardResetTarget = null;
    }}
  >
    <div class="modal-dialog modal-sm" role="dialog" aria-modal="true" aria-label={t("tools.reflogConfirmHardResetTitle")}>
      <div class="modal-header">
        <h4 style="margin: 0; font-size: 13px; font-weight: 600; color: var(--danger-color, #ef4444); display: flex; align-items: center; gap: 6px;">
          <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
            <path d="M6.457 1.047c.659-1.234 2.427-1.234 3.086 0l6.082 11.378A1.75 1.75 0 0 1 14.082 15H1.918a1.75 1.75 0 0 1-1.543-2.575Zm1.763.707a.25.25 0 0 0-.44 0L1.698 13.132a.25.25 0 0 0 .22.368h12.164a.25.25 0 0 0 .22-.368Zm.53 3.996v2.5a.75.75 0 0 1-1.5 0v-2.5a.75.75 0 0 1 1.5 0ZM9 11a1 1 0 1 1-2 0 1 1 0 0 1 2 0Z"/>
          </svg>
          {t("tools.reflogConfirmHardResetTitle")}
        </h4>
        <button type="button" class="btn-ghost btn-xs" onclick={() => { hardResetTarget = null; }}>✕</button>
      </div>
      <div class="modal-body" style="font-size: 12px; line-height: 1.5; color: var(--text-primary); display: flex; flex-direction: column; gap: 10px;">
        <p style="margin: 0;">
          {t("tools.reflogConfirmHardReset", {
            sha: hardResetTarget.short_sha,
            selector: hardResetTarget.selector
          })}
        </p>
        <div style="background: var(--bg-secondary); border: 1px solid var(--border-color); border-radius: var(--radius-sm); padding: 8px; font-size: 11px;">
          <div style="display: flex; gap: 6px; align-items: center; margin-bottom: 4px;">
            <span class="commit-sha">{hardResetTarget.short_sha}</span>
            <span class="badge-neutral">{hardResetTarget.selector}</span>
          </div>
          <div style="color: var(--text-muted); word-break: break-word;">{hardResetTarget.action}</div>
        </div>
      </div>
      <div class="modal-footer" style="display: flex; justify-content: flex-end; gap: 8px;">
        <button type="button" class="btn-secondary btn-sm" onclick={() => { hardResetTarget = null; }}>
          {t("common.cancel")}
        </button>
        <button type="button" class="btn-primary btn-sm btn-danger" onclick={confirmHardReset}>
          {t("tools.hardResetBtn")}
        </button>
      </div>
    </div>
  </div>
{/if}
