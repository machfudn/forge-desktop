<script lang="ts">
  import type { CommitInfo } from "../types/git";

  interface Props {
    commits: CommitInfo[];
    checkRunsMap?: Record<string, { status: string; conclusion?: string }>;
    onCherryPick?: (sha: string) => void;
    onRevert?: (sha: string) => void;
    onTag?: (sha: string) => void;
    onReset?: (sha: string) => void;
    onSquash?: (commitIndex: number, sha: string) => void;
    onInteractiveRebase?: () => void;
  }

  let {
    commits,
    checkRunsMap = {},
    onCherryPick,
    onRevert,
    onTag,
    onReset,
    onSquash,
    onInteractiveRebase,
  }: Props = $props();

  let selectedSha = $state<string | null>(null);

  function handleSelectCommit(sha: string) {
    selectedSha = selectedSha === sha ? null : sha;
  }
</script>

{#if commits.length === 0}
  <div style="padding: 32px; text-align: center; color: var(--text-muted);">
    Belum ada riwayat commit di branch ini.
  </div>
{:else}
  <div class="history-container">
    <div style="padding: 8px 16px; border-bottom: 1px solid var(--border-default); background-color: var(--bg-panel); font-size: 11px; font-weight: 600; color: var(--text-muted); display: flex; justify-content: space-between; align-items: center;">
      <div style="display: grid; grid-template-columns: 80px 1fr 140px 100px; gap: 12px; flex: 1;">
        <span>SHA</span>
        <span>KOMIT</span>
        <span>AUTHOR</span>
        <span style="text-align: right;">TANGGAL</span>
      </div>
      {#if onInteractiveRebase && commits.length > 1}
        <button
          onclick={onInteractiveRebase}
          class="btn-secondary"
          style="font-size: 10px; padding: 2px 8px; margin-left: 16px;"
          title="Buka dialog Interactive Rebase untuk mengurutkan atau membuang komit"
        >
          Interactive Rebase
        </button>
      {/if}
    </div>
    {#each commits as commit, index (commit.sha)}
      {@const isSelected = selectedSha === commit.sha}
      <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="commit-row {isSelected ? 'selected' : ''}"
        tabindex="0"
        onclick={() => handleSelectCommit(commit.sha)}
        style="cursor: pointer; {isSelected ? 'background: var(--bg-subtle); border-left: 3px solid var(--accent-blue);' : ''}"
      >
        <span class="commit-sha" style="display: flex; align-items: center; gap: 6px;">
          {#if checkRunsMap[commit.sha]}
            {@const ci = checkRunsMap[commit.sha]}
            <span
              style="width: 7px; height: 7px; border-radius: 50%; display: inline-block; background-color: {ci.conclusion === 'success' ? 'var(--diff-add-text)' : ci.conclusion === 'failure' ? 'var(--diff-del-text)' : '#e3b341'};"
              title="CI: {ci.conclusion || ci.status}"
            ></span>
          {/if}
          {commit.short_sha}
        </span>
        <span class="commit-summary" title={commit.summary}>{commit.summary}</span>
        <span class="commit-author">{commit.author}</span>
        <span class="commit-date">{commit.date}</span>
      </div>

      {#if isSelected}
        <div style="background: var(--bg-canvas); padding: 8px 16px; border-bottom: 1px solid var(--border-default); display: flex; gap: 6px; align-items: center; font-size: 11px;">
          <span style="color: var(--text-muted); margin-right: 4px;">Aksi Komit {commit.short_sha}:</span>
          {#if onSquash && index > 0}
            <button
              onclick={() => onSquash(index, commit.sha)}
              class="btn-secondary"
              style="font-size: 10px; padding: 2px 8px;"
              title="Squash {index + 1} komit dari HEAD ke komit ini"
            >
              Squash {index + 1} komit
            </button>
          {/if}
          {#if onCherryPick}
            <button
              onclick={() => onCherryPick(commit.sha)}
              class="btn-secondary"
              style="font-size: 10px; padding: 2px 8px;"
              title="Salin komit ini ke branch saat ini"
            >
              Cherry-pick
            </button>
          {/if}
          {#if onRevert}
            <button
              onclick={() => onRevert(commit.sha)}
              class="btn-secondary"
              style="font-size: 10px; padding: 2px 8px;"
              title="Balikkan perubahan komit ini"
            >
              Revert
            </button>
          {/if}
          {#if onTag}
            <button
              onclick={() => onTag(commit.sha)}
              class="btn-secondary"
              style="font-size: 10px; padding: 2px 8px;"
              title="Buat tag di komit ini"
            >
              Buat Tag
            </button>
          {/if}
          {#if onReset}
            <button
              onclick={() => onReset(commit.sha)}
              class="btn-secondary"
              style="font-size: 10px; padding: 2px 8px; color: var(--diff-del-text);"
              title="Reset branch ke titik komit ini"
            >
              Reset ke sini
            </button>
          {/if}
        </div>
      {/if}
    {/each}
  </div>
{/if}
