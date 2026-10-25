<script lang="ts">
  interface Props {
    filePath: string;
    diffText: string;
  }

  let { filePath, diffText }: Props = $props();

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
  <div class="diff-header">{filePath}</div>
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
