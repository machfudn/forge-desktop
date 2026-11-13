<script lang="ts">
  import type { CommitInfo, BisectState } from "../types/git";
  import * as gitService from "../services/git";
  import { t } from "../i18n/index.svelte";

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
  let isLoading = $state(false);
  let inlineError = $state("");

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
    if (!badSha.trim() || !goodSha.trim()) {
      inlineError = t("bisect.selectBothCommits");
      return;
    }
    isLoading = true;
    try {
      const res = await gitService.gitBisectStart(repoPath, badSha.trim(), goodSha.trim());
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
  <div class="modal-card" style="width: 580px; max-width: 95vw;">
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

        <div class="modal-actions" style="margin-top: 8px;">
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
      {:else if bisectState?.is_active && bisectState?.current_commit}
        <!-- Screen 2: In-Progress Testing -->
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

        <div class="bisect-step-buttons">
          <button
            type="button"
            class="btn-bisect-action btn-good"
            disabled={isLoading}
            onclick={() => handleStep("good")}
          >
            ✓ {t("bisect.markGoodBtn")}
          </button>
          <button
            type="button"
            class="btn-bisect-action btn-bad"
            disabled={isLoading}
            onclick={() => handleStep("bad")}
          >
            ✗ {t("bisect.markBadBtn")}
          </button>
          <button
            type="button"
            class="btn-secondary"
            disabled={isLoading}
            onclick={() => handleStep("skip")}
          >
            ↷ {t("bisect.skipBtn")}
          </button>
        </div>

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
          <label class="form-label" for="bisect-bad-sha">{t("bisect.badCommitLabel")}</label>
          <input
            id="bisect-bad-sha"
            type="text"
            class="form-input"
            placeholder={t("bisect.badCommitPlaceholder")}
            bind:value={badSha}
          />
        </div>

        <div class="form-field">
          <label class="form-label" for="bisect-good-sha">{t("bisect.goodCommitLabel")}</label>
          <select
            id="bisect-good-sha"
            class="form-select"
            bind:value={goodSha}
          >
            <option value="">-- {t("bisect.goodCommitPlaceholder")} --</option>
            {#each commits.slice(1) as c (c.sha)}
              <option value={c.sha}>
                [{c.short_sha}] {c.summary} ({c.author})
              </option>
            {/each}
          </select>
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
