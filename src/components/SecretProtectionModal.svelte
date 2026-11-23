<script lang="ts">
  import { t } from "../i18n/index.svelte";
  import { copyTextToClipboard } from "../lib/contextMenu.svelte";
  import { openExternalUrl, pathBaseName } from "../utils/path";

  interface Props {
    open: boolean;
    repo: string;
    errorMessage: string;
    bypassUrl?: string | null;
    onClose: () => void;
    onRetry?: () => Promise<void>;
  }

  let {
    open,
    repo,
    errorMessage,
    bypassUrl = null,
    onClose,
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

  async function handleOpenBypassUrl() {
    if (!bypassUrl) return;
    await openExternalUrl(bypassUrl);
  }

  async function handleRetry() {
    if (!onRetry || isRetrying) return;
    isRetrying = true;
    try {
      await onRetry();
      onClose();
    } catch {
      // Handled by parent
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
      class="modal-dialog secret-modal-dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="secret-modal-title"
      tabindex="-1"
    >
      <!-- Header -->
      <div class="modal-header secret-modal-header">
        <div class="secret-header-title-wrap">
          <div class="secret-header-icon-box">
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
              <rect width="18" height="11" x="3" y="11" rx="2" ry="2" />
              <path d="M7 11V7a5 5 0 0 1 10 0v4" />
            </svg>
          </div>
          <div>
            <div style="display: flex; align-items: center; gap: 8px;">
              <h2 id="secret-modal-title" class="secret-title">{t("secretModal.title")}</h2>
              {#if repo}
                <span class="secret-repo-badge">{pathBaseName(repo)}</span>
              {/if}
            </div>
            <p class="secret-subtitle">{t("secretModal.subtitle")}</p>
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
      <div class="modal-body secret-modal-body">
        <!-- Explanation Banner -->
        <div class="secret-callout">
          <p class="secret-explanation-text">
            {t("secretModal.explanation")}
          </p>
          {#if bypassUrl}
            <div class="secret-bypass-box">
              <span class="secret-bypass-label">{t("secretModal.bypassHint")}</span>
              <div class="secret-bypass-url-row">
                <code class="secret-bypass-code">{bypassUrl}</code>
                <button
                  type="button"
                  class="btn-secondary btn-xs"
                  onclick={handleOpenBypassUrl}
                  title={t("secretModal.openBypassBtn")}
                >
                  <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true" style="margin-right: 3px;">
                    <path d="M3.75 2h3.5a.75.75 0 0 1 0 1.5h-3.5a.25.25 0 0 0-.25.25v8.5c0 .138.112.25.25.25h8.5a.25.25 0 0 0 .25-.25v-3.5a.75.75 0 0 1 1.5 0v3.5A1.75 1.75 0 0 1 12.25 14h-8.5A1.75 1.75 0 0 1 2 12.25v-8.5C2 2.784 2.784 2 3.75 2Zm6.5.75a.75.75 0 0 1 .75-.75h3.25c.414 0 .75.336.75.75v3.25a.75.75 0 0 1-1.5 0V4.56l-4.22 4.22a.751.751 0 0 1-1.042-.018.751.751 0 0 1-.018-1.042L12.44 3.5h-1.44a.75.75 0 0 1-.75-.75Z" />
                  </svg>
                  {t("secretModal.openBypassBtn")}
                </button>
              </div>
            </div>
          {/if}
        </div>

        <!-- How to fix card -->
        <div class="secret-steps-card">
          <h3 class="secret-steps-title">{t("secretModal.howToFixTitle")}</h3>
          <ul class="secret-steps-list">
            <li>
              <span>{t("secretModal.option1")}</span>
            </li>
            <li>
              <span>{t("secretModal.option2")}</span>
            </li>
          </ul>
        </div>

        <!-- Error Log Accordion -->
        <div class="secret-error-section">
          <div class="secret-error-header">
            <button
              type="button"
              class="secret-toggle-logs-btn"
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
            <button type="button" class="secret-copy-btn" onclick={handleCopyError}>
              <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
                <path d="M0 6.75C0 5.784.784 5 1.75 5h1.5a.75.75 0 0 1 0 1.5h-1.5a.25.25 0 0 0-.25.25v7.5c0 .138.112.25.25.25h7.5a.25.25 0 0 0 .25-.25v-1.5a.75.75 0 0 1 1.5 0v1.5A1.75 1.75 0 0 1 9.25 16h-7.5A1.75 1.75 0 0 1 0 14.25Z" />
                <path d="M5 1.75C5 .784 5.784 0 6.75 0h7.5C15.216 0 16 .784 16 1.75v7.5A1.75 1.75 0 0 1 14.25 11h-7.5A1.75 1.75 0 0 1 5 9.25Zm1.75-.25a.25.25 0 0 0-.25.25v7.5c0 .138.112.25.25.25h7.5a.25.25 0 0 0 .25-.25v-7.5a.25.25 0 0 0-.25-.25Z" />
              </svg>
              {copied ? t("secretModal.copied") : t("secretModal.copyError")}
            </button>
          </div>
          {#if showRawLogs}
            <pre class="secret-raw-log-block">{errorMessage}</pre>
          {/if}
        </div>
      </div>

      <!-- Footer Actions -->
      <div class="modal-footer secret-modal-footer">
        <button
          type="button"
          class="btn-secondary btn-sm"
          onclick={onClose}
          disabled={isRetrying}
        >
          {t("common.close")}
        </button>

        {#if bypassUrl}
          <button
            type="button"
            class="btn-secondary btn-sm"
            onclick={handleOpenBypassUrl}
            disabled={isRetrying}
          >
            <svg width="13" height="13" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true" style="margin-right: 4px;">
              <path d="M3.75 2h3.5a.75.75 0 0 1 0 1.5h-3.5a.25.25 0 0 0-.25.25v8.5c0 .138.112.25.25.25h8.5a.25.25 0 0 0 .25-.25v-3.5a.75.75 0 0 1 1.5 0v3.5A1.75 1.75 0 0 1 12.25 14h-8.5A1.75 1.75 0 0 1 2 12.25v-8.5C2 2.784 2.784 2 3.75 2Zm6.5.75a.75.75 0 0 1 .75-.75h3.25c.414 0 .75.336.75.75v3.25a.75.75 0 0 1-1.5 0V4.56l-4.22 4.22a.751.751 0 0 1-1.042-.018.751.751 0 0 1-.018-1.042L12.44 3.5h-1.44a.75.75 0 0 1-.75-.75Z" />
            </svg>
            {t("secretModal.openBypassBtn")}
          </button>
        {/if}

        {#if onRetry}
          <button
            type="button"
            class="btn-primary btn-sm secret-confirm-btn"
            onclick={handleRetry}
            disabled={isRetrying}
          >
            {#if isRetrying}
              <span class="secret-spinner" aria-hidden="true"></span>
              {t("workflowModal.retrying")}
            {:else}
              {t("secretModal.retryPush")}
            {/if}
          </button>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .secret-modal-dialog {
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

  .secret-modal-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    padding: 16px 20px 14px;
    border-bottom: 1px solid var(--border-muted);
    background-color: var(--bg-surface);
  }

  .secret-header-title-wrap {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .secret-header-icon-box {
    width: 36px;
    height: 36px;
    border-radius: var(--radius-md, 6px);
    background-color: var(--danger-bg, rgba(239, 68, 68, 0.12));
    border: 1px solid var(--danger-border, rgba(239, 68, 68, 0.3));
    color: var(--danger-solid-hover, #f85149);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .secret-title {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .secret-repo-badge {
    font-size: 11px;
    padding: 1px 6px;
    border-radius: var(--radius-sm, 4px);
    background-color: var(--bg-canvas);
    border: 1px solid var(--border-muted);
    color: var(--text-muted);
    font-family: var(--font-mono);
  }

  .secret-subtitle {
    margin: 2px 0 0;
    font-size: 12px;
    color: var(--text-secondary);
    line-height: 1.4;
  }

  .secret-modal-body {
    padding: 16px 20px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    overflow-y: auto;
    max-height: 70vh;
  }

  .secret-callout {
    padding: 12px 14px;
    border-radius: var(--radius-md, 6px);
    background-color: var(--bg-surface);
    border: 1px solid var(--border-default);
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .secret-explanation-text {
    margin: 0;
    font-size: 12.5px;
    line-height: 1.5;
    color: var(--text-secondary);
  }

  .secret-bypass-box {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding-top: 6px;
    border-top: 1px solid var(--border-muted);
  }

  .secret-bypass-label {
    font-size: 11.5px;
    font-weight: 600;
    color: var(--text-muted);
  }

  .secret-bypass-url-row {
    display: flex;
    align-items: center;
    gap: 8px;
    justify-content: space-between;
  }

  .secret-bypass-code {
    flex: 1;
    padding: 4px 8px;
    border-radius: var(--radius-sm, 4px);
    background-color: var(--bg-canvas);
    border: 1px solid var(--border-muted);
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--accent-blue, #58a6ff);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .secret-steps-card {
    padding: 14px 16px;
    border-radius: var(--radius-md, 6px);
    background-color: var(--bg-canvas);
    border: 1px solid var(--border-default);
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .secret-steps-title {
    margin: 0;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .secret-steps-list {
    margin: 0;
    padding-left: 18px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    font-size: 12px;
    color: var(--text-secondary);
    line-height: 1.45;
  }

  .secret-error-section {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: 4px;
  }

  .secret-error-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .secret-toggle-logs-btn,
  .secret-copy-btn {
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

  .secret-toggle-logs-btn:hover,
  .secret-copy-btn:hover {
    background-color: var(--bg-hover);
    color: var(--text-primary);
  }

  .secret-raw-log-block {
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

  .secret-modal-footer {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: 8px;
    border-top: 1px solid var(--border-muted);
    padding: 12px 20px;
    background-color: var(--bg-surface);
  }

  .secret-confirm-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .secret-spinner {
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
