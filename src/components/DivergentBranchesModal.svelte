<script lang="ts">
  import { t } from "../i18n/index.svelte";
  import { copyTextToClipboard } from "../lib/contextMenu.svelte";
  import { pathBaseName } from "../utils/path";

  interface Props {
    open: boolean;
    repo: string;
    branchName?: string | null;
    errorMessage: string;
    onClose: () => void;
    onPullRebase?: () => Promise<void>;
    onPullMerge?: () => Promise<void>;
  }

  let {
    open,
    repo,
    branchName = "main",
    errorMessage,
    onClose,
    onPullRebase,
    onPullMerge,
  }: Props = $props();

  let isRebasing = $state(false);
  let isMerging = $state(false);
  let showRawLogs = $state(false);
  let copied = $state(false);
  let actionError = $state("");

  async function handleCopyError() {
    await copyTextToClipboard(errorMessage);
    copied = true;
    setTimeout(() => {
      copied = false;
    }, 2000);
  }

  async function handlePullRebase() {
    if (!onPullRebase || isRebasing || isMerging) return;
    isRebasing = true;
    actionError = "";
    try {
      await onPullRebase();
      onClose();
    } catch (err) {
      actionError = String(err);
    } finally {
      isRebasing = false;
    }
  }

  async function handlePullMerge() {
    if (!onPullMerge || isRebasing || isMerging) return;
    isMerging = true;
    actionError = "";
    try {
      await onPullMerge();
      onClose();
    } catch (err) {
      actionError = String(err);
    } finally {
      isMerging = false;
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      if (!isRebasing && !isMerging) {
        e.stopPropagation();
        onClose();
      }
    }
  }
</script>

{#if open}
  <div
    class="modal-overlay"
    role="presentation"
    onclick={(e) => {
      if (e.target === e.currentTarget && !isRebasing && !isMerging) onClose();
    }}
    onkeydown={handleKeyDown}
  >
    <div
      class="modal-dialog divergent-dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="divergent-title"
      tabindex="-1"
    >
      <!-- Header -->
      <div class="modal-header divergent-header">
        <div class="divergent-title-wrap">
          <div class="divergent-icon-box">
            <svg
              width="20"
              height="20"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
              aria-hidden="true"
            >
              <line x1="6" y1="3" x2="6" y2="15" />
              <circle cx="18" cy="6" r="3" />
              <circle cx="6" cy="18" r="3" />
              <path d="M18 9a9 9 0 0 1-9 9" />
            </svg>
          </div>
          <div>
            <div style="display: flex; align-items: center; gap: 8px;">
              <h2 id="divergent-title" class="divergent-title">
                {t("divergentModal.title")}
              </h2>
              {#if repo}
                <span class="divergent-repo-badge">{pathBaseName(repo)}</span>
              {/if}
            </div>
            <p class="divergent-subtitle">
              {t("divergentModal.subtitle", { branch: branchName || "main" })}
            </p>
          </div>
        </div>
        <button
          type="button"
          class="icon-btn compact"
          onclick={onClose}
          disabled={isRebasing || isMerging}
          aria-label={t("common.closeDialog")}
        >
          <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
            <path d="M3.72 3.72a.75.75 0 0 1 1.06 0L8 6.94l3.22-3.22a.749.749 0 0 1 1.275.326.749.749 0 0 1-.215.734L9.06 8l3.22 3.22a.749.749 0 0 1-.326 1.275.749.749 0 0 1-.734-.215L8 9.06l-3.22 3.22a.751.751 0 0 1-1.042-.018.751.751 0 0 1-.018-1.042L6.94 8 3.72 4.78a.75.75 0 0 1 0-1.06Z" />
          </svg>
        </button>
      </div>

      <!-- Body -->
      <div class="modal-body divergent-body">
        <!-- Callout Explanation -->
        <div class="divergent-callout">
          <p class="divergent-explanation">
            {t("divergentModal.explanation")}
          </p>
        </div>

        <!-- Strategy Options Card -->
        <div class="divergent-strategies-card">
          <div class="divergent-strategies-title">{t("divergentModal.strategiesTitle")}</div>
          <div class="divergent-strategies-grid">
            <div class="divergent-strategy-item">
              <span class="divergent-strategy-tag tag-primary">{t("divergentModal.rebaseTag")}</span>
              <div>
                <strong class="divergent-strategy-name">{t("divergentModal.rebaseLabel")}</strong>
                <p class="divergent-strategy-desc">{t("divergentModal.rebaseDesc")}</p>
              </div>
            </div>
            <div class="divergent-strategy-item">
              <span class="divergent-strategy-tag tag-secondary">{t("divergentModal.mergeTag")}</span>
              <div>
                <strong class="divergent-strategy-name">{t("divergentModal.mergeLabel")}</strong>
                <p class="divergent-strategy-desc">{t("divergentModal.mergeDesc")}</p>
              </div>
            </div>
          </div>
        </div>

        {#if actionError}
          <div class="divergent-error-box">
            {actionError}
          </div>
        {/if}

        <!-- Raw Error Log Accordion -->
        <div class="divergent-error-section">
          <div class="divergent-error-header">
            <button
              type="button"
              class="divergent-toggle-logs-btn"
              onclick={() => (showRawLogs = !showRawLogs)}
            >
              <svg
                width="12"
                height="12"
                viewBox="0 0 16 16"
                fill="currentColor"
                style="transform: rotate({showRawLogs ? 90 : 0}deg); transition: transform 0.15s ease;"
                aria-hidden="true"
              >
                <path d="M6.22 3.22a.75.75 0 0 1 1.06 0l4.25 4.25a.75.75 0 0 1 0 1.06l-4.25 4.25a.75.75 0 0 1-1.06-1.06L9.94 8 6.22 4.28a.75.75 0 0 1 0-1.06Z" />
              </svg>
              {showRawLogs ? t("workflowModal.hideRawLogs") : t("workflowModal.showRawLogs")}
            </button>
            <button type="button" class="divergent-copy-btn" onclick={handleCopyError}>
              <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
                <path d="M0 6.75C0 5.784.784 5 1.75 5h1.5a.75.75 0 0 1 0 1.5h-1.5a.25.25 0 0 0-.25.25v7.5c0 .138.112.25.25.25h7.5a.25.25 0 0 0 .25-.25v-1.5a.75.75 0 0 1 1.5 0v1.5A1.75 1.75 0 0 1 9.25 16h-7.5A1.75 1.75 0 0 1 0 14.25Z" />
                <path d="M5 1.75C5 .784 5.784 0 6.75 0h7.5C15.216 0 16 .784 16 1.75v7.5A1.75 1.75 0 0 1 14.25 11h-7.5A1.75 1.75 0 0 1 5 9.25Zm1.75-.25a.25.25 0 0 0-.25.25v7.5c0 .138.112.25.25.25h7.5a.25.25 0 0 0 .25-.25v-7.5a.25.25 0 0 0-.25-.25Z" />
              </svg>
              {copied ? t("secretModal.copied") : t("secretModal.copyError")}
            </button>
          </div>
          {#if showRawLogs}
            <pre class="divergent-raw-log-block">{errorMessage}</pre>
          {/if}
        </div>
      </div>

      <!-- Footer Actions: 2 Distinct Choice Buttons (Pull Rebase & Pull Merge) -->
      <div class="modal-footer divergent-footer">
        <button
          type="button"
          class="btn-secondary btn-sm"
          onclick={onClose}
          disabled={isRebasing || isMerging}
        >
          {t("common.close")}
        </button>

        <!-- Button 2: Pull with Merge -->
        {#if onPullMerge}
          <button
            type="button"
            class="btn-secondary btn-sm divergent-action-btn"
            onclick={handlePullMerge}
            disabled={isRebasing || isMerging}
            title={t("divergentModal.mergeTooltip")}
          >
            {#if isMerging}
              <span class="divergent-spinner" aria-hidden="true"></span>
              {t("divergentModal.merging")}
            {:else}
              <svg width="13" height="13" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true" style="margin-right: 4px;">
                <path d="M5 3.25a.75.75 0 1 1-1.5 0 .75.75 0 0 1 1.5 0Zm0 2.122a2.25 2.25 0 1 0-1.5 0v5.256a2.25 2.25 0 1 0 1.5 0V5.372Zm7.5 5.378a.75.75 0 1 1-1.5 0 .75.75 0 0 1 1.5 0Zm0-2.122a2.25 2.25 0 1 0-1.5 0v.628a.75.75 0 0 1-.75.75h-2.5a.75.75 0 0 0-.75.75v1.244a2.25 2.25 0 1 0 1.5 0V12.5h2.5a2.25 2.25 0 0 0 2.25-2.25v-.628ZM5 12.75a.75.75 0 1 1-1.5 0 .75.75 0 0 1 1.5 0Z" />
              </svg>
              {t("divergentModal.pullMergeBtn")}
            {/if}
          </button>
        {/if}

        <!-- Button 1: Pull with Rebase (Primary) -->
        {#if onPullRebase}
          <button
            type="button"
            class="btn-primary btn-sm divergent-action-btn"
            onclick={handlePullRebase}
            disabled={isRebasing || isMerging}
            title={t("divergentModal.rebaseTooltip")}
          >
            {#if isRebasing}
              <span class="divergent-spinner" aria-hidden="true"></span>
              {t("divergentModal.rebasing")}
            {:else}
              <svg width="13" height="13" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true" style="margin-right: 4px;">
                <path d="M11.93 8.5a4.002 4.002 0 0 1-7.86 1H2.75a.75.75 0 0 1 0-1.5h1.32a4.002 4.002 0 0 1 7.86.5Zm-4.93-3a1.5 1.5 0 1 0 0-3 1.5 1.5 0 0 0 0 3Zm0 9a1.5 1.5 0 1 0 0-3 1.5 1.5 0 0 0 0 3Z" />
              </svg>
              {t("divergentModal.pullRebaseBtn")}
            {/if}
          </button>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .divergent-dialog {
    width: 92vw;
    max-width: 580px;
    background-color: var(--bg-panel);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-lg, 8px);
    box-shadow: var(--shadow-xl);
    overflow: hidden;
    display: flex;
    flex-direction: column;
    animation: modalPopIn 0.2s cubic-bezier(0.16, 1, 0.3, 1);
  }

  @keyframes modalPopIn {
    from {
      opacity: 0;
      transform: scale(0.97) translateY(4px);
    }
    to {
      opacity: 1;
      transform: scale(1) translateY(0);
    }
  }

  .divergent-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    padding: 16px 20px 14px;
    border-bottom: 1px solid var(--border-muted);
    background-color: var(--bg-surface);
  }

  .divergent-title-wrap {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .divergent-icon-box {
    width: 36px;
    height: 36px;
    border-radius: var(--radius-md, 6px);
    background-color: var(--accent-subtle, rgba(99, 102, 241, 0.12));
    border: 1px solid var(--accent-border, rgba(99, 102, 241, 0.3));
    color: var(--accent-solid, #6366f1);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .divergent-title {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .divergent-repo-badge {
    font-size: 11px;
    padding: 1px 6px;
    border-radius: var(--radius-sm, 4px);
    background-color: var(--bg-canvas);
    border: 1px solid var(--border-muted);
    color: var(--text-muted);
    font-family: var(--font-mono);
  }

  .divergent-subtitle {
    margin: 2px 0 0;
    font-size: 12px;
    color: var(--text-secondary);
    line-height: 1.4;
  }

  .divergent-body {
    padding: 16px 20px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    background-color: var(--bg-panel);
    max-height: 70vh;
    overflow-y: auto;
  }

  .divergent-callout {
    padding: 12px 14px;
    background-color: var(--bg-surface);
    border: 1px solid var(--border-muted);
    border-left: 3px solid var(--accent-solid, #6366f1);
    border-radius: var(--radius-sm, 4px);
  }

  .divergent-explanation {
    margin: 0;
    font-size: 13px;
    line-height: 1.5;
    color: var(--text-primary);
  }

  .divergent-strategies-card {
    padding: 12px 14px;
    background-color: var(--bg-surface);
    border: 1px solid var(--border-muted);
    border-radius: var(--radius-md, 6px);
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .divergent-strategies-title {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .divergent-strategies-grid {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .divergent-strategy-item {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    background-color: var(--bg-canvas);
    padding: 10px 12px;
    border-radius: var(--radius-sm, 4px);
    border: 1px solid var(--border-muted);
  }

  .divergent-strategy-tag {
    font-size: 10px;
    font-weight: 600;
    padding: 2px 6px;
    border-radius: var(--radius-sm, 4px);
    flex-shrink: 0;
    margin-top: 1px;
  }

  .tag-primary {
    background-color: var(--accent-subtle, rgba(99, 102, 241, 0.12));
    border: 1px solid var(--accent-border, rgba(99, 102, 241, 0.3));
    color: var(--accent-solid, #6366f1);
  }

  .tag-secondary {
    background-color: var(--bg-surface);
    border: 1px solid var(--border-muted);
    color: var(--text-muted);
  }

  .divergent-strategy-name {
    display: block;
    font-size: 12px;
    color: var(--text-primary);
    margin-bottom: 2px;
  }

  .divergent-strategy-desc {
    margin: 0;
    font-size: 11px;
    color: var(--text-secondary);
    line-height: 1.4;
  }

  .divergent-error-box {
    padding: 10px 12px;
    background-color: var(--danger-bg, rgba(239, 68, 68, 0.1));
    border: 1px solid var(--danger-border, rgba(239, 68, 68, 0.3));
    border-radius: var(--radius-sm, 4px);
    color: var(--danger-solid-hover, #f85149);
    font-size: 12px;
    line-height: 1.4;
  }

  .divergent-error-section {
    border-top: 1px solid var(--border-muted);
    padding-top: 10px;
  }

  .divergent-error-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .divergent-toggle-logs-btn,
  .divergent-copy-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    background: none;
    border: none;
    padding: 4px 6px;
    font-size: 11px;
    color: var(--text-muted);
    cursor: pointer;
    border-radius: var(--radius-sm, 4px);
    transition: color 0.15s ease;
  }

  .divergent-toggle-logs-btn:hover,
  .divergent-copy-btn:hover {
    color: var(--text-primary);
    background-color: var(--bg-canvas);
  }

  .divergent-raw-log-block {
    margin: 8px 0 0;
    padding: 10px;
    background-color: var(--bg-canvas);
    border: 1px solid var(--border-muted);
    border-radius: var(--radius-sm, 4px);
    font-family: var(--font-mono);
    font-size: 11px;
    line-height: 1.45;
    color: var(--text-secondary);
    white-space: pre-wrap;
    word-break: break-all;
    max-height: 140px;
    overflow-y: auto;
  }

  .divergent-footer {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: 8px;
    padding: 12px 20px;
    border-top: 1px solid var(--border-muted);
    background-color: var(--bg-surface);
  }

  .divergent-action-btn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }

  .divergent-spinner {
    width: 12px;
    height: 12px;
    border: 2px solid rgba(255, 255, 255, 0.3);
    border-top-color: #fff;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    display: inline-block;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
