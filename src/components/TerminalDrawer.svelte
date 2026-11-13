<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import * as gitService from "../services/git";
  import { t } from "../i18n/index.svelte";
  import { parseAnsiText, stripAnsi, type AnsiSpan } from "../lib/ansi";

  interface Props {
    repoPath: string;
    currentBranch?: string;
    onClose: () => void;
    onRefreshRepo?: () => Promise<void> | void;
  }

  let { repoPath, currentBranch, onClose, onRefreshRepo }: Props = $props();

  interface LogChunk {
    id: number;
    stream: "stdout" | "stderr" | "system" | "input";
    text: string;
  }

  interface TerminalSession {
    id: string;
    title: string;
    cwd: string;
    running: boolean;
    activeCmd: string;
    chunks: LogChunk[];
    cmdHistory: string[];
    historyIdx: number;
    exitCode: number | null;
    shellKind: string;
    startTime: number | null;
  }

  const QUICK_ACTIONS = [
    "git status -sb",
    "git log --oneline -5",
    "git diff --stat",
    "git branch -av",
    "git fetch",
  ];

  const GIT_VERBS = [
    "status",
    "log",
    "diff",
    "checkout",
    "switch",
    "branch",
    "commit",
    "push",
    "pull",
    "fetch",
    "merge",
    "rebase",
    "stash",
    "reset",
    "restore",
    "add",
    "remote",
    "tag",
    "clean",
    "show",
  ];

  let seq = 0;
  let tabCounter = 1;

  function createSession(tabNum: number, startCwd: string): TerminalSession {
    const id = `term-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`;
    const baseName = startCwd.split(/[/\\]/).filter(Boolean).pop() || "repo";
    return {
      id,
      title: `${baseName} (${tabNum})`,
      cwd: startCwd,
      running: false,
      activeCmd: "",
      chunks: [],
      cmdHistory: [],
      historyIdx: -1,
      exitCode: null,
      shellKind: "auto",
      startTime: null,
    };
  }

  function loadInitialHeight(): number {
    try {
      const raw = localStorage.getItem("forge_terminal_height");
      if (raw) {
        const val = parseInt(raw, 10);
        if (!isNaN(val) && val >= 140 && val <= window.innerHeight * 0.85) {
          return val;
        }
      }
    } catch {}
    return 240;
  }

  // State
  let height = $state(loadInitialHeight());
  let isMaximized = $state(false);
  let isResizing = $state(false);
  let startY = 0;
  let startH = 0;

  const initialTermId = `term-${Date.now()}`;
  let sessions = $state<TerminalSession[]>([
    {
      id: initialTermId,
      title: "1",
      cwd: "",
      running: false,
      activeCmd: "",
      chunks: [],
      cmdHistory: [],
      historyIdx: -1,
      exitCode: null,
      shellKind: "auto",
      startTime: null,
    },
  ]);
  let activeId = $state<string>(initialTermId);
  let activeSession = $derived(sessions.find((s) => s.id === activeId) ?? sessions[0]);

  let commandInput = $state("");
  let searchOpen = $state(false);
  let searchQuery = $state("");
  let copiedCwd = $state(false);
  let copiedOutput = $state(false);
  let userScrolledUp = $state(false);
  let hasNewOutputBelow = $state(false);
  let now = $state(Date.now());

  let outputEl = $state<HTMLDivElement | null>(null);
  let inputEl = $state<HTMLInputElement | null>(null);
  let searchInputEl = $state<HTMLInputElement | null>(null);

  // Interval for elapsed time counter when command running
  $effect(() => {
    const hasRunning = sessions.some((s) => s.running);
    if (!hasRunning) return;
    const interval = setInterval(() => {
      now = Date.now();
    }, 1000);
    return () => clearInterval(interval);
  });

  // Fetch initial cwd for session
  function refreshSessionCwd(session: TerminalSession) {
    if (!session) return;
    void gitService
      .getTerminalCwd(session.id, repoPath)
      .then((c) => {
        if (c) session.cwd = c;
      })
      .catch(() => {});
  }

  $effect(() => {
    if (!repoPath) return;
    for (const s of sessions) {
      if (!s.cwd) {
        s.cwd = repoPath;
        const baseName = repoPath.split(/[/\\]/).filter(Boolean).pop() || "repo";
        s.title = `${baseName} (1)`;
      }
      refreshSessionCwd(s);
    }
  });

  // Listen to Tauri terminal events
  $effect(() => {
    let unlistenOut: (() => void) | null = null;
    let unlistenExit: (() => void) | null = null;
    let mounted = true;

    void listen<{ session_id: string; stream: string; text: string }>(
      "terminal-output",
      (ev) => {
        if (!mounted) return;
        const target = sessions.find((s) => s.id === ev.payload.session_id);
        if (!target) return;
        const stream =
          ev.payload.stream === "stderr"
            ? "stderr"
            : ev.payload.stream === "system"
              ? "system"
              : "stdout";
        pushChunkToSession(target, stream, ev.payload.text);
      }
    ).then((fn) => {
      unlistenOut = fn;
    });

    void listen<{ session_id: string; exit_code: number; cwd: string }>(
      "terminal-exit",
      (ev) => {
        if (!mounted) return;
        const target = sessions.find((s) => s.id === ev.payload.session_id);
        if (!target) return;
        if (ev.payload.cwd) target.cwd = ev.payload.cwd;
        target.exitCode = ev.payload.exit_code;
        target.running = false;
        target.startTime = null;
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

  function pushChunkToSession(
    session: TerminalSession,
    stream: LogChunk["stream"],
    text: string
  ) {
    session.chunks = [...session.chunks.slice(-400), { id: ++seq, stream, text }];

    if (session.id === activeId) {
      if (!userScrolledUp) {
        queueMicrotask(() => {
          if (outputEl) outputEl.scrollTop = outputEl.scrollHeight;
        });
      } else {
        hasNewOutputBelow = true;
      }
    }
  }

  function handleOutputScroll() {
    if (!outputEl) return;
    const distFromBottom =
      outputEl.scrollHeight - outputEl.scrollTop - outputEl.clientHeight;
    const isUp = distFromBottom > 35;
    userScrolledUp = isUp;
    if (!isUp) {
      hasNewOutputBelow = false;
    }
  }

  function scrollToBottom() {
    if (!outputEl) return;
    outputEl.scrollTop = outputEl.scrollHeight;
    userScrolledUp = false;
    hasNewOutputBelow = false;
  }

  // Resizer pointer events
  function handleResizerPointerDown(e: PointerEvent) {
    isResizing = true;
    startY = e.clientY;
    startH = height;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }

  function handleResizerPointerMove(e: PointerEvent) {
    if (!isResizing) return;
    const delta = startY - e.clientY;
    const maxH = Math.floor(window.innerHeight - 150);
    height = Math.max(140, Math.min(maxH, startH + delta));
    if (isMaximized) isMaximized = false;
  }

  function handleResizerPointerUp() {
    if (!isResizing) return;
    isResizing = false;
    try {
      localStorage.setItem("forge_terminal_height", String(height));
    } catch {}
  }

  function toggleMaximize() {
    isMaximized = !isMaximized;
  }

  // Tab management
  function addTab() {
    tabCounter += 1;
    const newSession = createSession(tabCounter, activeSession?.cwd || repoPath);
    sessions = [...sessions, newSession];
    activeId = newSession.id;
    refreshSessionCwd(newSession);
    userScrolledUp = false;
    hasNewOutputBelow = false;
    queueMicrotask(() => inputEl?.focus());
  }

  async function closeTab(sessionId: string) {
    if (sessions.length <= 1) return;
    const target = sessions.find((s) => s.id === sessionId);
    if (target?.running) {
      try {
        await gitService.killTerminalCommand(sessionId);
      } catch {}
    }
    const idx = sessions.findIndex((s) => s.id === sessionId);
    sessions = sessions.filter((s) => s.id !== sessionId);
    if (activeId === sessionId) {
      const nextIdx = Math.max(0, idx - 1);
      activeId = sessions[nextIdx].id;
    }
    userScrolledUp = false;
    hasNewOutputBelow = false;
    queueMicrotask(() => inputEl?.focus());
  }

  function switchTab(sessionId: string) {
    activeId = sessionId;
    userScrolledUp = false;
    hasNewOutputBelow = false;
    queueMicrotask(() => {
      if (outputEl) outputEl.scrollTop = outputEl.scrollHeight;
      inputEl?.focus();
    });
  }

  // Command Execution
  async function runCmd(raw?: string) {
    const session = activeSession;
    if (!session) return;
    const cmd = (raw ?? commandInput).trim();
    if (!cmd || session.running) return;

    if (!raw) commandInput = "";
    session.cmdHistory = [cmd, ...session.cmdHistory.filter((c) => c !== cmd)].slice(0, 50);
    session.historyIdx = -1;
    session.running = true;
    session.activeCmd = cmd;
    session.startTime = Date.now();
    session.exitCode = null;

    // Derive tab title from command
    const firstWord = cmd.split(/\s+/)[0];
    if (firstWord) {
      session.title = firstWord;
    }

    pushChunkToSession(session, "input", `${session.cwd}> ${cmd}\n`);

    try {
      const res = await gitService.runTerminalCommand(
        session.id,
        session.cwd || repoPath,
        cmd,
        session.shellKind
      );
      if (res.cwd) session.cwd = res.cwd;
      session.exitCode = res.exit_code;
      if (res.exit_code !== 0) {
        pushChunkToSession(session, "system", `[exit ${res.exit_code}]\n`);
      }
      if (/^(git|gh)\b/i.test(cmd) && onRefreshRepo) {
        await onRefreshRepo();
      }
    } catch (err) {
      pushChunkToSession(session, "stderr", `${String(err)}\n`);
      session.exitCode = -1;
    } finally {
      session.running = false;
      session.startTime = null;
      queueMicrotask(() => inputEl?.focus());
    }
  }

  async function handleKill() {
    const session = activeSession;
    if (!session?.running) return;
    try {
      await gitService.killTerminalCommand(session.id);
      pushChunkToSession(session, "system", "[killed]\n");
      session.running = false;
      session.startTime = null;
      session.exitCode = 130;
    } catch (err) {
      pushChunkToSession(session, "stderr", `${String(err)}\n`);
    }
  }

  function handleTabAutocomplete() {
    const trimmed = commandInput.trim();
    if (trimmed === "git") {
      commandInput = "git status ";
      return;
    }

    const match = commandInput.match(/^git\s+([a-z]*)$/i);
    if (!match) return;

    const prefix = match[1].toLowerCase();
    const candidates = GIT_VERBS.filter((v) => v.startsWith(prefix));

    if (candidates.length === 1) {
      commandInput = `git ${candidates[0]} `;
    } else if (candidates.length > 1) {
      let common = prefix;
      for (let i = prefix.length; ; i++) {
        const char = candidates[0][i];
        if (!char) break;
        if (candidates.every((c) => c[i] === char)) {
          common += char;
        } else {
          break;
        }
      }
      if (common.length > prefix.length) {
        commandInput = `git ${common}`;
      } else {
        commandInput = `git ${candidates[0]} `;
      }
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    const session = activeSession;
    if (!session) return;

    if (e.key === "Enter") {
      e.preventDefault();
      void runCmd();
    } else if (e.key === "Tab") {
      e.preventDefault();
      handleTabAutocomplete();
    } else if (e.key === "ArrowUp") {
      if (session.cmdHistory.length === 0) return;
      e.preventDefault();
      session.historyIdx = Math.min(session.cmdHistory.length - 1, session.historyIdx + 1);
      commandInput = session.cmdHistory[session.historyIdx];
    } else if (e.key === "ArrowDown") {
      if (session.historyIdx <= 0) {
        session.historyIdx = -1;
        commandInput = "";
      } else {
        session.historyIdx -= 1;
        commandInput = session.cmdHistory[session.historyIdx];
      }
    } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "c" && session.running) {
      const sel = window.getSelection()?.toString();
      if (!sel) {
        e.preventDefault();
        void handleKill();
      }
    } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "l") {
      e.preventDefault();
      session.chunks = [];
    } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "f") {
      e.preventDefault();
      toggleSearch();
    }
  }

  function handleContainerKeydown(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key === "PageDown") {
      e.preventDefault();
      const currIdx = sessions.findIndex((s) => s.id === activeId);
      const nextIdx = (currIdx + 1) % sessions.length;
      switchTab(sessions[nextIdx].id);
    } else if ((e.ctrlKey || e.metaKey) && e.key === "PageUp") {
      e.preventDefault();
      const currIdx = sessions.findIndex((s) => s.id === activeId);
      const prevIdx = (currIdx - 1 + sessions.length) % sessions.length;
      switchTab(sessions[prevIdx].id);
    } else if (e.altKey && e.key >= "1" && e.key <= "9") {
      const targetIdx = parseInt(e.key, 10) - 1;
      if (targetIdx < sessions.length) {
        e.preventDefault();
        switchTab(sessions[targetIdx].id);
      }
    }
  }

  function toggleSearch() {
    searchOpen = !searchOpen;
    if (searchOpen) {
      queueMicrotask(() => searchInputEl?.focus());
    } else {
      searchQuery = "";
      queueMicrotask(() => inputEl?.focus());
    }
  }

  async function copyCwd() {
    if (!activeSession?.cwd) return;
    try {
      await navigator.clipboard.writeText(activeSession.cwd);
      copiedCwd = true;
      setTimeout(() => {
        copiedCwd = false;
      }, 1500);
    } catch {}
  }

  async function copyOutputBuffer() {
    if (!activeSession) return;
    const fullText = activeSession.chunks.map((c) => stripAnsi(c.text)).join("");
    try {
      await navigator.clipboard.writeText(fullText);
      copiedOutput = true;
      setTimeout(() => {
        copiedOutput = false;
      }, 1500);
    } catch {}
  }

  // Format cwd for display
  function formatDisplayCwd(fullPath: string): string {
    if (!fullPath) return "";
    if (fullPath === repoPath) {
      return fullPath.split(/[/\\]/).filter(Boolean).pop() || fullPath;
    }
    if (fullPath.startsWith(repoPath)) {
      const rel = fullPath.slice(repoPath.length).replace(/^[/\\]+/, "");
      return rel ? `./${rel}` : fullPath;
    }
    return fullPath;
  }

  // Format elapsed time
  function formatElapsed(start: number | null): string {
    if (!start) return "";
    const secs = Math.max(0, Math.floor((now - start) / 1000));
    const mins = Math.floor(secs / 60);
    const remSecs = secs % 60;
    return `${mins}:${remSecs.toString().padStart(2, "0")}`;
  }

  // ANSI parse cache to avoid retokenizing on every render
  const ansiCache = new Map<string, AnsiSpan[]>();
  function spansOf(text: string): AnsiSpan[] {
    let spans = ansiCache.get(text);
    if (!spans) {
      spans = parseAnsiText(text);
      if (ansiCache.size > 800) ansiCache.clear();
      ansiCache.set(text, spans);
    }
    return spans;
  }

  interface Segment {
    text: string;
    hit: boolean;
  }

  function splitByQuery(text: string, query: string): Segment[] {
    if (!query) return [{ text, hit: false }];
    const out: Segment[] = [];
    const haystack = text.toLowerCase();
    const needle = query.toLowerCase();
    let cursor = 0;
    while (true) {
      const idx = haystack.indexOf(needle, cursor);
      if (idx === -1) break;
      if (idx > cursor) out.push({ text: text.slice(cursor, idx), hit: false });
      out.push({ text: text.slice(idx, idx + needle.length), hit: true });
      cursor = idx + needle.length;
    }
    if (cursor < text.length) out.push({ text: text.slice(cursor), hit: false });
    return out;
  }

  // Helper for search match count
  let searchMatchCount = $derived.by(() => {
    const q = searchQuery.trim().toLowerCase();
    if (!q || !activeSession) return 0;
    let count = 0;
    for (const c of activeSession.chunks) {
      const plain = stripAnsi(c.text).toLowerCase();
      let pos = 0;
      while ((pos = plain.indexOf(q, pos)) !== -1) {
        count += 1;
        pos += q.length;
      }
    }
    return count;
  });
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<section
  class="terminal-drawer {isMaximized ? 'is-maximized' : ''}"
  style={isMaximized ? undefined : `height: ${height}px;`}
  aria-label={t("terminal.title")}
  onkeydown={handleContainerKeydown}
>
  <!-- Vertical resizer handle -->
  {#if !isMaximized}
    <div
      class="terminal-resizer {isResizing ? 'is-resizing' : ''}"
      onpointerdown={handleResizerPointerDown}
      onpointermove={handleResizerPointerMove}
      onpointerup={handleResizerPointerUp}
      onpointercancel={handleResizerPointerUp}
      role="separator"
      aria-label="Resize terminal"
      title="Drag to resize terminal"
    ></div>
  {/if}

  <!-- Terminal Header Toolbar -->
  <div class="terminal-header">
    <div class="terminal-header-left">
      <!-- Tabs strip -->
      <div class="terminal-tabs-strip" role="tablist">
        {#each sessions as s, i (s.id)}
          <div
            role="tab"
            tabindex="0"
            aria-selected={s.id === activeId}
            class="terminal-tab {s.id === activeId ? 'is-active' : ''}"
            onclick={() => switchTab(s.id)}
            onkeydown={(e) => {
              if (e.key === "Enter" || e.key === " ") {
                e.preventDefault();
                switchTab(s.id);
              }
            }}
            title="Alt+{i + 1}: {s.cwd}"
          >
            <span
              class="terminal-tab-dot {s.running ? 'is-running' : s.exitCode && s.exitCode !== 0 ? 'is-error' : ''}"
            ></span>
            <span class="terminal-tab-icon">
              <svg
                width="11"
                height="11"
                viewBox="0 0 16 16"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
                aria-hidden="true"
              >
                <polyline points="4 5 8 8 4 11" />
                <line x1="9" y1="11" x2="13" y2="11" />
              </svg>
            </span>
            <span class="terminal-tab-title">{i + 1}: {s.title}</span>
            {#if sessions.length > 1}
              <button
                type="button"
                class="terminal-tab-close"
                onclick={(e) => {
                  e.stopPropagation();
                  void closeTab(s.id);
                }}
                title={t("terminal.closeTab")}
                aria-label={t("terminal.closeTab")}
              >
                <svg
                  width="10"
                  height="10"
                  viewBox="0 0 16 16"
                  fill="currentColor"
                  aria-hidden="true"
                >
                  <path
                    d="M3.72 3.72a.75.75 0 0 1 1.06 0L8 6.94l3.22-3.22a.749.749 0 0 1 1.275.326.749.749 0 0 1-.215.734L9.06 8l3.22 3.22a.749.749 0 0 1-.326 1.275.749.749 0 0 1-.734-.215L8 9.06l-3.22 3.22a.751.751 0 0 1-1.042-.018.751.751 0 0 1-.018-1.042L6.94 8 3.72 4.78a.75.75 0 0 1 0-1.06Z"
                  />
                </svg>
              </button>
            {/if}
          </div>
        {/each}
        <button
          type="button"
          class="terminal-tab-add-btn"
          onclick={addTab}
          title={t("terminal.newTab")}
          aria-label={t("terminal.newTab")}
        >
          <svg
            width="11"
            height="11"
            viewBox="0 0 16 16"
            fill="currentColor"
            aria-hidden="true"
          >
            <path
              d="M7.75 2a.75.75 0 0 1 .75.75V7h4.25a.75.75 0 0 1 0 1.5H8.5v4.25a.75.75 0 0 1-1.5 0V8.5H2.75a.75.75 0 0 1 0-1.5H7V2.75A.75.75 0 0 1 7.75 2Z"
            />
          </svg>
        </button>
      </div>
    </div>

    <!-- Header Actions Right -->
    <div class="terminal-header-right">
      <!-- Process Status Badge -->
      {#if activeSession.running}
        <div class="terminal-status-badge is-running" title="Command running">
          <span class="terminal-tab-dot is-running"></span>
          <span>{activeSession.activeCmd.split(/\s+/)[0]} {formatElapsed(activeSession.startTime)}</span>
        </div>
      {:else if activeSession.exitCode !== null}
        <div
          class="terminal-status-badge {activeSession.exitCode === 0 ? '' : 'is-error'}"
          title="Last exit code: {activeSession.exitCode}"
        >
          <span>Exit {activeSession.exitCode}</span>
        </div>
      {/if}

      <!-- Shell selection -->
      <select
        class="form-select terminal-shell-select"
        value={activeSession.shellKind}
        onchange={(e) => {
          activeSession.shellKind = (e.currentTarget as HTMLSelectElement).value;
        }}
        aria-label={t("terminal.shellLabel")}
      >
        <option value="auto">Auto Shell</option>
        <option value="powershell">PowerShell</option>
        <option value="cmd">CMD</option>
        <option value="bash">Bash</option>
        <option value="sh">sh</option>
      </select>

      <!-- Kill Process button -->
      {#if activeSession.running}
        <button
          type="button"
          class="terminal-btn btn-danger"
          onclick={handleKill}
          title="Kill active process (Ctrl+C)"
        >
          <svg
            width="10"
            height="10"
            viewBox="0 0 16 16"
            fill="currentColor"
            aria-hidden="true"
          >
            <rect x="2" y="2" width="12" height="12" rx="2" />
          </svg>
          <span class="terminal-btn-label">{t("terminal.killBtn")}</span>
        </button>
      {/if}

      <!-- Search toggle button -->
      <button
        type="button"
        class="terminal-btn {searchOpen ? 'is-highlighted' : ''}"
        onclick={toggleSearch}
        title="Search output (Ctrl+F)"
        aria-label="Search output"
      >
        <svg
          width="12"
          height="12"
          viewBox="0 0 16 16"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
          aria-hidden="true"
        >
          <circle cx="7" cy="7" r="4.5" />
          <line x1="10.5" y1="10.5" x2="14" y2="14" />
        </svg>
      </button>

      <!-- Copy output button -->
      <button
        type="button"
        class="terminal-btn"
        onclick={copyOutputBuffer}
        title={t("terminal.copyOutput")}
      >
        {#if copiedOutput}
          <svg
            width="11"
            height="11"
            viewBox="0 0 16 16"
            fill="currentColor"
            aria-hidden="true"
            style="color: var(--diff-add-text);"
          >
            <path
              d="M13.78 4.22a.75.75 0 0 1 0 1.06l-7.25 7.25a.75.75 0 0 1-1.06 0L2.22 9.28a.751.751 0 0 1 .018-1.042.751.751 0 0 1 1.042-.018L6 10.94l6.72-6.72a.75.75 0 0 1 1.06 0Z"
            />
          </svg>
          <span class="terminal-btn-label">{t("terminal.copiedOutput")}</span>
        {:else}
          <svg
            width="11"
            height="11"
            viewBox="0 0 16 16"
            fill="currentColor"
            aria-hidden="true"
          >
            <path
              d="M0 6.75C0 5.784.784 5 1.75 5h1.5a.75.75 0 0 1 0 1.5h-1.5a.25.25 0 0 0-.25.25v7.5c0 .138.112.25.25.25h7.5a.25.25 0 0 0 .25-.25v-1.5a.75.75 0 0 1 1.5 0v1.5A1.75 1.75 0 0 1 9.25 16h-7.5A1.75 1.75 0 0 1 0 14.25Z"
            /><path
              d="M5 1.75C5 .784 5.784 0 6.75 0h7.5C15.216 0 16 .784 16 1.75v7.5A1.75 1.75 0 0 1 14.25 11h-7.5A1.75 1.75 0 0 1 5 9.25Zm1.75-.25a.25.25 0 0 0-.25.25v7.5c0 .138.112.25.25.25h7.5a.25.25 0 0 0 .25-.25v-7.5a.25.25 0 0 0-.25-.25Z"
            />
          </svg>
          <span class="terminal-btn-label">{t("terminal.copyOutput")}</span>
        {/if}
      </button>

      <!-- Clear buffer button -->
      <button
        type="button"
        class="terminal-btn"
        onclick={() => (activeSession.chunks = [])}
        title="Clear output (Ctrl+L)"
      >
        <svg
          width="11"
          height="11"
          viewBox="0 0 16 16"
          fill="currentColor"
          aria-hidden="true"
        >
          <path
            d="M11 1.75V3h2.25a.75.75 0 0 1 0 1.5H2.75a.75.75 0 0 1 0-1.5H5V1.75C5 .784 5.784 0 6.75 0h2.5C10.216 0 11 .784 11 1.75ZM4.496 6.675l.66 6.6a1.75 1.75 0 0 0 1.744 1.575h2.2a1.75 1.75 0 0 0 1.744-1.575l.66-6.6a.75.75 0 1 0-1.492-.15l-.66 6.6a.25.25 0 0 1-.249.225h-2.2a.25.25 0 0 1-.249-.225l-.66-6.6a.75.75 0 0 0-1.492.15ZM6.5 1.75V3h3V1.75a.25.25 0 0 0-.25-.25h-2.5a.25.25 0 0 0-.25.25Z"
          />
        </svg>
        <span class="terminal-btn-label">{t("terminal.clearBtn")}</span>
      </button>

      <!-- Maximize / Restore button -->
      <button
        type="button"
        class="terminal-btn icon-only"
        onclick={toggleMaximize}
        title={isMaximized ? t("terminal.restore") : t("terminal.maximize")}
        aria-label={isMaximized ? t("terminal.restore") : t("terminal.maximize")}
      >
        {#if isMaximized}
          <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
            <path
              d="M4 2.5A1.5 1.5 0 0 1 5.5 1h6A1.5 1.5 0 0 1 13 2.5V8a1.5 1.5 0 0 1-1.5 1.5H11v2a1.5 1.5 0 0 1-1.5 1.5h-6A1.5 1.5 0 0 1 2 11.5V6a1.5 1.5 0 0 1 1.5-1.5H5V2.5A1.5 1.5 0 0 1 4 2.5Zm1.5 3V8a1.5 1.5 0 0 1 1.5 1.5H9.5V2.5H5.5v3ZM3.5 6v5.5h6V6h-6Z"
            />
          </svg>
        {:else}
          <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
            <path
              d="M3 2.5A1.5 1.5 0 0 1 4.5 1h7A1.5 1.5 0 0 1 13 2.5v11a1.5 1.5 0 0 1-1.5 1.5h-7A1.5 1.5 0 0 1 3 13.5v-11Zm1.5 0v11h7v-11h-7Z"
            />
          </svg>
        {/if}
      </button>

      <!-- Close drawer button -->
      <button
        type="button"
        class="terminal-btn icon-only"
        onclick={onClose}
        title={t("common.close")}
        aria-label={t("common.close")}
      >
        <svg
          width="12"
          height="12"
          viewBox="0 0 16 16"
          fill="currentColor"
          aria-hidden="true"
        >
          <path
            d="M3.72 3.72a.75.75 0 0 1 1.06 0L8 6.94l3.22-3.22a.749.749 0 0 1 1.275.326.749.749 0 0 1-.215.734L9.06 8l3.22 3.22a.749.749 0 0 1-.326 1.275.749.749 0 0 1-.734-.215L8 9.06l-3.22 3.22a.751.751 0 0 1-1.042-.018.751.751 0 0 1-.018-1.042L6.94 8 3.72 4.78a.75.75 0 0 1 0-1.06Z"
          />
        </svg>
      </button>
    </div>
  </div>

  <!-- Search Bar Drawer -->
  {#if searchOpen}
    <div class="terminal-search-bar">
      <input
        bind:this={searchInputEl}
        type="text"
        class="terminal-search-input"
        bind:value={searchQuery}
        placeholder={t("terminal.searchPlaceholder")}
        onkeydown={(e) => {
          if (e.key === "Escape") {
            e.preventDefault();
            toggleSearch();
          }
        }}
      />
      {#if searchQuery.trim()}
        <span class="terminal-search-count">
          {searchMatchCount} match{searchMatchCount === 1 ? "" : "es"}
        </span>
      {/if}
      <button
        type="button"
        class="terminal-btn icon-only"
        onclick={toggleSearch}
        title="Close search"
        aria-label="Close search"
      >
        <svg
          width="11"
          height="11"
          viewBox="0 0 16 16"
          fill="currentColor"
          aria-hidden="true"
        >
          <path
            d="M3.72 3.72a.75.75 0 0 1 1.06 0L8 6.94l3.22-3.22a.749.749 0 0 1 1.275.326.749.749 0 0 1-.215.734L9.06 8l3.22 3.22a.749.749 0 0 1-.326 1.275.749.749 0 0 1-.734-.215L8 9.06l-3.22 3.22a.751.751 0 0 1-1.042-.018.751.751 0 0 1-.018-1.042L6.94 8 3.72 4.78a.75.75 0 0 1 0-1.06Z"
          />
        </svg>
      </button>
    </div>
  {/if}

  <!-- Terminal Output Area -->
  <div bind:this={outputEl} class="terminal-output" onscroll={handleOutputScroll}>
    {#if activeSession.chunks.length === 0}
      <div class="terminal-empty">
        <p style="margin: 0 0 8px 0;">{t("terminal.emptyHint")}</p>
        <div style="display: flex; flex-wrap: wrap; gap: 6px; align-items: center;">
          <span style="font-size: 10.5px; color: var(--text-muted);">{t("terminal.quickActions")}:</span>
          {#each QUICK_ACTIONS as action}
            <button
              type="button"
              class="terminal-quick-chip"
              disabled={activeSession.running}
              onclick={() => void runCmd(action)}
            >
              {action}
            </button>
          {/each}
        </div>
      </div>
    {:else}
      {#each activeSession.chunks as c (c.id)}
        {#if c.stream === "input"}
          <pre class="terminal-chunk stream-input">{c.text}</pre>
        {:else}
          <pre class="terminal-chunk stream-{c.stream}">{#each spansOf(c.text) as span}{#if searchQuery.trim() && span.text.toLowerCase().includes(searchQuery.toLowerCase())}{#each splitByQuery(span.text, searchQuery) as seg}{#if seg.hit}<mark class="terminal-search-match">{seg.text}</mark>{:else}<span class={span.classes}>{seg.text}</span>{/if}{/each}{:else}<span class={span.classes}>{span.text}</span>{/if}{/each}</pre>
        {/if}
      {/each}
    {/if}

    <!-- Floating New Output Badge when user scrolled up -->
    {#if hasNewOutputBelow && userScrolledUp}
      <button type="button" class="terminal-scroll-badge" onclick={scrollToBottom}>
        <svg
          width="11"
          height="11"
          viewBox="0 0 16 16"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
          aria-hidden="true"
        >
          <line x1="8" y1="3" x2="8" y2="13"/>
          <polyline points="4 9 8 13 12 9"/>
        </svg>
        <span>{t("terminal.newOutputBadge")}</span>
      </button>
    {/if}
  </div>

  <!-- Quick Action Chips Bar (when chunks exist) -->
  {#if activeSession.chunks.length > 0}
    <div class="terminal-quick-chips">
      {#each QUICK_ACTIONS as action}
        <button
          type="button"
          class="terminal-quick-chip"
          disabled={activeSession.running}
          onclick={() => void runCmd(action)}
        >
          {action}
        </button>
      {/each}
    </div>
  {/if}

  <!-- Terminal Input Row -->
  <div class="terminal-input-row">
    <!-- CWD Breadcrumb Button -->
    <button
      type="button"
      class="terminal-cwd-btn"
      onclick={copyCwd}
      title="{t("terminal.cwdCopy")}: {activeSession.cwd}"
    >
      {#if copiedCwd}
        <svg
          width="11"
          height="11"
          viewBox="0 0 16 16"
          fill="currentColor"
          aria-hidden="true"
          style="color: var(--diff-add-text);"
        >
          <path
            d="M13.78 4.22a.75.75 0 0 1 0 1.06l-7.25 7.25a.75.75 0 0 1-1.06 0L2.22 9.28a.751.751 0 0 1 .018-1.042.751.751 0 0 1 1.042-.018L6 10.94l6.72-6.72a.75.75 0 0 1 1.06 0Z"
          />
        </svg>
      {:else}
        <svg
          width="11"
          height="11"
          viewBox="0 0 16 16"
          fill="currentColor"
          aria-hidden="true"
        >
          <path
            d="M1.75 1A1.75 1.75 0 0 0 0 2.75v10.5C0 14.216.784 15 1.75 15h12.5A1.75 1.75 0 0 0 16 13.25v-8.5A1.75 1.75 0 0 0 14.25 3H7.5a.25.25 0 0 1-.2-.1l-.9-1.2C6.07 1.26 5.55 1 5 1H1.75Z"
          />
        </svg>
      {/if}
      <span>{copiedCwd ? t("terminal.cwdCopied") : formatDisplayCwd(activeSession.cwd)}</span>
    </button>

    <!-- Branch Badge -->
    <div
      class="terminal-branch-badge"
      title="Git Branch: {currentBranch || 'HEAD'}"
    >
      <svg
        width="10"
        height="10"
        viewBox="0 0 16 16"
        fill="currentColor"
        aria-hidden="true"
      >
        <path
          d="M9.5 3.25a2.25 2.25 0 1 1 3 2.122V6A2.5 2.5 0 0 1 10 8.5H6a1 1 0 0 0-1 1v1.128a2.251 2.251 0 1 1-1.5 0V5.372a2.25 2.25 0 1 1 1.5 0v1.836A2.493 2.493 0 0 1 6 7h4a1 1 0 0 0 1-1v-.628A2.25 2.25 0 0 1 9.5 3.25Z"
        />
      </svg>
      <span>{currentBranch || "HEAD"}</span>
    </div>

    <span class="terminal-prompt">$</span>
    <input
      bind:this={inputEl}
      type="text"
      class="terminal-input"
      bind:value={commandInput}
      onkeydown={handleKeydown}
      placeholder={t("terminal.inputPlaceholder")}
      disabled={activeSession.running}
    />

    {#if activeSession.running}
      <button
        type="button"
        class="terminal-btn btn-danger"
        onclick={handleKill}
        title="Kill active process (Ctrl+C)"
      >
        <svg
          width="10"
          height="10"
          viewBox="0 0 16 16"
          fill="currentColor"
          aria-hidden="true"
        >
          <rect x="2" y="2" width="12" height="12" rx="2" />
        </svg>
        <span class="terminal-btn-label">{t("terminal.killBtn")}</span>
      </button>
    {:else}
      <button
        type="button"
        class="btn-primary btn-xs"
        style="display: inline-flex; align-items: center; gap: 4px; padding: 0 8px; height: 24px; flex-shrink: 0;"
        disabled={!commandInput.trim()}
        onclick={() => void runCmd()}
      >
        <svg
          width="10"
          height="10"
          viewBox="0 0 16 16"
          fill="currentColor"
          aria-hidden="true"
        >
          <path
            d="M4.5 3a.5.5 0 0 0-.5.5v9a.5.5 0 0 0 .76.43l7.5-4.5a.5.5 0 0 0 0-.86l-7.5-4.5A.5.5 0 0 0 4.5 3Z"
          />
        </svg>
        <span>{t("terminal.runBtn")}</span>
      </button>
    {/if}
  </div>
</section>
