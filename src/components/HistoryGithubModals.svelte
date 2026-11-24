<script lang="ts">
  import type { CommitInfo, IssueInfo, ParsedGitHubRepo, ParsedHostingRepo, PullRequestInfo, RebaseAction, RepoStatus } from "../types/git";
  import * as gitService from "../services/git";
  import { openExternalUrl } from "../utils/path";
  import { t } from "../i18n/index.svelte";

  export type HistoryGithubModalType =
    | "tag"
    | "rename_tag"
    | "squash"
    | "rebase"
    | "reword"
    | "pr"
    | "confirm";

  export interface RebaseItem {
    sha: string;
    short_sha: string;
    summary: string;
    author: string;
    action: RebaseAction;
    message?: string;
  }

  interface Props {
    modal: HistoryGithubModalType;
    activeRepo: string;
    currentBranch: string;
    githubRepoInfo: ParsedGitHubRepo | null;
    hostingRepoInfo?: ParsedHostingRepo | null;
    pullRequests: PullRequestInfo[];
    issues: IssueInfo[];
    prTab: "prs" | "issues";
    isLoadingGithub: boolean;
    githubError?: string;
    hasGithubAccount?: boolean;
    onOpenAuth?: () => void;
    tagTargetSha: string | null;
    renameTargetTag?: string | null;
    squashCount: number;
    squashInitialSummary: string;
    rebaseBaseSha: string;
    rebaseItems: RebaseItem[];
    rewordTargetCommit?: CommitInfo | null;

    status?: RepoStatus | null;
    onRequestConfirm?: (
      title: string,
      message: string,
      action: () => Promise<void>,
      danger?: boolean
    ) => void;
    confirmTitle: string;
    confirmMessage: string;
    confirmDanger: boolean;
    onConfirmSubmit: () => Promise<void>;
    onSwitchPrTab: (tab: "prs" | "issues") => Promise<void>;
    onCheckoutPr: (prNumber: number, headBranch?: string) => Promise<void>;
    onReloadRepo: () => Promise<void>;
    onReloadHistory: () => Promise<void>;
    onReloadTags: () => Promise<void>;
    onStatus: (msg: string, isError?: boolean) => void;
    onClose: () => void;
  }

  let {
    modal,
    activeRepo,
    currentBranch,
    githubRepoInfo,
    hostingRepoInfo = null,
    pullRequests,
    issues,
    prTab,
    isLoadingGithub,
    githubError = "",
    hasGithubAccount = false,
    onOpenAuth,
    tagTargetSha,
    renameTargetTag = null,
    squashCount,
    squashInitialSummary,
    rebaseBaseSha,
    rebaseItems = $bindable(),
    rewordTargetCommit = null,

    status = null,
    onRequestConfirm,
    confirmTitle,
    confirmMessage,
    confirmDanger,
    onConfirmSubmit,
    onSwitchPrTab,
    onCheckoutPr,
    onReloadRepo,
    onReloadHistory,
    onReloadTags,
    onStatus,
    onClose,
  }: Props = $props();

  let inlineError = $state("");
  let tagNameInput = $state("");
  let newTagNameInput = $state("");
  let squashSummary = $state("");
  let squashDescription = $state("");
  let rewordSummary = $state("");
  let rewordDescription = $state("");
  let isRewording = $state(false);
  let isSquashing = $state(false);
  let isRebasing = $state(false);

  $effect(() => {
    inlineError = "";
    if (modal === "tag") {
      tagNameInput = "";
    } else if (modal === "rename_tag") {
      newTagNameInput = renameTargetTag || "";
    } else if (modal === "squash") {
      squashSummary = squashInitialSummary;
      squashDescription = "";
    } else if (modal === "reword") {
      rewordSummary = rewordTargetCommit?.summary ?? "";
      rewordDescription = "";
      if (rewordTargetCommit && activeRepo) {
        gitService
          .getCommitDetail(activeRepo, rewordTargetCommit.sha)
          .then((d) => {
            if (modal === "reword" && d.description) {
              rewordDescription = d.description;
            }
          })
          .catch(() => {});
      }
    }
  });

  async function handleCreateTagSubmit() {
    const name = tagNameInput.trim();
    if (!name) return;
    try {
      onStatus(t("hgModal.creatingTagStatus", { name }));
      await gitService.createTag(activeRepo, name, tagTargetSha || undefined);
      onStatus(t("hgModal.tagCreatedStatus", { name }));
      await onReloadTags();
      onClose();
    } catch (err) {
      inlineError = String(err);
    }
  }

  async function handleRenameTagSubmit() {
    const newName = newTagNameInput.trim();
    if (!newName || !renameTargetTag || newName === renameTargetTag) return;
    try {
      onStatus(t("hgModal.renamingTagStatus", { old: renameTargetTag, new: newName }));
      await gitService.renameTag(activeRepo, renameTargetTag, newName);
      onStatus(t("hgModal.tagRenamedStatus", { old: renameTargetTag, new: newName }));
      await onReloadTags();
      onClose();
    } catch (err) {
      inlineError = String(err);
    }
  }

  async function handleSquashSubmit() {
    if (!squashSummary.trim() || isSquashing) return;
    isSquashing = true;
    try {
      onStatus(t("hgModal.squashingStatus", { count: squashCount }));
      await gitService.squashCommits(
        activeRepo,
        squashCount,
        squashSummary.trim(),
        squashDescription.trim() || undefined
      );
      onClose();
      onStatus(t("hgModal.squashedStatus", { count: squashCount }));
      await onReloadRepo();
      await onReloadHistory();
    } catch (err) {
      onStatus(String(err), true);
    } finally {
      isSquashing = false;
    }
  }

  function moveRebaseItem(index: number, direction: "up" | "down") {
    const targetIdx = direction === "up" ? index - 1 : index + 1;
    if (targetIdx < 0 || targetIdx >= rebaseItems.length) return;
    const next = [...rebaseItems];
    const [item] = next.splice(index, 1);
    next.splice(targetIdx, 0, item);
    rebaseItems = next;
  }

  let draggedItemIdx = $state<number | null>(null);

  function handleDragStart(index: number, e: DragEvent) {
    draggedItemIdx = index;
    if (e.dataTransfer) {
      e.dataTransfer.effectAllowed = "move";
      e.dataTransfer.setData("text/plain", String(index));
    }
  }

  function handleDragOver(e: DragEvent) {
    e.preventDefault();
    if (e.dataTransfer) {
      e.dataTransfer.dropEffect = "move";
    }
  }

  function handleDrop(targetIndex: number, e: DragEvent) {
    e.preventDefault();
    if (draggedItemIdx !== null && draggedItemIdx !== targetIndex) {
      const next = [...rebaseItems];
      const [item] = next.splice(draggedItemIdx, 1);
      next.splice(targetIndex, 0, item);
      rebaseItems = next;
    }
    draggedItemIdx = null;
  }

  function setRebaseAction(index: number, action: RebaseAction) {
    rebaseItems = rebaseItems.map((item, i) =>
      i === index ? { ...item, action, message: item.message ?? item.summary } : item
    );
  }

  const rebaseSummary = $derived.by(() => {
    let pick = 0, reword = 0, edit = 0, squash = 0, fixup = 0, drop = 0;
    for (const item of rebaseItems) {
      if (item.action === "pick") pick++;
      else if (item.action === "reword") reword++;
      else if (item.action === "edit") edit++;
      else if (item.action === "squash") squash++;
      else if (item.action === "fixup") fixup++;
      else if (item.action === "drop") drop++;
    }
    return { pick, reword, edit, squash, fixup, drop, total: rebaseItems.length };
  });

  async function executeInteractiveRebase() {
    if (isRebasing || !activeRepo || !rebaseBaseSha) return;
    const activeSteps = rebaseItems.filter((i) => i.action !== "drop");
    if (activeSteps.length === 0) {
      inlineError = t("hgModal.rebaseNeedPick");
      return;
    }
    isRebasing = true;
    try {
      const hasLock = await gitService.checkIndexLock(activeRepo);
      if (hasLock && onRequestConfirm) {
        onRequestConfirm(
          t("lockModal.title"),
          t("lockModal.message"),
          async () => {
            try {
              await gitService.clearIndexLock(activeRepo);
              onStatus(t("lockModal.clearedStatus"));
              inlineError = "";
              await executeInteractiveRebase();
            } catch (err) {
              inlineError = String(err);
            }
          },
          true
        );
        return;
      }
    } catch {
      // Ignore check failure
    }
    try {
      onStatus(t("hgModal.rebasingStatus"));
      await gitService.interactiveRebase(
        activeRepo,
        rebaseBaseSha,
        rebaseItems.map((i) => ({
          sha: i.sha,
          action: i.action,
          message: i.message?.trim() || undefined,
        }))
      );
      onClose();
      onStatus(t("hgModal.rebasedStatus"));
      await onReloadRepo();
      await onReloadHistory();
    } catch (err) {
      const errMsg = String(err);
      if (errMsg.includes("index.lock") && onRequestConfirm) {
        onRequestConfirm(
          t("lockModal.title"),
          t("lockModal.message"),
          async () => {
            try {
              await gitService.clearIndexLock(activeRepo);
              onStatus(t("lockModal.clearedStatus"));
              inlineError = "";
              await executeInteractiveRebase();
            } catch (clearErr) {
              inlineError = String(clearErr);
            }
          },
          true
        );
      } else {
        inlineError = errMsg;
      }
    } finally {
      isRebasing = false;
    }
  }
  async function executeRewordSubmit() {
    if (!rewordSummary.trim() || !activeRepo || !rewordTargetCommit) return;

    try {
      const currentRepoStatus = await gitService.inspectRepository(activeRepo);
      if (currentRepoStatus.files.length > 0) {
        inlineError = t("rewordModal.dirtyWarning");
        return;
      }
    } catch {
      if (status && status.files.length > 0) {
        inlineError = t("rewordModal.dirtyWarning");
        return;
      }
    }

    try {
      const hasLock = await gitService.checkIndexLock(activeRepo);
      if (hasLock && onRequestConfirm) {
        onRequestConfirm(
          t("lockModal.title"),
          t("lockModal.message"),
          async () => {
            try {
              await gitService.clearIndexLock(activeRepo);
              onStatus(t("lockModal.clearedStatus"));
              inlineError = "";
              await executeRewordSubmit();
            } catch (err) {
              inlineError = String(err);
            }
          },
          true
        );
        return;
      }
    } catch {
      // Ignore check failure
    }

    isRewording = true;
    inlineError = "";
    onStatus(t("rewordModal.updatingStatus"));

    try {
      const summary = rewordSummary.trim();
      const desc = rewordDescription.trim();
      const fullMessage = desc ? `${summary}\n\n${desc}` : summary;

      await gitService.rewordCommit(activeRepo, rewordTargetCommit.sha, fullMessage);

      onClose();
      onStatus(t("rewordModal.updatedStatus"));
      await onReloadRepo();
      await onReloadHistory();
    } catch (err) {
      const errMsg = String(err);
      if (errMsg.includes("index.lock") && onRequestConfirm) {
        onRequestConfirm(
          t("lockModal.title"),
          t("lockModal.message"),
          async () => {
            try {
              await gitService.clearIndexLock(activeRepo);
              onStatus(t("lockModal.clearedStatus"));
              inlineError = "";
              await executeRewordSubmit();
            } catch (clearErr) {
              inlineError = String(clearErr);
            }
          },
          true
        );
      } else {
        inlineError = errMsg;
        onStatus(errMsg, true);
      }
    } finally {
      isRewording = false;
    }
  }


  function handleCreatePR() {
    if (!currentBranch || currentBranch === "HEAD terlepas") return;
    if (hostingRepoInfo) {
      const base = hostingRepoInfo.web_url.replace(/\/+$/, "");
      let url = `${base}/compare/${encodeURIComponent(currentBranch)}?expand=1`;
      if (hostingRepoInfo.provider === "gitlab") {
        url = `${base}/-/merge_requests/new?merge_request%5Bsource_branch%5D=${encodeURIComponent(currentBranch)}`;
      } else if (hostingRepoInfo.provider === "bitbucket") {
        url = `${base}/pull-requests/new?source=${encodeURIComponent(currentBranch)}`;
      } else if (hostingRepoInfo.provider === "azure_devops") {
        url = `${base}/pullrequestcreate?sourceRef=${encodeURIComponent(currentBranch)}`;
      }
      openExternalUrl(url);
      return;
    }
    if (!githubRepoInfo) return;
    openExternalUrl(
      `https://${githubRepoInfo.host}/${githubRepoInfo.owner}/${githubRepoInfo.repo}/compare/${encodeURIComponent(currentBranch)}?expand=1`
    );
  }

  const modalTitle = $derived.by(() => {
    switch (modal) {
      case "tag": return t("hgModal.titleTag");
      case "rename_tag": return t("hgModal.titleRenameTag");
      case "squash": return t("hgModal.titleSquash", { count: squashCount });
      case "rebase": return t("hgModal.titleRebase");
      case "reword": return t("rewordModal.title");
      case "pr": {
        const label = hostingRepoInfo
          ? `${hostingRepoInfo.namespace}/${hostingRepoInfo.repo}`
          : githubRepoInfo
            ? `${githubRepoInfo.owner}/${githubRepoInfo.repo}`
            : t("hgModal.titlePrFallback");
        return t("hgModal.titlePr", { repo: label });
      }
      case "confirm": return confirmTitle;
    }
  });

  const modalWidth = $derived(
    modal === "rebase" ? "680px" : modal === "pr" ? "560px" : modal === "squash" ? "480px" : modal === "reword" ? "520px" : "420px"
  );
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
    style="--modal-w: {modalWidth};"
    role="dialog"
    aria-modal="true"
    aria-labelledby="hg-modal-title"
  >
    <div class="modal-header">
      <span id="hg-modal-title" style={modal === "confirm" && confirmDanger ? "color: var(--diff-del-text);" : ""}>
        {modalTitle}
      </span>
      <div style="display: flex; align-items: center; gap: 8px;">
        {#if modal === "pr"}
          <button
            type="button"
            onclick={handleCreatePR}
            class="btn-primary btn-xs"
            disabled={(!hostingRepoInfo && !githubRepoInfo) || currentBranch === "HEAD terlepas"}
          >
            {t("hgModal.createPrBtn")}
          </button>
        {/if}
        <button type="button" class="icon-btn compact" onclick={onClose} aria-label={t("common.closeDialog")}>
          <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
            <path d="M3.72 3.72a.75.75 0 0 1 1.06 0L8 6.94l3.22-3.22a.749.749 0 0 1 1.275.326.749.749 0 0 1-.215.734L9.06 8l3.22 3.22a.749.749 0 0 1-.326 1.275.749.749 0 0 1-.734-.215L8 9.06l-3.22 3.22a.751.751 0 0 1-1.042-.018.751.751 0 0 1-.018-1.042L6.94 8 3.72 4.78a.75.75 0 0 1 0-1.06Z" />
          </svg>
        </button>
      </div>
    </div>

    {#if inlineError}
      <div class="alert-banner" style="border-radius: var(--radius-sm); border: 1px solid var(--danger-border);">
        <span>{inlineError}</span>
      </div>
    {/if}

    <div class="modal-body">
      {#if modal === "tag"}
        <div style="font-size: 12px; color: var(--text-secondary);">
          {t("hgModal.targetCommit")} <span style="font-family: var(--font-mono); font-weight: 600; color: var(--text-primary);">{tagTargetSha?.slice(0, 7) || "HEAD"}</span>
        </div>
        <div class="form-field">
          <label class="form-label" for="tag-name-input">{t("hgModal.tagNameLabel")}</label>
          <input
            id="tag-name-input"
            type="text"
            class="form-input"
            placeholder={t("hgModal.tagNamePlaceholder")}
            bind:value={tagNameInput}
            onkeydown={(e) => {
              if (e.key === "Enter") handleCreateTagSubmit();
            }}
          />
        </div>

      {:else if modal === "rename_tag"}
        <div style="font-size: 12px; color: var(--text-secondary);">
          {t("hgModal.currentTagLabel")} <span style="font-family: var(--font-mono); font-weight: 600; color: var(--text-primary);">{renameTargetTag}</span>
        </div>
        <div class="form-field">
          <label class="form-label" for="tag-rename-input">{t("hgModal.newTagNameLabel")}</label>
          <input
            id="tag-rename-input"
            type="text"
            class="form-input"
            placeholder={t("hgModal.newTagNamePlaceholder")}
            bind:value={newTagNameInput}
            onkeydown={(e) => {
              if (e.key === "Enter") handleRenameTagSubmit();
            }}
          />
        </div>

      {:else if modal === "squash"}
        <div style="font-size: 12px; color: var(--text-secondary);">
          {t("hgModal.squashInfo", { count: squashCount })}
        </div>
        <div class="form-field">
          <div class="form-label-row">
            <label class="form-label" for="squash-summary">{t("hgModal.squashSummaryLabel")}</label>
            <span class="form-field-hint">{t("rewordModal.summaryHint")}</span>
          </div>
          <input
            id="squash-summary"
            type="text"
            class="form-input reword-summary-input"
            placeholder={t("hgModal.squashSummaryPlaceholder")}
            bind:value={squashSummary}
          />
        </div>
        <div class="form-field">
          <div class="form-label-row">
            <label class="form-label" for="squash-desc">{t("hgModal.squashDescLabel")}</label>
            <span class="form-field-hint">{t("rewordModal.descHint")}</span>
          </div>
          <textarea
            id="squash-desc"
            class="form-input reword-description-input"
            placeholder={t("hgModal.squashDescPlaceholder")}
            rows="5"
            bind:value={squashDescription}
          ></textarea>
        </div>
      {:else if modal === "reword"}
        <div class="reword-context">
          <span class="reword-context-label">{t("rewordModal.targetLabel")}</span>
          <code>{rewordTargetCommit?.short_sha || ""}</code>
          <span class="reword-context-summary">{rewordTargetCommit?.summary || ""}</span>
        </div>
        <div class="reword-form-intro">{t("rewordModal.formHint")}</div>
        <div class="form-field">
          <div class="form-label-row">
            <label class="form-label" for="reword-summary">{t("rewordModal.summaryLabel")}</label>
            <span class="form-field-hint">{t("rewordModal.summaryHint")}</span>
          </div>
          <input
            id="reword-summary"
            type="text"
            class="form-input reword-summary-input"
            placeholder={t("rewordModal.summaryPlaceholder")}
            bind:value={rewordSummary}
            disabled={isRewording}
            onkeydown={(e) => {
              if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
                void executeRewordSubmit();
              }
            }}
          />
        </div>
        <div class="form-field">
          <div class="form-label-row">
            <label class="form-label" for="reword-desc">{t("rewordModal.descLabel")}</label>
            <span class="form-field-hint">{t("rewordModal.descHint")}</span>
          </div>
          <textarea
            id="reword-desc"
            class="form-input reword-description-input"
            placeholder={t("rewordModal.descPlaceholder")}
            rows="6"
            bind:value={rewordDescription}
            disabled={isRewording}
            onkeydown={(e) => {
              if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
                void executeRewordSubmit();
              }
            }}
          ></textarea>
        </div>
        <div class="reword-submit-hint">
          <kbd>Ctrl</kbd><span>/</span><kbd>⌘</kbd><span>+</span><kbd>Enter</kbd>
          <span>{t("rewordModal.submitHint")}</span>
        </div>
      {:else if modal === "rebase"}
        <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 6px;">
          <p style="font-size: 12px; color: var(--text-secondary); margin: 0;">
            {t("hgModal.rebaseHint")}
            <code style="font-family: var(--font-mono); color: var(--text-accent);">{rebaseBaseSha.substring(0, 7)}</code>
          </p>
          <span style="font-size: 11px; color: var(--text-muted); font-family: var(--font-mono);">
            {t("interactiveRebase.commitsCount", { count: rebaseItems.length })}
          </span>
        </div>

        <div class="list-box rebase-item-list" style="max-height: 380px; padding: 6px; display: flex; flex-direction: column; gap: 6px; overflow-y: auto;">
          {#each rebaseItems as item, idx (item.sha)}
            <div
              class="rebase-card {item.action === 'drop' ? 'is-dropped' : ''}"
              draggable="true"
              ondragstart={(e) => handleDragStart(idx, e)}
              ondragover={handleDragOver}
              ondrop={(e) => handleDrop(idx, e)}
              role="listitem"
            >
              <div class="rebase-drag-handle" title={t("history.dragToReorder")}>
                ⋮⋮
              </div>

              <div class="rebase-move-btns">
                <button
                  type="button"
                  class="btn-secondary btn-xs rebase-arrow-btn"
                  disabled={idx === 0}
                  aria-label={t("hgModal.moveUpAria", { sha: item.short_sha })}
                  onclick={() => moveRebaseItem(idx, "up")}
                >
                  ▲
                </button>
                <button
                  type="button"
                  class="btn-secondary btn-xs rebase-arrow-btn"
                  disabled={idx === rebaseItems.length - 1}
                  aria-label={t("hgModal.moveDownAria", { sha: item.short_sha })}
                  onclick={() => moveRebaseItem(idx, "down")}
                >
                  ▼
                </button>
              </div>

              <select
                class="form-select form-select-xs rebase-action-select action-{item.action}"
                value={item.action}
                onchange={(e) => setRebaseAction(idx, e.currentTarget.value as RebaseAction)}
              >
                <option value="pick">pick</option>
                <option value="reword">reword</option>
                <option value="edit">edit</option>
                <option value="squash">squash</option>
                <option value="fixup">fixup</option>
                <option value="drop">drop</option>
              </select>

              <span class="rebase-sha-badge">
                {item.short_sha}
              </span>

              <div class="rebase-content-cell">
                {#if item.action === "reword"}
                  <input
                    type="text"
                    class="form-input form-input-xs"
                    placeholder={t("interactiveRebase.rewordPlaceholder")}
                    bind:value={item.message}
                  />
                {:else if item.action === "squash"}
                  <input
                    type="text"
                    class="form-input form-input-xs"
                    placeholder={item.summary}
                    bind:value={item.message}
                  />
                {:else}
                  <span
                    class="rebase-summary-text {item.action === 'drop' ? 'is-dropped' : ''}"
                    title={item.summary}
                  >
                    {item.summary}
                  </span>
                {/if}
              </div>

              <span class="rebase-author-text">
                {item.author}
              </span>
            </div>
          {/each}
        </div>

        <div class="rebase-summary-bar">
          <span class="rebase-summary-pill pick">{rebaseSummary.pick} pick</span>
          <span class="rebase-summary-pill reword">{rebaseSummary.reword} reword</span>
          <span class="rebase-summary-pill edit">{rebaseSummary.edit} edit</span>
          <span class="rebase-summary-pill squash">{rebaseSummary.squash} squash</span>
          <span class="rebase-summary-pill fixup">{rebaseSummary.fixup} fixup</span>
          <span class="rebase-summary-pill drop">{rebaseSummary.drop} drop</span>
        </div>
      {:else if modal === "pr"}
        <div class="seg-group" style="align-self: flex-start;">
          <button
            type="button"
            class="seg-btn {prTab === 'prs' ? 'active' : ''}"
            onclick={() => onSwitchPrTab("prs")}
          >
            {t("hgModal.prsTab", { count: pullRequests.length })}
          </button>
          <button
            type="button"
            class="seg-btn {prTab === 'issues' ? 'active' : ''}"
            onclick={() => onSwitchPrTab("issues")}
          >
            {t("hgModal.issuesTab", { count: issues.length })}
          </button>
        </div>
        {#if githubError}
          <div class="alert-banner" style="border-radius: var(--radius-sm); border: 1px solid var(--danger-border); flex-wrap: wrap;">
            <span style="flex: 1; min-width: 180px;">{githubError}</span>
            {#if !hasGithubAccount && onOpenAuth}
              <button type="button" class="btn-secondary btn-xs" onclick={onOpenAuth}>
                {t("hgModal.signInGithub")}
              </button>
            {/if}
          </div>
        {/if}

        {#if isLoadingGithub}
          <div style="padding: 28px; text-align: center; color: var(--text-muted); font-size: 12px;">
            {t("hgModal.loadingGithub")}
          </div>
        {:else if prTab === "prs"}
          <div class="list-box" style="max-height: 340px;">
            {#if pullRequests.length === 0}
              <div style="padding: 24px; text-align: center; color: var(--text-muted); font-size: 12px;">
                {t("hgModal.noOpenPrs")}
              </div>
            {:else}
              {#each pullRequests as pr (pr.id)}
                <div class="list-row">
                  <div style="flex: 1; min-width: 0;">
                    <div style="font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;">
                      <span style="color: var(--text-accent); font-family: var(--font-mono);">#{pr.number}</span> {pr.title}
                    </div>
                    <div style="font-size: 10px; color: var(--text-muted);">
                      {t("hgModal.byAuthor", { author: pr.author })} · {pr.head_branch} &rarr; {pr.base_branch}
                    </div>
                  </div>
                  <div style="display: flex; gap: 4px; flex-shrink: 0;">
                    <button type="button" onclick={() => onCheckoutPr(pr.number, pr.head_branch)} class="btn-primary btn-xs">
                      {t("hgModal.checkout")}
                    </button>
                    <button type="button" onclick={() => openExternalUrl(pr.html_url)} class="btn-secondary btn-xs">
                      {t("hgModal.web")}
                    </button>
                  </div>
                </div>
              {/each}
            {/if}
          </div>
        {:else}
          <div class="list-box" style="max-height: 340px;">
            {#if issues.length === 0}
              <div style="padding: 24px; text-align: center; color: var(--text-muted); font-size: 12px;">
                {t("hgModal.noOpenIssues")}
              </div>
            {:else}
              {#each issues as issue (issue.id)}
                <div class="list-row">
                  <div style="flex: 1; min-width: 0;">
                    <div style="font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;">
                      <span style="color: var(--diff-add-text); font-family: var(--font-mono);">#{issue.number}</span> {issue.title}
                    </div>
                    <div style="font-size: 10px; color: var(--text-muted);">
                      {issue.comments_count === 1
                        ? t("hgModal.issueMetaOne", { author: issue.author, count: issue.comments_count })
                        : t("hgModal.issueMetaMany", { author: issue.author, count: issue.comments_count })}
                    </div>
                  </div>
                  <button type="button" onclick={() => openExternalUrl(issue.html_url)} class="btn-secondary btn-xs">
                    {t("hgModal.web")}
                  </button>
                </div>
              {/each}
            {/if}
          </div>
        {/if}

      {:else if modal === "confirm"}
        <p style="font-size: 12px; color: var(--text-secondary); line-height: 1.5;">
          {confirmMessage}
        </p>
      {/if}
    </div>

    <div class="modal-actions">
      {#if modal === "tag"}
        <button type="button" onclick={onClose} class="btn-secondary">{t("common.cancel")}</button>
        <button type="button" onclick={handleCreateTagSubmit} class="btn-primary" disabled={!tagNameInput.trim()}>
          {t("hgModal.createTagBtn")}
        </button>
      {:else if modal === "rename_tag"}
        <button type="button" onclick={onClose} class="btn-secondary">{t("common.cancel")}</button>
        <button type="button" onclick={handleRenameTagSubmit} class="btn-primary" disabled={!newTagNameInput.trim() || newTagNameInput.trim() === renameTargetTag}>
          {t("hgModal.renameTagBtn")}
        </button>
      {:else if modal === "squash"}
        <button type="button" class="btn-secondary" onclick={onClose} disabled={isSquashing}>{t("common.cancel")}</button>
        <button type="button" class="btn-primary" onclick={handleSquashSubmit} disabled={!squashSummary.trim() || isSquashing}>
          {isSquashing ? t("hgModal.squashingStatus", { count: squashCount }) : t("hgModal.confirmSquashBtn")}
        </button>
      {:else if modal === "reword"}
        <button type="button" class="btn-secondary" onclick={onClose} disabled={isRewording}>{t("common.cancel")}</button>
        <button
          type="button"
          class="btn-primary"
          onclick={executeRewordSubmit}
          disabled={!rewordSummary.trim() || isRewording}
        >
          {isRewording ? t("rewordModal.updatingStatus") : t("rewordModal.saveBtn")}
        </button>
      {:else if modal === "rebase"}
        <button type="button" class="btn-secondary" onclick={onClose} disabled={isRebasing}>{t("common.cancel")}</button>
        <button
          type="button"
          class="btn-primary"
          onclick={executeInteractiveRebase}
          disabled={isRebasing || rebaseItems.filter((i) => i.action !== "drop").length === 0}
        >
          {isRebasing ? t("interactiveRebase.rebasing") : t("hgModal.startRebaseBtn")}
        </button>
      {:else if modal === "confirm"}
        <button type="button" class="btn-secondary" onclick={onClose}>{t("common.cancel")}</button>
        <button
          type="button"
          class="btn-primary {confirmDanger ? 'btn-danger-solid' : ''}"
          onclick={onConfirmSubmit}
        >
          {t("common.confirmContinue")}
        </button>
      {:else}
        <button type="button" onclick={onClose} class="btn-secondary">{t("common.close")}</button>
      {/if}
    </div>
  </div>
</div>
