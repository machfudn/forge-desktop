<script lang="ts">
  import type { AheadBehind, BranchInfo } from "../types/git";
  import * as gitService from "../services/git";
  import { formatBranchDisplay, t } from "../i18n/index.svelte";
  import {
    copyTextToClipboard,
    openContextMenu,
    type ContextMenuItem,
  } from "../lib/contextMenu.svelte";
  interface Props {
    activeRepo: string;
    currentBranch: string;
    branches: BranchInfo[];
    branchDivergenceMap: Record<string, AheadBehind>;
    isLoadingBranches?: boolean;
    onInspectBranch?: (branchName: string) => void;
    hasDirtyFiles: boolean;
    dirtyFileCount: number;
    hasRemote: boolean;
    hasCommits: boolean;
    initialRenameBranch?: string;
    onClose: () => void;
    onReloadRepo: () => Promise<void>;
    onStatus: (msg: string, isError?: boolean) => void;
    onRequestConfirm: (
      title: string,
      message: string,
      action: () => Promise<void>,
      danger?: boolean
    ) => void;
    onPushError?: (err: string, branch: string) => boolean | Promise<boolean>;
    onOperationError?: (
      err: string,
      operation: string,
      retryAction?: () => Promise<void>
    ) => boolean | Promise<boolean>;
  }

  let {
    activeRepo,
    currentBranch,
    branches = $bindable(),
    branchDivergenceMap,
    isLoadingBranches = false,
    onInspectBranch,
    hasDirtyFiles,
    dirtyFileCount,
    hasRemote,
    hasCommits,
    initialRenameBranch,
    onClose,
    onReloadRepo,
    onStatus,
    onRequestConfirm,
    onPushError,
    onOperationError,
  }: Props = $props();

  let pushingBranch = $state<string | null>(null);
  let branchSearch = $state("");
  let newBranchName = $state("");
  let branchToRename = $state<string | null>(null);
  let renameBranchInput = $state("");
  let alsoRenameOnRemote = $state(false);
  let branchToDelete = $state<BranchInfo | null>(null);
  let alsoDeleteOnRemote = $state(false);
  let hoveredBranchName = $state<string | null>(null);
  let focusedBranchName = $state<string | null>(null);
  let inlineError = $state("");
  let branchFilterTab = $state<"all" | "local" | "remote">("all");
  let isFetchingBranches = $state(false);

  let renameTargetBranch = $derived(branches.find((b) => b.name === branchToRename));

  $effect(() => {
    if (initialRenameBranch && !branchToRename) {
      handleStartRename(initialRenameBranch);
    }
  });

  let localBranches = $derived(branches.filter((b) => !b.is_remote));
  let remoteBranches = $derived(branches.filter((b) => Boolean(b.is_remote)));

  let localBranchNames = $derived(new Set(localBranches.map((b) => b.name)));
  let remoteOnlyBranches = $derived(
    remoteBranches.filter((b) => {
      const slashIdx = b.name.indexOf("/");
      const targetName = slashIdx >= 0 ? b.name.slice(slashIdx + 1) : b.name;
      return !localBranchNames.has(targetName);
    })
  );

  let unifiedBranches = $derived([...localBranches, ...remoteOnlyBranches]);

  let filteredBranches = $derived.by(() => {
    const q = branchSearch.trim().toLowerCase();
    const pool =
      branchFilterTab === "local"
        ? localBranches
        : branchFilterTab === "remote"
          ? remoteBranches
          : unifiedBranches;
    if (!q) return pool;
    return pool.filter((b) => b.name.toLowerCase().includes(q));
  });

  async function handleFetchAllBranches() {
    inlineError = "";
    isFetchingBranches = true;
    try {
      await gitService.fetchAllRemotes(activeRepo);
      branches = await gitService.listBranches(activeRepo);
      onStatus(t("branchModal.fetchedStatus"));
    } catch (err) {
      inlineError = String(err);
      onStatus(String(err), true);
    } finally {
      isFetchingBranches = false;
    }
  }


  async function handleSwitchBranch(name: string) {
    if (hasDirtyFiles) {
      onRequestConfirm(
        t("branchModal.smartStashTitle"),
        t("branchModal.smartStashMsg", { count: dirtyFileCount, name }),
        async () => {
          try {
            onStatus(t("branchModal.stashingStatus"));
            await gitService.gitStashSave(activeRepo, t("branchModal.autoStashMsg", { name }));
            onStatus(t("branchModal.switchingStatus", { name }));
            await gitService.switchBranch(activeRepo, name);
            onStatus(t("branchModal.switchedWithStash", { name }));
            await onReloadRepo();
            onClose();
          } catch (err) {
            onStatus(String(err), true);
          }
        },
        false
      );
      return;
    }

    try {
      await gitService.switchBranch(activeRepo, name);
      await onReloadRepo();
      onClose();
    } catch (err) {
      const errStr = String(err);
      inlineError = errStr;
      let handled = false;
      if (onOperationError) {
        handled = Boolean(
          await onOperationError(errStr, "Switch Branch", () => handleSwitchBranch(name))
        );
      }
      if (handled) {
        onClose();
      } else {
        onStatus(errStr, true);
      }
    }
  }

  async function handleCreateBranch() {
    const name = newBranchName.trim();
    if (!name) {
      inlineError = t("branchModal.emptyNameError");
      return;
    }
    inlineError = "";
    try {
      await gitService.createBranch(activeRepo, name);
      onStatus(t("branchModal.createdStatus", { name }));
      await onReloadRepo();
      onClose();
    } catch (err) {
      inlineError = String(err);
      onStatus(String(err), true);
    }
  }

  function promptDeleteBranch(b: BranchInfo) {
    if (b.is_current) {
      inlineError = t("branchModal.cannotDeleteCurrentTooltip");
      return;
    }
    inlineError = "";
    branchToRename = null;
    branchToDelete = b;
    alsoDeleteOnRemote = false;
  }

  async function handleExecuteDelete() {
    if (!branchToDelete) return;
    const target = branchToDelete;
    const deletedName = target.name;
    branchToDelete = null;
    inlineError = "";

    // Optimistic UI update: instantly remove from list (0ms latency)
    branches = branches.filter((b) => b.name !== deletedName);

    try {
      if (target.is_remote) {
        onStatus(t("branchModal.deletingRemoteStatus", { name: target.name }));
        await gitService.deleteRemoteBranch(activeRepo, target.name, target.remote_name || undefined);
        branches = await gitService.listBranches(activeRepo);
        await onReloadRepo();
        onStatus(t("branchModal.deletedRemoteStatus", { name: target.name }));
      } else {
        await gitService.deleteBranch(activeRepo, target.name, true);
        if (alsoDeleteOnRemote && hasRemote && (target.upstream || target.is_pushed)) {
          const rem = target.upstream?.split("/")[0] || "origin";
          if (!target.upstream_gone) {
            try {
              onStatus(t("branchModal.deletingRemoteStatus", { name: target.name }));
              await gitService.deleteRemoteBranch(activeRepo, target.name, rem);
            } catch (rErr) {
              onStatus(String(rErr), true);
            }
          }
        }
        branches = await gitService.listBranches(activeRepo);
        await onReloadRepo();
        onStatus(t("branchModal.deletedStatus", { name: target.name }));
      }
    } catch (err) {
      // Rollback on unexpected error
      branches = await gitService.listBranches(activeRepo);
      inlineError = String(err);
      onStatus(String(err), true);
    }
  }

  function handleStartRename(name: string) {
    branchToRename = name;
    renameBranchInput = name;
    alsoRenameOnRemote = false;
    branchToDelete = null;
    inlineError = "";
  }

  async function handleConfirmRename() {
    if (!branchToRename) return;
    const oldName = branchToRename;
    const newName = renameBranchInput.trim();
    if (!newName || newName === oldName) {
      branchToRename = null;
      return;
    }
    const targetBranch = branches.find((b) => b.name === oldName);
    const syncRemote = alsoRenameOnRemote && hasRemote && targetBranch && (targetBranch.upstream || targetBranch.is_pushed);
    const remoteTarget = targetBranch?.upstream?.split("/")[0] || "origin";
    const wasUpstreamGone = targetBranch?.upstream_gone ?? false;

    // Optimistic UI update: instantly rename in branches list (0ms latency)
    branches = branches.map((b) => (b.name === oldName ? { ...b, name: newName } : b));
    branchToRename = null;
    inlineError = "";

    try {
      await gitService.renameBranch(activeRepo, oldName, newName);
      if (syncRemote) {
        onStatus(t("branchModal.renamingRemoteStatus", { name: newName }));
        if (!wasUpstreamGone) {
          try {
            await gitService.deleteRemoteBranch(activeRepo, oldName, remoteTarget);
          } catch (remoteErr) {
            onStatus(String(remoteErr), true);
          }
        }
        await gitService.gitPush(activeRepo, remoteTarget, newName);
      }
      branches = await gitService.listBranches(activeRepo);
      await onReloadRepo();
      onStatus(t("branchModal.renamedStatus", { name: newName }));
    } catch (err) {
      // Rollback on unexpected error
      branches = await gitService.listBranches(activeRepo);
      inlineError = String(err);
      onStatus(String(err), true);
    }
  }

  function handleMergeBranch(targetName: string) {
    onRequestConfirm(
      t("branchModal.mergeConfirmTitle"),
      t("branchModal.mergeConfirmMsg", { target: targetName, current: formatBranchDisplay(currentBranch) }),
      async () => {
        try {
          onStatus(t("branchModal.mergingStatus", { name: targetName }));
          const res = await gitService.mergeBranch(activeRepo, targetName);
          onStatus(res || t("branchModal.mergedStatus"));
          await onReloadRepo();
          onClose();
        } catch (err) {
          const errStr = String(err);
          let handled = false;
          if (onOperationError) {
            handled = Boolean(
              await onOperationError(errStr, "Merge", async () => {
                const r = await gitService.mergeBranch(activeRepo, targetName);
                onStatus(r || t("branchModal.mergedStatus"));
                await onReloadRepo();
              })
            );
          }
          if (handled) {
            onClose();
          } else {
            inlineError = errStr;
            onStatus(errStr, true);
          }
        }
      },
      false
    );
  }

  function handleRebase(targetName: string) {
    onRequestConfirm(
      t("branchModal.rebaseConfirmTitle"),
      t("branchModal.rebaseConfirmMsg", { current: formatBranchDisplay(currentBranch), target: targetName }),
      async () => {
        try {
          onStatus(t("branchModal.rebasingStatus", { name: targetName }));
          const res = await gitService.rebaseBranch(activeRepo, targetName);
          onStatus(res || t("branchModal.rebasedStatus"));
          await onReloadRepo();
          onClose();
        } catch (err) {
          const errStr = String(err);
          let handled = false;
          if (onOperationError) {
            handled = Boolean(
              await onOperationError(errStr, "Rebase", async () => {
                const r = await gitService.rebaseBranch(activeRepo, targetName);
                onStatus(r || t("branchModal.rebasedStatus"));
                await onReloadRepo();
              })
            );
          }
          if (handled) {
            onClose();
          } else {
            inlineError = errStr;
            onStatus(errStr, true);
          }
        }
      },
      false
    );
  }

  async function handlePushBranch(name: string) {
    inlineError = "";
    pushingBranch = name;
    try {
      onStatus(t("branchModal.pushingStatus", { name }));
      const res = await gitService.gitPush(activeRepo, undefined, name);
      branches = await gitService.listBranches(activeRepo);
      await onReloadRepo();
      onStatus(res || t("branchModal.pushedStatus", { name }));
    } catch (err) {
      inlineError = String(err);
      let handled = false;
      if (onPushError) {
        handled = Boolean(await onPushError(inlineError, name));
      }
      if (!handled) {
        onStatus(t("branchModal.pushFailedStatus", { name, err: String(err) }), true);
      }
    } finally {
      pushingBranch = null;
    }
  }

  function openBranchContextMenu(e: MouseEvent, b: BranchInfo) {
    const items: ContextMenuItem[] = [];

    if (b.is_remote) {
      items.push({
        label: t("ctx.branch.switchTo"),
        onSelect: () => {
          void handleSwitchBranch(b.name);
        },
      });
      items.push({
        label: t("ctx.branch.copyName"),
        onSelect: () => {
          void copyTextToClipboard(
            b.name,
            onStatus,
            t("ctx.status.copiedBranch", { branch: b.name })
          );
        },
      });
      items.push({ type: "divider" });
      items.push({
        label: t("ctx.branch.deleteRemote"),
        danger: true,
        onSelect: () => promptDeleteBranch(b),
      });
      openContextMenu(e, items);
      return;
    }

    if (!b.is_current) {
      items.push({
        label: t("ctx.branch.switchTo"),
        onSelect: () => {
          void handleSwitchBranch(b.name);
        },
      });
    }
    if (hasRemote && hasCommits && (!b.is_pushed || b.ahead > 0)) {
      items.push({
        label: t("ctx.branch.push"),
        disabled: pushingBranch === b.name,
        onSelect: () => {
          void handlePushBranch(b.name);
        },
      });
    }
    if (!b.is_current) {
      items.push({
        label: t("ctx.branch.mergeInto", { target: formatBranchDisplay(currentBranch) }),
        onSelect: () => handleMergeBranch(b.name),
      });
      items.push({
        label: t("ctx.branch.rebaseOnto", {
          current: formatBranchDisplay(currentBranch),
          target: b.name,
        }),
        onSelect: () => handleRebase(b.name),
      });
    }

    items.push({ type: "divider" });
    items.push({
      label: b.is_current ? t("ctx.branch.renameActive") : t("ctx.branch.rename"),
      onSelect: () => handleStartRename(b.name),
    });
    items.push({
      label: t("ctx.branch.copyName"),
      onSelect: () => {
        void copyTextToClipboard(
          b.name,
          onStatus,
          t("ctx.status.copiedBranch", { branch: b.name })
        );
      },
    });
    items.push({
      label: t("ctx.branch.delete"),
      danger: !b.is_current,
      disabled: b.is_current,
      onSelect: () => {
        if (!b.is_current) promptDeleteBranch(b);
      },
    });

    openContextMenu(e, items);
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="modal-overlay"
  role="presentation"
  onclick={(e) => {
    if (e.target === e.currentTarget) onClose();
  }}
  onkeydown={(e) => {
    if (e.key === "Escape") onClose();
  }}
>
  <div
    class="modal-dialog"
    style="--modal-w: 510px;"
    role="dialog"
    aria-modal="true"
    aria-labelledby="branch-modal-title"
  >
    <div class="modal-header">
      <span id="branch-modal-title">{t("branchModal.title")}</span>
      <button type="button" class="icon-btn compact" onclick={onClose} aria-label={t("common.closeDialog")}>
        <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
          <path d="M3.72 3.72a.75.75 0 0 1 1.06 0L8 6.94l3.22-3.22a.749.749 0 0 1 1.275.326.749.749 0 0 1-.215.734L9.06 8l3.22 3.22a.749.749 0 0 1-.326 1.275.749.749 0 0 1-.734-.215L8 9.06l-3.22 3.22a.751.751 0 0 1-1.042-.018.751.751 0 0 1-.018-1.042L6.94 8 3.72 4.78a.75.75 0 0 1 0-1.06Z" />
        </svg>
      </button>
    </div>

    <div class="modal-body">
      <input
        type="search"
        class="form-input"
        aria-label={t("branchModal.filterAria")}
        placeholder={t("branchModal.filterPlaceholder")}
        bind:value={branchSearch}
      />
      {#if hasRemote}
        <div class="branch-toolbar">
          <div class="branch-tabs" role="tablist">
            <button
              type="button"
              role="tab"
              aria-selected={branchFilterTab === "all"}
              class="branch-tab {branchFilterTab === 'all' ? 'active' : ''}"
              onclick={() => (branchFilterTab = "all")}
            >
              {t("branchModal.tabAll", { count: unifiedBranches.length })}
            </button>
            <button
              type="button"
              role="tab"
              aria-selected={branchFilterTab === "local"}
              class="branch-tab {branchFilterTab === 'local' ? 'active' : ''}"
              onclick={() => (branchFilterTab = "local")}
            >
              {t("branchModal.tabLocal", { count: localBranches.length })}
            </button>
            <button
              type="button"
              role="tab"
              aria-selected={branchFilterTab === "remote"}
              class="branch-tab {branchFilterTab === 'remote' ? 'active' : ''}"
              onclick={() => (branchFilterTab = "remote")}
            >
              {t("branchModal.tabRemote", { count: remoteBranches.length })}
            </button>
          </div>
          <button
            type="button"
            class="btn-secondary btn-xs"
            disabled={isFetchingBranches}
            onclick={handleFetchAllBranches}
          >
            {isFetchingBranches ? t("branchModal.fetchingBtn") : t("branchModal.fetchAllBtn")}
          </button>
        </div>
      {/if}
      <div style="display: flex; flex-wrap: wrap; gap: 6px;">
        <input
          type="text"
          class="form-input"
          aria-label={t("branchModal.newNameAria")}
          placeholder={t("branchModal.newNamePlaceholder")}
          style="flex: 1 1 180px;"
          bind:value={newBranchName}
          onkeydown={(e) => {
            if (e.key === "Enter") handleCreateBranch();
          }}
        />
        <button type="button" onclick={handleCreateBranch} class="btn-primary">{t("branchModal.createBtn")}</button>
      </div>

      {#if inlineError}
        <div class="alert-banner" style="border-radius: var(--radius-sm); border: 1px solid var(--danger-border);">
          <span>{inlineError}</span>
        </div>
      {/if}

      {#if branchToDelete}
        <div style="display: flex; flex-direction: column; gap: 8px; padding: 10px; background: var(--bg-subtle); border-radius: var(--radius-sm); border: 1px solid var(--danger-border);">
          <div style="font-size: 12px; font-weight: 600; color: var(--danger-text);">
            {branchToDelete.is_remote ? t("branchModal.deleteRemoteTitle") : t("branchModal.deleteTitle")}
          </div>
          <div style="font-size: 11px; color: var(--text-secondary); line-height: 1.4;">
            {branchToDelete.is_remote
              ? t("branchModal.deleteRemoteMsg", { name: branchToDelete.name })
              : t("branchModal.deleteMsg", { name: branchToDelete.name })}
          </div>
          {#if !branchToDelete.is_remote && hasRemote && (branchToDelete.upstream || branchToDelete.is_pushed)}
            <label style="display: inline-flex; align-items: center; gap: 6px; font-size: 11px; cursor: pointer; color: var(--text-primary);">
              <input type="checkbox" bind:checked={alsoDeleteOnRemote} />
              <span>{t("branchModal.alsoDeleteRemoteCheckbox", { upstream: branchToDelete.upstream || branchToDelete.name })}</span>
            </label>
          {/if}
          <div style="display: flex; gap: 6px; justify-content: flex-end; margin-top: 2px;">
            <button type="button" onclick={() => (branchToDelete = null)} class="btn-secondary btn-xs">{t("common.cancel")}</button>
            <button type="button" onclick={handleExecuteDelete} class="btn-primary btn-xs btn-danger">{t("common.delete")}</button>
          </div>
        </div>
      {/if}

      {#if branchToRename}
        <div style="display: flex; flex-direction: column; gap: 8px; padding: 10px; background: var(--bg-subtle); border-radius: var(--radius-sm); border: 1px solid var(--border-default);">
          <div style="font-size: 12px; font-weight: 600; color: var(--text-primary);">
            {t("branchModal.renameLabel", { name: branchToRename })}
          </div>
          <div style="display: flex; gap: 6px; align-items: center;">
            <input
              type="text"
              class="form-input"
              aria-label={t("branchModal.renameAria")}
              bind:value={renameBranchInput}
              style="flex: 1; font-size: 11px;"
              onkeydown={(e) => {
                if (e.key === "Enter") handleConfirmRename();
              }}
            />
          </div>
          {#if renameTargetBranch && hasRemote && (renameTargetBranch.upstream || renameTargetBranch.is_pushed)}
            <label style="display: inline-flex; align-items: center; gap: 6px; font-size: 11px; cursor: pointer; color: var(--text-secondary);">
              <input type="checkbox" bind:checked={alsoRenameOnRemote} />
              <span>{t("branchModal.alsoRenameRemoteCheckbox", { upstream: renameTargetBranch.upstream || renameTargetBranch.name })}</span>
            </label>
          {/if}
          <div style="display: flex; gap: 6px; justify-content: flex-end; margin-top: 2px;">
            <button type="button" onclick={() => (branchToRename = null)} class="btn-secondary btn-xs">{t("common.cancel")}</button>
            <button type="button" onclick={handleConfirmRename} class="btn-primary btn-xs">{t("common.save")}</button>
          </div>
        </div>
      {/if}

      <div class="list-box">
        {#if filteredBranches.length === 0}
          <div style="padding: 20px; text-align: center; color: var(--text-muted); font-size: 12px;">
            {#if isLoadingBranches}
              {t("branchModal.loading")}
            {:else if !hasCommits && branches.length === 0}
              {t("branchModal.unbornEmpty", { branch: formatBranchDisplay(currentBranch) })}
            {:else}
              {t("branchModal.noneFound")}
            {/if}
          </div>
        {:else}
          {#each filteredBranches as b (b.name)}
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
              class="list-row {b.is_current ? 'is-active' : ''}"
              oncontextmenu={(e) => openBranchContextMenu(e, b)}
              onmouseenter={() => {
                hoveredBranchName = b.name;
                if (!b.is_current) onInspectBranch?.(b.name);
              }}
              onmouseleave={() => {
                if (hoveredBranchName === b.name) {
                  hoveredBranchName = null;
                }
              }}
            >
              <button
                type="button"
                onclick={() => handleSwitchBranch(b.name)}
                title={b.is_remote ? t("branchModal.checkoutRemoteTitle", { name: b.name }) : undefined}
                onfocus={() => {
                  focusedBranchName = b.name;
                  if (!b.is_current) onInspectBranch?.(b.name);
                }}
                onblur={() => {
                  if (focusedBranchName === b.name) {
                    focusedBranchName = null;
                  }
                }}
                style="flex: 1; min-width: 0; text-align: left; display: flex; align-items: center; gap: 6px;"
              >
                <span style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap; {b.is_current ? 'font-weight: 600;' : ''}">
                  {b.name}
                </span>
                {#if b.is_remote}
                  <span class="badge-accent" style="flex-shrink: 0;">{b.remote_name || t("branchModal.remoteBadge")}</span>
                {:else if b.is_current}
                  <span style="color: var(--text-accent); font-size: 10px; font-weight: 600; flex-shrink: 0;">{t("branchModal.activeTag")}</span>
                {/if}
                {#if !b.is_remote}
                  {#if b.upstream_gone}
                    <span class="badge-warn" style="flex-shrink: 0;" title={t("branchModal.remoteGoneTitle", { upstream: b.upstream || "" })}>
                      {t("branchModal.remoteGone")}
                    </span>
                  {:else if !hasRemote}
                    <span class="badge-neutral" style="flex-shrink: 0;" title={t("branchModal.localOnlyTitle")}>
                      {t("branchModal.localOnly")}
                    </span>
                  {:else if b.upstream}
                    {#if b.ahead === 0 && b.behind === 0}
                      <span class="badge-success" style="flex-shrink: 0;" title={t("branchModal.syncedTitle", { upstream: b.upstream })}>
                        {t("branchModal.synced")}
                      </span>
                    {:else}
                      <span class="badge-warning tabular-nums" style="flex-shrink: 0;" title={t("branchModal.upstreamTitle", { upstream: b.upstream })}>
                        {b.ahead > 0 ? `${t("branchModal.unpushedPrefix")} ↑${b.ahead}` : ""}{b.ahead > 0 && b.behind > 0 ? " " : ""}{b.behind > 0 ? `↓${b.behind}` : ""}
                      </span>
                    {/if}
                  {:else if b.is_pushed}
                    <span class="badge-neutral" style="flex-shrink: 0;" title={t("branchModal.onRemoteTitle")}>
                      {t("branchModal.onRemote")}
                    </span>
                  {:else}
                    <span class="badge-warning" style="flex-shrink: 0;" title={t("branchModal.neverPushedTitle")}>
                      {t("branchModal.unpushedPrefix")}
                    </span>
                  {/if}
                {/if}
                {#if !b.is_current && (hoveredBranchName === b.name || focusedBranchName === b.name) && branchDivergenceMap[b.name]}
                  {@const div = branchDivergenceMap[b.name]}
                  {#if div.ahead > 0 || div.behind > 0}
                    <span class="tabular-nums" style="font-size: 10px; color: var(--text-muted); font-family: var(--font-mono); display: inline-flex; gap: 4px; flex-shrink: 0;" title={t("branchModal.divergenceTitle", { branch: formatBranchDisplay(currentBranch) })}>
                      {#if div.ahead > 0}<span style="color: var(--diff-add-text);">+{div.ahead}</span>{/if}
                      {#if div.behind > 0}<span style="color: var(--diff-del-text);">-{div.behind}</span>{/if}
                    </span>
                  {/if}
                {/if}
              </button>

              <div class="branch-row-actions">
                {#if !b.is_remote && hasRemote && hasCommits && (!b.is_pushed || b.ahead > 0)}
                  <button
                    type="button"
                    onclick={() => handlePushBranch(b.name)}
                    disabled={pushingBranch === b.name}
                    class="btn-primary btn-xs"
                    title={b.upstream
                      ? t("branchModal.pushToTitle", { upstream: b.upstream })
                      : t("branchModal.pushSetUpstreamTitle", { name: b.name })}
                  >
                    {pushingBranch === b.name ? t("branchModal.pushingBtn") : t("branchModal.pushBtn")}
                  </button>
                {/if}
                {#if !b.is_current}
                  <button type="button" onclick={() => handleMergeBranch(b.name)} class="btn-secondary btn-xs" title={t("branchModal.mergeTitle")}>
                    {t("branchModal.mergeBtn")}
                  </button>
                  <button type="button" onclick={() => handleRebase(b.name)} class="btn-secondary btn-xs" title={t("branchModal.rebaseTitle")}>
                    {t("branchModal.rebaseBtn")}
                  </button>
                {/if}
                {#if !b.is_remote}
                  <button
                    type="button"
                    onclick={() => handleStartRename(b.name)}
                    class="btn-secondary btn-xs"
                    title={t("branchModal.renameTitle")}
                  >
                    {t("branchModal.renameBtn")}
                  </button>
                  <button
                    type="button"
                    onclick={() => {
                      if (!b.is_current) promptDeleteBranch(b);
                    }}
                    disabled={b.is_current}
                    class="btn-secondary btn-xs btn-danger branch-delete-btn"
                    style={b.is_current ? "opacity: 0.45; cursor: not-allowed;" : ""}
                    title={b.is_current ? t("branchModal.cannotDeleteCurrentTooltip") : t("branchModal.deleteBtnTitle")}
                  >
                    {t("branchModal.deleteBtn")}
                  </button>
                {:else}
                  <button
                    type="button"
                    onclick={() => promptDeleteBranch(b)}
                    class="btn-secondary btn-xs btn-danger branch-delete-btn"
                    title={t("branchModal.deleteRemoteBtnTitle")}
                  >
                    {t("branchModal.deleteRemoteBtn")}
                  </button>
                {/if}
              </div>
            </div>
          {/each}
        {/if}
      </div>
    </div>

    <div class="modal-actions">
      <button type="button" class="btn-secondary" onclick={onClose}>{t("common.close")}</button>
    </div>
  </div>
</div>
