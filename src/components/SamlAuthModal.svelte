<script lang="ts">
  import { t } from "../i18n/index.svelte";
  import { copyTextToClipboard } from "../lib/contextMenu.svelte";
  import { openExternalUrl, pathBaseName } from "../utils/path";

  interface Props {
    open: boolean;
    repo: string;
    accountLogin?: string | null;
    errorMessage: string;
    onClose: () => void;
    onRetry?: () => Promise<void>;
  }

  let {
    open,
    repo,
    accountLogin = null,
    errorMessage,
    onClose,
    onRetry,
  }: Props = $props();

  let isRetrying = $state(false);
  let showRawLogs = $state(false);
  let copied = $state(false);

  async function handleCopyError() {
    await copyTextToClipboard(errorMessage);
    copied = true;
    setTimeout(() => {
      copied = false;
    }, 2000);
  }

  async function handleOpenTokenSettings() {
    await openExternalUrl("https://github.com/settings/tokens");
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
      if (!isRetrying) {
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
      if (e.target === e.currentTarget && !isRetrying) onClose();
    }}
    onkeydown={handleKeyDown}
  >
    <div
      class="modal-dialog saml-modal-dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="saml-modal-title"
      tabindex="-1"
    >
      <!-- Header -->
      <div class="modal-header saml-modal-header">
        <div class="saml-title-wrap">
          <div class="saml-icon-box">
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
              <h2 id="saml-modal-title" class="saml-title">
                {t("samlModal.title")}
              </h2>
              {#if repo}
                <span class="saml-repo-badge">{pathBaseName(repo)}</span>
              {/if}
            </div>
            <p class="saml-subtitle">
              {t("samlModal.subtitle")}
            </p>
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
      <div class="modal-body saml-modal-body">
        <!-- Callout Explanation -->
        <div class="saml-callout">
          <p class="saml-explanation">
            {t("samlModal.explanation")}
          </p>
        </div>

        {#if accountLogin}
          <div class="saml-account-banner">
            <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true" style="color: var(--text-muted); flex-shrink: 0;">
              <path d="M10.561 8.073a6.005 6.005 0 0 1 3.432 5.142.75.75 0 1 1-1.498.07 4.5 4.5 0 0 0-8.99 0 .75.75 0 0 1-1.498-.07 6.004 6.004 0 0 1 3.431-5.142 3.999 3.999 0 1 1 5.123 0ZM10.5 5a2.5 2.5 0 1 0-5 0 2.5 2.5 0 0 0 5 0Z"/>
            </svg>
            <span>{t("workflowModal.connectedAccount", { login: accountLogin })}</span>
          </div>
        {/if}

        <!-- Steps Card -->
        <div class="saml-steps-card">
          <div class="saml-steps-title">{t("samlModal.stepsTitle")}</div>
          <ol class="saml-steps-list">
            <li>{t("samlModal.step1")}</li>
            <li>{t("samlModal.step2")}</li>
            <li>{t("samlModal.step3")}</li>
          </ol>
        </div>

        <!-- Raw Error Log Accordion -->
        <div class="saml-error-section">
          <div class="saml-error-header">
            <button
              type="button"
              class="saml-toggle-logs-btn"
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
            <button type="button" class="saml-copy-btn" onclick={handleCopyError}>
              <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
                <path d="M0 6.75C0 5.784.784 5 1.75 5h1.5a.75.75 0 0 1 0 1.5h-1.5a.25.25 0 0 0-.25.25v7.5c0 .138.112.25.25.25h7.5a.25.25 0 0 0 .25-.25v-1.5a.75.75 0 0 1 1.5 0v1.5A1.75 1.75 0 0 1 9.25 16h-7.5A1.75 1.75 0 0 1 0 14.25Z" />
                <path d="M5 1.75C5 .784 5.784 0 6.75 0h7.5C15.216 0 16 .784 16 1.75v7.5A1.75 1.75 0 0 1 14.25 11h-7.5A1.75 1.75 0 0 1 5 9.25Zm1.75-.25a.25.25 0 0 0-.25.25v7.5c0 .138.112.25.25.25h7.5a.25.25 0 0 0 .25-.25v-7.5a.25.25 0 0 0-.25-.25Z" />
              </svg>
              {copied ? t("secretModal.copied") : t("secretModal.copyError")}
            </button>
          </div>
          {#if showRawLogs}
            <pre class="saml-raw-log-block">{errorMessage}</pre>
          {/if}
        </div>
      </div>

      <!-- Footer Actions -->
      <div class="modal-footer saml-footer">
        <button
          type="button"
          class="btn-secondary btn-sm"
          onclick={onClose}
          disabled={isRetrying}
        >
          {t("common.close")}
        </button>

        <button
          type="button"
          class="btn-secondary btn-sm"
          onclick={handleOpenTokenSettings}
          disabled={isRetrying}
        >
          <svg width="13" height="13" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true" style="margin-right: 4px;">
            <path d="M3.75 2h3.5a.75.75 0 0 1 0 1.5h-3.5a.25.25 0 0 0-.25.25v8.5c0 .138.112.25.25.25h8.5a.25.25 0 0 0 .25-.25v-3.5a.75.75 0 0 1 1.5 0v3.5A1.75 1.75 0 0 1 12.25 14h-8.5A1.75 1.75 0 0 1 2 12.25v-8.5C2 2.784 2.784 2 3.75 2Zm6.5.75a.75.75 0 0 1 .75-.75h3.25c.414 0 .75.336.75.75v3.25a.75.75 0 0 1-1.5 0V4.56l-4.22 4.22a.751.751 0 0 1-1.042-.018.751.751 0 0 1-.018-1.042L12.44 3.5h-1.44a.75.75 0 0 1-.75-.75Z" />
          </svg>
          {t("samlModal.openSettingsBtn")}
        </button>

        {#if onRetry}
          <button
            type="button"
            class="btn-primary btn-sm saml-confirm-btn"
            onclick={handleRetry}
            disabled={isRetrying}
          >
            {#if isRetrying}
              <span class="saml-spinner" aria-hidden="true"></span>
              {t("workflowModal.retrying")}
            {:else}
              {t("samlModal.retryBtn")}
            {/if}
          </button>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .saml-modal-dialog {
    width: 92vw;
    max-width: 560px;
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

  .saml-modal-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    padding: 16px 20px 14px;
    border-bottom: 1px solid var(--border-muted);
    background-color: var(--bg-surface);
  }

  .saml-title-wrap {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .saml-icon-box {
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

  .saml-title {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .saml-repo-badge {
    font-size: 11px;
    padding: 1px 6px;
    border-radius: var(--radius-sm, 4px);
    background-color: var(--bg-canvas);
    border: 1px solid var(--border-muted);
    color: var(--text-muted);
    font-family: var(--font-mono);
  }

  .saml-subtitle {
    margin: 2px 0 0;
    font-size: 12px;
    color: var(--text-secondary);
    line-height: 1.4;
  }

  .saml-modal-body {
    padding: 16px 20px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    background-color: var(--bg-panel);
    max-height: 70vh;
    overflow-y: auto;
  }

  .saml-callout {
    padding: 12px 14px;
    background-color: var(--bg-surface);
    border: 1px solid var(--border-muted);
    border-left: 3px solid var(--accent-solid, #6366f1);
    border-radius: var(--radius-sm, 4px);
  }

  .saml-explanation {
    margin: 0;
    font-size: 13px;
    line-height: 1.5;
    color: var(--text-primary);
  }

  .saml-account-banner {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    background-color: var(--bg-canvas);
    border: 1px solid var(--border-muted);
    border-radius: var(--radius-sm, 4px);
    font-size: 12px;
    color: var(--text-secondary);
  }

  .saml-steps-card {
    padding: 12px 14px;
    background-color: var(--bg-surface);
    border: 1px solid var(--border-muted);
    border-radius: var(--radius-md, 6px);
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .saml-steps-title {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .saml-steps-list {
    margin: 0;
    padding-left: 18px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 12px;
    color: var(--text-secondary);
    line-height: 1.45;
  }

  .saml-error-section {
    border-top: 1px solid var(--border-muted);
    padding-top: 10px;
  }

  .saml-error-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .saml-toggle-logs-btn,
  .saml-copy-btn {
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

  .saml-toggle-logs-btn:hover,
  .saml-copy-btn:hover {
    color: var(--text-primary);
    background-color: var(--bg-canvas);
  }

  .saml-raw-log-block {
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

  .saml-footer {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: 8px;
    padding: 12px 20px;
    border-top: 1px solid var(--border-muted);
    background-color: var(--bg-surface);
  }

  .saml-confirm-btn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }

  .saml-spinner {
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
