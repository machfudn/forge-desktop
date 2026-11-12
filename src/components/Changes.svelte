<script lang="ts">
  import type { FileEntry, RepoStatus } from "../types/git";

  interface Props {
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
    onOpenInEditor?: (filePath: string) => void;
    issues?: { number: number; title: string }[];
  }

  let {
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
    onOpenInEditor,
    issues = [],
  }: Props = $props();

  const EMOJI_LIST = [
    { code: ":sparkles:", emoji: "✨", desc: "Fitur baru" },
    { code: ":bug:", emoji: "🐛", desc: "Perbaikan bug" },
    { code: ":memo:", emoji: "📝", desc: "Dokumentasi" },
    { code: ":art:", emoji: "🎨", desc: "Format / struktur kode" },
    { code: ":recycle:", emoji: "♻️", desc: "Refaktor kode" },
    { code: ":zap:", emoji: "⚡", desc: "Performa" },
    { code: ":tada:", emoji: "🎉", desc: "Rilis / inisialisasi" },
    { code: ":rocket:", emoji: "🚀", desc: "Deploy" },
    { code: ":wrench:", emoji: "🔧", desc: "Tooling / konfigurasi" },
    { code: ":fire:", emoji: "🔥", desc: "Hapus file / kode" },
    { code: ":white_check_mark:", emoji: "✅", desc: "Pengujian / tes" },
    { code: ":lock:", emoji: "🔒", desc: "Keamanan" },
    { code: ":lipstick:", emoji: "💄", desc: "Styling / UI" },
  ];

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

  let stagedCount = $derived(status.files.filter((f) => f.staged).length);
  let totalCount = $derived(status.files.length);

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
      return { class: "status-D", text: "CONFLICT" };
    }
    const code = file.status_code.trim();
    const c = code.charAt(0) || "M";
    if (c === "A") return { class: "status-A", text: code };
    if (c === "D") return { class: "status-D", text: code };
    if (c === "?") return { class: "status-Q", text: code };
    return { class: "status-M", text: code };
  }
</script>

{#if status.in_merge}
  <div class="alert-banner">
    <span>Proses Merge Aktif</span>
    <button type="button" onclick={onAbortMerge} class="btn-secondary btn-xs btn-danger">
      Batalkan Merge
    </button>
  </div>
{:else if status.in_rebase}
  <div class="alert-banner">
    <span>Proses Rebase Aktif</span>
    <button type="button" onclick={onAbortRebase} class="btn-secondary btn-xs btn-danger">
      Batalkan Rebase
    </button>
  </div>
{/if}

<div class="sidebar-header">
  <h3 class="tabular-nums">
    Perubahan ({totalCount}{#if stagedCount > 0} · {stagedCount} staged{/if})
  </h3>
  <div class="sidebar-actions">
    {#if totalCount > 0 && onDiscardAll}
      <button
        type="button"
        class="btn-secondary btn-xs btn-danger"
        onclick={onDiscardAll}
        title="Batalkan semua perubahan file"
      >
        Discard
      </button>
    {/if}
    <button
      type="button"
      class="btn-secondary btn-xs"
      onclick={onStageAll}
      disabled={totalCount === 0}
      title="Stage semua perubahan"
    >
      Stage
    </button>
    <button
      type="button"
      class="btn-secondary btn-xs"
      onclick={onUnstageAll}
      disabled={stagedCount === 0}
      title="Unstage semua perubahan"
    >
      Unstage
    </button>
  </div>
</div>

<ul class="file-list" id="file-list-container">
  {#if status.files.length === 0}
    <li class="empty-state" style="padding: 32px 16px;">
      <span>{status.has_commits ? "Working tree bersih" : "Repositori baru (belum ada komit)"}</span>
    </li>
  {:else}
    {#each status.files as file (file.path)}
      {@const badge = getBadge(file)}
      {@const isSelected = selectedFile?.path === file.path}
      {@const slashIdx = Math.max(file.path.lastIndexOf("/"), file.path.lastIndexOf("\\"))}
      <li class="file-item {isSelected ? 'selected' : ''}">
        <input
          type="checkbox"
          class="file-stage-cb"
          checked={file.staged}
          disabled={file.is_conflict}
          onchange={() => onToggleStage(file)}
          aria-label="Stage {file.path}"
        />
        <button
          type="button"
          class="file-select-btn"
          onclick={() => onSelectFile(file)}
          title={file.path}
        >
          <span class="file-status-badge {badge.class}">{badge.text}</span>
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
        </button>

        {#if file.is_conflict && onOpenInEditor}
          <button
            type="button"
            class="btn-secondary btn-xs btn-danger"
            title="Buka file konflik di editor eksternal"
            onclick={() => onOpenInEditor(file.path)}
          >
            Edit
          </button>
        {:else if onDiscardFile}
          <button
            type="button"
            class="btn-discard"
            title="Batalkan perubahan pada {file.path}"
            aria-label="Batalkan perubahan {file.path}"
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
  </div>

  {#if autocompleteType === "emoji" && filteredEmojis.length > 0}
    <div class="autocomplete-popover" role="listbox" aria-label="Pilih Emoji">
      <div class="autocomplete-header">PILIH EMOJI (:)</div>
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
    <div class="autocomplete-popover" role="listbox" aria-label="Pilih Issue">
      <div class="autocomplete-header">PILIH ISSUE / PR (#)</div>
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
      aria-label="Ringkasan komit"
      placeholder="Ringkasan (: emoji, # issue)"
      maxlength="150"
      bind:value={commitSummary}
      oninput={(e) => checkAutocomplete(e.currentTarget.value, "summary")}
      onkeydown={handleKeydown}
    />
    <span
      class="commit-char-count {commitSummary.length > 72 ? 'is-over-limit' : ''}"
      title="Panjang ringkasan komit (disarankan maksimal 72 karakter)"
    >
      {commitSummary.length}/72
    </span>
  </div>

  {#if showDescription}
    <textarea
      class="commit-input"
      aria-label="Deskripsi detail komit"
      placeholder="Deskripsi detail komit (opsional)"
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
        aria-label="Nama Co-Author"
        placeholder="Nama Co-Author"
        style="font-size: 11px;"
        bind:value={coAuthorName}
      />
      <input
        type="email"
        class="commit-input"
        aria-label="Email Co-Author"
        placeholder="email@domain.com"
        style="font-size: 11px;"
        bind:value={coAuthorEmail}
      />
    </div>
  {/if}

  <div class="commit-meta-row">
    <label class="checkbox-label" title={!status.has_commits ? "Belum ada komit untuk diamandemen" : undefined}>
      <input type="checkbox" bind:checked={isAmend} disabled={!status.has_commits} />
      <span>Amend</span>
    </label>
    <div style="display: flex; gap: 8px;">
      <button
        type="button"
        class="link-btn"
        onclick={() => (showCoAuthors = !showCoAuthors)}
      >
        {showCoAuthors ? "- Co-Author" : "+ Co-Author"}
      </button>
      <button
        type="button"
        class="link-btn"
        onclick={() => (showDescription = !showDescription)}
      >
        {showDescription ? "- Deskripsi" : "+ Deskripsi"}
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
      <span>Selesaikan konflik dahulu</span>
    {:else if isAmend}
      <span>Amandemen komit terakhir</span>
    {:else}
      <span class="tabular-nums">Komit ({stagedCount} file)</span>
      <span class="kbd-hint">Ctrl+↵</span>
    {/if}
  </button>

  {#if onUndoCommit && status.has_commits}
    <button
      type="button"
      onclick={onUndoCommit}
      class="btn-secondary btn-xs"
      style="width: 100%;"
      title="Batalkan komit terakhir dan kembalikan perubahannya ke staging"
    >
      Undo Komit Terakhir
    </button>
  {/if}
</div>
