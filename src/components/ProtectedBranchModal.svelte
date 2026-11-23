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
    onCreateBranchAndPush?: (newBranchName: string) => Promise<void>;
    onOpenBranchModal?: () => void;
  }

  let {
    open,
    repo,
    branchName = "main",
    errorMessage,
    onClose,
    onCreateBranchAndPush,
    onOpenBranchModal,
  }: Props = $props();

  let newBranchInput = $state("");
  let copied = $state(false);
  let showRawLogs = $state(false);
  let isCreatingBranch = $state(false);
  let actionError = $state("");

  $effect(() => {
    if (open) {
      const sanitized = (branchName || "main").replace(/^refs\/heads\//, "");
      newBranchInput = `feature/${sanitized}-patch`;
      actionError = "";
    }
  });

  async function handleCopyError() {
    await copyTextToClipboard(errorMessage);
    copied = true;
    setTimeout(() => {
      copied = false;
    }, 2000);
  }

  async function handleCreateBranch() {
    const target = newBranchInput.trim();
    if (!target || !onCreateBranchAndPush || isCreatingBranch) return;
    isCreatingBranch = true;
    actionError = "";
    try {
      await onCreateBranchAndPush(target);
      onClose();
    } catch (err) {
      actionError = String(err);
    } finally {
      isCreatingBranch = false;
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.stopPropagation();
      onClose();
    } else if (e.key === "Enter" && !e.shiftKey && !e.ctrlKey) {
      if (newBranchInput.trim() && onCreateBranchAndPush && !isCreatingBranch) {
        e.preventDefault();
        void handleCreateBranch();
      }
    }
  }
</script>

{#if open}
  <div
    class="modal-overlay"
    role="presentation"
    onclick={(e) => {
      if (e.target === e.currentTarget && !isCreatingBranch) onClose();
    }}
    onkeydown={handleKeyDown}
  >
    <div
      class="modal-dialog protected-modal-dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="protected-modal-title"
      tabindex="-1"
    >
      <!-- Header -->
      <div class="modal-header protected-modal-header">
        <div class="protected-header-title-wrap">
          <div class="protected-header-icon-box">
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
              <h2 id="protected-modal-title" class="protected-title">{t("protectedBranchModal.title")}</h2>
              {#if repo}
                <span class="protected-repo-badge">{pathBaseName(repo)}</span>
              {/if}
            </div>
            <p class="protected-subtitle">
              {t("protectedBranchModal.subtitle", { branch: branchName || "main" })}
            </p>
          </div>
        </div>
        <button
          type="button"
          class="icon-btn compact"
          onclick={onClose}
          disabled={isCreatingBranch}
          aria-label={t("common.closeDialog")}
        >
          <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
            <path d="M3.72 3.72a.75.75 0 0 1 1.06 0L8 6.94l3.22-3.22a.749.749 0 0 1 1.275.326.749.749 0 0 1-.215.734L9.06 8l3.22 3.22a.749.749 0 0 1-.326 1.275.749.749 0 0 1-.734-.215L8 9.06l-3.22 3.22a.751.751 0 0 1-1.042-.018.751.751 0 0 1-.018-1.042L6.94 8 3.72 4.78a.75.75 0 0 1 0-1.06Z" />
          </svg>
        </button>
      </div>

      <!-- Body -->
      <div class="modal-body protected-modal-body">
        <!-- Explanation Banner -->
        <div class="protected-callout">
          <p class="protected-explanation-text">
            {t("protectedBranchModal.explanation")}
          </p>
        </div>

        <!-- Quick Branch & Push Form -->
        {#if onCreateBranchAndPush}
          <div class="protected-form-card">
            <label class="form-label" for="new-branch-input">
              {t("protectedBranchModal.newBranchLabel")}
            </label>
            <div class="protected-input-row">
              <input
                id="new-branch-input"
                type="text"
                class="form-input"
                bind:value={newBranchInput}
                disabled={isCreatingBranch}
                placeholder={t("protectedBranchModal.newBranchPlaceholder")}
              />
              <button
                type="button"
                class="btn-primary btn-sm protected-action-btn"
                onclick={handleCreateBranch}
                disabled={isCreatingBranch || !newBranchInput.trim()}
              >
                {#if isCreatingBranch}
                  <span class="protected-spinner" aria-hidden="true"></span>
                  {t("protectedBranchModal.creating")}
                {:else}
                  {t("protectedBranchModal.createAndPushBtn")}
                {/if}
              </button>
            </div>
            {#if actionError}
              <div class="protected-action-error">
                {actionError}
              </div>
            {/if}
          </div>
        {/if}

        <!-- Error Log Accordion -->
        <div class="protected-error-section">
          <div class="protected-error-header">
            <button
              type="button"
              class="protected-toggle-logs-btn"
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
            <button type="button" class="protected-copy-btn" onclick={handleCopyError}>
              <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
                <path d="M0 6.75C0 5.784.784 5 1.75 5h1.5a.75.75 0 0 1 0 1.5h-1.5a.25.25 0 0 0-.25.25v7.5c0 .138.112.25.25.25h7.5a.25.25 0 0 0 .25-.25v-1.5a.75.75 0 0 1 1.5 0v1.5A1.75 1.75 0 0 1 9.25 16h-7.5A1.75 1.75 0 0 1 0 14.25Z" />
                <path d="M5 1.75C5 .784 5.784 0 6.75 0h7.5C15.216 0 16 .784 16 1.75v7.5A1.75 1.75 0 0 1 14.25 11h-7.5A1.75 1.75 0 0 1 5 9.25Zm1.75-.25a.25.25 0 0 0-.25.25v7.5c0 .138.112.25.25.25h7.5a.25.25 0 0 0 .25-.25v-7.5a.25.25 0 0 0-.25-.25Z" />
              </svg>
              {copied ? t("secretModal.copied") : t("secretModal.copyError")}
            </button>
          </div>
          {#if showRawLogs}
            <pre class="protected-raw-log-block">{errorMessage}</pre>
          {/if}
        </div>
      </div>

      <!-- Footer Actions -->
      <div class="modal-footer protected-modal-footer">
        <button
          type="button"
          class="btn-secondary btn-sm"
          onclick={onClose}
          disabled={isCreatingBranch}
        >
          {t("common.close")}
        </button>

        {#if onOpenBranchModal}
          <button
            type="button"
            class="btn-secondary btn-sm"
            onclick={() => {
              onClose();
              onOpenBranchModal();
            }}
            disabled={isCreatingBranch}
          >
            {t("protectedBranchModal.openBranchList")}
          </button>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .protected-modal-dialog {
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

  .protected-modal-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    padding: 16px 20px 14px;
    border-bottom: 1px solid var(--border-muted);
    background-color: var(--bg-surface);
  }

  .protected-header-title-wrap {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .protected-header-icon-box {
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

  .protected-title {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .protected-repo-badge {
    font-size: 11px;
    padding: 1px 6px;
    border-radius: var(--radius-sm, 4px);
    background-color: var(--bg-canvas);
    border: 1px solid var(--border-muted);
    color: var(--text-muted);
    font-family: var(--font-mono);
  }

  .protected-subtitle {
    margin: 2px 0 0;
    font-size: 12px;
    color: var(--text-secondary);
    line-height: 1.4;
  }

  .protected-modal-body {
    padding: 16px 20px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    overflow-y: auto;
    max-height: 70vh;
  }

  .protected-callout {
    padding: 12px 14px;
    border-radius: var(--radius-md, 6px);
    background-color: var(--bg-surface);
    border: 1px solid var(--border-default);
  }

  .protected-explanation-text {
    margin: 0;
    font-size: 12.5px;
    line-height: 1.5;
    color: var(--text-secondary);
  }

  .protected-form-card {
    padding: 14px 16px;
    border-radius: var(--radius-md, 6px);
    background-color: var(--bg-canvas);
    border: 1px solid var(--border-default);
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .protected-input-row {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .protected-action-btn {
    white-space: nowrap;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }

  .protected-action-error {
    font-size: 11.5px;
    color: var(--danger-solid-hover, #f85149);
    padding-top: 4px;
  }

  .protected-error-section {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: 4px;
  }

  .protected-error-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .protected-toggle-logs-btn,
  .protected-copy-btn {
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

  .protected-toggle-logs-btn:hover,
  .protected-copy-btn:hover {
    background-color: var(--bg-hover);
    color: var(--text-primary);
  }

  .protected-raw-log-block {
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

  .protected-modal-footer {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: 8px;
    border-top: 1px solid var(--border-muted);
    padding: 12px 20px;
    background-color: var(--bg-surface);
  }

  .protected-spinner {
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
