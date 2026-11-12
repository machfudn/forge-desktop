<script lang="ts">
  import * as gitService from "../services/git";
  import { t } from "../i18n/index.svelte";

  interface Props {
    repoPath?: string;
    initialTask?: "custom_ask" | "explain_diff" | "explain_commit" | "review_changes";
    initialPrompt?: string;
    initialContext?: string;
    onClose: () => void;
    onOpenSettings: () => void;
    onStatus?: (msg: string, isError?: boolean) => void;
  }

  let {
    repoPath = "",
    initialTask = "custom_ask",
    initialPrompt = "",
    initialContext = "",
    onClose,
    onOpenSettings,
    onStatus,
  }: Props = $props();

  let taskKind = $state<"custom_ask" | "explain_diff" | "explain_commit" | "review_changes">("custom_ask");
  let prompt = $state("");
  let contextText = $state("");
  let includeRepoContext = $state(true);
  let running = $state(false);
  let answer = $state("");
  let metaText = $state("");
  let errorMsg = $state("");
  let autoStarted = false;

  $effect(() => {
    taskKind = initialTask;
    prompt = initialPrompt;
    contextText = initialContext;
    if (!autoStarted && (initialTask === "explain_diff" || initialTask === "explain_commit")) {
      autoStarted = true;
      void runAi();
    }
  });

  async function runAi() {
    if (running) return;
    if (taskKind === "custom_ask" && !prompt.trim()) {
      errorMsg = t("ai.promptRequired");
      return;
    }
    running = true;
    errorMsg = "";
    try {
      const res = await gitService.aiRunTask(
        taskKind,
        prompt,
        includeRepoContext && repoPath ? repoPath : undefined,
        contextText || undefined
      );
      answer = res.content;
      metaText = `${res.model} · ${res.latency_ms} ms`;
    } catch (err) {
      errorMsg = String(err);
      onStatus?.(String(err), true);
    } finally {
      running = false;
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="modal-overlay"
  onclick={(e) => {
    if (e.target === e.currentTarget) onClose();
  }}
  onkeydown={(e) => {
    if (e.key === "Escape") onClose();
  }}
>
  <div class="modal-dialog modal-lg" role="dialog" aria-modal="true" aria-label={t("ai.assistantTitle")}>
    <div class="modal-header">
      <div style="display: flex; align-items: center; gap: 8px;">
        <h3>✦ {t("ai.assistantTitle")}</h3>
        {#if metaText}
          <span class="badge-neutral">{metaText}</span>
        {/if}
      </div>
      <div style="display: flex; align-items: center; gap: 6px;">
        <button type="button" class="btn-secondary btn-xs" onclick={() => { onClose(); onOpenSettings(); }}>
          {t("ai.configureBtn")}
        </button>
        <button type="button" class="btn-secondary btn-xs" onclick={onClose}>{t("common.close")}</button>
      </div>
    </div>

    <div class="modal-body">
      <div style="display: flex; gap: 6px; flex-wrap: wrap;">
        <button
          type="button"
          class="seg-btn {taskKind === 'custom_ask' ? 'active' : ''}"
          onclick={() => (taskKind = "custom_ask")}
        >
          {t("ai.modeAsk")}
        </button>
        <button
          type="button"
          class="seg-btn {taskKind === 'review_changes' ? 'active' : ''}"
          onclick={() => (taskKind = "review_changes")}
        >
          {t("ai.modeReview")}
        </button>
        <button
          type="button"
          class="seg-btn {taskKind === 'explain_diff' ? 'active' : ''}"
          onclick={() => (taskKind = "explain_diff")}
        >
          {t("ai.modeExplainDiff")}
        </button>
        <button
          type="button"
          class="seg-btn {taskKind === 'explain_commit' ? 'active' : ''}"
          onclick={() => (taskKind = "explain_commit")}
        >
          {t("ai.modeExplainCommit")}
        </button>
      </div>

      <div class="form-field">
        <label class="form-label" for="ai-prompt-input">{t("ai.promptLabel")}</label>
        <textarea
          id="ai-prompt-input"
          class="form-input"
          rows="3"
          bind:value={prompt}
          placeholder={t("ai.promptPlaceholder")}
          onkeydown={(e) => {
            if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
              e.preventDefault();
              void runAi();
            }
          }}
        ></textarea>
      </div>

      {#if repoPath}
        <label class="checkbox-label">
          <input type="checkbox" bind:checked={includeRepoContext} />
          <span>{t("ai.includeRepoContext")}</span>
        </label>
      {/if}

      {#if errorMsg}
        <div class="alert-banner" style="border-radius: var(--radius-sm);">
          <span>{errorMsg}</span>
        </div>
      {/if}

      {#if answer}
        <div class="ai-answer-box">
          <div class="ai-answer-header">
            <span>{t("ai.responseHeader")}</span>
            <button
              type="button"
              class="btn-secondary btn-xs"
              onclick={() => {
                void navigator.clipboard.writeText(answer);
                onStatus?.(t("ai.copiedResponse"));
              }}
            >
              {t("ai.copyBtn")}
            </button>
          </div>
          <pre class="ai-answer-text">{answer}</pre>
        </div>
      {/if}
    </div>

    <div class="modal-actions">
      <button type="button" class="btn-secondary" onclick={onClose}>{t("common.close")}</button>
      <button type="button" class="btn-primary" disabled={running} onclick={runAi}>
        {running ? t("ai.runningBtn") : t("ai.sendBtn")}
      </button>
    </div>
  </div>
</div>
