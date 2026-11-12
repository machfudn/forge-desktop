<script lang="ts">
  import type { ImageDiffData } from "../types/git";
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
    filePath: string;
    diffText: string;
    imageDiffData?: ImageDiffData | null;
    ignoreWhitespace?: boolean;
    isLfs?: boolean;
    isStaged?: boolean;
    onToggleWhitespace?: () => void;
    onAddToGitignore?: (path: string) => void;
    onStageHunk?: (patchText: string, reverse: boolean) => void;
    onStageLines?: (hunkPatch: string, selectedIndices: number[], reverse: boolean) => void;
    onInspectFile?: (filePath: string, mode: "blame" | "history") => void;
    onAiExplainDiff?: (filePath: string, diffText: string) => void;
    onRevealFile?: (filePath: string) => void;
    onOpenInEditor?: (filePath: string) => void;
    onStatus?: (msg: string, isError?: boolean) => void;
  }

  let {
    repoPath = "",
    externalEditorLabel = "Editor",
    filePath,
    diffText = "",
    imageDiffData = null,
    ignoreWhitespace = false,
    isLfs = false,
    isStaged = false,
    onToggleWhitespace,
    onAddToGitignore,
    onStageHunk,
    onStageLines,
    onInspectFile,
    onAiExplainDiff,
    onRevealFile,
    onOpenInEditor,
    onStatus,
  }: Props = $props();
  let isImage = $derived(
    /\.(png|jpe?g|gif|webp|svg|ico|bmp)$/i.test(filePath) &&
      !!imageDiffData &&
      (!!imageDiffData.old_image_url || !!imageDiffData.new_image_url)
  );
  let isMarkdown = $derived(/\.(md|markdown)$/i.test(filePath));

  let viewMode = $state<"unified" | "split" | "markdown">("unified");
  let wrapLines = $state(true);
  let imageViewMode = $state<"2-up" | "swipe" | "onion">("2-up");
  let swipePercent = $state(50);
  let onionOpacity = $state(50);
  $effect(() => {
    if (!isMarkdown && viewMode === "markdown") {
      viewMode = "unified";
    }
  });

  function escapeHtml(str: string): string {
    return str
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;")
      .replace(/'/g, "&#39;");
  }

  const TOKEN_REGEX =
    /(\/\/.*$|\/\*.*?\*\/)|("(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*'|`(?:\\.|[^`\\])*`)|\b(fn|let|const|var|function|return|if|else|for|while|match|pub|struct|enum|import|from|export|type|interface|class|def|async|await|package|impl|trait|use|mod|mut|self|true|false|null|undefined)\b|\b(\d+(?:\.\d+)?)\b/g;

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
      return `${escapeHtml(prefix)}<span class="tok-com">${escapeHtml(content)}</span>`;
    }

    let out = escapeHtml(prefix);
    let lastIndex = 0;
    TOKEN_REGEX.lastIndex = 0;

    let match: RegExpExecArray | null;
    while ((match = TOKEN_REGEX.exec(content)) !== null) {
      if (match.index > lastIndex) {
        out += escapeHtml(content.slice(lastIndex, match.index));
      }
      const tokenText = escapeHtml(match[0]);
      if (match[1]) {
        out += `<span class="tok-com">${tokenText}</span>`;
      } else if (match[2]) {
        out += `<span class="tok-str">${tokenText}</span>`;
      } else if (match[3]) {
        out += `<span class="tok-kw">${tokenText}</span>`;
      } else if (match[4]) {
        out += `<span class="tok-num">${tokenText}</span>`;
      } else {
        out += tokenText;
      }
      lastIndex = match.index + match[0].length;
    }

    if (lastIndex < content.length) {
      out += escapeHtml(content.slice(lastIndex));
    }
    return out;
  }

  interface ParsedLine {
    line: string;
    kind: "add" | "del" | "meta" | "ctx";
    oldLn: number | null;
    newLn: number | null;
    html: string;
    isHunkHeader: boolean;
    hunkPatch?: string;
    hunkKey?: string;
    bodyIdx?: number;
  }
  interface HunkBlock {
    lines: string[];
    minusHeader: string;
    plusHeader: string;
  }

  function extractHunks(diff: string, path: string): HunkBlock[] {
    const raw = diff.split("\n");
    const hunks: HunkBlock[] = [];
    let minusHeader = `--- a/${path}`;
    let plusHeader = `+++ b/${path}`;
    let currentHunk: HunkBlock | null = null;

    for (const l of raw) {
      if (l.startsWith("--- ")) {
        minusHeader = l;
      } else if (l.startsWith("+++ ")) {
        plusHeader = l;
      } else if (l.startsWith("@@")) {
        if (currentHunk) hunks.push(currentHunk);
        currentHunk = { lines: [l], minusHeader, plusHeader };
      } else if (currentHunk) {
        currentHunk.lines.push(l);
      }
    }
    if (currentHunk) hunks.push(currentHunk);
    return hunks;
  }

  function parseHunkHeaderStart(line: string): { oldStart: number; newStart: number } {
    const m = line.match(/^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/);
    if (!m) return { oldStart: 1, newStart: 1 };
    return { oldStart: Number(m[1]), newStart: Number(m[2]) };
  }

  function parseLines(diff: string, path: string): ParsedLine[] {
    if (!diff || !diff.trim()) return [];
    const rawLines = diff.split("\n");
    const hunks = extractHunks(diff, path);
    let hunkIdx = 0;
    let activeHunkPatch: string | undefined = undefined;
    let activeHunkKey: string | undefined = undefined;
    let activeBodyIdx = 0;
    let oldLn = 0;
    let newLn = 0;
    let inHunk = false;

    return rawLines.map((line) => {
      if (line.startsWith("@@")) {
        const starts = parseHunkHeaderStart(line);
        oldLn = starts.oldStart;
        newLn = starts.newStart;
        inHunk = true;
        const curIdx = hunkIdx;
        const hunk = hunks[hunkIdx++];
        const hunkPatch = hunk
          ? `${hunk.minusHeader}\n${hunk.plusHeader}\n${hunk.lines.join("\n")}\n`
          : undefined;
        activeHunkPatch = hunkPatch;
        activeHunkKey = `hunk-${curIdx}`;
        activeBodyIdx = 0;
        return {
          line,
          kind: "meta",
          oldLn: null,
          newLn: null,
          html: highlightLine(line),
          isHunkHeader: true,
          hunkPatch,
          hunkKey: activeHunkKey,
        };
      }

      if (
        line.startsWith("diff ") ||
        line.startsWith("index ") ||
        line.startsWith("---") ||
        line.startsWith("+++")
      ) {
        return {
          line,
          kind: "meta",
          oldLn: null,
          newLn: null,
          html: highlightLine(line),
          isHunkHeader: false,
        };
      }

      if (line.startsWith("+")) {
        const ln = inHunk ? newLn++ : null;
        const bIdx = inHunk ? activeBodyIdx++ : undefined;
        return {
          line,
          kind: "add",
          oldLn: null,
          newLn: ln,
          html: highlightLine(line),
          isHunkHeader: false,
          hunkPatch: activeHunkPatch,
          hunkKey: activeHunkKey,
          bodyIdx: bIdx,
        };
      }

      if (line.startsWith("-")) {
        const ln = inHunk ? oldLn++ : null;
        const bIdx = inHunk ? activeBodyIdx++ : undefined;
        return {
          line,
          kind: "del",
          oldLn: ln,
          newLn: null,
          html: highlightLine(line),
          isHunkHeader: false,
          hunkPatch: activeHunkPatch,
          hunkKey: activeHunkKey,
          bodyIdx: bIdx,
        };
      }

      const o = inHunk && !line.startsWith("\\") ? oldLn++ : null;
      const n = inHunk && !line.startsWith("\\") ? newLn++ : null;
      const bIdx = inHunk && !line.startsWith("\\") ? activeBodyIdx++ : undefined;
      if (inHunk && line.startsWith("\\")) {
        activeBodyIdx++;
      }
      return {
        line,
        kind: "ctx",
        oldLn: o,
        newLn: n,
        html: highlightLine(line),
        isHunkHeader: false,
        hunkPatch: activeHunkPatch,
        hunkKey: activeHunkKey,
        bodyIdx: bIdx,
      };
    });
  }

  interface SplitRow {
    leftLn: number | null;
    leftType: "del" | "context" | "meta" | "empty";
    leftHtml: string;
    rightLn: number | null;
    rightType: "add" | "context" | "meta" | "empty";
    rightHtml: string;
    isHunkHeader?: boolean;
    hunkPatch?: string;
  }

  function parseSplit(diff: string, path: string): SplitRow[] {
    if (!diff || !diff.trim()) return [];
    const rawLines = diff.split("\n");
    const hunks = extractHunks(diff, path);
    let hunkIdx = 0;
    const rows: SplitRow[] = [];

    let delBuffer: { line: string; ln: number }[] = [];
    let addBuffer: { line: string; ln: number }[] = [];
    let oldLn = 1;
    let newLn = 1;

    function flushBuffers() {
      const maxLen = Math.max(delBuffer.length, addBuffer.length);
      for (let i = 0; i < maxLen; i++) {
        const left = delBuffer[i];
        const right = addBuffer[i];
        rows.push({
          leftLn: left ? left.ln : null,
          leftType: left ? "del" : "empty",
          leftHtml: left ? highlightLine(left.line) : "",
          rightLn: right ? right.ln : null,
          rightType: right ? "add" : "empty",
          rightHtml: right ? highlightLine(right.line) : "",
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
        const starts = parseHunkHeaderStart(line);
        oldLn = starts.oldStart;
        newLn = starts.newStart;
        const hunk = hunks[hunkIdx++];
        const hunkPatch = hunk
          ? `${hunk.minusHeader}\n${hunk.plusHeader}\n${hunk.lines.join("\n")}\n`
          : undefined;
        const h = highlightLine(line);
        rows.push({
          leftLn: null,
          leftType: "meta",
          leftHtml: h,
          rightLn: null,
          rightType: "meta",
          rightHtml: h,
          isHunkHeader: true,
          hunkPatch,
        });
      } else if (line.startsWith("-")) {
        delBuffer.push({ line, ln: oldLn++ });
      } else if (line.startsWith("+")) {
        addBuffer.push({ line, ln: newLn++ });
      } else {
        flushBuffers();
        const h = highlightLine(line);
        const isMetaNote = line.startsWith("\\");
        rows.push({
          leftLn: isMetaNote ? null : oldLn++,
          leftType: "context",
          leftHtml: h,
          rightLn: isMetaNote ? null : newLn++,
          rightType: "context",
          rightHtml: h,
        });
      }
    }
    flushBuffers();
    return rows;
  }

  function formatInlineMarkdown(raw: string): string {
    const inlineRegex = /(`[^`]+`)|(\*\*[^*]+\*\*)|(\*[^*]+\*)/g;
    let out = "";
    let last = 0;
    let m: RegExpExecArray | null;
    while ((m = inlineRegex.exec(raw)) !== null) {
      if (m.index > last) out += escapeHtml(raw.slice(last, m.index));
      if (m[1]) {
        out += `<code>${escapeHtml(m[1].slice(1, -1))}</code>`;
      } else if (m[2]) {
        out += `<strong>${escapeHtml(m[2].slice(2, -2))}</strong>`;
      } else if (m[3]) {
        out += `<em>${escapeHtml(m[3].slice(1, -1))}</em>`;
      }
      last = m.index + m[0].length;
    }
    if (last < raw.length) out += escapeHtml(raw.slice(last));
    return out;
  }

  function renderMarkdownDiff(text: string): { type: "add" | "del" | "ctx"; html: string }[] {
    const rawLines = (text || "").split("\n");
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

      let html: string;
      if (content.startsWith("### ")) {
        html = `<h3>${formatInlineMarkdown(content.slice(4))}</h3>`;
      } else if (content.startsWith("## ")) {
        html = `<h2>${formatInlineMarkdown(content.slice(3))}</h2>`;
      } else if (content.startsWith("# ")) {
        html = `<h1>${formatInlineMarkdown(content.slice(2))}</h1>`;
      } else if (content.startsWith("> ")) {
        html = `<blockquote>${formatInlineMarkdown(content.slice(2))}</blockquote>`;
      } else if (content.startsWith("- [x] ") || content.startsWith("* [x] ")) {
        html = `<label class="checkbox-label"><input type="checkbox" checked disabled /> <span>${formatInlineMarkdown(content.slice(6))}</span></label>`;
      } else if (content.startsWith("- [ ] ") || content.startsWith("* [ ] ")) {
        html = `<label class="checkbox-label"><input type="checkbox" disabled /> <span>${formatInlineMarkdown(content.slice(6))}</span></label>`;
      } else if (content.startsWith("- ") || content.startsWith("* ")) {
        html = `<li style="margin-left: 16px;">${formatInlineMarkdown(content.slice(2))}</li>`;
      } else {
        html = formatInlineMarkdown(content);
      }

      result.push({ type, html });
    }
    return result;
  }

  let safeDiffText = $derived(diffText ?? "");
  let lines = $derived(parseLines(safeDiffText, filePath));
  let splitRows = $derived(parseSplit(safeDiffText, filePath));
  let stats = $derived({
    adds: lines.filter((l) => l.kind === "add").length,
    dels: lines.filter((l) => l.kind === "del").length,
  });

  let selectedLineMap = $state<Record<string, true>>({});
  let selectedLineCount = $derived(Object.keys(selectedLineMap).length);

  $effect(() => {
    // Clear line selections whenever file or diff changes
    void filePath;
    void diffText;
    selectedLineMap = {};
  });

  function lineSelectKey(item: ParsedLine): string | null {
    if (!item.hunkKey || item.bodyIdx === undefined) return null;
    return `${item.hunkKey}:${item.bodyIdx}`;
  }

  function toggleLineSelection(item: ParsedLine) {
    const key = lineSelectKey(item);
    if (!key) return;
    if (selectedLineMap[key]) {
      const next = { ...selectedLineMap };
      delete next[key];
      selectedLineMap = next;
    } else {
      selectedLineMap = { ...selectedLineMap, [key]: true };
    }
  }

  async function applySelectedLines() {
    if (!onStageLines || selectedLineCount === 0) return;
    // Group selected indices by hunkKey and apply sequentially
    const byHunk = new Map<string, { patch: string; indices: number[] }>();
    for (const item of lines) {
      const k = lineSelectKey(item);
      if (!k || !selectedLineMap[k] || !item.hunkPatch || item.bodyIdx === undefined) continue;
      const bucket = byHunk.get(item.hunkKey!) ?? { patch: item.hunkPatch, indices: [] };
      bucket.indices.push(item.bodyIdx);
      byHunk.set(item.hunkKey!, bucket);
    }
    selectedLineMap = {};
    for (const { patch, indices } of byHunk.values()) {
      await onStageLines(patch, indices, isStaged);
    }
  }
  function openDiffContextMenu(e: MouseEvent) {
    if (e.target instanceof HTMLInputElement) return;
    const selectedText = window.getSelection()?.toString() || "";
    const fullPath = repoPath ? joinRepoPath(repoPath, filePath) : filePath;
    const items: ContextMenuItem[] = [];

    if (selectedText.trim().length > 0) {
      items.push({
        label: t("ctx.edit.copy"),
        hint: "Ctrl+C",
        onSelect: () => {
          void copyTextToClipboard(selectedText, onStatus, t("ctx.status.copiedSelection"));
        },
      });
      items.push({ type: "divider" });
    }

    if (filePath) {
      if (repoPath) {
        items.push({
          label: t("ctx.common.copyFilePath"),
          onSelect: () => {
            void copyTextToClipboard(fullPath, onStatus, t("ctx.status.copiedPath"));
          },
        });
      }
      items.push({
        label: t("ctx.common.copyRelativePath"),
        onSelect: () => {
          void copyTextToClipboard(filePath, onStatus, t("ctx.status.copiedPath"));
        },
      });
    }

    if ((onRevealFile || onOpenInEditor) && filePath) {
      items.push({ type: "divider" });
      if (onRevealFile) {
        items.push({
          label: t("ctx.common.revealInFileManager"),
          onSelect: () => onRevealFile(filePath),
        });
      }
      if (onOpenInEditor) {
        items.push({
          label: t("ctx.common.openInEditor", { editor: externalEditorLabel }),
          onSelect: () => onOpenInEditor(filePath),
        });
      }
    }

    if ((!isImage && viewMode !== "markdown") || onToggleWhitespace || onAddToGitignore) {
      items.push({ type: "divider" });
      if (!isImage && viewMode !== "markdown") {
        items.push({
          label: wrapLines ? t("ctx.diff.unwrapLines") : t("ctx.diff.wrapLines"),
          onSelect: () => {
            wrapLines = !wrapLines;
          },
        });
      }
      if (onToggleWhitespace && !isImage) {
        items.push({
          label: ignoreWhitespace ? t("ctx.diff.showWhitespace") : t("ctx.diff.ignoreWhitespace"),
          onSelect: onToggleWhitespace,
        });
      }
      if (onAddToGitignore && filePath && filePath !== ".gitignore") {
        items.push({
          label: t("ctx.diff.addToGitignore"),
          onSelect: () => onAddToGitignore(filePath),
        });
      }
    }

    openContextMenu(e, items);
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="diff-container" oncontextmenu={openDiffContextMenu}>
  <div class="diff-header">
    <div class="diff-file-meta">
      <span class="diff-file-path" title={filePath}>{filePath}</span>
      {#if !isImage && (stats.adds > 0 || stats.dels > 0)}
        <span class="tabular-nums" style="font-size: 11px; display: inline-flex; gap: 6px; font-family: var(--font-mono);">
          {#if stats.adds > 0}<span style="color: var(--diff-add-text);">+{stats.adds}</span>{/if}
          {#if stats.dels > 0}<span style="color: var(--diff-del-text);">-{stats.dels}</span>{/if}
        </span>
      {/if}
      {#if isLfs}
        <span class="badge-lfs">Git LFS</span>
      {/if}
    </div>

    <div class="diff-toolbar">
      {#if !isImage}
        <div class="seg-group" role="group" aria-label={t("diff.viewModeAria")}>
          <button
            type="button"
            class="seg-btn {viewMode === 'unified' ? 'active' : ''}"
            aria-pressed={viewMode === "unified"}
            onclick={() => (viewMode = "unified")}
          >
            {t("diff.unified")}
          </button>
          <button
            type="button"
            class="seg-btn {viewMode === 'split' ? 'active' : ''}"
            aria-pressed={viewMode === "split"}
            onclick={() => (viewMode = "split")}
          >
            {t("diff.split")}
          </button>
          {#if isMarkdown}
            <button
              type="button"
              class="seg-btn {viewMode === 'markdown' ? 'active' : ''}"
              aria-pressed={viewMode === "markdown"}
              onclick={() => (viewMode = "markdown")}
            >
              {t("diff.renderedMd")}
            </button>
          {/if}
        </div>
      {/if}

      {#if !isImage && viewMode !== "markdown"}
        <button
          type="button"
          onclick={() => (wrapLines = !wrapLines)}
          class="btn-secondary btn-xs {wrapLines ? 'btn-accent' : ''}"
          aria-pressed={wrapLines}
          title={t("diff.wrapTitle")}
        >
          {t("diff.wrap")}
        </button>
      {/if}
      {#if onToggleWhitespace && !isImage}
        <button
          type="button"
          onclick={onToggleWhitespace}
          class="btn-secondary btn-xs {ignoreWhitespace ? 'btn-accent' : ''}"
          aria-pressed={ignoreWhitespace}
          title={t("diff.ignoreWsTitle")}
        >
          {ignoreWhitespace ? t("diff.wsIgnored") : t("diff.ignoreWs")}
        </button>
      {/if}
      {#if onInspectFile && repoPath && filePath}
        <button
          type="button"
          onclick={() => onInspectFile(filePath, "blame")}
          class="btn-secondary btn-xs"
          title={t("diff.blameTitle")}
        >
          {t("diff.blameBtn")}
        </button>
        <button
          type="button"
          onclick={() => onInspectFile(filePath, "history")}
          class="btn-secondary btn-xs"
          title={t("diff.fileHistoryTitle")}
        >
          {t("diff.fileHistoryBtn")}
        </button>
      {/if}
      {#if onAiExplainDiff && !isImage && safeDiffText.trim()}
        <button
          type="button"
          onclick={() => onAiExplainDiff(filePath, safeDiffText)}
          class="btn-secondary btn-xs"
          title={t("ai.explainDiffTitle")}
        >
          ✦ {t("ai.explainDiffBtn")}
        </button>
      {/if}
      {#if onAddToGitignore}
        <button
          type="button"
          onclick={() => onAddToGitignore?.(filePath)}
          class="btn-secondary btn-xs"
          title={t("diff.addGitignoreTitle")}
        >
          + .gitignore
        </button>
      {/if}
    </div>
  </div>

  {#if isImage && imageDiffData}
    <div class="image-diff-stage">
      <div class="seg-group" role="group" aria-label={t("diff.imageModeAria")}>
        <button
          type="button"
          class="seg-btn {imageViewMode === '2-up' ? 'active' : ''}"
          aria-pressed={imageViewMode === "2-up"}
          onclick={() => (imageViewMode = "2-up")}
        >
          {t("diff.image2Up")}
        </button>
        <button
          type="button"
          class="seg-btn {imageViewMode === 'swipe' ? 'active' : ''}"
          aria-pressed={imageViewMode === "swipe"}
          onclick={() => (imageViewMode = "swipe")}
        >
          {t("diff.imageSwipe")}
        </button>
        <button
          type="button"
          class="seg-btn {imageViewMode === 'onion' ? 'active' : ''}"
          aria-pressed={imageViewMode === "onion"}
          onclick={() => (imageViewMode = "onion")}
        >
          {t("diff.imageOnion")}
        </button>
      </div>

      {#if imageViewMode === "2-up"}
        <div class="image-diff-grid">
          {#if imageDiffData.old_image_url}
            <div class="form-field" style="align-items: center;">
              <span class="form-label" style="color: var(--diff-del-text);">{t("diff.imageBefore")}</span>
              <div class="image-diff-frame">
                <img src={imageDiffData.old_image_url} alt={t("diff.imageBeforeAlt")} />
              </div>
            </div>
          {/if}
          {#if imageDiffData.new_image_url}
            <div class="form-field" style="align-items: center;">
              <span class="form-label" style="color: var(--diff-add-text);">{t("diff.imageAfter")}</span>
              <div class="image-diff-frame">
                <img src={imageDiffData.new_image_url} alt={t("diff.imageAfterAlt")} />
              </div>
            </div>
          {/if}
        </div>
      {:else if imageViewMode === "swipe"}
        <div class="form-field" style="align-items: center; width: 100%; max-width: 520px; gap: 10px;">
          <label class="form-label tabular-nums" for="image-swipe-range">
            {t("diff.swipeLabel", { percent: swipePercent })}
          </label>
          <input id="image-swipe-range" type="range" min="0" max="100" bind:value={swipePercent} style="width: 100%;" />
          <div class="image-diff-overlay-box">
            {#if imageDiffData.old_image_url}
              <img src={imageDiffData.old_image_url} alt={t("diff.imageBeforeAlt")} />
            {/if}
            {#if imageDiffData.new_image_url}
              <div style="position: absolute; inset: 0; display: grid; place-items: center; clip-path: inset(0 {100 - swipePercent}% 0 0); border-right: 2px solid var(--accent-blue);">
                <img src={imageDiffData.new_image_url} alt={t("diff.imageAfterAlt")} />
              </div>
            {/if}
          </div>
        </div>
      {:else}
        <div class="form-field" style="align-items: center; width: 100%; max-width: 520px; gap: 10px;">
          <label class="form-label tabular-nums" for="image-onion-range">
            {t("diff.onionLabel", { percent: onionOpacity })}
          </label>
          <input id="image-onion-range" type="range" min="0" max="100" bind:value={onionOpacity} style="width: 100%;" />
          <div class="image-diff-overlay-box">
            {#if imageDiffData.old_image_url}
              <img src={imageDiffData.old_image_url} alt={t("diff.imageBeforeAlt")} />
            {/if}
            {#if imageDiffData.new_image_url}
              <div style="position: absolute; inset: 0; display: grid; place-items: center; opacity: {onionOpacity / 100};">
                <img src={imageDiffData.new_image_url} alt={t("diff.imageAfterAlt")} />
              </div>
            {/if}
          </div>
        </div>
      {/if}
    </div>
  {:else if viewMode === "markdown"}
    <div class="md-diff-view">
      {#each renderMarkdownDiff(safeDiffText) as item}
        <div
          class="diff-line {item.type === 'add' ? 'add' : item.type === 'del' ? 'del' : ''}"
          style="padding: 3px 10px; font-family: inherit; white-space: normal;"
        >
          {@html item.html}
        </div>
      {/each}
    </div>
  {:else if lines.length === 0}
    <div class="empty-state">
      <span>{t("diff.emptyDiff")}</span>
    </div>
  {:else if viewMode === "unified"}
    {#if onStageLines && selectedLineCount > 0}
      <div class="line-stage-bar">
        <span class="tabular-nums">{t("diff.selectedLinesCount", { count: selectedLineCount })}</span>
        <button
          type="button"
          class="btn-primary btn-xs"
          onclick={applySelectedLines}
        >
          {isStaged ? t("diff.unstageSelectedLinesBtn") : t("diff.stageSelectedLinesBtn")}
        </button>
        <button
          type="button"
          class="btn-secondary btn-xs"
          onclick={() => (selectedLineMap = {})}
        >
          {t("diff.clearSelectionBtn")}
        </button>
      </div>
    {/if}
    <div class="diff-content {wrapLines ? 'is-wrapped' : ''}">
      {#each lines as item}
        {#if item.isHunkHeader}
          <div class="diff-line-row meta-row">
            <span class="diff-code">{@html item.html}</span>
            {#if item.hunkPatch && onStageHunk}
              <button
                type="button"
                class="btn-secondary btn-xs {isStaged ? 'btn-danger' : ''}"
                style={!isStaged ? "border-color: var(--diff-add-text); color: var(--diff-add-text);" : ""}
                onclick={() => onStageHunk(item.hunkPatch!, isStaged)}
                title={isStaged ? t("diff.unstageHunkTitle") : t("diff.stageHunkTitle")}
              >
                {isStaged ? t("diff.unstageHunkBtn") : t("diff.stageHunkBtn")}
              </button>
            {/if}
          </div>
        {:else}
          {@const canStageLine = !!onStageLines && (item.kind === "add" || item.kind === "del") && !!item.hunkPatch && item.bodyIdx !== undefined}
          {@const selKey = lineSelectKey(item)}
          {@const isLineSelected = !!(selKey && selectedLineMap[selKey])}
          <div class="diff-line-row {item.kind === 'add' ? 'add' : item.kind === 'del' ? 'del' : ''} {isLineSelected ? 'is-line-selected' : ''}">
            {#if onStageLines}
              <span class="diff-line-select-cell">
                {#if canStageLine}
                  <input
                    type="checkbox"
                    class="diff-line-cb"
                    checked={isLineSelected}
                    onchange={() => toggleLineSelection(item)}
                    aria-label={t("diff.selectLineAria")}
                  />
                {/if}
              </span>
            {/if}
            <span class="diff-ln">{item.oldLn ?? ""}</span>
            <span class="diff-ln">{item.newLn ?? ""}</span>
            <span class="diff-line {item.kind === 'meta' ? 'meta' : ''} diff-code">{@html item.html}</span>
            {#if canStageLine}
              <button
                type="button"
                class="diff-single-line-btn {isStaged ? 'is-unstage' : 'is-stage'}"
                title={isStaged ? t("diff.unstageSingleLineTitle") : t("diff.stageSingleLineTitle")}
                onclick={() => onStageLines!(item.hunkPatch!, [item.bodyIdx!], isStaged)}
              >
                {isStaged ? "−" : "+"}
              </button>
            {/if}
          </div>
        {/if}
      {/each}
    </div>
  {:else}
    <div class="diff-content {wrapLines ? 'is-wrapped' : ''}" style="padding: 0;">
      {#each splitRows as row}
        {#if row.isHunkHeader}
          <div class="diff-line-row meta-row">
            <span class="diff-code">{@html row.leftHtml}</span>
            {#if row.hunkPatch && onStageHunk}
              <button
                type="button"
                class="btn-secondary btn-xs {isStaged ? 'btn-danger' : ''}"
                style={!isStaged ? "border-color: var(--diff-add-text); color: var(--diff-add-text);" : ""}
                onclick={() => onStageHunk(row.hunkPatch!, isStaged)}
                title={isStaged ? t("diff.unstageHunkTitle") : t("diff.stageHunkTitle")}
              >
                {isStaged ? t("diff.unstageHunkBtn") : t("diff.stageHunkBtn")}
              </button>
            {/if}
          </div>
        {:else}
          <div class="split-diff-grid">
            <div class="split-diff-cell left {row.leftType === 'del' ? 'del' : ''}">
              <span class="diff-ln">{row.leftLn ?? ""}</span>
              <span class="split-diff-code diff-code">{@html row.leftHtml}</span>
            </div>
            <div class="split-diff-cell {row.rightType === 'add' ? 'add' : ''}">
              <span class="diff-ln">{row.rightLn ?? ""}</span>
              <span class="split-diff-code diff-code">{@html row.rightHtml}</span>
            </div>
          </div>
        {/if}
      {/each}
    </div>
  {/if}
</div>
