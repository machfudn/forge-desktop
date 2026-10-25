<script lang="ts">
  import type { CommitInfo } from "../types/git";

  interface Props {
    commits: CommitInfo[];
  }

  let { commits }: Props = $props();
</script>

{#if commits.length === 0}
  <div style="padding: 32px; text-align: center; color: var(--text-muted);">
    Belum ada riwayat commit di branch ini.
  </div>
{:else}
  <div class="history-container">
    <div style="padding: 8px 16px; border-bottom: 1px solid var(--border-default); background-color: var(--bg-panel); font-size: 11px; font-weight: 600; color: var(--text-muted); display: grid; grid-template-columns: 80px 1fr 140px 100px; gap: 12px;">
      <span>SHA</span>
      <span>KOMIT</span>
      <span>AUTHOR</span>
      <span style="text-align: right;">TANGGAL</span>
    </div>
    {#each commits as commit (commit.sha)}
      <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <div class="commit-row" tabindex="0">
        <span class="commit-sha">{commit.short_sha}</span>
        <span class="commit-summary" title={commit.summary}>{commit.summary}</span>
        <span class="commit-author">{commit.author}</span>
        <span class="commit-date">{commit.date}</span>
      </div>
    {/each}
  </div>
{/if}
