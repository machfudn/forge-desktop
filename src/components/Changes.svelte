<script lang="ts">
  import type { FileEntry, RepoStatus } from "../types/git";
  import { t } from "../i18n/index.svelte";
  import { joinRepoPath } from "../utils/path";
  import {
    copyTextToClipboard,
    openContextMenu,
    type ContextMenuItem,
  } from "../lib/contextMenu.svelte";

  interface Props {
    repoPath?: string;
    externalEditorLabel?: string;
    status: RepoStatus;
    selectedFile: FileEntry | null;
    onSelectFile: (file: FileEntry) => void;
    onToggleStage: (file: FileEntry) => void;
    onStageAll: () => void;
    onUnstageAll: () => void;
    onCommit: (summary: string, description?: string, amend?: boolean) => void;
    onUndoCommit?: () => void;
    onAbortMerge?: () => void;
    onAbortRebase?: () => void;
    onDiscardFile?: (file: FileEntry) => void;
    onDiscardAll?: () => void;
    onAddToGitignore?: (pattern: string) => void;
    onRevealFile?: (filePath: string) => void;
    onOpenInEditor?: (filePath: string) => void;
    onOpenConflictEditor?: (filePath: string) => void;
    onInspectFile?: (filePath: string, mode: "blame" | "history") => void;
    onAiGenerateCommit?: () => Promise<{ summary?: string | null; description?: string | null } | null>;
    onOpenStash?: () => void;
    onStatus?: (msg: string, isError?: boolean) => void;
    issues?: { number: number; title: string }[];
  }

  let {
    repoPath = "",
    externalEditorLabel = "Editor",
    status,
    selectedFile,
    onSelectFile,
    onToggleStage,
    onStageAll,
    onUnstageAll,
    onCommit,
    onUndoCommit,
    onAbortMerge,
    onAbortRebase,
    onDiscardFile,
    onDiscardAll,
    onAddToGitignore,
    onRevealFile,
    onOpenInEditor,
    onOpenConflictEditor,
    onInspectFile,
    onAiGenerateCommit,
    onOpenStash,
    onStatus,
    issues = [],
  }: Props = $props();
  const EMOJI_LIST = $derived([
    { code: ":sparkles:", emoji: "✨", desc: t("changes.emoji.sparkles") },
    { code: ":bug:", emoji: "🐛", desc: t("changes.emoji.bug") },
    { code: ":memo:", emoji: "📝", desc: t("changes.emoji.memo") },
    { code: ":art:", emoji: "🎨", desc: t("changes.emoji.art") },
    { code: ":recycle:", emoji: "♻️", desc: t("changes.emoji.recycle") },
    { code: ":zap:", emoji: "⚡", desc: t("changes.emoji.zap") },
    { code: ":tada:", emoji: "🎉", desc: t("changes.emoji.tada") },
    { code: ":rocket:", emoji: "🚀", desc: t("changes.emoji.rocket") },
    { code: ":wrench:", emoji: "🔧", desc: t("changes.emoji.wrench") },
    { code: ":fire:", emoji: "🔥", desc: t("changes.emoji.fire") },
    { code: ":white_check_mark:", emoji: "✅", desc: t("changes.emoji.whiteCheckMark") },
    { code: ":lock:", emoji: "🔒", desc: t("changes.emoji.lock") },
    { code: ":lipstick:", emoji: "💄", desc: t("changes.emoji.lipstick") },
  ]);

  let autocompleteTarget = $state<"summary" | "description" | null>(null);
  let autocompleteQuery = $state("");
  let autocompleteType = $state<"emoji" | "issue" | null>(null);

  let filteredEmojis = $derived(
    autocompleteType === "emoji"
      ? EMOJI_LIST.filter(
          (e) =>
            e.code.toLowerCase().includes(autocompleteQuery.toLowerCase()) ||
            e.desc.toLowerCase().includes(autocompleteQuery.toLowerCase())
        )
      : []
  );

  let filteredIssues = $derived(
    autocompleteType === "issue" && issues
      ? issues.filter(
          (i) =>
            String(i.number).includes(autocompleteQuery) ||
            i.title.toLowerCase().includes(autocompleteQuery.toLowerCase())
        )
      : []
  );

  function checkAutocomplete(text: string, target: "summary" | "description") {
    const emojiMatch = text.match(/:([a-z0-9_]*)$/i);
    if (emojiMatch) {
      autocompleteTarget = target;
      autocompleteType = "emoji";
      autocompleteQuery = emojiMatch[1];
      return;
    }
    const issueMatch = text.match(/#([0-9]*)$/);
    if (issueMatch) {
      autocompleteTarget = target;
      autocompleteType = "issue";
      autocompleteQuery = issueMatch[1];
      return;
    }
    autocompleteTarget = null;
    autocompleteType = null;
  }

  function insertEmoji(emojiChar: string) {
    if (autocompleteTarget === "summary") {
      commitSummary = commitSummary.replace(/:([a-z0-9_]*)$/i, emojiChar + " ");
    } else if (autocompleteTarget === "description") {
      commitDescription = commitDescription.replace(/:([a-z0-9_]*)$/i, emojiChar + " ");
    }
    autocompleteType = null;
    autocompleteTarget = null;
  }

  function insertIssue(num: number) {
    const replacement = `#${num} `;
    if (autocompleteTarget === "summary") {
      commitSummary = commitSummary.replace(/#([0-9]*)$/, replacement);
    } else if (autocompleteTarget === "description") {
      commitDescription = commitDescription.replace(/#([0-9]*)$/, replacement);
    }
    autocompleteType = null;
    autocompleteTarget = null;
  }

  let commitSummary = $state("");
  let commitDescription = $state("");
  let isAmend = $state(false);
  let showDescription = $state(false);
  let showCoAuthors = $state(false);
  let coAuthorName = $state("");
  let coAuthorEmail = $state("");
  let isGeneratingAiCommit = $state(false);

  let stagedCount = $derived(status.files.filter((f) => f.staged).length);
  let totalCount = $derived(status.files.length);

  async function handleAiGenerateCommit() {
    if (!onAiGenerateCommit || isGeneratingAiCommit) return;
    isGeneratingAiCommit = true;
    try {
      const res = await onAiGenerateCommit();
      if (res?.summary) {
        commitSummary = res.summary;
      }
      if (res?.description) {
        commitDescription = res.description;
        showDescription = true;
      }
    } finally {
      isGeneratingAiCommit = false;
    }
  }
  function handleCommit() {
    const summary = commitSummary.trim();
    let desc = commitDescription.trim();
    if (showCoAuthors && coAuthorName.trim() && coAuthorEmail.trim()) {
      const coAuthorTrailer = `Co-authored-by: ${coAuthorName.trim()} <${coAuthorEmail.trim()}>`;
      desc = desc ? `${desc}\n\n${coAuthorTrailer}` : coAuthorTrailer;
    }
    if ((summary || isAmend) && (stagedCount > 0 || isAmend) && !status.has_conflicts) {
      onCommit(summary, desc || undefined, isAmend);
      commitSummary = "";
      commitDescription = "";
      coAuthorName = "";
      coAuthorEmail = "";
      isAmend = false;
      showDescription = false;
      showCoAuthors = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape" && autocompleteType) {
      autocompleteType = null;
      autocompleteTarget = null;
      return;
    }
    if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
      handleCommit();
    }
  }

  function getBadge(file: FileEntry) {
    if (file.is_conflict) {
      return { class: "status-D", text: "CONFLICT", isAdded: false };
    }
    const code = file.status_code.trim();
    const c = code.charAt(0) || "M";
    if (c === "A" || c === "?") return { class: "status-A", text: "+", isAdded: true };
    if (c === "D") return { class: "status-D", text: code, isAdded: false };
    return { class: "status-M", text: code, isAdded: false };
  }

  function openFileContextMenu(e: MouseEvent, file: FileEntry) {
    if (selectedFile?.path !== file.path) {
      onSelectFile(file);
    }
    const slashIdx = Math.max(file.path.lastIndexOf("/"), file.path.lastIndexOf("\\"));
    const dirPath = slashIdx >= 0 ? file.path.slice(0, slashIdx) : "";
    const isDeleted = file.status_code.trim().startsWith("D");
    const fullPath = repoPath ? joinRepoPath(repoPath, file.path) : file.path;

    const items: ContextMenuItem[] = [
      {
        label: file.staged ? t("ctx.changes.unstage") : t("ctx.changes.stage"),
        disabled: file.is_conflict,
        onSelect: () => onToggleStage(file),
      },
    ];

    if (onDiscardFile) {
      items.push({
        label: t("ctx.changes.discardChanges"),
        danger: true,
        onSelect: () => onDiscardFile(file),
      });
    }

    if (onAddToGitignore && file.path !== ".gitignore") {
      items.push({ type: "divider" });
      items.push({
        label: t("ctx.changes.ignoreFile"),
        onSelect: () => onAddToGitignore(file.path),
      });
      if (dirPath) {
        items.push({
          label: t("ctx.changes.ignoreFolder", { folder: `${dirPath}/` }),
          onSelect: () => onAddToGitignore(`${dirPath}/`),
        });
      }
    }

    items.push({ type: "divider" });
    items.push({
      label: t("ctx.common.copyFilePath"),
      onSelect: () => {
        void copyTextToClipboard(fullPath, onStatus, t("ctx.status.copiedPath"));
      },
    });
    items.push({
      label: t("ctx.common.copyRelativePath"),
      onSelect: () => {
        void copyTextToClipboard(file.path, onStatus, t("ctx.status.copiedPath"));
      },
    });

    if (file.is_conflict && onOpenConflictEditor) {
      items.push({ type: "divider" });
      items.push({
        label: t("ctx.changes.openConflictEditor"),
        onSelect: () => onOpenConflictEditor(file.path),
      });
    }

    if (onInspectFile && !file.status_code.trim().startsWith("?")) {
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

  function openListContextMenu(e: MouseEvent) {
    const items: ContextMenuItem[] = [
      {
        label: t("ctx.changes.stageAll"),
        disabled: totalCount === 0,
        onSelect: onStageAll,
      },
      {
        label: t("ctx.changes.unstageAll"),
        disabled: stagedCount === 0,
        onSelect: onUnstageAll,
      },
    ];
    if (onDiscardAll) {
      items.push({
        label: t("ctx.changes.discardAll"),
        danger: true,
        disabled: totalCount === 0,
        onSelect: onDiscardAll,
      });
    }
    if (onOpenStash && totalCount > 0) {
      items.push({ type: "divider" });
      items.push({
        label: t("ctx.changes.stashAll"),
        onSelect: onOpenStash,
      });
    }
    openContextMenu(e, items);
  }
</script>

{#if status.in_merge}
  <div class="alert-banner">
    <span>{t("changes.inMerge")}</span>
    <button type="button" onclick={onAbortMerge} class="btn-secondary btn-xs btn-danger">
      {t("changes.abortMerge")}
    </button>
  </div>
{:else if status.in_rebase}
  <div class="alert-banner">
    <span>{t("changes.inRebase")}</span>
    <button type="button" onclick={onAbortRebase} class="btn-secondary btn-xs btn-danger">
      {t("changes.abortRebase")}
    </button>
  </div>
{/if}

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="sidebar-header" oncontextmenu={openListContextMenu}>
  <h3 class="tabular-nums">
    {stagedCount > 0
      ? t("changes.headerTitleStaged", { total: totalCount, staged: stagedCount })
      : t("changes.headerTitle", { total: totalCount })}
  </h3>
  <div class="sidebar-actions">
    {#if totalCount > 0 && onDiscardAll}
      <button
        type="button"
        class="btn-secondary btn-xs btn-danger"
        onclick={onDiscardAll}
        title={t("changes.discardAllTitle")}
      >
        {t("changes.discardAllBtn")}
      </button>
    {/if}
    <button
      type="button"
      class="btn-secondary btn-xs"
      onclick={onStageAll}
      disabled={totalCount === 0}
      title={t("changes.stageAllTitle")}
    >
      {t("changes.stageAllBtn")}
    </button>
    <button
      type="button"
      class="btn-secondary btn-xs"
      onclick={onUnstageAll}
      disabled={stagedCount === 0}
      title={t("changes.unstageAllTitle")}
    >
      {t("changes.unstageAllBtn")}
    </button>
  </div>
</div>

<ul class="file-list" id="file-list-container" oncontextmenu={openListContextMenu}>
  {#if status.files.length === 0}
    <li class="empty-state" style="padding: 32px 16px;">
      <span>{status.has_commits ? t("changes.emptyClean") : t("changes.emptyNoCommits")}</span>
    </li>
  {:else}
    {#each status.files as file (file.path)}
      {@const badge = getBadge(file)}
      {@const isSelected = selectedFile?.path === file.path}
      {@const slashIdx = Math.max(file.path.lastIndexOf("/"), file.path.lastIndexOf("\\"))}
      <li
        class="file-item {isSelected ? 'selected' : ''}"
        oncontextmenu={(e) => openFileContextMenu(e, file)}
      >
        <input
          type="checkbox"
          class="file-stage-cb"
          checked={file.staged}
          disabled={file.is_conflict}
          onchange={() => onToggleStage(file)}
          aria-label={t("changes.stageFileAria", { path: file.path })}
        />
        <button
          type="button"
          class="file-select-btn"
          onclick={() => onSelectFile(file)}
          title={file.path}
        >
          {#if file.is_lfs}
            <span class="badge-lfs">LFS</span>
          {/if}
          <span class="file-name">
            {#if slashIdx >= 0}
              <span class="file-dir">{file.path.slice(0, slashIdx + 1)}</span>
              <span class="file-base">{file.path.slice(slashIdx + 1)}</span>
            {:else}
              <span class="file-base">{file.path}</span>
            {/if}
          </span>
          <span class="file-status-badge {badge.class}">
            {#if badge.isAdded}
              <svg width="10" height="10" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
                <path d="M7.75 2a.75.75 0 0 1 .75.75V7h4.25a.75.75 0 0 1 0 1.5H8.5v4.25a.75.75 0 0 1-1.5 0V8.5H2.75a.75.75 0 0 1 0-1.5H7V2.75A.75.75 0 0 1 7.75 2Z" />
              </svg>
            {:else}
              {badge.text}
            {/if}
          </span>
        </button>

        {#if file.is_conflict && (onOpenConflictEditor || onOpenInEditor)}
          <button
            type="button"
            class="btn-secondary btn-xs btn-danger"
            title={onOpenConflictEditor ? t("changes.resolveConflictTitle") : t("changes.editConflictTitle")}
            onclick={() => (onOpenConflictEditor ? onOpenConflictEditor(file.path) : onOpenInEditor?.(file.path))}
          >
            {onOpenConflictEditor ? t("changes.resolveConflictBtn") : t("changes.editConflictBtn")}
          </button>
        {:else if onDiscardFile}
          <button
            type="button"
            class="btn-discard"
            title={t("changes.discardFileTitle", { path: file.path })}
            aria-label={t("changes.discardFileAria", { path: file.path })}
            onclick={() => onDiscardFile(file)}
          >
            <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
              <path d="M3.72 3.72a.75.75 0 0 1 1.06 0L8 6.94l3.22-3.22a.749.749 0 0 1 1.275.326.749.749 0 0 1-.215.734L9.06 8l3.22 3.22a.749.749 0 0 1-.326 1.275.749.749 0 0 1-.734-.215L8 9.06l-3.22 3.22a.751.751 0 0 1-1.042-.018.751.751 0 0 1-.018-1.042L6.94 8 3.72 4.78a.75.75 0 0 1 0-1.06Z" />
            </svg>
          </button>
        {/if}
      </li>
    {/each}
  {/if}
</ul>

<div class="commit-box">
  <div class="prefix-row">
    {#each ["feat: ", "fix: ", "docs: ", "refactor: ", "chore: "] as prefix}
      <button
        type="button"
        class="prefix-chip"
        onclick={() => {
          if (!commitSummary.startsWith(prefix)) {
            commitSummary = prefix + commitSummary.replace(/^[a-z]+:\s*/, "");
          }
        }}
      >
        {prefix.trim()}
      </button>
    {/each}
    {#if onAiGenerateCommit}
      <button
        type="button"
        class="prefix-chip ai-chip"
        disabled={isGeneratingAiCommit || totalCount === 0}
        onclick={handleAiGenerateCommit}
        title={t("ai.generateCommitTitle")}
      >
        {isGeneratingAiCommit ? "…" : `✦ ${t("ai.generateCommitBtn")}`}
      </button>
    {/if}
  </div>

  {#if autocompleteType === "emoji" && filteredEmojis.length > 0}
    <div class="autocomplete-popover" role="listbox" aria-label={t("changes.chooseEmojiAria")}>
      <div class="autocomplete-header">{t("changes.chooseEmojiHeader")}</div>
      {#each filteredEmojis as item}
        <button
          type="button"
          class="autocomplete-item"
          onclick={() => insertEmoji(item.emoji)}
        >
          <span style="font-size: 14px;">{item.emoji}</span>
          <span style="font-family: var(--font-mono); color: var(--text-accent);">{item.code}</span>
          <span style="color: var(--text-secondary); margin-left: auto;">{item.desc}</span>
        </button>
      {/each}
    </div>
  {:else if autocompleteType === "issue" && filteredIssues.length > 0}
    <div class="autocomplete-popover" role="listbox" aria-label={t("changes.chooseIssueAria")}>
      <div class="autocomplete-header">{t("changes.chooseIssueHeader")}</div>
      {#each filteredIssues as issue}
        <button
          type="button"
          class="autocomplete-item"
          onclick={() => insertIssue(issue.number)}
        >
          <span style="font-family: var(--font-mono); font-weight: 600; color: var(--text-accent);">
            #{issue.number}
          </span>
          <span style="color: var(--text-primary); text-overflow: ellipsis; overflow: hidden; white-space: nowrap;">
            {issue.title}
          </span>
        </button>
      {/each}
    </div>
  {/if}

  <div class="commit-summary-wrap">
    <input
      type="text"
      class="commit-input"
      aria-label={t("changes.summaryAria")}
      placeholder={t("changes.summaryPlaceholder")}
      maxlength="150"
      bind:value={commitSummary}
      oninput={(e) => checkAutocomplete(e.currentTarget.value, "summary")}
      onkeydown={handleKeydown}
    />
    <span
      class="commit-char-count {commitSummary.length > 72 ? 'is-over-limit' : ''}"
      title={t("changes.summaryCountTitle")}
    >
      {commitSummary.length}/72
    </span>
  </div>

  {#if showDescription}
    <textarea
      class="commit-input"
      aria-label={t("changes.descAria")}
      placeholder={t("changes.descPlaceholder")}
      rows="3"
      bind:value={commitDescription}
      oninput={(e) => checkAutocomplete(e.currentTarget.value, "description")}
      onkeydown={handleKeydown}
      style="resize: vertical;"
    ></textarea>
  {/if}

  {#if showCoAuthors}
    <div class="coauthor-row">
      <input
        type="text"
        class="commit-input"
        aria-label={t("changes.coAuthorNamePlaceholder")}
        placeholder={t("changes.coAuthorNamePlaceholder")}
        style="font-size: 11px;"
        bind:value={coAuthorName}
      />
      <input
        type="email"
        class="commit-input"
        aria-label={t("changes.coAuthorEmailAria")}
        placeholder={t("changes.coAuthorEmailPlaceholder")}
        style="font-size: 11px;"
        bind:value={coAuthorEmail}
      />
    </div>
  {/if}

  <div class="commit-meta-row">
    <label class="checkbox-label" title={!status.has_commits ? t("changes.amendDisabledTitle") : undefined}>
      <input type="checkbox" bind:checked={isAmend} disabled={!status.has_commits} />
      <span>{t("changes.amendLabel")}</span>
    </label>
    <div style="display: flex; gap: 8px;">
      <button
        type="button"
        class="link-btn"
        onclick={() => (showCoAuthors = !showCoAuthors)}
      >
        {showCoAuthors ? t("changes.coAuthorHide") : t("changes.coAuthorShow")}
      </button>
      <button
        type="button"
        class="link-btn"
        onclick={() => (showDescription = !showDescription)}
      >
        {showDescription ? t("changes.descHide") : t("changes.descShow")}
      </button>
    </div>
  </div>

  <button
    type="button"
    class="btn-commit"
    disabled={(stagedCount === 0 && !isAmend) || status.has_conflicts || (!commitSummary.trim() && !isAmend)}
    onclick={handleCommit}
  >
    {#if status.has_conflicts}
      <span>{t("changes.resolveConflictsFirst")}</span>
    {:else if isAmend}
      <span>{t("changes.amendLastCommit")}</span>
    {:else}
      <span class="tabular-nums">
        {stagedCount === 1
          ? t("changes.commitBtnOne", { count: stagedCount })
          : t("changes.commitBtnMany", { count: stagedCount })}
      </span>
      <span class="kbd-hint">Ctrl+↵</span>
    {/if}
  </button>

  {#if onUndoCommit && status.has_commits}
    <button
      type="button"
      onclick={onUndoCommit}
      class="btn-secondary btn-xs"
      style="width: 100%;"
      title={t("changes.undoLastCommitTitle")}
    >
      {t("changes.undoLastCommit")}
    </button>
  {/if}
</div>
