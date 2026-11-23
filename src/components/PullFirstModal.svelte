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
    onPullAndRetry?: () => Promise<void>;
    onFetchOnly?: () => Promise<void>;
  }

  let {
    open,
    repo,
    branchName = "main",
    errorMessage,
    onClose,
    onPullAndRetry,
    onFetchOnly,
  }: Props = $props();

  let isPulling = $state(false);
  let isFetching = $state(false);
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

  async function handlePullAndRetry() {
    if (!onPullAndRetry || isPulling || isFetching) return;
    isPulling = true;
    actionError = "";
    try {
      await onPullAndRetry();
      onClose();
    } catch (err) {
      actionError = String(err);
    } finally {
      isPulling = false;
    }
  }

  async function handleFetchOnly() {
    if (!onFetchOnly || isPulling || isFetching) return;
    isFetching = true;
    actionError = "";
    try {
      await onFetchOnly();
      onClose();
    } catch (err) {
      actionError = String(err);
    } finally {
      isFetching = false;
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      if (!isPulling && !isFetching) {
        e.stopPropagation();
        onClose();
      }
    } else if (e.key === "Enter" && !e.shiftKey && !e.ctrlKey) {
      if (!isPulling && !isFetching && onPullAndRetry) {
        e.preventDefault();
        void handlePullAndRetry();
      }
    }
  }
</script>

{#if open}
  <div
    class="modal-overlay"
    role="presentation"
    onclick={(e) => {
      if (e.target === e.currentTarget && !isPulling && !isFetching) onClose();
    }}
    onkeydown={handleKeyDown}
  >
    <div
      class="modal-dialog pull-first-dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="pull-first-title"
      tabindex="-1"
    >
      <!-- Header -->
      <div class="modal-header pull-first-header">
        <div class="pull-first-title-wrap">
          <div class="pull-first-icon-box">
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
              <path d="M12 5v14" />
              <path d="m19 12-7 7-7-7" />
            </svg>
          </div>
          <div>
            <div style="display: flex; align-items: center; gap: 8px;">
              <h2 id="pull-first-title" class="pull-first-title">
                {t("pullFirstModal.title")}
              </h2>
              {#if repo}
                <span class="pull-first-repo-badge">{pathBaseName(repo)}</span>
              {/if}
            </div>
            <p class="pull-first-subtitle">
              {t("pullFirstModal.subtitle", { branch: branchName || "main" })}
            </p>
          </div>
        </div>
        <button
          type="button"
          class="icon-btn compact"
          onclick={onClose}
          disabled={isPulling || isFetching}
          aria-label={t("common.closeDialog")}
        >
          <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
            <path d="M3.72 3.72a.75.75 0 0 1 1.06 0L8 6.94l3.22-3.22a.749.749 0 0 1 1.275.326.749.749 0 0 1-.215.734L9.06 8l3.22 3.22a.749.749 0 0 1-.326 1.275.749.749 0 0 1-.734-.215L8 9.06l-3.22 3.22a.751.751 0 0 1-1.042-.018.751.751 0 0 1-.018-1.042L6.94 8 3.72 4.78a.75.75 0 0 1 0-1.06Z" />
          </svg>
        </button>
      </div>

      <!-- Body -->
      <div class="modal-body pull-first-body">
        <!-- Callout -->
        <div class="pull-first-callout">
          <p class="pull-first-explanation">
            {t("pullFirstModal.explanation")}
          </p>
        </div>

        {#if actionError}
          <div class="pull-first-error-box">
            {actionError}
          </div>
        {/if}

        <!-- Raw Error Log Accordion -->
        <div class="pull-first-error-section">
          <div class="pull-first-error-header">
            <button
              type="button"
              class="pull-first-toggle-logs-btn"
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
            <button type="button" class="pull-first-copy-btn" onclick={handleCopyError}>
              <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
                <path d="M0 6.75C0 5.784.784 5 1.75 5h1.5a.75.75 0 0 1 0 1.5h-1.5a.25.25 0 0 0-.25.25v7.5c0 .138.112.25.25.25h7.5a.25.25 0 0 0 .25-.25v-1.5a.75.75 0 0 1 1.5 0v1.5A1.75 1.75 0 0 1 9.25 16h-7.5A1.75 1.75 0 0 1 0 14.25Z" />
                <path d="M5 1.75C5 .784 5.784 0 6.75 0h7.5C15.216 0 16 .784 16 1.75v7.5A1.75 1.75 0 0 1 14.25 11h-7.5A1.75 1.75 0 0 1 5 9.25Zm1.75-.25a.25.25 0 0 0-.25.25v7.5c0 .138.112.25.25.25h7.5a.25.25 0 0 0 .25-.25v-7.5a.25.25 0 0 0-.25-.25Z" />
              </svg>
              {copied ? t("secretModal.copied") : t("secretModal.copyError")}
            </button>
          </div>
          {#if showRawLogs}
            <pre class="pull-first-raw-log-block">{errorMessage}</pre>
          {/if}
        </div>
      </div>

      <!-- Footer Actions -->
      <div class="modal-footer pull-first-footer">
        <button
          type="button"
          class="btn-secondary btn-sm"
          onclick={onClose}
          disabled={isPulling || isFetching}
        >
          {t("common.close")}
        </button>

        {#if onFetchOnly}
          <button
            type="button"
            class="btn-secondary btn-sm"
            onclick={handleFetchOnly}
            disabled={isPulling || isFetching}
            title={t("pullFirstModal.fetchTooltip")}
          >
            {#if isFetching}
              <span class="pull-first-spinner" aria-hidden="true"></span>
              {t("pullFirstModal.fetching")}
            {:else}
              <svg width="13" height="13" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true" style="margin-right: 4px;">
                <path d="M1.75 2.5h10.5a.75.75 0 0 1 0 1.5H1.75a.75.75 0 0 1 0-1.5Zm0 5h6.5a.75.75 0 0 1 0 1.5h-6.5a.75.75 0 0 1 0-1.5Zm0 5h12.5a.75.75 0 0 1 0 1.5H1.75a.75.75 0 0 1 0-1.5Z" />
              </svg>
              {t("pullFirstModal.fetchOnlyBtn")}
            {/if}
          </button>
        {/if}

        {#if onPullAndRetry}
          <button
            type="button"
            class="btn-primary btn-sm pull-first-confirm-btn"
            onclick={handlePullAndRetry}
            disabled={isPulling || isFetching}
          >
            {#if isPulling}
              <span class="pull-first-spinner" aria-hidden="true"></span>
              {t("pullFirstModal.pullingAndRetrying")}
            {:else}
              <svg width="13" height="13" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true" style="margin-right: 4px;">
                <path d="M8 2.5a.75.75 0 0 1 .75.75v6.19l2.72-2.72a.75.75 0 1 1 1.06 1.06l-4 4a.75.75 0 0 1-1.06 0l-4-4a.75.75 0 1 1 1.06-1.06l2.72 2.72V3.25A.75.75 0 0 1 8 2.5Z" />
              </svg>
              {t("pullFirstModal.pullAndRetryBtn")}
            {/if}
          </button>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .pull-first-dialog {
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

  .pull-first-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    padding: 16px 20px 14px;
    border-bottom: 1px solid var(--border-muted);
    background-color: var(--bg-surface);
  }

  .pull-first-title-wrap {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .pull-first-icon-box {
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

  .pull-first-title {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .pull-first-repo-badge {
    font-size: 11px;
    padding: 1px 6px;
    border-radius: var(--radius-sm, 4px);
    background-color: var(--bg-canvas);
    border: 1px solid var(--border-muted);
    color: var(--text-muted);
    font-family: var(--font-mono);
  }

  .pull-first-subtitle {
    margin: 2px 0 0;
    font-size: 12px;
    color: var(--text-secondary);
    line-height: 1.4;
  }

  .pull-first-body {
    padding: 16px 20px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    background-color: var(--bg-panel);
    max-height: 70vh;
    overflow-y: auto;
  }

  .pull-first-callout {
    padding: 12px 14px;
    background-color: var(--bg-surface);
    border: 1px solid var(--border-muted);
    border-left: 3px solid var(--accent-solid, #6366f1);
    border-radius: var(--radius-sm, 4px);
  }

  .pull-first-explanation {
    margin: 0;
    font-size: 13px;
    line-height: 1.5;
    color: var(--text-primary);
  }

  .pull-first-error-box {
    padding: 10px 12px;
    background-color: var(--danger-bg, rgba(239, 68, 68, 0.1));
    border: 1px solid var(--danger-border, rgba(239, 68, 68, 0.3));
    border-radius: var(--radius-sm, 4px);
    color: var(--danger-solid-hover, #f85149);
    font-size: 12px;
    line-height: 1.4;
  }

  .pull-first-error-section {
    border-top: 1px solid var(--border-muted);
    padding-top: 10px;
  }

  .pull-first-error-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .pull-first-toggle-logs-btn,
  .pull-first-copy-btn {
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

  .pull-first-toggle-logs-btn:hover,
  .pull-first-copy-btn:hover {
    color: var(--text-primary);
    background-color: var(--bg-canvas);
  }

  .pull-first-raw-log-block {
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

  .pull-first-footer {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: 8px;
    padding: 12px 20px;
    border-top: 1px solid var(--border-muted);
    background-color: var(--bg-surface);
  }

  .pull-first-confirm-btn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }

  .pull-first-spinner {
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
