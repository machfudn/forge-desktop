<script lang="ts">
  interface Props {
    filePath: string;
    diffText: string;
    ignoreWhitespace?: boolean;
    onToggleWhitespace?: () => void;
    onAddToGitignore?: (path: string) => void;
  }

  let {
    filePath,
    diffText,
    ignoreWhitespace = false,
    onToggleWhitespace,
    onAddToGitignore,
  }: Props = $props();

  function parseLines(diff: string) {
    if (!diff.trim()) return [];
    return diff.split("\n").map((line) => {
      let className = "diff-line";
      if (line.startsWith("+") && !line.startsWith("+++")) {
        className += " add";
      } else if (line.startsWith("-") && !line.startsWith("---")) {
        className += " del";
      } else if (line.startsWith("@")) {
        className += " meta";
      }
      return { line, className };
    });
  }

  let lines = $derived(parseLines(diffText));
</script>

<div class="diff-container">
  <div class="diff-header" style="display: flex; justify-content: space-between; align-items: center;">
    <span style="font-family: var(--font-mono); font-size: 12px;">{filePath}</span>
    <div style="display: flex; gap: 6px;">
      {#if onToggleWhitespace}
        <button
          onclick={onToggleWhitespace}
          class="btn-secondary"
          style="font-size: 10px; padding: 2px 8px; {ignoreWhitespace ? 'border-color: var(--accent-blue); color: var(--accent-blue);' : ''}"
          title="Abaikan perbedaan spasi dan indentasi (git diff -w)"
        >
          {ignoreWhitespace ? "Abaikan Spasi: ON" : "Abaikan Spasi: OFF"}
        </button>
      {/if}
      {#if onAddToGitignore}
        <button
          onclick={() => onAddToGitignore?.(filePath)}
          class="btn-secondary"
          style="font-size: 10px; padding: 2px 8px;"
          title="Tambahkan file ini ke .gitignore"
        >
          + ke .gitignore
        </button>
      {/if}
    </div>
  </div>
  {#if lines.length === 0}
    <div class="diff-content" style="padding: 24px; color: var(--text-muted); text-align: center;">
      Tidak ada perbedaan teks untuk ditampilkan.
    </div>
  {:else}
    <div class="diff-content">
      {#each lines as { line, className }}
        <div class={className}>{line}</div>
      {/each}
    </div>
  {/if}
</div>
