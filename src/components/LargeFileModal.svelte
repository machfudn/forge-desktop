<script lang="ts">
  import { t } from "../i18n/index.svelte";
  import { copyTextToClipboard } from "../lib/contextMenu.svelte";
  import { openExternalUrl, pathBaseName } from "../utils/path";

  interface Props {
    open: boolean;
    repo: string;
    fileName?: string | null;
    fileSize?: string | null;
    errorMessage: string;
    onClose: () => void;
    onSoftReset?: () => Promise<void>;
  }

  let {
    open,
    repo,
    fileName = null,
    fileSize = null,
    errorMessage,
    onClose,
    onSoftReset,
  }: Props = $props();

  let isResetting = $state(false);
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

  async function handleSoftReset() {
    if (!onSoftReset || isResetting) return;
    isResetting = true;
    actionError = "";
    try {
      await onSoftReset();
      onClose();
    } catch (err) {
      actionError = String(err);
    } finally {
      isResetting = false;
    }
  }

  async function handleOpenLfsDocs() {
    await openExternalUrl("https://git-lfs.com");
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      if (!isResetting) {
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
      if (e.target === e.currentTarget && !isResetting) onClose();
    }}
    onkeydown={handleKeyDown}
  >
    <div
      class="modal-dialog large-file-dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="large-file-title"
      tabindex="-1"
    >
      <!-- Header -->
      <div class="modal-header large-file-header">
        <div class="large-file-title-wrap">
          <div class="large-file-icon-box">
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
              <path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z" />
              <polyline points="14 2 14 8 20 8" />
              <line x1="12" y1="18" x2="12" y2="12" />
              <line x1="9" y1="15" x2="15" y2="15" />
            </svg>
          </div>
          <div>
            <div style="display: flex; align-items: center; gap: 8px;">
              <h2 id="large-file-title" class="large-file-title">
                {t("largeFileModal.title")}
              </h2>
              {#if repo}
                <span class="large-file-repo-badge">{pathBaseName(repo)}</span>
              {/if}
            </div>
            <p class="large-file-subtitle">
              {t("largeFileModal.subtitle")}
            </p>
          </div>
        </div>
        <button
          type="button"
          class="icon-btn compact"
          onclick={onClose}
          disabled={isResetting}
          aria-label={t("common.closeDialog")}
        >
          <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
            <path d="M3.72 3.72a.75.75 0 0 1 1.06 0L8 6.94l3.22-3.22a.749.749 0 0 1 1.275.326.749.749 0 0 1-.215.734L9.06 8l3.22 3.22a.749.749 0 0 1-.326 1.275.749.749 0 0 1-.734-.215L8 9.06l-3.22 3.22a.751.751 0 0 1-1.042-.018.751.751 0 0 1-.018-1.042L6.94 8 3.72 4.78a.75.75 0 0 1 0-1.06Z" />
          </svg>
        </button>
      </div>

      <!-- Body -->
      <div class="modal-body large-file-body">
        <!-- Callout Explanation -->
        <div class="large-file-callout">
          <p class="large-file-explanation">
            {t("largeFileModal.explanation")}
          </p>
        </div>

        <!-- Detected File Card -->
        {#if fileName}
          <div class="large-file-card">
            <div class="large-file-card-label">{t("largeFileModal.detectedFileLabel")}</div>
            <div class="large-file-info-row">
              <div class="large-file-name-wrap">
                <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true" style="color: var(--text-muted); flex-shrink: 0;">
                  <path d="M2 1.75C2 .784 2.784 0 3.75 0h6.586c.464 0 .909.184 1.237.513l2.914 2.914c.329.328.513.773.513 1.237v9.586A1.75 1.75 0 0 1 13.25 16h-9.5A1.75 1.75 0 0 1 2 14.25Zm1.75-.25a.25.25 0 0 0-.25.25v12.5c0 .138.112.25.25.25h9.5a.25.25 0 0 0 .25-.25V6h-2.75A1.75 1.75 0 0 1 9 4.25V1.5Zm7.25.56V4.25c0 .138.112.25.25.25h2.69Z"/>
                </svg>
                <code class="large-file-path" title={fileName}>{fileName}</code>
              </div>
              {#if fileSize}
                <span class="large-file-size-badge">{fileSize}</span>
              {/if}
            </div>
          </div>
        {/if}

        <!-- Solutions Overview -->
        <div class="large-file-solutions-card">
          <div class="large-file-solutions-title">{t("largeFileModal.solutionsTitle")}</div>
          <div class="large-file-options-grid">
            <div class="large-file-option-item">
              <span class="large-file-option-tag tag-primary">{t("largeFileModal.option1Tag")}</span>
              <p class="large-file-option-text">
                {t("largeFileModal.option1Desc")}
              </p>
            </div>
            <div class="large-file-option-item">
              <span class="large-file-option-tag tag-secondary">{t("largeFileModal.option2Tag")}</span>
              <p class="large-file-option-text">
                {t("largeFileModal.option2Desc")}
              </p>
            </div>
          </div>
        </div>

        {#if actionError}
          <div class="large-file-error-box">
            {actionError}
          </div>
        {/if}

        <!-- Error Log Accordion -->
        <div class="large-file-error-section">
          <div class="large-file-error-header">
            <button
              type="button"
              class="large-file-toggle-logs-btn"
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
            <button type="button" class="large-file-copy-btn" onclick={handleCopyError}>
              <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
                <path d="M0 6.75C0 5.784.784 5 1.75 5h1.5a.75.75 0 0 1 0 1.5h-1.5a.25.25 0 0 0-.25.25v7.5c0 .138.112.25.25.25h7.5a.25.25 0 0 0 .25-.25v-1.5a.75.75 0 0 1 1.5 0v1.5A1.75 1.75 0 0 1 9.25 16h-7.5A1.75 1.75 0 0 1 0 14.25Z" />
                <path d="M5 1.75C5 .784 5.784 0 6.75 0h7.5C15.216 0 16 .784 16 1.75v7.5A1.75 1.75 0 0 1 14.25 11h-7.5A1.75 1.75 0 0 1 5 9.25Zm1.75-.25a.25.25 0 0 0-.25.25v7.5c0 .138.112.25.25.25h7.5a.25.25 0 0 0 .25-.25v-7.5a.25.25 0 0 0-.25-.25Z" />
              </svg>
              {copied ? t("secretModal.copied") : t("secretModal.copyError")}
            </button>
          </div>
          {#if showRawLogs}
            <pre class="large-file-raw-log-block">{errorMessage}</pre>
          {/if}
        </div>
      </div>

      <!-- Footer Actions: 2 Distinct Choice Buttons -->
      <div class="modal-footer large-file-footer">
        <button
          type="button"
          class="btn-secondary btn-sm"
          onclick={onClose}
          disabled={isResetting}
        >
          {t("common.close")}
        </button>

        <!-- Option 2 Button: Git LFS Guide -->
        <button
          type="button"
          class="btn-secondary btn-sm"
          onclick={handleOpenLfsDocs}
          disabled={isResetting}
          title={t("largeFileModal.openLfsTooltip")}
        >
          <svg width="13" height="13" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true" style="margin-right: 4px;">
            <path d="M3.75 2h3.5a.75.75 0 0 1 0 1.5h-3.5a.25.25 0 0 0-.25.25v8.5c0 .138.112.25.25.25h8.5a.25.25 0 0 0 .25-.25v-3.5a.75.75 0 0 1 1.5 0v3.5A1.75 1.75 0 0 1 12.25 14h-8.5A1.75 1.75 0 0 1 2 12.25v-8.5C2 2.784 2.784 2 3.75 2Zm6.5.75a.75.75 0 0 1 .75-.75h3.25c.414 0 .75.336.75.75v3.25a.75.75 0 0 1-1.5 0V4.56l-4.22 4.22a.751.751 0 0 1-1.042-.018.751.751 0 0 1-.018-1.042L12.44 3.5h-1.44a.75.75 0 0 1-.75-.75Z" />
          </svg>
          {t("largeFileModal.openLfsBtn")}
        </button>

        <!-- Option 1 Button: Soft Reset HEAD~1 -->
        {#if onSoftReset}
          <button
            type="button"
            class="btn-primary btn-sm large-file-reset-btn"
            onclick={handleSoftReset}
            disabled={isResetting}
            title={t("largeFileModal.softResetTooltip")}
          >
            {#if isResetting}
              <span class="large-file-spinner" aria-hidden="true"></span>
              {t("largeFileModal.resetting")}
            {:else}
              <svg width="13" height="13" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true" style="margin-right: 4px;">
                <path d="M3.38 8A4.5 4.5 0 0 1 7.92 4h.83l-1.47 1.47a.75.75 0 0 0 1.06 1.06l2.75-2.75a.75.75 0 0 0 0-1.06L8.34.47a.75.75 0 0 0-1.06 1.06L8.75 3h-.83A6 6 0 0 0 1.9 8a.75.75 0 0 0 1.48 0Z" />
              </svg>
              {t("largeFileModal.softResetBtn")}
            {/if}
          </button>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .large-file-dialog {
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

  .large-file-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    padding: 16px 20px 14px;
    border-bottom: 1px solid var(--border-muted);
    background-color: var(--bg-surface);
  }

  .large-file-title-wrap {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .large-file-icon-box {
    width: 36px;
    height: 36px;
    border-radius: var(--radius-md, 6px);
    background-color: var(--warning-bg, rgba(245, 158, 11, 0.12));
    border: 1px solid var(--warning-border, rgba(245, 158, 11, 0.3));
    color: var(--warning-solid, #f59e0b);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .large-file-title {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .large-file-repo-badge {
    font-size: 11px;
    padding: 1px 6px;
    border-radius: var(--radius-sm, 4px);
    background-color: var(--bg-canvas);
    border: 1px solid var(--border-muted);
    color: var(--text-muted);
    font-family: var(--font-mono);
  }

  .large-file-subtitle {
    margin: 2px 0 0;
    font-size: 12px;
    color: var(--text-secondary);
    line-height: 1.4;
  }

  .large-file-body {
    padding: 16px 20px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    background-color: var(--bg-panel);
    max-height: 70vh;
    overflow-y: auto;
  }

  .large-file-callout {
    padding: 12px 14px;
    background-color: var(--bg-surface);
    border: 1px solid var(--border-muted);
    border-left: 3px solid var(--warning-solid, #f59e0b);
    border-radius: var(--radius-sm, 4px);
  }

  .large-file-explanation {
    margin: 0;
    font-size: 13px;
    line-height: 1.5;
    color: var(--text-primary);
  }

  .large-file-card {
    padding: 12px 14px;
    background-color: var(--bg-canvas);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md, 6px);
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .large-file-card-label {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-muted);
  }

  .large-file-info-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }

  .large-file-name-wrap {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }

  .large-file-path {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--text-primary);
    background-color: var(--bg-surface);
    padding: 2px 6px;
    border-radius: var(--radius-sm, 4px);
    border: 1px solid var(--border-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .large-file-size-badge {
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 600;
    padding: 2px 8px;
    border-radius: var(--radius-sm, 4px);
    background-color: var(--danger-bg, rgba(239, 68, 68, 0.12));
    border: 1px solid var(--danger-border, rgba(239, 68, 68, 0.3));
    color: var(--danger-solid-hover, #f85149);
    flex-shrink: 0;
  }

  .large-file-solutions-card {
    padding: 12px 14px;
    background-color: var(--bg-surface);
    border: 1px solid var(--border-muted);
    border-radius: var(--radius-md, 6px);
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .large-file-solutions-title {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .large-file-options-grid {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .large-file-option-item {
    display: flex;
    align-items: flex-start;
    gap: 8px;
  }

  .large-file-option-tag {
    font-size: 10px;
    font-weight: 600;
    padding: 1px 6px;
    border-radius: var(--radius-sm, 4px);
    flex-shrink: 0;
    margin-top: 2px;
  }

  .tag-primary {
    background-color: var(--accent-subtle, rgba(99, 102, 241, 0.12));
    border: 1px solid var(--accent-border, rgba(99, 102, 241, 0.3));
    color: var(--accent-solid, #6366f1);
  }

  .tag-secondary {
    background-color: var(--bg-canvas);
    border: 1px solid var(--border-muted);
    color: var(--text-muted);
  }

  .large-file-option-text {
    margin: 0;
    font-size: 12px;
    line-height: 1.45;
    color: var(--text-secondary);
  }

  .large-file-error-box {
    padding: 10px 12px;
    background-color: var(--danger-bg, rgba(239, 68, 68, 0.1));
    border: 1px solid var(--danger-border, rgba(239, 68, 68, 0.3));
    border-radius: var(--radius-sm, 4px);
    color: var(--danger-solid-hover, #f85149);
    font-size: 12px;
    line-height: 1.4;
  }

  .large-file-error-section {
    border-top: 1px solid var(--border-muted);
    padding-top: 10px;
  }

  .large-file-error-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .large-file-toggle-logs-btn,
  .large-file-copy-btn {
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

  .large-file-toggle-logs-btn:hover,
  .large-file-copy-btn:hover {
    color: var(--text-primary);
    background-color: var(--bg-canvas);
  }

  .large-file-raw-log-block {
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

  .large-file-footer {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: 8px;
    padding: 12px 20px;
    border-top: 1px solid var(--border-muted);
    background-color: var(--bg-surface);
  }

  .large-file-reset-btn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }

  .large-file-spinner {
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
