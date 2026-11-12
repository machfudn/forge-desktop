<script lang="ts">
  import type { IssueInfo, ParsedGitHubRepo, PullRequestInfo } from "../types/git";
  import * as gitService from "../services/git";
  import { openExternalUrl } from "../utils/path";
  import { t } from "../i18n/index.svelte";

  export type HistoryGithubModalType =
    | "tag"
    | "squash"
    | "rebase"
    | "pr"
    | "confirm";

  export interface RebaseItem {
    sha: string;
    short_sha: string;
    summary: string;
    author: string;
    action: "pick" | "drop";
  }

  interface Props {
    modal: HistoryGithubModalType;
    activeRepo: string;
    currentBranch: string;
    githubRepoInfo: ParsedGitHubRepo | null;
    pullRequests: PullRequestInfo[];
    issues: IssueInfo[];
    prTab: "prs" | "issues";
    isLoadingGithub: boolean;
    githubError?: string;
    hasGithubAccount?: boolean;
    onOpenAuth?: () => void;
    tagTargetSha: string | null;
    squashCount: number;
    squashInitialSummary: string;
    rebaseBaseSha: string;
    rebaseItems: RebaseItem[];
    confirmTitle: string;
    confirmMessage: string;
    confirmDanger: boolean;
    onConfirmSubmit: () => Promise<void>;
    onSwitchPrTab: (tab: "prs" | "issues") => Promise<void>;
    onCheckoutPr: (prNumber: number) => Promise<void>;
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
    pullRequests,
    issues,
    prTab,
    isLoadingGithub,
    githubError = "",
    hasGithubAccount = false,
    onOpenAuth,
    tagTargetSha,
    squashCount,
    squashInitialSummary,
    rebaseBaseSha,
    rebaseItems = $bindable(),
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
  let squashSummary = $state("");
  let squashDescription = $state("");

  $effect(() => {
    inlineError = "";
    if (modal === "tag") {
      tagNameInput = "";
    } else if (modal === "squash") {
      squashSummary = squashInitialSummary;
      squashDescription = "";
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

  async function handleSquashSubmit() {
    if (!squashSummary.trim()) return;
    onClose();
    try {
      onStatus(t("hgModal.squashingStatus", { count: squashCount }));
      await gitService.squashCommits(
        activeRepo,
        squashCount,
        squashSummary.trim(),
        squashDescription.trim() || undefined
      );
      onStatus(t("hgModal.squashedStatus", { count: squashCount }));
      await onReloadRepo();
      await onReloadHistory();
    } catch (err) {
      onStatus(String(err), true);
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

  function toggleRebaseAction(index: number) {
    rebaseItems = rebaseItems.map((item, i) =>
      i === index ? { ...item, action: item.action === "pick" ? "drop" : "pick" } : item
    );
  }

  async function executeInteractiveRebase() {
    if (!activeRepo || !rebaseBaseSha) return;
    const activeSteps = rebaseItems.filter((i) => i.action === "pick");
    if (activeSteps.length === 0) {
      inlineError = t("hgModal.rebaseNeedPick");
      return;
    }
    try {
      onStatus(t("hgModal.rebasingStatus"));
      await gitService.interactiveRebase(
        activeRepo,
        rebaseBaseSha,
        rebaseItems.map((i) => ({ sha: i.sha, action: i.action }))
      );
      onClose();
      onStatus(t("hgModal.rebasedStatus"));
      await onReloadRepo();
      await onReloadHistory();
    } catch (err) {
      inlineError = String(err);
    }
  }

  function handleCreatePR() {
    if (!githubRepoInfo || !currentBranch || currentBranch === "HEAD terlepas") return;
    const url = `https://${githubRepoInfo.host}/${githubRepoInfo.owner}/${githubRepoInfo.repo}/compare/${encodeURIComponent(currentBranch)}?expand=1`;
    openExternalUrl(url);
  }

  const modalTitle = $derived.by(() => {
    switch (modal) {
      case "tag": return t("hgModal.titleTag");
      case "squash": return t("hgModal.titleSquash", { count: squashCount });
      case "rebase": return t("hgModal.titleRebase");
      case "pr":
        return t("hgModal.titlePr", {
          repo: githubRepoInfo
            ? `${githubRepoInfo.owner}/${githubRepoInfo.repo}`
            : t("hgModal.titlePrFallback"),
        });
      case "confirm": return confirmTitle;
    }
  });

  const modalWidth = $derived(
    modal === "rebase" ? "580px" : modal === "pr" ? "560px" : modal === "squash" ? "440px" : "420px"
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
            disabled={!githubRepoInfo || currentBranch === "HEAD terlepas"}
            title={t("hgModal.openBrowserPrTitle")}
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

      {:else if modal === "squash"}
        <div style="font-size: 12px; color: var(--text-secondary);">
          {t("hgModal.squashInfo", { count: squashCount })}
        </div>
        <div class="form-field">
          <label class="form-label" for="squash-summary">{t("hgModal.squashSummaryLabel")}</label>
          <input id="squash-summary" type="text" class="form-input" placeholder={t("hgModal.squashSummaryPlaceholder")} bind:value={squashSummary} />
        </div>
        <div class="form-field">
          <label class="form-label" for="squash-desc">{t("hgModal.squashDescLabel")}</label>
          <textarea id="squash-desc" class="form-input" placeholder={t("hgModal.squashDescPlaceholder")} rows="3" bind:value={squashDescription} style="resize: vertical;"></textarea>
        </div>

      {:else if modal === "rebase"}
        <p style="font-size: 12px; color: var(--text-secondary);">
          {t("hgModal.rebaseHint")}
          <code style="font-family: var(--font-mono); color: var(--text-accent);">{rebaseBaseSha.substring(0, 7)}</code>.
        </p>
        <div class="list-box" style="max-height: 300px; padding: 6px; display: flex; flex-direction: column; gap: 5px;">
          {#each rebaseItems as item, idx (item.sha)}
            <div style="display: flex; align-items: center; gap: 8px; padding: 6px 8px; border-radius: var(--radius-sm); background: var(--bg-panel); border: 1px solid var(--border-muted);">
              <div style="display: flex; flex-direction: column; gap: 2px;">
                <button
                  type="button"
                  class="btn-secondary btn-xs"
                  style="padding: 0 5px; font-size: 9px; line-height: 12px; height: 14px;"
                  disabled={idx === 0}
                  aria-label={t("hgModal.moveUpAria", { sha: item.short_sha })}
                  onclick={() => moveRebaseItem(idx, "up")}
                >
                  ▲
                </button>
                <button
                  type="button"
                  class="btn-secondary btn-xs"
                  style="padding: 0 5px; font-size: 9px; line-height: 12px; height: 14px;"
                  disabled={idx === rebaseItems.length - 1}
                  aria-label={t("hgModal.moveDownAria", { sha: item.short_sha })}
                  onclick={() => moveRebaseItem(idx, "down")}
                >
                  ▼
                </button>
              </div>
              <button
                type="button"
                class="btn-secondary btn-xs"
                style="font-weight: 700; min-width: 48px; text-transform: uppercase; border-color: {item.action === 'pick' ? 'var(--diff-add-text)' : 'var(--diff-del-text)'}; color: {item.action === 'pick' ? 'var(--diff-add-text)' : 'var(--diff-del-text)'};"
                onclick={() => toggleRebaseAction(idx)}
              >
                {item.action}
              </button>
              <span style="font-family: var(--font-mono); font-size: 11px; color: var(--text-muted); font-variant-numeric: tabular-nums;">
                {item.short_sha}
              </span>
              <span
                style="flex: 1; font-size: 12px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: {item.action === 'drop' ? 'var(--text-muted)' : 'var(--text-primary)'}; {item.action === 'drop' ? 'text-decoration: line-through;' : ''}"
                title={item.summary}
              >
                {item.summary}
              </span>
              <span style="font-size: 11px; color: var(--text-muted); max-width: 100px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
                {item.author}
              </span>
            </div>
          {/each}
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
                    <button type="button" onclick={() => onCheckoutPr(pr.number)} class="btn-primary btn-xs">
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
      {:else if modal === "squash"}
        <button type="button" class="btn-secondary" onclick={onClose}>{t("common.cancel")}</button>
        <button type="button" class="btn-primary" onclick={handleSquashSubmit} disabled={!squashSummary.trim()}>
          {t("hgModal.confirmSquashBtn")}
        </button>
      {:else if modal === "rebase"}
        <button type="button" class="btn-secondary" onclick={onClose}>{t("common.cancel")}</button>
        <button
          type="button"
          class="btn-primary"
          onclick={executeInteractiveRebase}
          disabled={rebaseItems.filter((i) => i.action === "pick").length === 0}
        >
          {t("hgModal.startRebaseBtn")}
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
