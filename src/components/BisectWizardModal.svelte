<script lang="ts">
  import type { CommitInfo, BisectState } from "../types/git";
  import * as gitService from "../services/git";
  import { t } from "../i18n/index.svelte";
  import { copyTextToClipboard } from "../lib/contextMenu.svelte";

  interface Props {
    repoPath: string;
    commits: CommitInfo[];
    onClose: () => void;
    onStatus: (msg: string, isError?: boolean) => void;
    onReloadRepo: () => Promise<void>;
    onInspectCommit?: (sha: string) => void;
  }

  let {
    repoPath,
    commits = [],
    onClose,
    onStatus,
    onReloadRepo,
    onInspectCommit,
  }: Props = $props();

  let bisectState = $state<BisectState | null>(null);
  let badSha = $state("HEAD");
  let goodSha = $state("");
  let isManualGoodSha = $state(false);
  let isLoading = $state(false);
  let inlineError = $state("");
  let copiedSha = $state(false);
  let logOpen = $state(false);

  $effect(() => {
    if (repoPath) {
      isLoading = true;
      gitService
        .gitBisectStatus(repoPath)
        .then((st) => {
          bisectState = st;
        })
        .catch(() => {
          bisectState = null;
        })
        .finally(() => {
          isLoading = false;
        });
    }
  });

  async function handleStart() {
    inlineError = "";
    const bad = badSha.trim();
    const good = goodSha.trim();
    if (!bad || !good) {
      inlineError = t("bisect.selectBothCommits");
      return;
    }
    if (bad === good) {
      inlineError = t("bisect.selectBothCommits");
      return;
    }
    isLoading = true;
    try {
      const res = await gitService.gitBisectStart(repoPath, bad, good);
      bisectState = res;
      onStatus(t("bisect.startedStatus"));
      await onReloadRepo();
    } catch (err) {
      inlineError = String(err);
    } finally {
      isLoading = false;
    }
  }

  async function handleStep(action: "good" | "bad" | "skip") {
    inlineError = "";
    isLoading = true;
    try {
      const res = await gitService.gitBisectStep(repoPath, action);
      bisectState = res;
      onStatus(t("bisect.stepStatus"));
      await onReloadRepo();
    } catch (err) {
      inlineError = String(err);
    } finally {
      isLoading = false;
    }
  }

  async function handleReset() {
    inlineError = "";
    isLoading = true;
    try {
      await gitService.gitBisectReset(repoPath);
      onStatus(t("bisect.resetStatus"));
      await onReloadRepo();
      onClose();
    } catch (err) {
      inlineError = String(err);
    } finally {
      isLoading = false;
    }
  }

  async function handleCopySha(sha: string) {
    try {
      const ok = await copyTextToClipboard(sha);
      if (ok) {
        copiedSha = true;
        setTimeout(() => {
          copiedSha = false;
        }, 2000);
      }
    } catch {
      // ignore
    }
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="modal-overlay"
  role="presentation"
  onclick={(e) => {
    if (e.target === e.currentTarget) onClose();
  }}
  onkeydown={(e) => {
    if (e.key === "Escape") onClose();
  }}
>
  <div
    class="modal-dialog bisect-modal"
    role="dialog"
    aria-modal="true"
    aria-label={t("bisect.title")}
    style="--modal-w: 580px;"
  >
    <div class="modal-header">
      <div style="display: flex; flex-direction: column; gap: 2px;">
        <h2 class="modal-title">{t("bisect.title")}</h2>
        <span style="font-size: 11px; color: var(--text-secondary);">
          {#if bisectState?.first_bad_commit}
            {t("bisect.culpritSubtitle")}
          {:else if bisectState?.is_active}
            {t("bisect.activeSubtitle")}
          {:else}
            {t("bisect.setupSubtitle")}
          {/if}
        </span>
      </div>
      <button
        type="button"
        class="btn-icon"
        aria-label={t("common.close")}
        onclick={onClose}
      >
        ✕
      </button>
    </div>

    <div class="modal-body" style="display: flex; flex-direction: column; gap: 12px;">
      {#if inlineError}
        <div class="form-error">{inlineError}</div>
      {/if}

      {#if isLoading && !bisectState}
        <div style="padding: 24px; text-align: center; color: var(--text-muted); font-size: 12px;">
          {t("common.loading")}
        </div>
      {:else if bisectState?.first_bad_commit}
        <!-- Screen 3: Culprit Found -->
        <div class="bisect-culprit-card">
          <div class="bisect-culprit-header">
            <span class="bisect-culprit-badge">{t("bisect.culpritFound")}</span>
            <span class="bisect-sha-tag">{bisectState.first_bad_commit.short_sha}</span>
          </div>

          <div class="bisect-culprit-msg">
            {bisectState.first_bad_commit.summary}
          </div>

          <div class="bisect-culprit-meta">
            <span>{bisectState.first_bad_commit.author}</span>
            <span>·</span>
            <span>{bisectState.first_bad_commit.date}</span>
          </div>
        </div>

        {#if bisectState.message}
          <details class="bisect-log-details" bind:open={logOpen}>
            <summary class="bisect-log-summary">
              <span>▾</span>
              <span>{t("bisect.logTitle")}</span>
            </summary>
            <div class="bisect-log-body">
              {bisectState.message}
            </div>
          </details>
        {/if}

        <div class="modal-actions" style="margin-top: 8px;">
          <button
            type="button"
            class="btn-secondary bisect-copy-btn"
            onclick={() => handleCopySha(bisectState!.first_bad_commit!.sha)}
          >
            {#if copiedSha}
              <svg width="13" height="13" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <polyline points="3 8.5 6.5 12 13 4"/>
              </svg>
              <span>{t("bisect.copiedSha")}</span>
            {:else}
              <svg width="13" height="13" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                <rect x="5.5" y="5.5" width="8" height="8" rx="1.5"/>
                <path d="M10.5 5.5V3a1.5 1.5 0 0 0-1.5-1.5H3A1.5 1.5 0 0 0 1.5 3v6A1.5 1.5 0 0 0 3 10.5h2.5"/>
              </svg>
              <span>{t("bisect.copySha")}</span>
            {/if}
          </button>
          {#if onInspectCommit}
            <button
              type="button"
              class="btn-secondary"
              onclick={() => onInspectCommit?.(bisectState!.first_bad_commit!.sha)}
            >
              {t("bisect.inspectCommitBtn")}
            </button>
          {/if}
          <button
            type="button"
            class="btn-primary"
            disabled={isLoading}
            onclick={handleReset}
          >
            {t("bisect.finishBtn")}
          </button>
        </div>
      {:else if bisectState?.is_active}
        <!-- Screen 2: In-Progress Testing -->
        {#if bisectState.current_commit}
          <div class="bisect-testing-card">
            <div class="bisect-testing-header">
              <span class="form-label" style="margin: 0;">{t("bisect.currentTesting")}</span>
              <span class="bisect-sha-tag">{bisectState.current_commit.short_sha}</span>
            </div>

            <div class="bisect-commit-summary">
              {bisectState.current_commit.summary}
            </div>

            <div class="bisect-commit-meta">
              <span>{bisectState.current_commit.author}</span>
              <span>·</span>
              <span>{bisectState.current_commit.date}</span>
            </div>

            {#if bisectState.remaining_steps_hint}
              <div class="bisect-steps-hint">
                {bisectState.remaining_steps_hint}
              </div>
            {/if}

            <p style="font-size: 11px; color: var(--text-secondary); margin: 4px 0 0;">
              {t("bisect.testingHint")}
            </p>
          </div>
        {:else}
          <!-- In active state without specific commit (e.g. merge base or ambiguous status) -->
          <div class="bisect-testing-card">
            <div class="bisect-commit-summary">
              {bisectState.message || t("bisect.activeSubtitle")}
            </div>
            {#if bisectState.remaining_steps_hint}
              <div class="bisect-steps-hint">
                {bisectState.remaining_steps_hint}
              </div>
            {/if}
          </div>
        {/if}

        <div class="bisect-step-buttons">
          <button
            type="button"
            class="btn-bisect-action btn-good"
            disabled={isLoading}
            onclick={() => handleStep("good")}
          >
            <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <polyline points="3 8.5 6.5 12 13 4"/>
            </svg>
            <span>{t("bisect.markGoodBtn")}</span>
          </button>
          <button
            type="button"
            class="btn-bisect-action btn-bad"
            disabled={isLoading}
            onclick={() => handleStep("bad")}
          >
            <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <line x1="4" y1="4" x2="12" y2="12"/>
              <line x1="12" y1="4" x2="4" y2="12"/>
            </svg>
            <span>{t("bisect.markBadBtn")}</span>
          </button>
          <button
            type="button"
            class="btn-bisect-action btn-skip"
            disabled={isLoading}
            onclick={() => handleStep("skip")}
          >
            <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <polyline points="9 3 13 7 9 11"/>
              <path d="M3 13v-3a3 3 0 0 1 3-3h7"/>
            </svg>
            <span>{t("bisect.skipBtn")}</span>
          </button>
        </div>

        {#if bisectState.message}
          <details class="bisect-log-details" bind:open={logOpen}>
            <summary class="bisect-log-summary">
              <span>▾</span>
              <span>{t("bisect.logTitle")}</span>
            </summary>
            <div class="bisect-log-body">
              {bisectState.message}
            </div>
          </details>
        {/if}

        <div class="modal-actions" style="margin-top: 8px;">
          <button
            type="button"
            class="btn-secondary btn-danger"
            disabled={isLoading}
            onclick={handleReset}
          >
            {t("bisect.abortBtn")}
          </button>
        </div>
      {:else}
        <!-- Screen 1: Setup Form -->
        <div class="form-field">
          <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 4px;">
            <label class="form-label" for="bisect-bad-sha" style="margin: 0;">{t("bisect.badCommitLabel")}</label>
          </div>
          <div style="display: flex; gap: 6px;">
            <input
              id="bisect-bad-sha"
              type="text"
              class="form-input"
              style="flex: 1;"
              placeholder={t("bisect.badCommitPlaceholder")}
              bind:value={badSha}
            />
            {#if commits.length > 0}
              <select
                class="form-select"
                style="width: 150px;"
                aria-label={t("bisect.pickCommit")}
                onchange={(e) => {
                  const val = (e.target as HTMLSelectElement).value;
                  if (val) badSha = val;
                }}
              >
                <option value="">{t("bisect.pickCommit")}</option>
                <option value="HEAD">HEAD</option>
                {#each commits.slice(0, 30) as c (c.sha)}
                  <option value={c.sha}>
                    [{c.short_sha}] {c.summary.slice(0, 24)}
                  </option>
                {/each}
              </select>
            {/if}
          </div>
        </div>

        <div class="form-field">
          <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 4px;">
            <label class="form-label" for="bisect-good-sha" style="margin: 0;">{t("bisect.goodCommitLabel")}</label>
            <button
              type="button"
              class="bisect-toggle-btn"
              onclick={() => {
                isManualGoodSha = !isManualGoodSha;
              }}
            >
              {isManualGoodSha ? t("bisect.toggleSelect") : t("bisect.toggleManual")}
            </button>
          </div>

          {#if isManualGoodSha || commits.length === 0}
            <input
              id="bisect-good-sha"
              type="text"
              class="form-input"
              placeholder={t("bisect.goodCommitPlaceholder")}
              bind:value={goodSha}
            />
          {:else}
            <select
              id="bisect-good-sha"
              class="form-select"
              bind:value={goodSha}
            >
              <option value="">-- {t("bisect.goodCommitPlaceholder")} --</option>
              {#each commits as c (c.sha)}
                {#if c.sha !== badSha}
                  <option value={c.sha}>
                    [{c.short_sha}] {c.summary} ({c.author})
                  </option>
                {/if}
              {/each}
            </select>
          {/if}
        </div>

        <div class="modal-actions" style="margin-top: 8px;">
          <button type="button" class="btn-secondary" onclick={onClose}>
            {t("common.cancel")}
          </button>
          <button
            type="button"
            class="btn-primary"
            disabled={isLoading || !badSha.trim() || !goodSha.trim()}
            onclick={handleStart}
          >
            {t("bisect.startBtn")}
          </button>
        </div>
      {/if}
    </div>
  </div>
</div>
