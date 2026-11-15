<script lang="ts">
  import type { FileEntry, RepoStatus, GitHooksInfo } from "../types/git";
  import { detectGitHooks } from "../services/git";
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
    panelMode?: "files" | "commit";
    selectedFile: FileEntry | null;
    onSelectFile: (file: FileEntry) => void;
    onToggleStage: (file: FileEntry) => void;
    onDiscardSelected: (files: FileEntry[]) => void;
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
    panelMode = "files",
    selectedFile,
    onSelectFile,
    onToggleStage,
    onStageAll,
    onUnstageAll,
    onDiscardSelected,
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
  let selectAllCheckbox = $state<HTMLInputElement>();
  let fileFilter = $state<"all" | "staged" | "unstaged">("all");
  let visibleFiles = $derived(status.files.filter((file) => fileFilter === "all" || file.staged === (fileFilter === "staged")));
  let selectedFiles = $derived(status.files.filter((file) => file.staged));
  let visibleSelectedCount = $derived(visibleFiles.filter((file) => file.staged).length);
  let allFilesIndeterminate = $derived(visibleSelectedCount > 0 && visibleSelectedCount < visibleFiles.length);
  $effect(() => { if (selectAllCheckbox) selectAllCheckbox.indeterminate = allFilesIndeterminate; });

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
  let showDescription = $state(true);
  let showCoAuthors = $state(false);
  let coAuthorName = $state("");
  let coAuthorEmail = $state("");
  let isGeneratingAiCommit = $state(false);

  let gitHooks = $state<GitHooksInfo | null>(null);
  let showConventional = $state(false);
  let convType = $state("");
  let convScope = $state("");
  let convBreaking = $state(false);
  let convSummary = $state("");

  const CONVENTIONAL_TYPES = [
    { type: "feat", label: "feat", descKey: "changes.typeFeat" },
    { type: "fix", label: "fix", descKey: "changes.typeFix" },
    { type: "docs", label: "docs", descKey: "changes.typeDocs" },
    { type: "style", label: "style", descKey: "changes.typeStyle" },
    { type: "refactor", label: "refactor", descKey: "changes.typeRefactor" },
    { type: "perf", label: "perf", descKey: "changes.typePerf" },
    { type: "test", label: "test", descKey: "changes.typeTest" },
    { type: "build", label: "build", descKey: "changes.typeBuild" },
    { type: "ci", label: "ci", descKey: "changes.typeCi" },
    { type: "chore", label: "chore", descKey: "changes.typeChore" },
  ];

  $effect(() => {
    if (repoPath) {
      detectGitHooks(repoPath)
        .then((info) => {
          gitHooks = info;
        })
        .catch(() => {
          gitHooks = null;
        });
    } else {
      gitHooks = null;
    }
  });

  function buildConventionalString(): string {
    if (!convType) return convSummary.trimStart();
    const scopePart = convScope.trim() ? `(${convScope.trim()})` : "";
    const breakingPart = convBreaking ? "!" : "";
    const summaryPart = convSummary.trimStart();
    return `${convType}${scopePart}${breakingPart}: ${summaryPart}`;
  }

  function updateConventionalSummary() {
    commitSummary = buildConventionalString();
  }
  function toggleConventional() {
    showConventional = !showConventional;
    if (showConventional) {
      const match = commitSummary.match(/^([a-z]+)(?:\(([^)]+)\))?(!)?:\s*(.*)$/i);
      if (match) {
        const t = match[1].toLowerCase();
        if (CONVENTIONAL_TYPES.some((ct) => ct.type === t)) {
          convType = t;
        }
        convScope = match[2] || "";
        convBreaking = Boolean(match[3]);
        convSummary = match[4] || "";
      } else {
        convSummary = commitSummary.trim();
        updateConventionalSummary();
      }
    }
  }

  let validationIssues = $derived.by<{ level: "warn" | "info"; msg: string }[]>(() => {
    const issuesList: { level: "warn" | "info"; msg: string }[] = [];
    const text = commitSummary.trim();
    if (!text) return issuesList;

    if (text.length > 72) {
      issuesList.push({ level: "warn", msg: t("changes.validationOver72") });
    }

    const descMatch = text.match(/^[a-z]+(?:\([^)]+\))?!?: (.*)$/);
    const descToCheck = descMatch ? descMatch[1] : text;
    if (descToCheck && /^[A-Z]/.test(descToCheck)) {
      issuesList.push({ level: "info", msg: t("changes.validationLowercase") });
    }

    if (text.endsWith(".")) {
      issuesList.push({ level: "info", msg: t("changes.validationNoPeriod") });
    }

    return issuesList;
  });
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
      convSummary = "";
      convScope = "";
      convBreaking = false;
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

{#if panelMode === "files"}
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
<div class="sidebar-header file-changes-header" oncontextmenu={openListContextMenu}>
  <div class="file-changes-title-row">
    <h3 class="tabular-nums">{stagedCount > 0 ? t("changes.headerTitleStaged", { total: totalCount, staged: stagedCount }) : t("changes.headerTitle", { total: totalCount })}</h3>
  </div>
  <div class="file-filter-group" role="group" aria-label={t("changes.filterAria")}>
    <button type="button" class="file-filter-btn {fileFilter === 'all' ? 'is-active' : ''}" aria-pressed={fileFilter === "all"} onclick={() => (fileFilter = "all")}>{t("changes.filterAll")}</button>
    <button type="button" class="file-filter-btn {fileFilter === 'staged' ? 'is-active' : ''}" aria-pressed={fileFilter === "staged"} onclick={() => (fileFilter = "staged")}>{t("changes.filterStaged")}</button>
    <button type="button" class="file-filter-btn {fileFilter === 'unstaged' ? 'is-active' : ''}" aria-pressed={fileFilter === "unstaged"} onclick={() => (fileFilter = "unstaged")}>{t("changes.filterUnstaged")}</button>
  </div>
  {#if selectedFiles.length > 0}
    <div class="file-selection-actions" aria-label={t("changes.selectionActionsAria")}>
      <span class="file-selection-count">{t("changes.selectedCount", { count: selectedFiles.length })}</span>
      {#if onDiscardSelected}<button type="button" class="btn-secondary btn-xs btn-danger" onclick={() => onDiscardSelected(selectedFiles)}>{t("changes.discardSelected")}</button>{/if}
    </div>
  {/if}
</div>

<ul class="file-list" id="file-list-container" oncontextmenu={openListContextMenu}>
  {#if visibleFiles.length === 0}
    <li class="empty-state" style="padding: 32px 16px;"><span>{status.files.length === 0 ? (status.has_commits ? t("changes.emptyClean") : t("changes.emptyNoCommits")) : t("changes.filterEmpty")}</span></li>
  {:else}
    {#each visibleFiles as file (file.path)}
      {@const badge = getBadge(file)}
      {@const isSelected = selectedFile?.path === file.path}
      {@const slashIdx = Math.max(file.path.lastIndexOf("/"), file.path.lastIndexOf("\\"))}
      <li
        class="file-item {isSelected ? 'selected' : ''}"
        oncontextmenu={(e) => openFileContextMenu(e, file)}
      >
        <input type="checkbox" class="file-stage-cb" checked={file.staged} disabled={file.is_conflict} onchange={() => onToggleStage(file)} aria-label={t("changes.stageFileAria", { path: file.path })} />
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
{/if}
{#if panelMode === "commit"}

<div class="commit-box">
  <div class="commit-box-heading">
    <div>
      <h3>{t("changes.commitMessageTitle")}</h3>
      <p>{t("changes.commitMessageHint")}</p>
    </div>
    <span class="commit-box-count">{stagedCount}/{totalCount}</span>
  </div>

  <div class="commit-box-toolbar" aria-label="Commit options">
    <button
      type="button"
      class="link-btn {showDescription ? 'is-active' : ''}"
      aria-pressed={showDescription}
      onclick={() => (showDescription = !showDescription)}
    >
      {t("changes.descToggle")}
    </button>
    {#if onAiGenerateCommit}
      <button type="button" class="link-btn" disabled={isGeneratingAiCommit || totalCount === 0} onclick={handleAiGenerateCommit} title={t("ai.generateCommitTitle")}>
        {isGeneratingAiCommit ? "…" : t("ai.generateCommitBtn")}
      </button>
    {/if}
    <button
      type="button"
      class="link-btn {showCoAuthors ? 'is-active' : ''}"
      aria-pressed={showCoAuthors}
      onclick={() => (showCoAuthors = !showCoAuthors)}
    >
      {t("changes.coAuthorToggle")}
    </button>
    <button
      type="button"
      class="link-btn {showConventional ? 'is-active' : ''}"
      aria-pressed={showConventional}
      onclick={toggleConventional}
    >
      {t("changes.formatToggle")}
    </button>
  </div>

  <div class="commit-summary-wrap">
    <label class="commit-field-label" for="commit-summary-input">{t("changes.summaryFieldLabel")}</label>
    <input
      id="commit-summary-input"
      type="text"
      class="commit-input commit-summary-input"
      aria-label={t("changes.summaryAria")}
      placeholder={t("changes.summaryPlaceholder")}
      maxlength="150"
      bind:value={commitSummary}
      oninput={(e) => {
        checkAutocomplete(e.currentTarget.value, "summary");
        if (showConventional) {
          const match = e.currentTarget.value.match(/^([a-z]+)(?:\(([^)]+)\))?(!)?:\s*(.*)$/i);
          convSummary = match ? match[4] || "" : e.currentTarget.value;
        }
      }}
      onkeydown={handleKeydown}
    />
    <span class="commit-char-count {commitSummary.length > 72 ? 'is-over-limit' : ''}" title={t("changes.summaryCountTitle")}>
      {commitSummary.length}/72
    </span>
  </div>

  {#if autocompleteType === "emoji" && filteredEmojis.length > 0}
    <div class="autocomplete-popover" role="listbox" aria-label={t("changes.chooseEmojiAria")}>
      <div class="autocomplete-header">{t("changes.chooseEmojiHeader")}</div>
      {#each filteredEmojis as item}
        <button type="button" class="autocomplete-item" onclick={() => insertEmoji(item.emoji)}>
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
        <button type="button" class="autocomplete-item" onclick={() => insertIssue(issue.number)}>
          <span style="font-family: var(--font-mono); font-weight: 600; color: var(--text-accent);">#{issue.number}</span>
          <span style="color: var(--text-primary); text-overflow: ellipsis; overflow: hidden; white-space: nowrap;">{issue.title}</span>
        </button>
      {/each}
    </div>
  {/if}




  {#if gitHooks && gitHooks.hook_names.length > 0}
    <div class="commit-hooks-indicator" title={gitHooks.hook_names.join(", ")}>
      <span class="hooks-pulse-dot"></span>
      <span class="hooks-text">{t("changes.hooksActive", { hooks: gitHooks.hook_names.join(", ") })}</span>
    </div>
  {/if}

  {#if showConventional}
    <div class="commit-type-row">
      <label class="commit-field-label" for="commit-type-select">{t("changes.conventionalTypeLabel")}</label>
      <div class="commit-type-controls">
        <select
          id="commit-type-select"
          class="form-select commit-type-select"
          bind:value={convType}
          onchange={updateConventionalSummary}
        >
          <option value="">{t("changes.conventionalTypeNone")}</option>
          {#each CONVENTIONAL_TYPES as ct}
            <option value={ct.type}>{ct.label}</option>
          {/each}
        </select>
        <input
          type="text"
          class="form-input commit-scope-input"
          aria-label={t("changes.conventionalScopeLabel")}
          placeholder={t("changes.conventionalScopePlaceholder")}
          bind:value={convScope}
          oninput={updateConventionalSummary}
          disabled={!convType}
        />
        <label class="commit-breaking-toggle" title={t("changes.conventionalBreaking")}>
          <input
            type="checkbox"
            bind:checked={convBreaking}
            onchange={updateConventionalSummary}
            disabled={!convType}
          />
          <span>!</span>
        </label>
      </div>
    </div>
  {/if}

  {#if validationIssues.length > 0}
    <div class="commit-validation-list">
      {#each validationIssues as issue}
        <span class="validation-badge badge-{issue.level}">
          {issue.msg}
        </span>
      {/each}
    </div>
  {/if}


  {#if showDescription}
    <div class="commit-description-field">
      <div class="commit-field-head">
        <label class="commit-field-label" for="commit-desc-input">{t("changes.descLabel")}</label>
        <span class="commit-field-hint">{t("changes.descOptionalHint")}</span>
      </div>
      <textarea
        id="commit-desc-input"
        class="commit-input commit-desc-input"
        aria-label={t("changes.descAria")}
        placeholder={t("changes.descPlaceholder")}
        rows="3"
        bind:value={commitDescription}
        oninput={(e) => checkAutocomplete(e.currentTarget.value, "description")}
        onkeydown={handleKeydown}
      ></textarea>
    </div>
  {/if}
  {#if showCoAuthors}
    <div class="commit-coauthor-field">
      <span class="commit-field-label">{t("changes.coAuthorFieldLabel")}</span>
      <div class="coauthor-row">
        <input
          type="text"
          class="commit-input"
          aria-label={t("changes.coAuthorNamePlaceholder")}
          placeholder={t("changes.coAuthorNamePlaceholder")}
          bind:value={coAuthorName}
        />
        <input
          type="email"
          class="commit-input"
          aria-label={t("changes.coAuthorEmailAria")}
          placeholder={t("changes.coAuthorEmailPlaceholder")}
          bind:value={coAuthorEmail}
        />
      </div>
    </div>
  {/if}

  <label class="commit-amend-option" title={!status.has_commits ? t("changes.amendDisabledTitle") : undefined}>
    <input type="checkbox" bind:checked={isAmend} disabled={!status.has_commits} />
    <span>{t("changes.amendLabel")}</span>
  </label>

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
{/if}
