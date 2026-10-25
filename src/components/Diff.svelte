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

  let viewMode = $state<"unified" | "split">("unified");

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

  interface SplitRow {
    leftLine: string;
    leftType: "del" | "context" | "meta" | "empty";
    rightLine: string;
    rightType: "add" | "context" | "meta" | "empty";
  }

  function parseSplit(diff: string): SplitRow[] {
    if (!diff.trim()) return [];
    const rawLines = diff.split("\n");
    const rows: SplitRow[] = [];

    let delBuffer: string[] = [];
    let addBuffer: string[] = [];

    function flushBuffers() {
      const maxLen = Math.max(delBuffer.length, addBuffer.length);
      for (let i = 0; i < maxLen; i++) {
        rows.push({
          leftLine: delBuffer[i] !== undefined ? delBuffer[i] : "",
          leftType: delBuffer[i] !== undefined ? "del" : "empty",
          rightLine: addBuffer[i] !== undefined ? addBuffer[i] : "",
          rightType: addBuffer[i] !== undefined ? "add" : "empty",
        });
      }
      delBuffer = [];
      addBuffer = [];
    }

    for (const line of rawLines) {
      if (
        line.startsWith("diff ") ||
        line.startsWith("index ") ||
        line.startsWith("---") ||
        line.startsWith("+++")
      ) {
        continue;
      }
      if (line.startsWith("@@")) {
        flushBuffers();
        rows.push({
          leftLine: line,
          leftType: "meta",
          rightLine: line,
          rightType: "meta",
        });
      } else if (line.startsWith("-")) {
        delBuffer.push(line);
      } else if (line.startsWith("+")) {
        addBuffer.push(line);
      } else {
        flushBuffers();
        rows.push({
          leftLine: line,
          leftType: "context",
          rightLine: line,
          rightType: "context",
        });
      }
    }
    flushBuffers();
    return rows;
  }

  let lines = $derived(parseLines(diffText));
  let splitRows = $derived(parseSplit(diffText));
</script>

<div class="diff-container">
  <div class="diff-header" style="display: flex; justify-content: space-between; align-items: center;">
    <span style="font-family: var(--font-mono); font-size: 12px;">{filePath}</span>
    <div style="display: flex; gap: 6px; align-items: center;">
      <div style="display: inline-flex; border: 1px solid var(--border-default); border-radius: var(--radius-sm); overflow: hidden;">
        <button
          onclick={() => (viewMode = "unified")}
          style="border: none; background: {viewMode === 'unified' ? 'var(--bg-subtle)' : 'transparent'}; color: {viewMode === 'unified' ? 'var(--text-primary)' : 'var(--text-muted)'}; padding: 2px 8px; font-size: 10px; cursor: pointer; font-weight: {viewMode === 'unified' ? '600' : 'normal'};"
        >
          Unified
        </button>
        <button
          onclick={() => (viewMode = "split")}
          style="border: none; border-left: 1px solid var(--border-default); background: {viewMode === 'split' ? 'var(--bg-subtle)' : 'transparent'}; color: {viewMode === 'split' ? 'var(--text-primary)' : 'var(--text-muted)'}; padding: 2px 8px; font-size: 10px; cursor: pointer; font-weight: {viewMode === 'split' ? '600' : 'normal'};"
        >
          Split
        </button>
      </div>

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
  {:else if viewMode === "unified"}
    <div class="diff-content">
      {#each lines as { line, className }}
        <div class={className}>{line}</div>
      {/each}
    </div>
  {:else}
    <div class="diff-content" style="display: flex; flex-direction: column;">
      {#each splitRows as row}
        <div style="display: grid; grid-template-columns: 1fr 1fr; border-bottom: 1px solid var(--border-muted);">
          <div
            class="diff-line {row.leftType === 'del' ? 'del' : row.leftType === 'meta' ? 'meta' : ''}"
            style="border-right: 1px solid var(--border-default); overflow-x: auto; white-space: pre; min-height: 19px;"
          >
            {row.leftLine}
          </div>
          <div
            class="diff-line {row.rightType === 'add' ? 'add' : row.rightType === 'meta' ? 'meta' : ''}"
            style="overflow-x: auto; white-space: pre; min-height: 19px;"
          >
            {row.rightLine}
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>
