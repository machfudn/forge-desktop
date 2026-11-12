<script lang="ts">
  import type { ConflictBlock, ConflictFileData } from "../types/git";
  import * as gitService from "../services/git";
  import { t } from "../i18n/index.svelte";

  interface Props {
    repoPath: string;
    conflictedFiles: string[];
    initialFilePath?: string | null;
    onClose: () => void;
    onResolved: (filePath: string) => Promise<void> | void;
    onStatus?: (msg: string, isError?: boolean) => void;
  }

  let {
    repoPath,
    conflictedFiles,
    initialFilePath = null,
    onClose,
    onResolved,
    onStatus,
  }: Props = $props();

  let selectedFile = $state<string>("");
  let conflictData = $state<ConflictFileData | null>(null);
  let outputText = $state("");
  let loading = $state(false);
  let saving = $state(false);
  let aiBusyBlock = $state<number | "all" | null>(null);
  let showBase = $state(false);
  let errorMsg = $state("");

  $effect(() => {
    const first =
      initialFilePath && conflictedFiles.includes(initialFilePath)
        ? initialFilePath
        : conflictedFiles[0] || "";
    if (first && first !== selectedFile) {
      selectedFile = first;
    }
  });

  $effect(() => {
    if (selectedFile && repoPath) {
      void loadFile(selectedFile);
    }
  });

  async function loadFile(filePath: string) {
    loading = true;
    errorMsg = "";
    try {
      const data = await gitService.getConflictFile(repoPath, filePath);
      conflictData = data;
      outputText = data.working_content;
    } catch (err) {
      errorMsg = String(err);
      conflictData = null;
    } finally {
      loading = false;
    }
  }

  function replaceBlockInOutput(block: ConflictBlock, replacement: string) {
    const normalized = outputText.replace(/\r\n/g, "\n");
    const lines = normalized.split("\n");
    // Locate exact matching conflict marker block near start_line or first matching block
    let start = -1;
    let end = -1;
    let seenBlocks = 0;
    for (let i = 0; i < lines.length; i++) {
      if (lines[i].startsWith("<<<<<<<")) {
        let j = i + 1;
        while (j < lines.length && !lines[j].startsWith(">>>>>>>")) {
          j++;
        }
        if (j < lines.length) {
          if (seenBlocks === block.index || start === -1) {
            start = i;
            end = j;
            if (seenBlocks === block.index) break;
          }
          seenBlocks++;
          i = j;
        }
      }
    }

    if (start !== -1 && end !== -1) {
      const repLines = replacement ? replacement.replace(/\r\n/g, "\n").split("\n") : [];
      const next = [...lines.slice(0, start), ...repLines, ...lines.slice(end + 1)];
      outputText = next.join("\n");
    }
  }

  function resolveAllSide(side: "ours" | "theirs" | "both") {
    const lines = outputText.replace(/\r\n/g, "\n").split("\n");
    const out: string[] = [];
    let i = 0;
    while (i < lines.length) {
      if (lines[i].startsWith("<<<<<<<")) {
        const ours: string[] = [];
        const theirs: string[] = [];
        let mode: "ours" | "base" | "theirs" = "ours";
        let j = i + 1;
        while (j < lines.length && !lines[j].startsWith(">>>>>>>")) {
          if (lines[j].startsWith("|||||||")) {
            mode = "base";
          } else if (lines[j].startsWith("=======")) {
            mode = "theirs";
          } else if (mode === "ours") {
            ours.push(lines[j]);
          } else if (mode === "theirs") {
            theirs.push(lines[j]);
          }
          j++;
        }
        if (j < lines.length) {
          if (side === "ours") out.push(...ours);
          else if (side === "theirs") out.push(...theirs);
          else out.push(...ours, ...theirs);
          i = j + 1;
          continue;
        }
      }
      out.push(lines[i]);
      i++;
    }
    outputText = out.join("\n");
  }

  let remainingMarkerCount = $derived(
    (outputText.match(/^<{7}/gm) || []).length
  );

  async function handleAiResolveBlock(block: ConflictBlock) {
    if (aiBusyBlock !== null) return;
    aiBusyBlock = block.index;
    errorMsg = "";
    try {
      const prompt = [
        `File: ${selectedFile}`,
        `--- OURS (${block.ours_label}) ---\n${block.ours}`,
        block.base ? `--- BASE ---\n${block.base}` : "",
        `--- THEIRS (${block.theirs_label}) ---\n${block.theirs}`,
      ]
        .filter(Boolean)
        .join("\n\n");
      const res = await gitService.aiRunTask("resolve_conflict", prompt, repoPath);
      replaceBlockInOutput(block, res.content);
      onStatus?.(t("conflict.aiResolvedBlock", { model: res.model }));
    } catch (err) {
      errorMsg = String(err);
      onStatus?.(String(err), true);
    } finally {
      aiBusyBlock = null;
    }
  }

  async function handleAiResolveWholeFile() {
    if (aiBusyBlock !== null || !conflictData) return;
    aiBusyBlock = "all";
    errorMsg = "";
    try {
      const prompt = `File: ${selectedFile}\n\nResolve all conflict markers in this file cleanly:\n\n${outputText}`;
      const res = await gitService.aiRunTask("resolve_conflict", prompt, repoPath);
      outputText = res.content;
      onStatus?.(t("conflict.aiResolvedFile", { model: res.model }));
    } catch (err) {
      errorMsg = String(err);
      onStatus?.(String(err), true);
    } finally {
      aiBusyBlock = null;
    }
  }

  async function handleSaveAndStage() {
    if (!selectedFile || saving) return;
    saving = true;
    errorMsg = "";
    try {
      await gitService.resolveConflictFile(repoPath, selectedFile, outputText, true);
      onStatus?.(t("conflict.savedAndStaged", { file: selectedFile }));
      await onResolved(selectedFile);
      const remaining = conflictedFiles.filter((f) => f !== selectedFile);
      if (remaining.length > 0) {
        selectedFile = remaining[0];
      } else {
        onClose();
      }
    } catch (err) {
      errorMsg = String(err);
      onStatus?.(String(err), true);
    } finally {
      saving = false;
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="modal-overlay"
  onclick={(e) => {
    if (e.target === e.currentTarget) onClose();
  }}
  onkeydown={(e) => {
    if (e.key === "Escape") onClose();
  }}
>
  <div class="modal-dialog modal-xl conflict-modal" role="dialog" aria-modal="true" aria-label={t("conflict.title")}>
    <div class="modal-header">
      <div style="display: flex; align-items: center; gap: 10px;">
        <h3>{t("conflict.title")}</h3>
        <span class="badge-neutral tabular-nums">
          {remainingMarkerCount === 0
            ? t("conflict.markersClean")
            : t("conflict.markersRemaining", { count: remainingMarkerCount })}
        </span>
      </div>
      <div style="display: flex; align-items: center; gap: 6px; flex-wrap: wrap;">
        <button type="button" class="btn-secondary btn-xs" onclick={() => (showBase = !showBase)}>
          {showBase ? t("conflict.hideBaseBtn") : t("conflict.showBaseBtn")}
        </button>
        <button type="button" class="btn-secondary btn-xs" onclick={() => resolveAllSide("ours")}>
          {t("conflict.acceptAllOurs")}
        </button>
        <button type="button" class="btn-secondary btn-xs" onclick={() => resolveAllSide("theirs")}>
          {t("conflict.acceptAllTheirs")}
        </button>
        <button
          type="button"
          class="btn-secondary btn-xs ai-btn"
          disabled={aiBusyBlock !== null || loading}
          onclick={handleAiResolveWholeFile}
        >
          {aiBusyBlock === "all" ? t("conflict.aiResolving") : `✦ ${t("conflict.aiResolveFile")}`}
        </button>
        <button type="button" class="btn-secondary btn-xs" onclick={onClose}>{t("common.close")}</button>
      </div>
    </div>

    <div class="conflict-layout">
      {#if conflictedFiles.length > 1}
        <aside class="conflict-file-sidebar">
          <div class="conflict-sidebar-title">{t("conflict.filesHeader")}</div>
          {#each conflictedFiles as cf (cf)}
            <button
              type="button"
              class="conflict-file-item {cf === selectedFile ? 'active' : ''}"
              onclick={() => (selectedFile = cf)}
              title={cf}
            >
              {cf}
            </button>
          {/each}
        </aside>
      {/if}

      <div class="conflict-main">
        {#if errorMsg}
          <div class="alert-banner" style="margin: 8px 12px 0; border-radius: var(--radius-sm);">
            <span>{errorMsg}</span>
          </div>
        {/if}

        {#if loading}
          <div class="empty-state"><span>{t("conflict.loading")}</span></div>
        {:else if conflictData}
          <div class="conflict-panes {showBase ? 'has-base' : ''}">
            <div class="conflict-pane">
              <div class="conflict-pane-head ours">{t("conflict.oursPane")}</div>
              <pre class="conflict-code">{conflictData.ours_content ?? t("conflict.stageUnavailable")}</pre>
            </div>
            {#if showBase}
              <div class="conflict-pane">
                <div class="conflict-pane-head base">{t("conflict.basePane")}</div>
                <pre class="conflict-code">{conflictData.base_content ?? t("conflict.stageUnavailable")}</pre>
              </div>
            {/if}
            <div class="conflict-pane">
              <div class="conflict-pane-head theirs">{t("conflict.theirsPane")}</div>
              <pre class="conflict-code">{conflictData.theirs_content ?? t("conflict.stageUnavailable")}</pre>
            </div>
          </div>

          {#if conflictData.blocks.length > 0}
            <div class="conflict-blocks-bar">
              {#each conflictData.blocks as block (block.index)}
                <div class="conflict-block-card">
                  <span class="conflict-block-label">
                    #{block.index + 1} (L{block.start_line}–{block.end_line})
                  </span>
                  <div class="conflict-block-actions">
                    <button
                      type="button"
                      class="btn-secondary btn-xs"
                      onclick={() => replaceBlockInOutput(block, block.ours)}
                    >
                      {t("conflict.acceptOurs")}
                    </button>
                    <button
                      type="button"
                      class="btn-secondary btn-xs"
                      onclick={() => replaceBlockInOutput(block, block.theirs)}
                    >
                      {t("conflict.acceptTheirs")}
                    </button>
                    <button
                      type="button"
                      class="btn-secondary btn-xs"
                      onclick={() => replaceBlockInOutput(block, `${block.ours}\n${block.theirs}`)}
                    >
                      {t("conflict.acceptBoth")}
                    </button>
                    <button
                      type="button"
                      class="btn-secondary btn-xs ai-btn"
                      disabled={aiBusyBlock !== null}
                      onclick={() => handleAiResolveBlock(block)}
                    >
                      {aiBusyBlock === block.index ? "…" : `✦ ${t("conflict.aiResolveBlock")}`}
                    </button>
                  </div>
                </div>
              {/each}
            </div>
          {/if}

          <div class="conflict-output-wrap">
            <div class="conflict-pane-head output">
              <span>{t("conflict.outputPane")} — {selectedFile}</span>
            </div>
            <textarea
              class="conflict-output-editor"
              bind:value={outputText}
              spellcheck="false"
              aria-label={t("conflict.outputPane")}
            ></textarea>
          </div>
        {/if}
      </div>
    </div>

    <div class="modal-actions">
      <button type="button" class="btn-secondary" onclick={onClose}>{t("common.cancel")}</button>
      <button
        type="button"
        class="btn-primary"
        disabled={saving || loading || !selectedFile || remainingMarkerCount > 0}
        onclick={handleSaveAndStage}
      >
        {saving ? t("conflict.savingBtn") : t("conflict.saveStageBtn")}
      </button>
    </div>
  </div>
</div>
