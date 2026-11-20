<script lang="ts">
  import type { BranchInfo, ReflogEntry, SubmoduleInfo, WorktreeInfo } from "../types/git";
  import * as gitService from "../services/git";
  import { t } from "../i18n/index.svelte";
  import { pickDirectory } from "../utils/path";

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

  // Reflog state
  let reflogEntries = $state<ReflogEntry[]>([]);
  let reflogSearch = $state("");
  let hardResetTarget = $state<ReflogEntry | null>(null);

  // Submodules state
  let submodules = $state<SubmoduleInfo[]>([]);
  let submoduleSearch = $state("");
  let showAddSubmodule = $state(false);
  let newSubUrl = $state("");
  let newSubPath = $state("");
  let copiedSubPath = $state<string | null>(null);

  // Worktrees state
  let worktrees = $state<WorktreeInfo[]>([]);
  let worktreeSearch = $state("");
  let showAddWorktree = $state(false);
  let localBranches = $state<BranchInfo[]>([]);
  let newWtPath = $state("");
  let newWtBranch = $state("");
  let newWtCreateBranch = $state(false);
  let lockModalTarget = $state<WorktreeInfo | null>(null);
  let lockReasonInput = $state("");
  let deleteModalTarget = $state<WorktreeInfo | null>(null);
  let forceDeleteCheckbox = $state(false);

  // Derived filter for Reflog
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

  // Derived filter for Submodules
  let filteredSubmodules = $derived.by(() => {
    const q = submoduleSearch.trim().toLowerCase();
    if (!q) return submodules;
    return submodules.filter(
      (s) =>
        s.path.toLowerCase().includes(q) ||
        (s.url && s.url.toLowerCase().includes(q)) ||
        s.status.toLowerCase().includes(q) ||
        s.sha.toLowerCase().includes(q) ||
        s.describe.toLowerCase().includes(q)
    );
  });

  // Derived branches in use for Worktrees
  let branchesInUse = $derived(
    new Set(worktrees.map((w) => w.branch).filter(Boolean) as string[])
  );

  // Derived filter for Worktrees
  let filteredWorktrees = $derived.by(() => {
    const q = worktreeSearch.trim().toLowerCase();
    if (!q) return worktrees;
    return worktrees.filter(
      (w) =>
        (w.branch && w.branch.toLowerCase().includes(q)) ||
        w.path.toLowerCase().includes(q) ||
        w.head.toLowerCase().includes(q)
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
        const [wtList, branchList] = await Promise.all([
          gitService.listWorktrees(repoPath),
          gitService.listBranches(repoPath).catch(() => []),
        ]);
        worktrees = wtList;
        localBranches = branchList;
        if (!newWtBranch && branchList.length > 0) {
          const defaultBranch = branchList.find((b) => b.is_current)?.name || branchList[0]?.name || "";
          newWtBranch = defaultBranch;
          suggestWorktreePath(defaultBranch);
        }
      }
    } catch (err) {
      errorMsg = String(err);
    } finally {
      loading = false;
    }
  }

  // Reflog handlers
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

  // Submodule handlers
  function onSubUrlInput(val: string) {
    newSubUrl = val;
    if (!newSubPath.trim() && val.trim()) {
      const clean = val.trim().replace(/\/+$/, "").replace(/\.git$/, "");
      const parts = clean.split(/[\\/]/);
      const last = parts[parts.length - 1];
      if (last && !last.includes(":") && !last.startsWith("-")) {
        newSubPath = last;
      }
    }
  }

  function getSubmoduleStatusBadge(status: string): { label: string; class: string } {
    switch (status) {
      case "clean":
        return { label: t("tools.submoduleStatusClean"), class: "submodule-status-clean" };
      case "modified":
        return { label: t("tools.submoduleStatusModified"), class: "submodule-status-modified" };
      case "conflict":
        return { label: t("tools.submoduleStatusConflict"), class: "submodule-status-conflict" };
      case "uninitialized":
      default:
        return { label: t("tools.submoduleStatusUninit"), class: "submodule-status-uninitialized" };
    }
  }

  function getSubmoduleFullPath(subPath: string): string {
    const base = repoPath.replace(/\\/g, "/").replace(/\/+$/, "");
    return `${base}/${subPath.replace(/\\/g, "/").replace(/^\/+/, "")}`;
  }

  async function handleOpenSubmoduleRepo(subPath: string) {
    const full = getSubmoduleFullPath(subPath);
    onClose();
    await onOpenWorktreeRepo(full);
  }

  async function handleOpenSubmoduleFolder(subPath: string) {
    const full = getSubmoduleFullPath(subPath);
    try {
      await gitService.openInFileManager(full);
    } catch (err) {
      onStatus?.(String(err), true);
    }
  }

  async function handleOpenSubmoduleTerminal(subPath: string) {
    const full = getSubmoduleFullPath(subPath);
    try {
      await gitService.openInTerminal(full);
    } catch (err) {
      onStatus?.(String(err), true);
    }
  }

  async function handleCopySubmoduleUrl(sub: SubmoduleInfo) {
    const textToCopy = sub.url || getSubmoduleFullPath(sub.path);
    try {
      await navigator.clipboard.writeText(textToCopy);
      copiedSubPath = sub.path;
      setTimeout(() => {
        if (copiedSubPath === sub.path) copiedSubPath = null;
      }, 2000);
    } catch {
      // ignore
    }
  }

  async function handleAddSubmodule() {
    const url = newSubUrl.trim();
    const p = newSubPath.trim().replace(/\\/g, "/").replace(/^\/+/, "");
    if (!url || !p) return;
    loading = true;
    errorMsg = "";
    try {
      await gitService.addSubmodule(repoPath, url, p);
      newSubUrl = "";
      newSubPath = "";
      showAddSubmodule = false;
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

  // Worktree handlers & helpers
  function normalizePath(p: string): string {
    return p.replace(/\\/g, "/").replace(/\/+$/, "").toLowerCase();
  }

  function isCurrentWorkspace(wtPath: string): boolean {
    return normalizePath(repoPath) === normalizePath(wtPath);
  }

  function suggestWorktreePath(branch: string) {
    if (!branch.trim()) return;
    const cleanBranch = branch.trim().replace(/[^a-zA-Z0-9._-]/g, "-");
    const sep = repoPath.includes("\\") ? "\\" : "/";
    const parts = repoPath.split(/[\\/]/).filter(Boolean);
    const parent = parts.slice(0, -1).join(sep) || (sep === "\\" ? "C:\\" : "/");
    const repoName = parts[parts.length - 1] || "worktree";
    newWtPath = `${parent}${sep}${repoName}-${cleanBranch}`;
  }

  async function handleBrowseWorktreeFolder() {
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

  async function handleOpenWorktreeFolder(path: string) {
    try {
      await gitService.openInFileManager(path);
    } catch (err) {
      onStatus?.(String(err), true);
    }
  }

  async function handleOpenWorktreeTerminal(path: string) {
    try {
      await gitService.openInTerminal(path);
    } catch (err) {
      onStatus?.(String(err), true);
    }
  }

  async function handleAddWorktree() {
    const p = newWtPath.trim();
    const b = newWtBranch.trim();
    if (!p || !b) return;
    loading = true;
    errorMsg = "";
    try {
      await gitService.addWorktree(repoPath, p, b, newWtCreateBranch);
      newWtPath = "";
      newWtBranch = "";
      showAddWorktree = false;
      onStatus?.(t("tools.worktreeAdded"));
      await loadTab("worktrees");
    } catch (err) {
      errorMsg = String(err);
      onStatus?.(String(err), true);
    } finally {
      loading = false;
    }
  }

  async function handleLockWorktree() {
    if (!lockModalTarget) return;
    loading = true;
    errorMsg = "";
    try {
      await gitService.lockWorktree(repoPath, lockModalTarget.path, lockReasonInput.trim() || undefined);
      onStatus?.(t("tools.worktreeLockedMsg"));
      lockModalTarget = null;
      lockReasonInput = "";
      await loadTab("worktrees");
    } catch (err) {
      errorMsg = String(err);
      onStatus?.(String(err), true);
    } finally {
      loading = false;
    }
  }

  async function handleUnlockWorktree(path: string) {
    loading = true;
    errorMsg = "";
    try {
      await gitService.unlockWorktree(repoPath, path);
      onStatus?.(t("tools.worktreeUnlockedMsg"));
      await loadTab("worktrees");
    } catch (err) {
      errorMsg = String(err);
      onStatus?.(String(err), true);
    } finally {
      loading = false;
    }
  }

  async function confirmRemoveWorktree() {
    if (!deleteModalTarget) return;
    const targetPath = deleteModalTarget.path;
    const force = forceDeleteCheckbox;
    deleteModalTarget = null;
    forceDeleteCheckbox = false;

    loading = true;
    errorMsg = "";
    try {
      await gitService.removeWorktree(repoPath, targetPath, force);
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

        <div class="tools-tab-toolbar">
          <div class="tools-search-box">
            <svg class="tools-search-icon" width="13" height="13" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
              <path d="M11.5 7a4.5 4.5 0 1 1-9 0 4.5 4.5 0 0 1 9 0Zm-.82 4.74a6 6 0 1 0-1.06 1.06l3.44 3.44a.75.75 0 1 0 1.06-1.06l-3.44-3.44Z" />
            </svg>
            <input
              type="text"
              class="tools-search-input"
              bind:value={reflogSearch}
              placeholder={t("tools.reflogSearchPlaceholder")}
            />
            {#if reflogSearch}
              <button
                type="button"
                class="tools-search-clear"
                onclick={() => { reflogSearch = ""; }}
                aria-label={t("common.clear")}
              >✕</button>
            {/if}
          </div>
          <button
            type="button"
            class="btn-secondary btn-xs"
            disabled={loading}
            onclick={() => loadTab("reflog")}
            title={t("common.refresh")}
            aria-label={t("common.refresh")}
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
        <div class="tools-tab-toolbar">
          <div class="tools-search-box">
            <svg class="tools-search-icon" width="13" height="13" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
              <path d="M11.5 7a4.5 4.5 0 1 1-9 0 4.5 4.5 0 0 1 9 0Zm-.82 4.74a6 6 0 1 0-1.06 1.06l3.44 3.44a.75.75 0 1 0 1.06-1.06l-3.44-3.44Z" />
            </svg>
            <input
              type="text"
              class="tools-search-input"
              bind:value={submoduleSearch}
              placeholder={t("tools.submodulesSearchPlaceholder")}
            />
            {#if submoduleSearch}
              <button
                type="button"
                class="tools-search-clear"
                onclick={() => { submoduleSearch = ""; }}
                aria-label={t("common.clear")}
              >✕</button>
            {/if}
          </div>

          <button
            type="button"
            class="btn-secondary btn-xs"
            disabled={loading}
            onclick={() => loadTab("submodules")}
            title={t("common.refresh")}
          >
            <svg class={loading ? "spin" : ""} width="12" height="12" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
              <path d="M1.705 8.001a6.3 6.3 0 0 1 10.47-4.664.75.75 0 0 1-.99 1.127 4.8 4.8 0 1 0 1.25 5.37.75.75 0 1 1 1.42.5 6.3 6.3 0 0 1-12.15-2.333Z" />
              <path d="m13.25 1.5.02 3.75-3.75-.02a.75.75 0 0 1 0-1.5l2.25.01-.01-2.24a.75.75 0 0 1 1.49 0Z" />
            </svg>
            <span>{t("common.refresh")}</span>
          </button>
          <button
            type="button"
            class="btn-secondary btn-xs"
            disabled={loading}
            onclick={handleUpdateSubmodules}
            title={t("tools.updateSubmodulesBtn")}
          >
            <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
              <path d="M8 12a4 4 0 1 1 0-8 4 4 0 0 1 0 8Zm0-1.5a2.5 2.5 0 1 0 0-5 2.5 2.5 0 0 0 0 5Z"/>
            </svg>
            <span>{t("tools.updateSubmodulesBtn")}</span>
          </button>
          <button
            type="button"
            class="btn-secondary btn-xs"
            disabled={loading}
            onclick={handleSyncSubmodules}
            title={t("tools.syncSubmodulesBtn")}
          >
            <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
              <path d="M2.5 8a5.5 5.5 0 0 1 9.39-3.89l.86-.86a7 7 0 1 0 .25 8.75l-1.07-.93A5.5 5.5 0 0 1 2.5 8Z"/>
            </svg>
            <span>{t("tools.syncSubmodulesBtn")}</span>
          </button>
          <button
            type="button"
            class={showAddSubmodule ? "btn-secondary btn-xs" : "btn-primary btn-xs"}
            onclick={() => { showAddSubmodule = !showAddSubmodule; }}
          >
            {showAddSubmodule ? t("common.cancel") : `+ ${t("tools.submoduleAddToggle")}`}
          </button>
        </div>

        {#if showAddSubmodule}
          <div class="tools-add-card">
            <div style="display: flex; align-items: center; justify-content: space-between;">
              <span style="font-weight: 600; font-size: 12px; color: var(--text-primary);">{t("tools.submoduleAddTitle")}</span>
              <button type="button" class="btn-ghost btn-xs" onclick={() => { showAddSubmodule = false; }}>✕</button>
            </div>
            <div style="display: flex; gap: 8px; flex-wrap: wrap;">
              <div style="flex: 2; min-width: 200px;">
                <label class="form-label" style="font-size: 11px; margin-bottom: 4px; display: block;" for="sub-url-input">
                  {t("tools.submoduleUrlPlaceholder")}
                </label>
                <input
                  id="sub-url-input"
                  type="text"
                  class="form-input form-input-sm"
                  style="width: 100%;"
                  bind:value={newSubUrl}
                  oninput={() => onSubUrlInput(newSubUrl)}
                  placeholder="https://github.com/org/repo.git"
                />
              </div>
              <div style="flex: 1; min-width: 140px;">
                <label class="form-label" style="font-size: 11px; margin-bottom: 4px; display: block;" for="sub-path-input">
                  {t("tools.submodulePathPlaceholder")}
                </label>
                <input
                  id="sub-path-input"
                  type="text"
                  class="form-input form-input-sm"
                  style="width: 100%;"
                  bind:value={newSubPath}
                  placeholder="src/vendor/lib"
                />
              </div>
            </div>
            <div style="display: flex; justify-content: flex-end; gap: 6px; margin-top: 4px;">
              <button
                type="button"
                class="btn-secondary btn-xs"
                onclick={() => { showAddSubmodule = false; }}
              >
                {t("common.cancel")}
              </button>
              <button
                type="button"
                class="btn-primary btn-xs"
                disabled={loading || !newSubUrl.trim() || !newSubPath.trim()}
                onclick={handleAddSubmodule}
              >
                {t("tools.addBtn")}
              </button>
            </div>
          </div>
        {/if}

        {#if loading && submodules.length === 0}
          <div class="empty-state" style="padding: 32px 16px;">
            <div class="spinner-ring" style="width: 22px; height: 22px; border-width: 2px; margin-bottom: 8px;"></div>
            <span style="font-size: 12px; color: var(--text-muted);">{t("common.loading")}</span>
          </div>
        {:else if submodules.length === 0}
          <div class="empty-state"><span>{t("tools.submodulesEmpty")}</span></div>
        {:else if filteredSubmodules.length === 0}
          <div class="empty-state"><span>{t("tools.submodulesNoMatch")}</span></div>
        {:else}
          <div class="list-box" style="max-height: 380px;">
            {#each filteredSubmodules as sub (sub.path)}
              {@const statusBadge = getSubmoduleStatusBadge(sub.status)}
              <div class="list-row" style="align-items: flex-start; gap: 12px;">
                <div style="min-width: 0; flex: 1;">
                  <div style="display: flex; align-items: center; gap: 8px; flex-wrap: wrap;">
                    <span style="font-weight: 600; font-size: 12px; color: var(--text-primary);">{sub.path}</span>
                    <span class="badge-status {statusBadge.class}">{statusBadge.label}</span>
                    <span class="commit-sha">{sub.sha.slice(0, 7)}</span>
                    {#if sub.describe}
                      <span class="badge-neutral" style="font-size: 10px;" title={sub.describe}>{sub.describe}</span>
                    {/if}
                  </div>
                  {#if sub.url}
                    <div style="display: flex; align-items: center; gap: 6px; font-size: 11px; color: var(--text-muted); margin-top: 4px; word-break: break-all;">
                      <span>{sub.url}</span>
                      <button
                        type="button"
                        class="btn-ghost btn-xs"
                        style="padding: 1px 4px; height: 18px; font-size: 10px;"
                        title={copiedSubPath === sub.path ? t("tools.submoduleUrlCopied") : t("tools.submoduleCopyUrl")}
                        onclick={() => handleCopySubmoduleUrl(sub)}
                      >
                        {copiedSubPath === sub.path ? `✓ ${t("tools.submoduleUrlCopied")}` : t("tools.submoduleCopyUrl")}
                      </button>
                    </div>
                  {/if}
                </div>
                <div style="display: flex; gap: 4px; flex-shrink: 0; align-items: center; margin-top: 2px;">
                  <button
                    type="button"
                    class="btn-secondary btn-xs"
                    disabled={loading}
                    title={t("tools.submoduleOpenRepo")}
                    onclick={() => handleOpenSubmoduleRepo(sub.path)}
                  >
                    {t("tools.submoduleOpenRepo")}
                  </button>
                  <button
                    type="button"
                    class="btn-secondary btn-xs"
                    disabled={loading}
                    title={t("tools.submoduleOpenFolder")}
                    onclick={() => handleOpenSubmoduleFolder(sub.path)}
                  >
                    <svg width="11" height="11" viewBox="0 0 16 16" fill="currentColor">
                      <path d="M1.75 2.5A.75.75 0 0 0 1 3.25v9.5c0 .414.336.75.75.75h12.5a.75.75 0 0 0 .75-.75v-7.5a.75.75 0 0 0-.75-.75H7.621a.75.75 0 0 1-.53-.22L5.808 3.015A1.75 1.75 0 0 0 4.57 2.5H1.75Z"/>
                    </svg>
                    <span>{t("tools.submoduleOpenFolder")}</span>
                  </button>
                  <button
                    type="button"
                    class="btn-secondary btn-xs"
                    disabled={loading}
                    title={t("tools.submoduleTerminal")}
                    onclick={() => handleOpenSubmoduleTerminal(sub.path)}
                  >
                    <svg width="11" height="11" viewBox="0 0 16 16" fill="currentColor">
                      <path d="M1 2.75C1 1.784 1.784 1 2.75 1h10.5c.966 0 1.75.784 1.75 1.75v10.5A1.75 1.75 0 0 1 13.25 15H2.75A1.75 1.75 0 0 1 1 13.25V2.75Zm1.75-.25a.25.25 0 0 0-.25.25v10.5c0 .138.112.25.25.25h10.5a.25.25 0 0 0 .25-.25V2.75a.25.25 0 0 0-.25-.25H2.75ZM4.28 5.22a.75.75 0 0 1 0 1.06L5.94 8l-1.66 1.72a.75.75 0 1 1-1.06-1.06L4.34 8l-1.12-1.16a.75.75 0 0 1 1.06-1.06Zm4.47 5.03h3.5a.75.75 0 0 1 0 1.5h-3.5a.75.75 0 0 1 0-1.5Z"/>
                    </svg>
                    <span>{t("tools.submoduleTerminal")}</span>
                  </button>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      {:else}
        <!-- Tab Worktrees -->
        <div class="tools-tab-toolbar">
          <div class="tools-search-box">
            <svg class="tools-search-icon" width="13" height="13" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
              <path d="M11.5 7a4.5 4.5 0 1 1-9 0 4.5 4.5 0 0 1 9 0Zm-.82 4.74a6 6 0 1 0-1.06 1.06l3.44 3.44a.75.75 0 1 0 1.06-1.06l-3.44-3.44Z" />
            </svg>
            <input
              type="text"
              class="tools-search-input"
              bind:value={worktreeSearch}
              placeholder={t("tools.worktreesSearchPlaceholder")}
            />
            {#if worktreeSearch}
              <button
                type="button"
                class="tools-search-clear"
                onclick={() => { worktreeSearch = ""; }}
                aria-label={t("common.clear")}
              >✕</button>
            {/if}
          </div>

          <button
            type="button"
            class="btn-secondary btn-xs"
            disabled={loading}
            onclick={() => loadTab("worktrees")}
            title={t("common.refresh")}
          >
            <svg class={loading ? "spin" : ""} width="12" height="12" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
              <path d="M1.705 8.001a6.3 6.3 0 0 1 10.47-4.664.75.75 0 0 1-.99 1.127 4.8 4.8 0 1 0 1.25 5.37.75.75 0 1 1 1.42.5 6.3 6.3 0 0 1-12.15-2.333Z" />
              <path d="m13.25 1.5.02 3.75-3.75-.02a.75.75 0 0 1 0-1.5l2.25.01-.01-2.24a.75.75 0 0 1 1.49 0Z" />
            </svg>
            <span>{t("common.refresh")}</span>
          </button>
          <button
            type="button"
            class="btn-secondary btn-xs"
            disabled={loading}
            onclick={handlePruneWorktrees}
            title={t("tools.pruneBtn")}
          >
            <svg width="11" height="11" viewBox="0 0 16 16" fill="currentColor">
              <path d="M11 1.75V3h2.25a.75.75 0 0 1 0 1.5H2.75a.75.75 0 0 1 0-1.5H5V1.75C5 .784 5.784 0 6.75 0h2.5C10.216 0 11 .784 11 1.75ZM4.496 6.675l.66 6.6A1.75 1.75 0 0 0 6.898 15h2.204a1.75 1.75 0 0 0 1.742-1.725l.66-6.6a.75.75 0 0 0-1.492-.15l-.66 6.6a.25.25 0 0 1-.249.225H6.898a.25.25 0 0 1-.249-.225l-.66-6.6a.75.75 0 0 0-1.493.15ZM6.5 1.5h3a.25.25 0 0 0 .25-.25V1.75a.25.25 0 0 0-.25-.25h-3a.25.25 0 0 0-.25.25v.25c0 .138.112.25.25.25Z" />
            </svg>
            <span>{t("tools.pruneBtn")}</span>
          </button>
          <button
            type="button"
            class={showAddWorktree ? "btn-secondary btn-xs" : "btn-primary btn-xs"}
            onclick={() => { showAddWorktree = !showAddWorktree; }}
          >
            {showAddWorktree ? t("common.cancel") : `+ ${t("tools.worktreeAddToggle")}`}
          </button>
        </div>

        {#if showAddWorktree}
          <div class="tools-add-card">
            <div style="display: flex; align-items: center; justify-content: space-between;">
              <span style="font-weight: 600; font-size: 12px; color: var(--text-primary);">{t("tools.worktreeAddBtn")}</span>
              <button type="button" class="btn-ghost btn-xs" onclick={() => { showAddWorktree = false; }}>✕</button>
            </div>
            <div style="display: flex; flex-direction: column; gap: 8px;">
              <div>
                <label class="form-label" style="font-size: 11px; margin-bottom: 4px; display: block;" for="wt-path-input">
                  {t("tools.worktreePathPlaceholder")}
                </label>
                <div style="display: flex; gap: 6px;">
                  <input
                    id="wt-path-input"
                    type="text"
                    class="form-input form-input-sm"
                    style="flex: 1;"
                    bind:value={newWtPath}
                    placeholder={t("tools.worktreePathPlaceholder")}
                  />
                  <button
                    type="button"
                    class="btn-secondary btn-xs"
                    onclick={handleBrowseWorktreeFolder}
                    title={t("tools.worktreeBrowseBtn")}
                  >
                    <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor">
                      <path d="M1.75 2.5A.75.75 0 0 0 1 3.25v9.5c0 .414.336.75.75.75h12.5a.75.75 0 0 0 .75-.75v-7.5a.75.75 0 0 0-.75-.75H7.621a.75.75 0 0 1-.53-.22L5.808 3.015A1.75 1.75 0 0 0 4.57 2.5H1.75Z"/>
                    </svg>
                    <span>{t("tools.worktreeBrowseBtn")}</span>
                  </button>
                </div>
              </div>

              <div>
                <label class="form-label" style="font-size: 11px; margin-bottom: 4px; display: block;" for="wt-branch-source">
                  {t("tools.worktreeBranchPlaceholder")}
                </label>
                {#if newWtCreateBranch}
                  <input
                    id="wt-branch-source"
                    type="text"
                    class="form-input form-input-sm"
                    style="width: 100%;"
                    bind:value={newWtBranch}
                    oninput={() => suggestWorktreePath(newWtBranch)}
                    placeholder={t("tools.worktreeBranchPlaceholder")}
                  />
                {:else}
                  <select
                    id="wt-branch-source"
                    class="form-select form-select-sm"
                    style="width: 100%;"
                    bind:value={newWtBranch}
                    onchange={() => suggestWorktreePath(newWtBranch)}
                  >
                    <option value="" disabled selected>{t("tools.worktreeBranchPlaceholder")}</option>
                    {#each localBranches as b}
                      {@const inUse = branchesInUse.has(b.name)}
                      <option value={b.name} disabled={inUse}>
                        {inUse ? t("tools.worktreeBranchInUse", { branch: b.name }) : b.name}
                      </option>
                    {/each}
                  </select>
                {/if}
                <label class="checkbox-label" style="font-size: 11px; margin-top: 6px; display: inline-flex; align-items: center; gap: 6px; cursor: pointer;">
                  <input type="checkbox" bind:checked={newWtCreateBranch} />
                  <span>{t("tools.worktreeNewBranch")}</span>
                </label>
              </div>
            </div>
            <div style="display: flex; justify-content: flex-end; gap: 6px; margin-top: 4px;">
              <button
                type="button"
                class="btn-secondary btn-xs"
                onclick={() => { showAddWorktree = false; }}
              >
                {t("common.cancel")}
              </button>
              <button
                type="button"
                class="btn-primary btn-xs"
                disabled={loading || !newWtPath.trim() || !newWtBranch.trim()}
                onclick={handleAddWorktree}
              >
                {t("tools.addBtn")}
              </button>
            </div>
          </div>
        {/if}

        {#if loading && worktrees.length === 0}
          <div class="empty-state" style="padding: 32px 16px;">
            <div class="spinner-ring" style="width: 22px; height: 22px; border-width: 2px; margin-bottom: 8px;"></div>
            <span style="font-size: 12px; color: var(--text-muted);">{t("common.loading")}</span>
          </div>
        {:else if worktrees.length === 0}
          <div class="empty-state"><span>{t("tools.worktreesEmpty")}</span></div>
        {:else if filteredWorktrees.length === 0}
          <div class="empty-state"><span>{t("tools.worktreesNoMatch")}</span></div>
        {:else}
          <div class="list-box" style="max-height: 380px;">
            {#each filteredWorktrees as wt (wt.path)}
              {@const isCurrent = isCurrentWorkspace(wt.path)}
              <div class="list-row {isCurrent ? 'is-active-workspace' : ''}" style="align-items: flex-start; gap: 12px;">
                <div style="min-width: 0; flex: 1;">
                  <div style="display: flex; align-items: center; gap: 8px; flex-wrap: wrap;">
                    <span style="font-weight: 600; font-size: 12px; font-family: var(--font-mono, monospace);">
                      {wt.branch || (wt.is_detached ? "DETACHED" : "bare")}
                    </span>
                    {#if isCurrent}
                      <span class="badge-status badge-active" style="font-size: 9px; padding: 0 5px;">{t("tools.worktreeActive")}</span>
                    {/if}
                    {#if wt.is_main}
                      <span class="badge-neutral" style="font-size: 9px; padding: 0 5px;">{t("tools.worktreeMain")}</span>
                    {/if}
                    {#if wt.is_locked}
                      <span class="badge-status badge-locked" style="font-size: 9px; padding: 0 5px;" title={wt.lock_reason || t("tools.worktreeLocked")}>
                        🔒 {t("tools.worktreeLocked")}
                      </span>
                    {/if}
                    {#if wt.is_prunable}
                      <span class="badge-status badge-prunable" style="font-size: 9px; padding: 0 5px;" title={wt.prunable_reason || t("tools.worktreeFolderMissing")}>
                        ⚠️ {t("tools.worktreeFolderMissing")}
                      </span>
                    {/if}
                    <span class="commit-sha">{wt.head.slice(0, 7)}</span>
                  </div>
                  <div style="font-size: 11px; color: var(--text-muted); margin-top: 3px; word-break: break-all;">
                    {wt.path}
                  </div>
                  {#if wt.lock_reason}
                    <div style="font-size: 10px; color: var(--warning-color, #f59e0b); margin-top: 2px; font-style: italic;">
                      Alasan: {wt.lock_reason}
                    </div>
                  {/if}
                </div>
                <div style="display: flex; gap: 4px; flex-shrink: 0; align-items: center; flex-wrap: wrap; justify-content: flex-end; margin-top: 2px;">
                  {#if !isCurrent && !wt.is_bare}
                    <button
                      type="button"
                      class="btn-secondary btn-xs"
                      disabled={wt.is_prunable || loading}
                      title={wt.is_prunable ? t("tools.worktreeFolderMissing") : t("common.open")}
                      onclick={async () => {
                        onClose();
                        await onOpenWorktreeRepo(wt.path);
                      }}
                    >
                      {t("common.open")}
                    </button>
                  {/if}
                  <button
                    type="button"
                    class="btn-secondary btn-xs"
                    disabled={wt.is_prunable || loading}
                    title={t("tools.submoduleOpenFolder")}
                    onclick={() => handleOpenWorktreeFolder(wt.path)}
                  >
                    <svg width="11" height="11" viewBox="0 0 16 16" fill="currentColor">
                      <path d="M1.75 2.5A.75.75 0 0 0 1 3.25v9.5c0 .414.336.75.75.75h12.5a.75.75 0 0 0 .75-.75v-7.5a.75.75 0 0 0-.75-.75H7.621a.75.75 0 0 1-.53-.22L5.808 3.015A1.75 1.75 0 0 0 4.57 2.5H1.75Z"/>
                    </svg>
                    <span>{t("tools.submoduleOpenFolder")}</span>
                  </button>
                  <button
                    type="button"
                    class="btn-secondary btn-xs"
                    disabled={wt.is_prunable || loading}
                    title={t("tools.submoduleTerminal")}
                    onclick={() => handleOpenWorktreeTerminal(wt.path)}
                  >
                    <svg width="11" height="11" viewBox="0 0 16 16" fill="currentColor">
                      <path d="M1 2.75C1 1.784 1.784 1 2.75 1h10.5c.966 0 1.75.784 1.75 1.75v10.5A1.75 1.75 0 0 1 13.25 15H2.75A1.75 1.75 0 0 1 1 13.25V2.75Zm1.75-.25a.25.25 0 0 0-.25.25v10.5c0 .138.112.25.25.25h10.5a.25.25 0 0 0 .25-.25V2.75a.25.25 0 0 0-.25-.25H2.75ZM4.28 5.22a.75.75 0 0 1 0 1.06L5.94 8l-1.66 1.72a.75.75 0 1 1-1.06-1.06L4.34 8l-1.12-1.16a.75.75 0 0 1 1.06-1.06Zm4.47 5.03h3.5a.75.75 0 0 1 0 1.5h-3.5a.75.75 0 0 1 0-1.5Z"/>
                    </svg>
                    <span>{t("tools.submoduleTerminal")}</span>
                  </button>
                  {#if wt.is_locked}
                    <button
                      type="button"
                      class="btn-secondary btn-xs"
                      disabled={loading}
                      title={t("tools.worktreeUnlockBtn")}
                      onclick={() => handleUnlockWorktree(wt.path)}
                    >
                      🔓 {t("tools.worktreeUnlockBtn")}
                    </button>
                  {:else if !wt.is_main}
                    <button
                      type="button"
                      class="btn-secondary btn-xs"
                      disabled={loading}
                      title={t("tools.worktreeLockBtn")}
                      onclick={() => { lockModalTarget = wt; lockReasonInput = ""; }}
                    >
                      🔒 {t("tools.worktreeLockBtn")}
                    </button>
                  {/if}
                  {#if !wt.is_main}
                    <button
                      type="button"
                      class="btn-secondary btn-xs btn-danger"
                      disabled={loading}
                      title={t("common.delete")}
                      onclick={() => { deleteModalTarget = wt; forceDeleteCheckbox = false; }}
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

{#if lockModalTarget}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="modal-overlay"
    style="z-index: 1100; background: rgba(0, 0, 0, 0.65);"
    onclick={(e) => {
      if (e.target === e.currentTarget) lockModalTarget = null;
    }}
    onkeydown={(e) => {
      if (e.key === "Escape") lockModalTarget = null;
    }}
  >
    <div class="modal-dialog modal-sm" role="dialog" aria-modal="true" aria-label={t("tools.worktreeLockDialogTitle")}>
      <div class="modal-header">
        <h4 style="margin: 0; font-size: 13px; font-weight: 600; display: flex; align-items: center; gap: 6px;">
          🔒 {t("tools.worktreeLockDialogTitle")}
        </h4>
        <button type="button" class="btn-ghost btn-xs" onclick={() => { lockModalTarget = null; }}>✕</button>
      </div>
      <div class="modal-body" style="font-size: 12px; line-height: 1.5; color: var(--text-primary); display: flex; flex-direction: column; gap: 10px;">
        <div>
          <strong>{lockModalTarget.branch || lockModalTarget.path}</strong>
        </div>
        <div style="display: flex; flex-direction: column; gap: 4px;">
          <label class="form-label" style="font-size: 11px;" for="wt-lock-reason-input">{t("tools.worktreeLockReasonPlaceholder")}</label>
          <input
            id="wt-lock-reason-input"
            type="text"
            class="form-input form-input-sm"
            bind:value={lockReasonInput}
            placeholder={t("tools.worktreeLockReasonPlaceholder")}
          />
        </div>
      </div>
      <div class="modal-footer" style="display: flex; justify-content: flex-end; gap: 8px;">
        <button type="button" class="btn-secondary btn-sm" onclick={() => { lockModalTarget = null; }}>
          {t("common.cancel")}
        </button>
        <button type="button" class="btn-primary btn-sm" onclick={handleLockWorktree}>
          {t("tools.worktreeLockBtn")}
        </button>
      </div>
    </div>
  </div>
{/if}

{#if deleteModalTarget}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="modal-overlay"
    style="z-index: 1100; background: rgba(0, 0, 0, 0.65);"
    onclick={(e) => {
      if (e.target === e.currentTarget) deleteModalTarget = null;
    }}
    onkeydown={(e) => {
      if (e.key === "Escape") deleteModalTarget = null;
    }}
  >
    <div class="modal-dialog modal-sm" role="dialog" aria-modal="true" aria-label={t("tools.worktreeConfirmDeleteTitle")}>
      <div class="modal-header">
        <h4 style="margin: 0; font-size: 13px; font-weight: 600; color: var(--danger-color, #ef4444); display: flex; align-items: center; gap: 6px;">
          <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
            <path d="M6.457 1.047c.659-1.234 2.427-1.234 3.086 0l6.082 11.378A1.75 1.75 0 0 1 14.082 15H1.918a1.75 1.75 0 0 1-1.543-2.575Zm1.763.707a.25.25 0 0 0-.44 0L1.698 13.132a.25.25 0 0 0 .22.368h12.164a.25.25 0 0 0 .22-.368Zm.53 3.996v2.5a.75.75 0 0 1-1.5 0v-2.5a.75.75 0 0 1 1.5 0ZM9 11a1 1 0 1 1-2 0 1 1 0 0 1 2 0Z"/>
          </svg>
          {t("tools.worktreeConfirmDeleteTitle")}
        </h4>
        <button type="button" class="btn-ghost btn-xs" onclick={() => { deleteModalTarget = null; }}>✕</button>
      </div>
      <div class="modal-body" style="font-size: 12px; line-height: 1.5; color: var(--text-primary); display: flex; flex-direction: column; gap: 10px;">
        <p style="margin: 0;">
          {t("tools.worktreeConfirmDelete", { path: deleteModalTarget.path })}
        </p>
        <div style="background: var(--bg-secondary); border: 1px solid var(--border-color); border-radius: var(--radius-sm); padding: 8px; font-size: 11px;">
          <div style="display: flex; gap: 6px; align-items: center; margin-bottom: 2px;">
            <span style="font-weight: 600;">{deleteModalTarget.branch || "DETACHED"}</span>
            <span class="commit-sha">{deleteModalTarget.head.slice(0, 7)}</span>
          </div>
          <div style="color: var(--text-muted); word-break: break-all;">{deleteModalTarget.path}</div>
        </div>
        <label class="checkbox-label" style="font-size: 11px; display: inline-flex; align-items: center; gap: 6px; cursor: pointer; color: var(--text-muted);">
          <input type="checkbox" bind:checked={forceDeleteCheckbox} />
          <span>{t("tools.worktreeForceDelete")}</span>
        </label>
      </div>
      <div class="modal-footer" style="display: flex; justify-content: flex-end; gap: 8px;">
        <button type="button" class="btn-secondary btn-sm" onclick={() => { deleteModalTarget = null; }}>
          {t("common.cancel")}
        </button>
        <button type="button" class="btn-primary btn-sm btn-danger" onclick={confirmRemoveWorktree}>
          {t("common.delete")}
        </button>
      </div>
    </div>
  </div>
{/if}
