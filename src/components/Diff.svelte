<script lang="ts">
  import type { ImageDiffData } from "../types/git";

  interface Props {
    filePath: string;
    diffText: string;
    imageDiffData?: ImageDiffData | null;
    ignoreWhitespace?: boolean;
    isLfs?: boolean;
    isStaged?: boolean;
    onToggleWhitespace?: () => void;
    onAddToGitignore?: (path: string) => void;
    onStageHunk?: (patchText: string, reverse: boolean) => void;
  }

  let {
    filePath,
    diffText,
    imageDiffData = null,
    ignoreWhitespace = false,
    isLfs = false,
    isStaged = false,
    onToggleWhitespace,
    onAddToGitignore,
    onStageHunk,
  }: Props = $props();

  let isImage = $derived(
    !!filePath.match(/\.(png|jpe?g|gif|webp|svg|ico|bmp)$/i) &&
      !!imageDiffData &&
      (!!imageDiffData.old_image_url || !!imageDiffData.new_image_url)
  );
  let isMarkdown = $derived(!!filePath.match(/\.(md|markdown)$/i));

  let viewMode = $state<"unified" | "split" | "markdown">("unified");
  let imageViewMode = $state<"2-up" | "swipe" | "onion">("2-up");
  let swipePercent = $state(50);
  let onionOpacity = $state(50);

  function escapeHtml(str: string): string {
    return str
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;")
      .replace(/'/g, "&#39;");
  }

  function highlightLine(line: string): string {
    if (
      line.startsWith("@@") ||
      line.startsWith("diff ") ||
      line.startsWith("index ") ||
      line.startsWith("---") ||
      line.startsWith("+++")
    ) {
      return escapeHtml(line);
    }
    const prefix =
      line.startsWith("+") || line.startsWith("-") || line.startsWith(" ")
        ? line.charAt(0)
        : "";
    const content = prefix ? line.slice(1) : line;

    const trimmed = content.trim();
    if (
      trimmed.startsWith("//") ||
      trimmed.startsWith("#") ||
      trimmed.startsWith("/*") ||
      trimmed.startsWith("*")
    ) {
      return `${prefix}<span class="tok-com">${escapeHtml(content)}</span>`;
    }

    const escaped = escapeHtml(content);
    const highlighted = escaped
      .replace(
        /(&quot;.*?&quot;|&#39;.*?&#39;|`.*?`)/g,
        '<span class="tok-str">$1</span>'
      )
      .replace(
        /\b(fn|let|const|var|function|return|if|else|for|while|match|pub|struct|enum|import|from|export|type|interface|class|def|async|await|package|impl|trait|use|mod|mut|self|true|false)\b/g,
        '<span class="tok-kw">$1</span>'
      )
      .replace(/\b(\d+(\.\d+)?)\b/g, '<span class="tok-num">$1</span>');

    return `${prefix}${highlighted}`;
  }

  interface ParsedLine {
    line: string;
    className: string;
    html: string;
    isHunkHeader: boolean;
    hunkPatch?: string;
  }

  interface HunkInfo {
    lines: string[];
  }

  function extractHunks(diff: string): HunkInfo[] {
    const raw = diff.split("\n");
    const hunks: HunkInfo[] = [];
    let currentHunk: HunkInfo | null = null;

    for (const l of raw) {
      if (l.startsWith("@@")) {
        if (currentHunk) {
          hunks.push(currentHunk);
        }
        currentHunk = { lines: [l] };
      } else if (currentHunk) {
        currentHunk.lines.push(l);
      }
    }
    if (currentHunk) {
      hunks.push(currentHunk);
    }
    return hunks;
  }

  function parseLines(diff: string, path: string): ParsedLine[] {
    if (!diff.trim()) return [];
    const rawLines = diff.split("\n");
    const hunks = extractHunks(diff);
    let hunkIdx = 0;

    return rawLines.map((line) => {
      let className = "diff-line";
      let isHunkHeader = false;
      let hunkPatch: string | undefined;

      if (line.startsWith("+") && !line.startsWith("+++")) {
        className += " add";
      } else if (line.startsWith("-") && !line.startsWith("---")) {
        className += " del";
      } else if (line.startsWith("@@")) {
        className += " meta";
        isHunkHeader = true;
        if (hunks[hunkIdx]) {
          hunkPatch = `--- a/${path}\n+++ b/${path}\n${hunks[hunkIdx].lines.join("\n")}\n`;
          hunkIdx++;
        }
      }
      return { line, className, html: highlightLine(line), isHunkHeader, hunkPatch };
    });
  }

  interface SplitRow {
    leftLine: string;
    leftType: "del" | "context" | "meta" | "empty";
    leftHtml: string;
    rightLine: string;
    rightType: "add" | "context" | "meta" | "empty";
    rightHtml: string;
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
        const left = delBuffer[i] !== undefined ? delBuffer[i] : "";
        const right = addBuffer[i] !== undefined ? addBuffer[i] : "";
        rows.push({
          leftLine: left,
          leftType: delBuffer[i] !== undefined ? "del" : "empty",
          leftHtml: left ? highlightLine(left) : "",
          rightLine: right,
          rightType: addBuffer[i] !== undefined ? "add" : "empty",
          rightHtml: right ? highlightLine(right) : "",
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
        const h = highlightLine(line);
        rows.push({
          leftLine: line,
          leftType: "meta",
          leftHtml: h,
          rightLine: line,
          rightType: "meta",
          rightHtml: h,
        });
      } else if (line.startsWith("-")) {
        delBuffer.push(line);
      } else if (line.startsWith("+")) {
        addBuffer.push(line);
      } else {
        flushBuffers();
        const h = highlightLine(line);
        rows.push({
          leftLine: line,
          leftType: "context",
          leftHtml: h,
          rightLine: line,
          rightType: "context",
          rightHtml: h,
        });
      }
    }
    flushBuffers();
    return rows;
  }

  function renderMarkdownDiff(text: string): { type: "add" | "del" | "ctx"; html: string }[] {
    const rawLines = text.split("\n");
    const result: { type: "add" | "del" | "ctx"; html: string }[] = [];

    for (const line of rawLines) {
      if (
        line.startsWith("diff ") ||
        line.startsWith("index ") ||
        line.startsWith("---") ||
        line.startsWith("+++") ||
        line.startsWith("@@")
      ) {
        continue;
      }
      let type: "add" | "del" | "ctx" = "ctx";
      let content = line;
      if (line.startsWith("+")) {
        type = "add";
        content = line.slice(1);
      } else if (line.startsWith("-")) {
        type = "del";
        content = line.slice(1);
      } else if (line.startsWith(" ")) {
        content = line.slice(1);
      }

      let escaped = escapeHtml(content);
      if (escaped.startsWith("### ")) {
        escaped = `<h3 style="font-size: 14px; font-weight: 700; margin: 4px 0;">${escaped.slice(4)}</h3>`;
      } else if (escaped.startsWith("## ")) {
        escaped = `<h2 style="font-size: 16px; font-weight: 700; margin: 6px 0; border-bottom: 1px solid var(--border-muted); padding-bottom: 2px;">${escaped.slice(3)}</h2>`;
      } else if (escaped.startsWith("# ")) {
        escaped = `<h1 style="font-size: 18px; font-weight: 700; margin: 8px 0; border-bottom: 1px solid var(--border-default); padding-bottom: 4px;">${escaped.slice(2)}</h1>`;
      } else if (escaped.startsWith("&gt; ")) {
        escaped = `<blockquote style="border-left: 3px solid var(--accent-blue); padding-left: 8px; margin: 4px 0; color: var(--text-secondary);">${escaped.slice(5)}</blockquote>`;
      } else if (escaped.startsWith("- [x] ") || escaped.startsWith("* [x] ")) {
        escaped = `<label style="display: flex; align-items: center; gap: 6px;"><input type="checkbox" checked disabled /> <span>${escaped.slice(6)}</span></label>`;
      } else if (escaped.startsWith("- [ ] ") || escaped.startsWith("* [ ] ")) {
        escaped = `<label style="display: flex; align-items: center; gap: 6px;"><input type="checkbox" disabled /> <span>${escaped.slice(6)}</span></label>`;
      } else if (escaped.startsWith("- ") || escaped.startsWith("* ")) {
        escaped = `<li style="margin-left: 16px;">${escaped.slice(2)}</li>`;
      }

      escaped = escaped.replace(/\*\*(.*?)\*\*/g, "<strong>$1</strong>");
      escaped = escaped.replace(/\*(.*?)\*/g, "<em>$1</em>");
      escaped = escaped.replace(/`([^`]+)`/g, "<code style='background: var(--bg-subtle); padding: 1px 4px; border-radius: 3px; font-family: var(--font-mono); font-size: 11px;'>$1</code>");

      result.push({ type, html: escaped });
    }
    return result;
  }

  let lines = $derived(parseLines(diffText, filePath));
  let splitRows = $derived(parseSplit(diffText));
</script>

<div class="diff-container">
  <div class="diff-header" style="display: flex; justify-content: space-between; align-items: center;">
    <div style="display: flex; align-items: center; gap: 8px;">
      <span style="font-family: var(--font-mono); font-size: 12px;">{filePath}</span>
      {#if isLfs}
        <span style="background: var(--text-accent); color: #fff; font-size: 10px; font-weight: 700; padding: 1px 5px; border-radius: 3px; font-family: var(--font-mono);">
          Git LFS
        </span>
      {/if}
    </div>
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
        {#if isMarkdown}
          <button
            onclick={() => (viewMode = "markdown")}
            style="border: none; border-left: 1px solid var(--border-default); background: {viewMode === 'markdown' ? 'var(--bg-subtle)' : 'transparent'}; color: {viewMode === 'markdown' ? 'var(--text-primary)' : 'var(--text-muted)'}; padding: 2px 8px; font-size: 10px; cursor: pointer; font-weight: {viewMode === 'markdown' ? '600' : 'normal'};"
          >
            Rendered MD
          </button>
        {/if}
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

  {#if isImage && imageDiffData}
    <div style="display: flex; flex-direction: column; align-items: center; padding: 16px; gap: 16px; height: 100%; overflow-y: auto;">
      <div style="display: inline-flex; border: 1px solid var(--border-default); border-radius: var(--radius-sm); overflow: hidden;">
        <button
          onclick={() => (imageViewMode = "2-up")}
          style="border: none; background: {imageViewMode === '2-up' ? 'var(--bg-subtle)' : 'transparent'}; color: {imageViewMode === '2-up' ? 'var(--text-primary)' : 'var(--text-muted)'}; padding: 3px 10px; font-size: 11px; cursor: pointer; font-weight: {imageViewMode === '2-up' ? '600' : 'normal'};"
        >
          2-Up
        </button>
        <button
          onclick={() => (imageViewMode = "swipe")}
          style="border: none; border-left: 1px solid var(--border-default); background: {imageViewMode === 'swipe' ? 'var(--bg-subtle)' : 'transparent'}; color: {imageViewMode === 'swipe' ? 'var(--text-primary)' : 'var(--text-muted)'}; padding: 3px 10px; font-size: 11px; cursor: pointer; font-weight: {imageViewMode === 'swipe' ? '600' : 'normal'};"
        >
          Swipe
        </button>
        <button
          onclick={() => (imageViewMode = "onion")}
          style="border: none; border-left: 1px solid var(--border-default); background: {imageViewMode === 'onion' ? 'var(--bg-subtle)' : 'transparent'}; color: {imageViewMode === 'onion' ? 'var(--text-primary)' : 'var(--text-muted)'}; padding: 3px 10px; font-size: 11px; cursor: pointer; font-weight: {imageViewMode === 'onion' ? '600' : 'normal'};"
        >
          Onion Skin
        </button>
      </div>

      {#if imageViewMode === "2-up"}
        <div style="display: flex; gap: 24px; justify-content: center; flex-wrap: wrap;">
          {#if imageDiffData.old_image_url}
            <div style="display: flex; flex-direction: column; align-items: center; gap: 6px;">
              <span style="font-size: 11px; font-weight: 600; color: var(--diff-del-text);">Sebelum (Old / Deleted)</span>
              <div style="border: 1px solid var(--border-default); background: var(--bg-canvas); padding: 8px; border-radius: var(--radius-md); max-width: 380px; max-height: 380px; display: grid; place-items: center;">
                <img src={imageDiffData.old_image_url} alt="Old version" style="max-width: 100%; max-height: 360px; object-fit: contain;" />
              </div>
            </div>
          {/if}
          {#if imageDiffData.new_image_url}
            <div style="display: flex; flex-direction: column; align-items: center; gap: 6px;">
              <span style="font-size: 11px; font-weight: 600; color: var(--diff-add-text);">Sesudah (New / Modified)</span>
              <div style="border: 1px solid var(--border-default); background: var(--bg-canvas); padding: 8px; border-radius: var(--radius-md); max-width: 380px; max-height: 380px; display: grid; place-items: center;">
                <img src={imageDiffData.new_image_url} alt="New version" style="max-width: 100%; max-height: 360px; object-fit: contain;" />
              </div>
            </div>
          {/if}
        </div>
      {:else if imageViewMode === "swipe"}
        <div style="display: flex; flex-direction: column; align-items: center; gap: 12px; width: 100%; max-width: 500px;">
          <input type="range" min="0" max="100" bind:value={swipePercent} style="width: 100%; cursor: pointer;" />
          <span style="font-size: 11px; color: var(--text-secondary);">Swipe: {swipePercent}% (Kiri: Sebelum | Kanan: Sesudah)</span>
          <div style="position: relative; border: 1px solid var(--border-default); border-radius: var(--radius-md); overflow: hidden; background: var(--bg-canvas); width: 100%; height: 380px; display: grid; place-items: center;">
            {#if imageDiffData.old_image_url}
              <img src={imageDiffData.old_image_url} alt="Old" style="max-width: 100%; max-height: 360px; object-fit: contain;" />
            {/if}
            {#if imageDiffData.new_image_url}
              <div style="position: absolute; inset: 0; display: grid; place-items: center; clip-path: inset(0 {100 - swipePercent}% 0 0); border-right: 2px solid var(--accent-blue);">
                <img src={imageDiffData.new_image_url} alt="New" style="max-width: 100%; max-height: 360px; object-fit: contain;" />
              </div>
            {/if}
          </div>
        </div>
      {:else if imageViewMode === "onion"}
        <div style="display: flex; flex-direction: column; align-items: center; gap: 12px; width: 100%; max-width: 500px;">
          <input type="range" min="0" max="100" bind:value={onionOpacity} style="width: 100%; cursor: pointer;" />
          <span style="font-size: 11px; color: var(--text-secondary);">Opacity Gambar Baru: {onionOpacity}%</span>
          <div style="position: relative; border: 1px solid var(--border-default); border-radius: var(--radius-md); overflow: hidden; background: var(--bg-canvas); width: 100%; height: 380px; display: grid; place-items: center;">
            {#if imageDiffData.old_image_url}
              <img src={imageDiffData.old_image_url} alt="Old" style="max-width: 100%; max-height: 360px; object-fit: contain;" />
            {/if}
            {#if imageDiffData.new_image_url}
              <div style="position: absolute; inset: 0; display: grid; place-items: center; opacity: {onionOpacity / 100};">
                <img src={imageDiffData.new_image_url} alt="New" style="max-width: 100%; max-height: 360px; object-fit: contain;" />
              </div>
            {/if}
          </div>
        </div>
      {/if}
    </div>
  {:else if viewMode === "markdown"}
    <div class="diff-content" style="padding: 16px; font-family: var(--font-sans); line-height: 1.6; overflow-y: auto;">
      {#each renderMarkdownDiff(diffText) as item}
        <div
          class="diff-line {item.type === 'add' ? 'add' : item.type === 'del' ? 'del' : ''}"
          style="padding: 3px 8px; font-family: inherit; white-space: normal;"
        >
          {@html item.html}
        </div>
      {/each}
    </div>
  {:else if lines.length === 0}
    <div class="diff-content" style="padding: 24px; color: var(--text-muted); text-align: center;">
      Tidak ada perbedaan teks untuk ditampilkan.
    </div>
  {:else if viewMode === "unified"}
    <div class="diff-content">
      {#each lines as { className, html, isHunkHeader, hunkPatch }}
        <div class={className} style={isHunkHeader ? "display: flex; justify-content: space-between; align-items: center;" : ""}>
          <span>{@html html}</span>
          {#if isHunkHeader && hunkPatch && onStageHunk}
            <button
              class="btn-secondary"
              style="font-size: 10px; padding: 1px 6px; margin-left: 8px; border-color: {isStaged ? 'var(--diff-del-text)' : 'var(--diff-add-text)'}; color: {isStaged ? 'var(--diff-del-text)' : 'var(--diff-add-text)'}; cursor: pointer;"
              onclick={(e) => { e.stopPropagation(); onStageHunk(hunkPatch, isStaged); }}
              title={isStaged ? "Batalkan stage hunk ini (Unstage hunk)" : "Stage hunk ini ke index"}
            >
              {isStaged ? "- Unstage Hunk" : "+ Stage Hunk"}
            </button>
          {/if}
        </div>
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
            {@html row.leftHtml}
          </div>
          <div
            class="diff-line {row.rightType === 'add' ? 'add' : row.rightType === 'meta' ? 'meta' : ''}"
            style="overflow-x: auto; white-space: pre; min-height: 19px;"
          >
            {@html row.rightHtml}
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>
