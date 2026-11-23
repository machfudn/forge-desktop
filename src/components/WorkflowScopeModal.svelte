<script lang="ts">
  import { t } from "../i18n/index.svelte";
  import { copyTextToClipboard } from "../lib/contextMenu.svelte";
  import { openExternalUrl, pathBaseName } from "../utils/path";

  interface Props {
    open: boolean;
    repo: string;
    errorMessage: string;
    workflowFile?: string | null;
    accountLogin?: string | null;
    onClose: () => void;
    onOpenAuth?: () => void;
    onRetry?: () => Promise<void>;
  }

  let {
    open,
    repo,
    errorMessage,
    workflowFile = null,
    accountLogin = null,
    onClose,
    onOpenAuth,
    onRetry,
  }: Props = $props();

  let copied = $state(false);
  let showRawLogs = $state(false);
  let isRetrying = $state(false);

  async function handleCopyError() {
    await copyTextToClipboard(errorMessage);
    copied = true;
    setTimeout(() => {
      copied = false;
    }, 2000);
  }

  async function handleOpenGithubTokens() {
    await openExternalUrl("https://github.com/settings/tokens");
  }

  async function handleRetry() {
    if (!onRetry || isRetrying) return;
    isRetrying = true;
    try {
      await onRetry();
      onClose();
    } catch {
      // Error handled by parent handler
    } finally {
      isRetrying = false;
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.stopPropagation();
      onClose();
    }
  }
</script>

{#if open}
  <div
    class="modal-overlay"
    role="presentation"
    onclick={(e) => {
      if (e.target === e.currentTarget && !isRetrying) onClose();
    }}
    onkeydown={handleKeyDown}
  >
    <div
      class="modal-dialog workflow-modal-dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="workflow-modal-title"
      tabindex="-1"
    >
      <!-- Header -->
      <div class="modal-header workflow-modal-header">
        <div class="workflow-header-title-wrap">
          <div class="workflow-header-icon-box">
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
              <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
              <line x1="12" y1="8" x2="12" y2="12" />
              <line x1="12" y1="16" x2="12.01" y2="16" />
            </svg>
          </div>
          <div>
            <div style="display: flex; align-items: center; gap: 8px;">
              <h2 id="workflow-modal-title" class="workflow-title">{t("workflowModal.title")}</h2>
              {#if repo}
                <span class="workflow-repo-badge">{pathBaseName(repo)}</span>
              {/if}
            </div>
            <p class="workflow-subtitle">{t("workflowModal.subtitle")}</p>
          </div>
        </div>
        <button
          type="button"
          class="icon-btn compact"
          onclick={onClose}
          disabled={isRetrying}
          aria-label={t("common.closeDialog")}
        >
          <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
            <path d="M3.72 3.72a.75.75 0 0 1 1.06 0L8 6.94l3.22-3.22a.749.749 0 0 1 1.275.326.749.749 0 0 1-.215.734L9.06 8l3.22 3.22a.749.749 0 0 1-.326 1.275.749.749 0 0 1-.734-.215L8 9.06l-3.22 3.22a.751.751 0 0 1-1.042-.018.751.751 0 0 1-.018-1.042L6.94 8 3.72 4.78a.75.75 0 0 1 0-1.06Z" />
          </svg>
        </button>
      </div>

      <!-- Body -->
      <div class="modal-body workflow-modal-body">
        <!-- Explanation Banner -->
        <div class="workflow-callout">
          <p class="workflow-explanation-text">
            {t("workflowModal.explanation")}
          </p>
          {#if workflowFile}
            <div class="workflow-file-row">
              <span class="workflow-file-label">{t("workflowModal.detectedFile")}</span>
              <code class="workflow-file-tag">{workflowFile}</code>
            </div>
          {/if}
        </div>

        <!-- Account Badge -->
        {#if accountLogin}
          <div class="workflow-account-row">
            <span class="workflow-account-pill">
              <svg width="13" height="13" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
                <path d="M8 0a8 8 0 0 0-2.5 15.6c.4.1.5-.2.5-.4v-1.5c-2.2.5-2.7-1.1-2.7-1.1-.4-.9-.9-1.2-.9-1.2-.7-.5.1-.5.1-.5.8.1 1.2.8 1.2.8.7 1.3 1.9.9 2.3.7.1-.5.3-.9.5-1.1-1.8-.2-3.6-.9-3.6-4 0-.9.3-1.6.8-2.1-.1-.2-.4-1 .1-2.1 0 0 .7-.2 2.2.8.6-.2 1.3-.3 2-.3.7 0 1.4.1 2 .3 1.5-1 2.2-.8 2.2-.8.5 1.1.2 1.9.1 2.1.5.6.8 1.3.8 2.1 0 3.1-1.9 3.8-3.7 4 .3.3.6.8.6 1.5v2.2c0 .2.1.5.5.4A8 8 0 0 0 8 0Z" />
              </svg>
              {t("workflowModal.connectedAccount", { login: accountLogin })}
            </span>
          </div>
        {/if}

        <!-- Steps Card -->
        <div class="workflow-steps-card">
          <h3 class="workflow-steps-title">{t("workflowModal.stepsTitle")}</h3>
          <ol class="workflow-steps-list">
            <li>
              <div class="step-content">
                <strong>{t("workflowModal.step1")}</strong>
              </div>
            </li>
            <li>
              <div class="step-content">
                <span>{t("workflowModal.step2Classic")}</span>
                <span class="step-alt-text">{t("workflowModal.step2FineGrained")}</span>
              </div>
            </li>
            <li>
              <div class="step-content">
                <span>{t("workflowModal.step3")}</span>
              </div>
            </li>
          </ol>
        </div>

        <!-- Error Log Accordion -->
        <div class="workflow-error-section">
          <div class="workflow-error-header">
            <button
              type="button"
              class="workflow-toggle-logs-btn"
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
            <button type="button" class="workflow-copy-btn" onclick={handleCopyError}>
              <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
                <path d="M0 6.75C0 5.784.784 5 1.75 5h1.5a.75.75 0 0 1 0 1.5h-1.5a.25.25 0 0 0-.25.25v7.5c0 .138.112.25.25.25h7.5a.25.25 0 0 0 .25-.25v-1.5a.75.75 0 0 1 1.5 0v1.5A1.75 1.75 0 0 1 9.25 16h-7.5A1.75 1.75 0 0 1 0 14.25Z" />
                <path d="M5 1.75C5 .784 5.784 0 6.75 0h7.5C15.216 0 16 .784 16 1.75v7.5A1.75 1.75 0 0 1 14.25 11h-7.5A1.75 1.75 0 0 1 5 9.25Zm1.75-.25a.25.25 0 0 0-.25.25v7.5c0 .138.112.25.25.25h7.5a.25.25 0 0 0 .25-.25v-7.5a.25.25 0 0 0-.25-.25Z" />
              </svg>
              {copied ? t("workflowModal.copied") : t("workflowModal.copyError")}
            </button>
          </div>
          {#if showRawLogs}
            <pre class="workflow-raw-log-block">{errorMessage}</pre>
          {/if}
        </div>
      </div>

      <!-- Footer Actions -->
      <div class="modal-footer workflow-modal-footer">
        <button
          type="button"
          class="btn-secondary btn-sm"
          onclick={onClose}
          disabled={isRetrying}
        >
          {t("common.close")}
        </button>

        {#if onOpenAuth}
          <button
            type="button"
            class="btn-secondary btn-sm"
            onclick={onOpenAuth}
            disabled={isRetrying}
          >
            {t("workflowModal.openAccounts")}
          </button>
        {/if}

        <button
          type="button"
          class="btn-secondary btn-sm"
          onclick={handleOpenGithubTokens}
          disabled={isRetrying}
        >
          <svg width="13" height="13" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true" style="margin-right: 4px;">
            <path d="M3.75 2h3.5a.75.75 0 0 1 0 1.5h-3.5a.25.25 0 0 0-.25.25v8.5c0 .138.112.25.25.25h8.5a.25.25 0 0 0 .25-.25v-3.5a.75.75 0 0 1 1.5 0v3.5A1.75 1.75 0 0 1 12.25 14h-8.5A1.75 1.75 0 0 1 2 12.25v-8.5C2 2.784 2.784 2 3.75 2Zm6.5.75a.75.75 0 0 1 .75-.75h3.25c.414 0 .75.336.75.75v3.25a.75.75 0 0 1-1.5 0V4.56l-4.22 4.22a.751.751 0 0 1-1.042-.018.751.751 0 0 1-.018-1.042L12.44 3.5h-1.44a.75.75 0 0 1-.75-.75Z" />
          </svg>
          {t("workflowModal.openGithubTokens")}
        </button>

        {#if onRetry}
          <button
            type="button"
            class="btn-primary btn-sm workflow-confirm-btn"
            onclick={handleRetry}
            disabled={isRetrying}
          >
            {#if isRetrying}
              <span class="workflow-spinner" aria-hidden="true"></span>
              {t("workflowModal.retrying")}
            {:else}
              {t("workflowModal.retryPush")}
            {/if}
          </button>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .workflow-modal-dialog {
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

  .workflow-modal-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    padding: 16px 20px 14px;
    border-bottom: 1px solid var(--border-muted);
    background-color: var(--bg-surface);
  }

  .workflow-header-title-wrap {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .workflow-header-icon-box {
    width: 36px;
    height: 36px;
    border-radius: var(--radius-md, 6px);
    background-color: var(--warn-bg, rgba(217, 119, 6, 0.12));
    border: 1px solid var(--warn-border, rgba(217, 119, 6, 0.3));
    color: var(--warn-text, #d97706);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .workflow-title {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .workflow-repo-badge {
    font-size: 11px;
    padding: 1px 6px;
    border-radius: var(--radius-sm, 4px);
    background-color: var(--bg-canvas);
    border: 1px solid var(--border-muted);
    color: var(--text-muted);
    font-family: var(--font-mono);
  }

  .workflow-subtitle {
    margin: 2px 0 0;
    font-size: 12px;
    color: var(--text-secondary);
    line-height: 1.4;
  }

  .workflow-modal-body {
    padding: 16px 20px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    overflow-y: auto;
    max-height: 70vh;
  }

  .workflow-callout {
    padding: 12px 14px;
    border-radius: var(--radius-md, 6px);
    background-color: var(--bg-surface);
    border: 1px solid var(--border-default);
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .workflow-explanation-text {
    margin: 0;
    font-size: 12.5px;
    line-height: 1.5;
    color: var(--text-secondary);
  }

  .workflow-file-row {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
    font-size: 12px;
  }

  .workflow-file-label {
    color: var(--text-muted);
    font-weight: 500;
  }

  .workflow-file-tag {
    padding: 2px 7px;
    border-radius: var(--radius-sm, 4px);
    background-color: var(--bg-canvas);
    border: 1px solid var(--border-muted);
    font-family: var(--font-mono);
    font-size: 11.5px;
    color: var(--accent-blue, #58a6ff);
  }

  .workflow-account-row {
    display: flex;
    align-items: center;
  }

  .workflow-account-pill {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    border-radius: 999px;
    background-color: var(--bg-surface);
    border: 1px solid var(--border-muted);
    font-size: 11.5px;
    color: var(--text-primary);
  }

  .workflow-steps-card {
    padding: 14px 16px;
    border-radius: var(--radius-md, 6px);
    background-color: var(--bg-canvas);
    border: 1px solid var(--border-default);
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .workflow-steps-title {
    margin: 0;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .workflow-steps-list {
    margin: 0;
    padding-left: 20px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    font-size: 12px;
    color: var(--text-secondary);
  }

  .step-content {
    display: flex;
    flex-direction: column;
    gap: 3px;
    line-height: 1.45;
  }

  .step-alt-text {
    font-size: 11px;
    color: var(--text-muted);
  }

  .workflow-error-section {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: 4px;
  }

  .workflow-error-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .workflow-toggle-logs-btn,
  .workflow-copy-btn {
    background: transparent;
    border: none;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 11.5px;
    color: var(--text-secondary);
    padding: 3px 6px;
    border-radius: var(--radius-sm, 4px);
    transition: background-color 0.15s ease, color 0.15s ease;
  }

  .workflow-toggle-logs-btn:hover,
  .workflow-copy-btn:hover {
    background-color: var(--bg-hover);
    color: var(--text-primary);
  }

  .workflow-raw-log-block {
    margin: 0;
    padding: 8px 10px;
    background-color: var(--bg-canvas);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-sm, 4px);
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--tok-keyword, #ff7b72);
    max-height: 120px;
    overflow-y: auto;
    white-space: pre-wrap;
    word-break: break-all;
  }

  .workflow-modal-footer {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: 8px;
    border-top: 1px solid var(--border-muted);
    padding: 12px 20px;
    background-color: var(--bg-surface);
  }

  .workflow-confirm-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .workflow-spinner {
    width: 12px;
    height: 12px;
    border: 2px solid rgba(255, 255, 255, 0.3);
    border-top-color: #ffffff;
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
