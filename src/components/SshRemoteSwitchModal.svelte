<script lang="ts">
  import { t } from "../i18n/index.svelte";
  import * as gitService from "../services/git";
  import { copyTextToClipboard } from "../lib/contextMenu.svelte";
  import { getSshDiagnosisType } from "../utils/remoteUrl";

  interface Props {
    open: boolean;
    repo: string;
    remoteName: string;
    currentUrl: string;
    suggestedUrl: string;
    errorMessage: string;
    accountLogin?: string | null;
    onClose: () => void;
    onSwitchSuccess: (newUrl: string) => void;
  }

  let {
    open,
    repo,
    remoteName,
    currentUrl,
    suggestedUrl,
    errorMessage,
    accountLogin = null,
    onClose,
    onSwitchSuccess,
  }: Props = $props();

  let isSwitching = $state(false);
  let switchError = $state("");
  let copied = $state(false);
  let showRawLogs = $state(false);

  let diagnosisType = $derived(getSshDiagnosisType(errorMessage));
  let diagnosisText = $derived.by(() => {
    switch (diagnosisType) {
      case "timeout":
        return t("sshModal.diagnosisTimeout");
      case "reset":
        return t("sshModal.diagnosisReset");
      case "dns":
        return t("sshModal.diagnosisDns");
      case "auth":
        return t("sshModal.diagnosisAuth");
      default:
        return t("sshModal.diagnosisGeneral");
    }
  });

  async function handleCopyError() {
    await copyTextToClipboard(errorMessage);
    copied = true;
    setTimeout(() => {
      copied = false;
    }, 2000);
  }

  async function handleConfirmSwitch() {
    if (isSwitching || !repo || !remoteName || !suggestedUrl) return;
    isSwitching = true;
    switchError = "";
    try {
      await gitService.setRemoteUrl(repo, remoteName, suggestedUrl);
      onSwitchSuccess(suggestedUrl);
    } catch (err) {
      switchError = String(err);
    } finally {
      isSwitching = false;
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.stopPropagation();
      onClose();
    } else if (e.key === "Enter" && !e.shiftKey && !e.ctrlKey) {
      const activeEl = document.activeElement;
      if (activeEl?.tagName !== "BUTTON" && activeEl?.tagName !== "A") {
        e.preventDefault();
        void handleConfirmSwitch();
      }
    }
  }
</script>

{#if open}
  <div
    class="modal-overlay"
    role="presentation"
    onclick={(e) => {
      if (e.target === e.currentTarget && !isSwitching) onClose();
    }}
    onkeydown={handleKeyDown}
  >
    <div
      class="modal-dialog ssh-switch-dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="ssh-modal-title"
      tabindex="-1"
    >
      <!-- Header -->
      <div class="modal-header ssh-modal-header">
        <div class="ssh-header-title-wrap">
          <div class="ssh-header-icon-box">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z"/>
              <line x1="12" y1="9" x2="12" y2="13"/>
              <line x1="12" y1="17" x2="12.01" y2="17"/>
            </svg>
          </div>
          <div>
            <h2 id="ssh-modal-title" class="ssh-title">{t("sshModal.title")}</h2>
            <p class="ssh-subtitle">{t("sshModal.subtitle")}</p>
          </div>
        </div>
        <button
          type="button"
          class="icon-btn compact"
          onclick={onClose}
          disabled={isSwitching}
          aria-label={t("common.closeDialog")}
        >
          <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
            <path d="M3.72 3.72a.75.75 0 0 1 1.06 0L8 6.94l3.22-3.22a.749.749 0 0 1 1.275.326.749.749 0 0 1-.215.734L9.06 8l3.22 3.22a.749.749 0 0 1-.326 1.275.749.749 0 0 1-.734-.215L8 9.06l-3.22 3.22a.751.751 0 0 1-1.042-.018.751.751 0 0 1-.018-1.042L6.94 8 3.72 4.78a.75.75 0 0 1 0-1.06Z" />
          </svg>
        </button>
      </div>

      <!-- Body -->
      <div class="modal-body ssh-modal-body">
        <!-- Diagnosis Callout -->
        <div class="ssh-diagnosis-callout {diagnosisType}">
          <div class="ssh-callout-icon">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <circle cx="12" cy="12" r="10"/>
              <line x1="12" y1="16" x2="12" y2="12"/>
              <line x1="12" y1="8" x2="12.01" y2="8"/>
            </svg>
          </div>
          <div class="ssh-callout-text">
            {diagnosisText}
          </div>
        </div>

        <!-- URL Comparison Cards -->
        <div class="ssh-conversion-container">
          <!-- Current Remote (SSH) -->
          <div class="ssh-url-card current">
            <div class="ssh-card-header">
              <span class="ssh-card-label">{t("sshModal.currentRemote")}</span>
              <span class="ssh-badge ssh-badge-warn">SSH</span>
            </div>
            <div class="ssh-card-url" title={currentUrl}>
              <code>{currentUrl}</code>
            </div>
          </div>

          <!-- Transition Divider -->
          <div class="ssh-transition-divider" aria-hidden="true">
            <div class="ssh-arrow-circle">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                <line x1="12" y1="5" x2="12" y2="19"/>
                <polyline points="19 12 12 19 5 12"/>
              </svg>
            </div>
          </div>

          <!-- Recommended Remote (HTTPS) -->
          <div class="ssh-url-card recommended">
            <div class="ssh-card-header">
              <span class="ssh-card-label">{t("sshModal.targetRemote")}</span>
              <span class="ssh-badge ssh-badge-success">HTTPS</span>
            </div>
            <div class="ssh-card-url" title={suggestedUrl}>
              <code>{suggestedUrl}</code>
            </div>
          </div>
        </div>

        <!-- Account or Auth Hint -->
        <div class="ssh-account-hint-box">
          {#if accountLogin}
            <div class="ssh-account-pill">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"/>
                <circle cx="12" cy="7" r="4"/>
              </svg>
              <span>{t("sshModal.activeAccountHint", { login: accountLogin })}</span>
            </div>
          {:else}
            <div class="ssh-hint-text">
              <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                <circle cx="12" cy="12" r="10"/>
                <line x1="12" y1="16" x2="12" y2="12"/>
                <line x1="12" y1="8" x2="12.01" y2="8"/>
              </svg>
              <span>{t("sshModal.noAccountHint")}</span>
            </div>
          {/if}
        </div>

        <!-- Collapsible Error Logs -->
        <div class="ssh-error-section">
          <div class="ssh-error-header">
            <button
              type="button"
              class="ssh-toggle-logs-btn"
              onclick={() => (showRawLogs = !showRawLogs)}
            >
              <svg
                width="12"
                height="12"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
                style="transform: rotate({showRawLogs ? 90 : 0}deg); transition: transform 0.15s ease;"
                aria-hidden="true"
              >
                <polyline points="9 18 15 12 9 6"/>
              </svg>
              <span>Error Log Detail</span>
            </button>
            <button
              type="button"
              class="ssh-copy-btn"
              onclick={handleCopyError}
              title="Salin pesan error ke clipboard"
            >
              {#if copied}
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                  <polyline points="20 6 9 17 4 12"/>
                </svg>
                <span>{t("sshModal.copied")}</span>
              {:else}
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                  <rect x="9" y="9" width="13" height="13" rx="2" ry="2"/>
                  <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/>
                </svg>
                <span>{t("sshModal.copyError")}</span>
              {/if}
            </button>
          </div>

          {#if showRawLogs}
            <pre class="ssh-raw-log-block"><code>{errorMessage}</code></pre>
          {/if}
        </div>

        {#if switchError}
          <div class="ssh-error-alert">
            <span>{switchError}</span>
          </div>
        {/if}
      </div>

      <!-- Actions -->
      <div class="modal-actions ssh-modal-actions">
        <button
          type="button"
          class="btn-secondary"
          onclick={onClose}
          disabled={isSwitching}
        >
          {t("common.cancel")}
        </button>
        <button
          type="button"
          class="btn-primary ssh-confirm-btn"
          onclick={handleConfirmSwitch}
          disabled={isSwitching || !suggestedUrl}
        >
          {#if isSwitching}
            <span class="ssh-spinner" aria-hidden="true"></span>
            <span>{t("sshModal.switching")}</span>
          {:else}
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <polyline points="20 6 9 17 4 12"/>
            </svg>
            <span>{t("sshModal.switchBtn")}</span>
          {/if}
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .ssh-switch-dialog {
    --modal-w: 520px;
    max-width: 520px;
  }

  .ssh-modal-header {
    border-bottom: 1px solid var(--border-muted);
    padding-bottom: 12px;
  }

  .ssh-header-title-wrap {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .ssh-header-icon-box {
    width: 34px;
    height: 34px;
    border-radius: var(--radius-md);
    background-color: var(--warn-bg);
    border: 1px solid var(--warn-border);
    color: var(--warn-text);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .ssh-title {
    font-size: 15px;
    font-weight: 600;
    margin: 0;
    color: var(--text-primary);
  }

  .ssh-subtitle {
    font-size: 12px;
    color: var(--text-secondary);
    margin: 2px 0 0;
  }

  .ssh-modal-body {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 6px 0;
  }

  .ssh-diagnosis-callout {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 10px 12px;
    border-radius: var(--radius-md);
    background-color: var(--bg-surface);
    border: 1px solid var(--border-default);
    font-size: 12.5px;
    line-height: 1.45;
    color: var(--text-primary);
  }

  .ssh-diagnosis-callout.timeout,
  .ssh-diagnosis-callout.reset {
    background-color: var(--warn-bg);
    border-color: var(--warn-border);
    color: var(--text-primary);
  }

  .ssh-callout-icon {
    color: var(--warn-text);
    flex-shrink: 0;
    margin-top: 1px;
  }

  .ssh-callout-text {
    flex: 1;
  }

  .ssh-conversion-container {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .ssh-url-card {
    padding: 10px 12px;
    border-radius: var(--radius-md);
    border: 1px solid var(--border-default);
    background-color: var(--bg-surface);
    display: flex;
    flex-direction: column;
    gap: 5px;
  }

  .ssh-url-card.current {
    border-color: var(--border-default);
  }

  .ssh-url-card.recommended {
    border-color: var(--accent-blue-solid, #1f6feb);
    background-color: var(--bg-selected);
  }

  .ssh-card-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .ssh-card-label {
    font-size: 11.5px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-secondary);
  }

  .ssh-badge {
    font-size: 10.5px;
    font-weight: 600;
    padding: 2px 6px;
    border-radius: var(--radius-sm);
    line-height: 1;
  }

  .ssh-badge-warn {
    background-color: var(--warn-bg);
    color: var(--warn-text);
    border: 1px solid var(--warn-border);
  }

  .ssh-badge-success {
    background-color: var(--diff-add-bg);
    color: var(--diff-add-text);
    border: 1px solid var(--accent-primary);
  }

  .ssh-card-url {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .ssh-card-url code {
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--text-primary);
  }

  .ssh-transition-divider {
    display: flex;
    justify-content: center;
    align-items: center;
    position: relative;
    height: 14px;
    margin: -2px 0;
    z-index: 2;
  }

  .ssh-arrow-circle {
    width: 22px;
    height: 22px;
    border-radius: 50%;
    background-color: var(--bg-panel);
    border: 1px solid var(--border-strong);
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--accent-blue);
    box-shadow: var(--shadow-sm);
  }

  .ssh-account-hint-box {
    display: flex;
    align-items: center;
    font-size: 12px;
    color: var(--text-secondary);
  }

  .ssh-account-pill {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 9px;
    border-radius: 999px;
    background-color: var(--bg-surface);
    border: 1px solid var(--border-muted);
    font-size: 11.5px;
    color: var(--text-primary);
  }

  .ssh-hint-text {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11.5px;
    color: var(--text-muted);
  }

  .ssh-error-section {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: 2px;
  }

  .ssh-error-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .ssh-toggle-logs-btn,
  .ssh-copy-btn {
    background: transparent;
    border: none;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 11.5px;
    color: var(--text-secondary);
    padding: 3px 6px;
    border-radius: var(--radius-sm);
    transition: background-color 0.15s ease, color 0.15s ease;
  }

  .ssh-toggle-logs-btn:hover,
  .ssh-copy-btn:hover {
    background-color: var(--bg-hover);
    color: var(--text-primary);
  }

  .ssh-raw-log-block {
    margin: 0;
    padding: 8px 10px;
    background-color: var(--bg-canvas);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-sm);
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--tok-keyword, #ff7b72);
    max-height: 120px;
    overflow-y: auto;
    white-space: pre-wrap;
    word-break: break-all;
  }

  .ssh-error-alert {
    padding: 8px 10px;
    border-radius: var(--radius-sm);
    background-color: var(--danger-bg);
    border: 1px solid var(--danger-border);
    color: var(--danger-solid-hover, #f85149);
    font-size: 12px;
  }

  .ssh-modal-actions {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: 8px;
    border-top: 1px solid var(--border-muted);
    padding-top: 12px;
  }

  .ssh-confirm-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .ssh-spinner {
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
