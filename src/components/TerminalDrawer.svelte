<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import * as gitService from "../services/git";
  import { t } from "../i18n/index.svelte";

  interface Props {
    repoPath: string;
    onClose: () => void;
    onRefreshRepo?: () => Promise<void> | void;
  }

  let { repoPath, onClose, onRefreshRepo }: Props = $props();

  interface LogChunk {
    id: number;
    stream: "stdout" | "stderr" | "system" | "input";
    text: string;
  }

  const sessionId = `term-${Math.random().toString(36).slice(2, 10)}`;
  let shellKind = $state("auto");
  let cwd = $state("");
  let commandInput = $state("");
  let running = $state(false);
  let chunks = $state<LogChunk[]>([]);
  let cmdHistory = $state<string[]>([]);
  let historyIdx = $state(-1);
  let seq = 0;
  let outputEl = $state<HTMLDivElement | null>(null);
  let inputEl = $state<HTMLInputElement | null>(null);

  function pushChunk(stream: LogChunk["stream"], text: string) {
    chunks = [...chunks.slice(-400), { id: ++seq, stream, text }];
    queueMicrotask(() => {
      if (outputEl) {
        outputEl.scrollTop = outputEl.scrollHeight;
      }
    });
  }

  $effect(() => {
    if (!repoPath) return;
    cwd = repoPath;
    void gitService
      .getTerminalCwd(sessionId, repoPath)
      .then((c) => {
        if (c) cwd = c;
      })
      .catch(() => {});
  });

  $effect(() => {
    let unlistenOut: (() => void) | null = null;
    let unlistenExit: (() => void) | null = null;
    let mounted = true;

    void listen<{ session_id: string; stream: string; text: string }>(
      "terminal-output",
      (ev) => {
        if (!mounted || ev.payload.session_id !== sessionId) return;
        const st =
          ev.payload.stream === "stderr"
            ? "stderr"
            : ev.payload.stream === "system"
              ? "system"
              : "stdout";
        pushChunk(st, ev.payload.text);
      }
    ).then((fn) => {
      unlistenOut = fn;
    });

    void listen<{ session_id: string; exit_code: number; cwd: string }>(
      "terminal-exit",
      (ev) => {
        if (!mounted || ev.payload.session_id !== sessionId) return;
        cwd = ev.payload.cwd || cwd;
      }
    ).then((fn) => {
      unlistenExit = fn;
    });

    return () => {
      mounted = false;
      unlistenOut?.();
      unlistenExit?.();
    };
  });

  async function runCmd(raw?: string) {
    const cmd = (raw ?? commandInput).trim();
    if (!cmd || running) return;
    if (!raw) commandInput = "";
    cmdHistory = [cmd, ...cmdHistory.filter((c) => c !== cmd)].slice(0, 50);
    historyIdx = -1;
    pushChunk("input", `${cwd}> ${cmd}\n`);
    running = true;
    try {
      const res = await gitService.runTerminalCommand(sessionId, repoPath, cmd, shellKind);
      cwd = res.cwd || cwd;
      if (res.exit_code !== 0) {
        pushChunk("system", `[exit ${res.exit_code}]\n`);
      }
      if (/^(git|gh)\b/i.test(cmd) && onRefreshRepo) {
        await onRefreshRepo();
      }
    } catch (err) {
      pushChunk("stderr", `${String(err)}\n`);
    } finally {
      running = false;
      queueMicrotask(() => inputEl?.focus());
    }
  }

  async function handleKill() {
    if (!running) return;
    try {
      await gitService.killTerminalCommand(sessionId);
      pushChunk("system", "[killed]\n");
    } catch (err) {
      pushChunk("stderr", `${String(err)}\n`);
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") {
      e.preventDefault();
      void runCmd();
    } else if (e.key === "ArrowUp") {
      if (cmdHistory.length === 0) return;
      e.preventDefault();
      historyIdx = Math.min(cmdHistory.length - 1, historyIdx + 1);
      commandInput = cmdHistory[historyIdx];
    } else if (e.key === "ArrowDown") {
      if (historyIdx <= 0) {
        historyIdx = -1;
        commandInput = "";
      } else {
        historyIdx -= 1;
        commandInput = cmdHistory[historyIdx];
      }
    }
  }
</script>

<section class="terminal-drawer" aria-label={t("terminal.title")}>
  <div class="terminal-header">
    <div style="display: flex; align-items: center; gap: 8px; min-width: 0;">
      <span class="terminal-title">{t("terminal.title")}</span>
      <span class="terminal-cwd" title={cwd}>{cwd}</span>
    </div>
    <div style="display: flex; align-items: center; gap: 6px;">
      <select class="form-select terminal-shell-select" bind:value={shellKind} aria-label={t("terminal.shellLabel")}>
        <option value="auto">Auto Shell</option>
        <option value="powershell">PowerShell</option>
        <option value="cmd">CMD</option>
        <option value="bash">Bash</option>
        <option value="sh">sh</option>
      </select>
      <button type="button" class="btn-secondary btn-xs" onclick={() => runCmd("git status -sb")} disabled={running}>
        git status
      </button>
      {#if running}
        <button type="button" class="btn-secondary btn-xs btn-danger" onclick={handleKill}>
          {t("terminal.killBtn")}
        </button>
      {/if}
      <button type="button" class="btn-secondary btn-xs" onclick={() => (chunks = [])}>
        {t("terminal.clearBtn")}
      </button>
      <button type="button" class="btn-secondary btn-xs" onclick={onClose}>
        {t("common.close")}
      </button>
    </div>
  </div>

  <div bind:this={outputEl} class="terminal-output">
    {#if chunks.length === 0}
      <div class="terminal-empty">{t("terminal.emptyHint")}</div>
    {:else}
      {#each chunks as c (c.id)}
        <pre class="terminal-chunk stream-{c.stream}">{c.text}</pre>
      {/each}
    {/if}
  </div>

  <div class="terminal-input-row">
    <span class="terminal-prompt">$</span>
    <input
      bind:this={inputEl}
      type="text"
      class="terminal-input"
      bind:value={commandInput}
      onkeydown={handleKeydown}
      placeholder={t("terminal.inputPlaceholder")}
      disabled={running}
    />
    <button
      type="button"
      class="btn-primary btn-xs"
      disabled={running || !commandInput.trim()}
      onclick={() => void runCmd()}
    >
      {running ? t("terminal.runningBtn") : t("terminal.runBtn")}
    </button>
  </div>
</section>
