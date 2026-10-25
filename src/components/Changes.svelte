<script lang="ts">
  import type { FileEntry, RepoStatus } from "../types/git";

  interface Props {
    status: RepoStatus;
    selectedFile: FileEntry | null;
    onSelectFile: (file: FileEntry) => void;
    onToggleStage: (file: FileEntry) => void;
    onStageAll: () => void;
    onUnstageAll: () => void;
    onCommit: (message: string) => void;
    onAbortMerge?: () => void;
    onAbortRebase?: () => void;
    onDiscardFile?: (file: FileEntry) => void;
  }

  let {
    status,
    selectedFile,
    onSelectFile,
    onToggleStage,
    onStageAll,
    onUnstageAll,
    onCommit,
    onAbortMerge,
    onAbortRebase,
    onDiscardFile,
  }: Props = $props();

  let commitMessage = $state("");
  let stagedCount = $derived(status.files.filter((f) => f.staged).length);
  let totalCount = $derived(status.files.length);

  function handleCommit() {
    const msg = commitMessage.trim();
    if (msg && stagedCount > 0 && !status.has_conflicts) {
      onCommit(msg);
      commitMessage = "";
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
        <span class="file-name" title={file.path}>{file.path}</span>
        <button
          class="btn-discard"
          title="Batalkan perubahan"
          onclick={(e) => { e.stopPropagation(); onDiscardFile?.(file); }}
          style="margin-left: auto; background: none; border: none; color: var(--text-muted); cursor: pointer; font-size: 12px; padding: 2px 6px;"
        >
          x
        </button>
      </li>
    {/each}
  {/if}
</ul>

<div class="commit-box">
  <input
    type="text"
    class="commit-input"
    placeholder="Ringkasan komit"
    maxlength="150"
    bind:value={commitMessage}
    onkeydown={handleKeydown}
  />
  <button
    class="btn-commit"
    disabled={stagedCount === 0 || status.has_conflicts}
    onclick={handleCommit}
  >
    {status.has_conflicts ? "Selesaikan konflik dahulu" : `Komit ke branch (${stagedCount} file)`}
  </button>
</div>
