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

  function insertIssue(num: number, _title?: string) {
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
    if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
      handleCommit();
    }
  }

  function getBadge(file: FileEntry) {
    if (file.is_conflict) {
      return { class: "status-D", text: "CONFLICT" };
    }
    const c = file.status_code.trim().charAt(0) || "M";
    if (c === "A") return { class: "status-A", text: file.status_code.trim() };
    if (c === "D") return { class: "status-D", text: file.status_code.trim() };
    if (c === "?") return { class: "status-Q", text: file.status_code.trim() };
    return { class: "status-M", text: file.status_code.trim() };
  }
</script>

{#if status.in_merge}
  <div style="background: rgba(248, 81, 73, 0.15); border-bottom: 1px solid var(--diff-del-text); padding: 8px 12px; display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
    <span style="color: var(--diff-del-text); font-weight: 600;">Proses Merge Aktif</span>
    <button onclick={onAbortMerge} class="btn-secondary" style="font-size: 10px; padding: 2px 6px; border-color: var(--diff-del-text); color: var(--diff-del-text);">Batalkan Merge</button>
  </div>
{:else if status.in_rebase}
  <div style="background: rgba(248, 81, 73, 0.15); border-bottom: 1px solid var(--diff-del-text); padding: 8px 12px; display: flex; justify-content: space-between; align-items: center; font-size: 11px;">
    <span style="color: var(--diff-del-text); font-weight: 600;">Proses Rebase Aktif</span>
    <button onclick={onAbortRebase} class="btn-secondary" style="font-size: 10px; padding: 2px 6px; border-color: var(--diff-del-text); color: var(--diff-del-text);">Batalkan Rebase</button>
  </div>
{/if}

<div class="sidebar-header">
  <h3>Perubahan ({totalCount})</h3>
  <div style="display: flex; gap: 4px;">
    {#if totalCount > 0 && onDiscardAll}
      <button class="btn-secondary" onclick={onDiscardAll} style="font-size: 11px; padding: 2px 6px; color: var(--diff-del-text); border-color: rgba(248,81,73,0.3);" title="Batalkan semua perubahan file">Discard Semua</button>
    {/if}
    <button class="btn-secondary" onclick={onStageAll} style="font-size: 11px; padding: 2px 6px;">Stage Semua</button>
    <button class="btn-secondary" onclick={onUnstageAll} style="font-size: 11px; padding: 2px 6px;">Unstage</button>
  </div>
</div>

<ul class="file-list" id="file-list-container">
  {#if status.files.length === 0}
    <li style="padding: 24px 12px; text-align: center; color: var(--text-muted);">Working tree bersih</li>
  {:else}
    {#each status.files as file (file.path)}
      {@const badge = getBadge(file)}
      {@const isSelected = selectedFile?.path === file.path}
      <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
      <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <li
        class="file-item {isSelected ? 'selected' : ''}"
        onclick={(e) => {
          const target = e.target as HTMLElement;
          if (target.tagName.toLowerCase() !== 'input' && !target.classList.contains('btn-discard')) {
            onSelectFile(file);
          }
        }}
        tabindex="0"
        onkeydown={(e) => { if (e.key === 'Enter') onSelectFile(file); }}
      >
        <input
          type="checkbox"
          class="file-stage-cb"
          checked={file.staged}
          disabled={file.is_conflict}
          onchange={() => onToggleStage(file)}
          aria-label="Stage file"
        />
        <span class="file-status-badge {badge.class}">{badge.text}</span>
        {#if file.is_lfs}
          <span style="background: var(--text-accent); color: #fff; font-size: 9px; font-weight: 700; padding: 1px 4px; border-radius: 3px; font-family: var(--font-mono); margin-right: 4px;">
            LFS
          </span>
        {/if}
        <span class="file-name" title={file.path}>{file.path}</span>
        {#if file.is_conflict && onOpenInEditor}
          <button
            class="btn-secondary"
            title="Buka file konflik di editor eksternal"
            onclick={(e) => { e.stopPropagation(); onOpenInEditor(file.path); }}
            style="font-size: 10px; padding: 1px 6px; margin-left: auto; border-color: var(--diff-del-text); color: var(--diff-del-text);"
          >
            Edit Konflik
          </button>
        {:else}
          <button
            class="btn-discard"
            title="Batalkan perubahan"
            onclick={(e) => { e.stopPropagation(); onDiscardFile?.(file); }}
            style="margin-left: auto; background: none; border: none; color: var(--text-muted); cursor: pointer; font-size: 12px; padding: 2px 6px;"
          >
            x
          </button>
        {/if}
      </li>
    {/each}
  {/if}
</ul>

<div class="commit-box" style="display: flex; flex-direction: column; gap: 6px;">
  <div style="display: flex; gap: 4px; overflow-x: auto; padding-bottom: 2px;">
    {#each ["feat: ", "fix: ", "docs: ", "refactor: ", "chore: "] as prefix}
      <button
        type="button"
        class="btn-secondary"
        style="font-size: 10px; padding: 1px 6px;"
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
    <div style="background: var(--bg-panel); border: 1px solid var(--border-default); border-radius: var(--radius-sm); max-height: 140px; overflow-y: auto; padding: 4px; box-shadow: 0 4px 12px rgba(0,0,0,0.3); z-index: 10;">
      <div style="font-size: 10px; color: var(--text-muted); padding: 2px 6px; font-weight: 600;">PILIH EMOJI (:)</div>
      {#each filteredEmojis as item}
        <div
          role="button"
          tabindex="0"
          style="display: flex; align-items: center; gap: 8px; padding: 4px 6px; cursor: pointer; border-radius: var(--radius-sm); font-size: 11px;"
          onmouseenter={(e) => (e.currentTarget.style.background = "var(--bg-subtle)")}
          onmouseleave={(e) => (e.currentTarget.style.background = "transparent")}
          onclick={() => insertEmoji(item.emoji)}
          onkeydown={(e) => { if (e.key === "Enter") insertEmoji(item.emoji); }}
        >
          <span style="font-size: 14px;">{item.emoji}</span>
          <span style="font-family: var(--font-mono); color: var(--text-accent);">{item.code}</span>
          <span style="color: var(--text-secondary); margin-left: auto;">{item.desc}</span>
        </div>
      {/each}
    </div>
  {:else if autocompleteType === "issue" && filteredIssues.length > 0}
    <div style="background: var(--bg-panel); border: 1px solid var(--border-default); border-radius: var(--radius-sm); max-height: 140px; overflow-y: auto; padding: 4px; box-shadow: 0 4px 12px rgba(0,0,0,0.3); z-index: 10;">
      <div style="font-size: 10px; color: var(--text-muted); padding: 2px 6px; font-weight: 600;">PILIH ISSUE / PR (#)</div>
      {#each filteredIssues as issue}
        <div
          role="button"
          tabindex="0"
          style="display: flex; align-items: center; gap: 6px; padding: 4px 6px; cursor: pointer; border-radius: var(--radius-sm); font-size: 11px;"
          onmouseenter={(e) => (e.currentTarget.style.background = "var(--bg-subtle)")}
          onmouseleave={(e) => (e.currentTarget.style.background = "transparent")}
          onclick={() => insertIssue(issue.number, issue.title)}
          onkeydown={(e) => { if (e.key === "Enter") insertIssue(issue.number, issue.title); }}
        >
          <span style="font-family: var(--font-mono); font-weight: 600; color: var(--text-accent);">#{issue.number}</span>
          <span style="color: var(--text-primary); text-overflow: ellipsis; overflow: hidden; white-space: nowrap;">{issue.title}</span>
        </div>
      {/each}
    </div>
  {/if}

  <input
    type="text"
    class="commit-input"
    placeholder="Ringkasan komit (Ketik : untuk emoji, # untuk issue)"
    maxlength="150"
    bind:value={commitSummary}
    oninput={(e) => checkAutocomplete(e.currentTarget.value, "summary")}
    onkeydown={handleKeydown}
  />

  {#if showDescription}
    <textarea
      class="commit-input"
      placeholder="Deskripsi detail komit (opsional, ketik : untuk emoji, # untuk issue)"
      rows="3"
      bind:value={commitDescription}
      oninput={(e) => checkAutocomplete(e.currentTarget.value, "description")}
      style="resize: vertical; font-size: 12px; font-family: inherit;"
    ></textarea>
  {/if}

  {#if showCoAuthors}
    <div style="display: flex; gap: 4px;">
      <input
        type="text"
        class="commit-input"
        placeholder="Nama Co-Author"
        style="flex: 1; font-size: 11px;"
        bind:value={coAuthorName}
      />
      <input
        type="email"
        class="commit-input"
        placeholder="co-author@example.com"
        style="flex: 1; font-size: 11px;"
        bind:value={coAuthorEmail}
      />
    </div>
  {/if}

  <div style="display: flex; justify-content: space-between; align-items: center; font-size: 11px; color: var(--text-secondary);">
    <label style="display: flex; align-items: center; gap: 4px; cursor: pointer;">
      <input type="checkbox" bind:checked={isAmend} />
      <span>Amend komit</span>
    </label>
    <div style="display: flex; gap: 8px;">
      <button
        type="button"
        onclick={() => (showCoAuthors = !showCoAuthors)}
        style="background: none; border: none; color: var(--text-accent); cursor: pointer; font-size: 11px; padding: 0;"
      >
        {showCoAuthors ? "Sembunyikan Co-Author" : "+ Co-Author"}
      </button>
      <button
        type="button"
        onclick={() => (showDescription = !showDescription)}
        style="background: none; border: none; color: var(--text-accent); cursor: pointer; font-size: 11px; padding: 0;"
      >
        {showDescription ? "Sembunyikan Deskripsi" : "+ Deskripsi"}
      </button>
    </div>
  </div>

  <button
    class="btn-commit"
    disabled={(stagedCount === 0 && !isAmend) || status.has_conflicts || (!commitSummary.trim() && !isAmend)}
    onclick={handleCommit}
  >
    {#if status.has_conflicts}
      Selesaikan konflik dahulu
    {:else if isAmend}
      Amandemen komit terakhir
    {:else}
      Komit ke branch ({stagedCount} file)
    {/if}
  </button>

  {#if onUndoCommit}
    <button
      type="button"
      onclick={onUndoCommit}
      class="btn-secondary"
      style="font-size: 11px; width: 100%; margin-top: 2px;"
      title="Batalkan komit terakhir dan kembalikan perubahannya ke staging"
    >
      Undo Komit Terakhir
    </button>
  {/if}
</div>
