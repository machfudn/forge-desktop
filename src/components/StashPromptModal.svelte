<script lang="ts">
  import { t } from "../i18n/index.svelte";
  import { pathBaseName } from "../utils/path";

  interface Props {
    open: boolean;
    repo: string;
    files: string[];
    operationName?: string;
    onClose: () => void;
    onStashAndProceed: () => Promise<void>;
  }

  let {
    open,
    repo,
    files = [],
    operationName = "Pull",
    onClose,
    onStashAndProceed,
  }: Props = $props();

  let isStashing = $state(false);
  let stashError = $state("");

  async function handleProceed() {
    if (isStashing) return;
    isStashing = true;
    stashError = "";
    try {
      await onStashAndProceed();
      onClose();
    } catch (err) {
      stashError = String(err);
    } finally {
      isStashing = false;
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.stopPropagation();
      onClose();
    } else if (e.key === "Enter" && !e.shiftKey && !e.ctrlKey) {
      if (!isStashing) {
        e.preventDefault();
        void handleProceed();
      }
    }
  }
</script>

{#if open}
  <div
    class="modal-overlay"
    role="presentation"
    onclick={(e) => {
      if (e.target === e.currentTarget && !isStashing) onClose();
    }}
    onkeydown={handleKeyDown}
  >
    <div
      class="modal-dialog stash-prompt-dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="stash-prompt-title"
      tabindex="-1"
    >
      <!-- Header -->
      <div class="modal-header stash-prompt-header">
        <div class="stash-prompt-title-wrap">
          <div class="stash-prompt-icon-box">
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
              <path d="M21 8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16Z" />
              <path d="m3.3 7 8.7 5 8.7-5" />
              <path d="M12 22V12" />
            </svg>
          </div>
          <div>
            <div style="display: flex; align-items: center; gap: 8px;">
              <h2 id="stash-prompt-title" class="stash-prompt-title">{t("stashPromptModal.title")}</h2>
              {#if repo}
                <span class="stash-repo-badge">{pathBaseName(repo)}</span>
              {/if}
            </div>
            <p class="stash-prompt-subtitle">
              {t("stashPromptModal.subtitle", { operation: operationName })}
            </p>
          </div>
        </div>
        <button
          type="button"
          class="icon-btn compact"
          onclick={onClose}
          disabled={isStashing}
          aria-label={t("common.closeDialog")}
        >
          <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
            <path d="M3.72 3.72a.75.75 0 0 1 1.06 0L8 6.94l3.22-3.22a.749.749 0 0 1 1.275.326.749.749 0 0 1-.215.734L9.06 8l3.22 3.22a.749.749 0 0 1-.326 1.275.749.749 0 0 1-.734-.215L8 9.06l-3.22 3.22a.751.751 0 0 1-1.042-.018.751.751 0 0 1-.018-1.042L6.94 8 3.72 4.78a.75.75 0 0 1 0-1.06Z" />
          </svg>
        </button>
      </div>

      <!-- Body -->
      <div class="modal-body stash-prompt-body">
        <!-- Explanation Banner -->
        <div class="stash-prompt-callout">
          <p class="stash-prompt-explanation">
            {t("stashPromptModal.explanation", { operation: operationName })}
          </p>
        </div>

        <!-- Affected Files Card -->
        {#if files.length > 0}
          <div class="stash-files-card">
            <div class="stash-files-header">
              <span class="stash-files-label">{t("stashPromptModal.filesLabel")}</span>
              <span class="stash-files-count">({files.length})</span>
            </div>
            <div class="stash-files-list">
              {#each files as f}
                <div class="stash-file-row">
                  <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true" style="color: var(--text-muted); flex-shrink: 0;">
                    <path d="M2 1.75C2 .784 2.784 0 3.75 0h6.586c.464 0 .909.184 1.237.513l2.914 2.914c.329.328.513.773.513 1.237v9.586A1.75 1.75 0 0 1 13.25 16h-9.5A1.75 1.75 0 0 1 2 14.25Zm1.75-.25a.25.25 0 0 0-.25.25v12.5c0 .138.112.25.25.25h9.5a.25.25 0 0 0 .25-.25V6h-2.75A1.75 1.75 0 0 1 9 4.25V1.5Zm7.25.56V4.25c0 .138.112.25.25.25h2.69Z"/>
                  </svg>
                  <code class="stash-file-code" title={f}>{f}</code>
                </div>
              {/each}
            </div>
          </div>
        {/if}

        {#if stashError}
          <div class="stash-prompt-error">
            {stashError}
          </div>
        {/if}
      </div>

      <!-- Footer Actions -->
      <div class="modal-footer stash-prompt-footer">
        <button
          type="button"
          class="btn-secondary btn-sm"
          onclick={onClose}
          disabled={isStashing}
        >
          {t("common.cancel")}
        </button>

        <button
          type="button"
          class="btn-primary btn-sm stash-confirm-btn"
          onclick={handleProceed}
          disabled={isStashing}
        >
          {#if isStashing}
            <span class="stash-spinner" aria-hidden="true"></span>
            {t("stashPromptModal.stashing")}
          {:else}
            {t("stashPromptModal.stashAndProceedBtn")}
          {/if}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .stash-prompt-dialog {
    width: 92vw;
    max-width: 540px;
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

  .stash-prompt-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    padding: 16px 20px 14px;
    border-bottom: 1px solid var(--border-muted);
    background-color: var(--bg-surface);
  }

  .stash-prompt-title-wrap {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .stash-prompt-icon-box {
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

  .stash-prompt-title {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .stash-repo-badge {
    font-size: 11px;
    padding: 1px 6px;
    border-radius: var(--radius-sm, 4px);
    background-color: var(--bg-canvas);
    border: 1px solid var(--border-muted);
    color: var(--text-muted);
    font-family: var(--font-mono);
  }

  .stash-prompt-subtitle {
    margin: 2px 0 0;
    font-size: 12px;
    color: var(--text-secondary);
    line-height: 1.4;
  }

  .stash-prompt-body {
    padding: 16px 20px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    overflow-y: auto;
    max-height: 70vh;
  }

  .stash-prompt-callout {
    padding: 12px 14px;
    border-radius: var(--radius-md, 6px);
    background-color: var(--bg-surface);
    border: 1px solid var(--border-default);
  }

  .stash-prompt-explanation {
    margin: 0;
    font-size: 12.5px;
    line-height: 1.5;
    color: var(--text-secondary);
  }

  .stash-files-card {
    padding: 12px 14px;
    border-radius: var(--radius-md, 6px);
    background-color: var(--bg-canvas);
    border: 1px solid var(--border-default);
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .stash-files-header {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .stash-files-label {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .stash-files-count {
    font-size: 11px;
    color: var(--text-muted);
  }

  .stash-files-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-height: 150px;
    overflow-y: auto;
  }

  .stash-file-row {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 11.5px;
    padding: 2px 0;
  }

  .stash-file-code {
    font-family: var(--font-mono);
    color: var(--tok-keyword, #ff7b72);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .stash-prompt-error {
    font-size: 11.5px;
    color: var(--danger-solid-hover, #f85149);
    padding: 6px 10px;
    background-color: var(--danger-bg);
    border: 1px solid var(--danger-border);
    border-radius: var(--radius-sm, 4px);
  }

  .stash-prompt-footer {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: 8px;
    border-top: 1px solid var(--border-muted);
    padding: 12px 20px;
    background-color: var(--bg-surface);
  }

  .stash-confirm-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .stash-spinner {
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
