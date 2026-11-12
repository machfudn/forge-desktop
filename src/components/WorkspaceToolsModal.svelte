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

  let reflogEntries = $state<ReflogEntry[]>([]);
  let submodules = $state<SubmoduleInfo[]>([]);
  let worktrees = $state<WorktreeInfo[]>([]);

  let newSubUrl = $state("");
  let newSubPath = $state("");

  let newWtPath = $state("");
  let newWtBranch = $state("");
  let newWtCreateBranch = $state(true);

  $effect(() => {
    activeTab = initialTab;
  });

  $effect(() => {
    if (repoPath) {
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
    try {
      await gitService.resetToCommit(repoPath, sha, hard);
      onStatus?.(t("tools.reflogResetDone", { sha: sha.slice(0, 7) }));
      await onRefreshRepo();
      await loadTab("reflog");
    } catch (err) {
      errorMsg = String(err);
      onStatus?.(String(err), true);
    }
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

  async function handleRemoveWorktree(path: string, force = false) {
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
              void loadTab("reflog");
            }}
          >
            {t("tools.tabReflog")}
          </button>
          <button
            type="button"
            class="seg-btn {activeTab === 'submodules' ? 'active' : ''}"
            onclick={() => {
              activeTab = "submodules";
              void loadTab("submodules");
            }}
          >
            {t("tools.tabSubmodules")}
          </button>
          <button
            type="button"
            class="seg-btn {activeTab === 'worktrees' ? 'active' : ''}"
            onclick={() => {
              activeTab = "worktrees";
              void loadTab("worktrees");
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
        <div style="font-size: 11px; color: var(--text-muted); margin-bottom: 4px;">
          {t("tools.reflogDesc")}
        </div>
        {#if loading}
          <div class="empty-state"><span>{t("common.loading")}</span></div>
        {:else if reflogEntries.length === 0}
          <div class="empty-state"><span>{t("tools.reflogEmpty")}</span></div>
        {:else}
          <div class="list-box" style="max-height: 380px;">
            {#each reflogEntries as entry (entry.selector + entry.sha)}
              <div class="list-row">
                <div style="min-width: 0; flex: 1;">
                  <div style="display: flex; align-items: center; gap: 8px;">
                    <span class="commit-sha">{entry.short_sha}</span>
                    <span class="badge-neutral">{entry.selector}</span>
                    <span style="font-size: 11px; color: var(--text-muted);">{entry.date_relative}</span>
                  </div>
                  <div style="font-size: 12px; color: var(--text-primary); margin-top: 2px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
                    {entry.action}
                  </div>
                </div>
                <div style="display: flex; gap: 4px; flex-shrink: 0;">
                  <button
                    type="button"
                    class="btn-secondary btn-xs"
                    onclick={() => handleReflogCherryPick(entry.sha)}
                  >
                    {t("history.cherryPickBtn")}
                  </button>
                  <button
                    type="button"
                    class="btn-secondary btn-xs"
                    onclick={() => handleReflogReset(entry.sha, false)}
                  >
                    {t("tools.softResetBtn")}
                  </button>
                  <button
                    type="button"
                    class="btn-secondary btn-xs btn-danger"
                    onclick={() => handleReflogReset(entry.sha, true)}
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
            <button type="button" class="btn-secondary btn-xs" disabled={loading} onclick={handlePruneWorktrees}>
              {t("tools.pruneBtn")}
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
              <div class="list-row">
                <div style="min-width: 0; flex: 1;">
                  <div style="display: flex; align-items: center; gap: 8px;">
                    <span style="font-weight: 600; font-size: 12px;">{wt.branch || (wt.is_detached ? "DETACHED" : "bare")}</span>
                    {#if wt.is_main}
                      <span class="badge-neutral">main</span>
                    {/if}
                    {#if wt.is_locked}
                      <span class="badge-neutral">locked</span>
                    {/if}
                    <span class="commit-sha">{wt.head.slice(0, 7)}</span>
                  </div>
                  <div style="font-size: 11px; color: var(--text-muted); margin-top: 2px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
                    {wt.path}
                  </div>
                </div>
                <div style="display: flex; gap: 4px; flex-shrink: 0;">
                  {#if !wt.is_bare}
                    <button
                      type="button"
                      class="btn-secondary btn-xs"
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
