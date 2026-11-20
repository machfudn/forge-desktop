<script lang="ts">
  import { onMount } from "svelte";
  import * as gitService from "./services/git";
  import type {
    AccountHealth,
    AccountOverview,
    AccountProfile,
    AheadBehind,
    AppRuntimeInfo,
    AppUpdateInfo,
    BranchInfo,
    CommitInfo,
    DiagnosticReport,
    FileEntry,
    ParsedHostingRepo,
    ImageDiffData,
    IssueInfo,
    ParsedGitHubRepo,
    PullRequestInfo,
    PullRequestReviewComment,
    RemoteInfo,
    RepoConfig,
    RepoQuickSummary,
    RepoStatus,
    TargetPackageMode,
    UpdateChannel,
  } from "./types/git";
  import Changes from "./components/Changes.svelte";
  import Diff from "./components/Diff.svelte";
  import History from "./components/History.svelte";
  import BranchModal from "./components/BranchModal.svelte";
  import ContextMenu from "./components/ContextMenu.svelte";
  import WorkspaceModals, { type WorkspaceModalType } from "./components/WorkspaceModals.svelte";
  import HistoryGithubModals, {
    type HistoryGithubModalType,
    type RebaseItem,
  } from "./components/HistoryGithubModals.svelte";
  import ConflictEditorModal from "./components/ConflictEditorModal.svelte";
  import FileInspectModal from "./components/FileInspectModal.svelte";
  import WorkspaceToolsModal from "./components/WorkspaceToolsModal.svelte";
  import TerminalDrawer from "./components/TerminalDrawer.svelte";
  import WorktreeModal from "./components/WorktreeModal.svelte";
  import BisectWizardModal from "./components/BisectWizardModal.svelte";
  import AiAssistantModal from "./components/AiAssistantModal.svelte";
  import SshRemoteSwitchModal from "./components/SshRemoteSwitchModal.svelte";
  import CommandPalette, { type PaletteCommand } from "./components/CommandPalette.svelte";
  import ProviderIcon from "./components/ProviderIcon.svelte";
  import { joinRepoPath, openExternalUrl, pathBaseName } from "./utils/path";
  import {
    isSshConnectionError,
    isSshRemoteUrl,
    convertSshToHttps,
  } from "./utils/remoteUrl";
  import {
    copyTextToClipboard,
    handleGlobalContextMenu,
    openContextMenu,
    type ContextMenuItem,
  } from "./lib/contextMenu.svelte";
  import {
    formatBranchDisplay,
    getLocale,
    initLocale,
    setLocale,
    t,
    type Locale,
    type TranslationKey,
  } from "./i18n/index.svelte";
  import { getAppErrorLogs, logAppError } from "./utils/appLog.svelte";

  type ModalType = "branch" | "worktree" | "bisect" | WorkspaceModalType | HistoryGithubModalType | null;

  let activeRepo = $state("");
  let activeView = $state<"changes" | "history">("changes");
  let currentStatus = $state<RepoStatus | null>(null);
  let selectedFile = $state<FileEntry | null>(null);
  let currentDiff = $state<string>("");
  let currentImageDiff = $state<ImageDiffData | null>(null);
  let ignoreWhitespace = $state(false);
  const EMPTY_AHEAD_BEHIND: AheadBehind = {
    ahead: 0,
    behind: 0,
    has_remote: false,
    has_upstream: false,
    upstream: null,
    remote_branch_exists: false,
    is_unborn: false,
    unpushed_commits: 0,
  };

  let commits = $state<CommitInfo[]>([]);
  let commitHistoryLimit = $state(100);
  let hasMoreCommits = $derived(commits.length >= commitHistoryLimit);
  let branches = $state<BranchInfo[]>([]);
  let tags = $state<string[]>([]);
  let aheadBehind = $state<AheadBehind>({ ...EMPTY_AHEAD_BEHIND });
  let repoRemotes = $state<RemoteInfo[]>([]);
  let hasRemote = $state(false);
  let pendingInitRepo = $state("");
  let initDirBranch = $state("main");
  let initDirGitignore = $state("None");
  let initDirRemoteUrl = $state("");
  let initDirCommit = $state(false);
  let initDirCommitMsg = $state("chore: initial commit");
  let statusMessage = $state(t("app.ready"));
  let statusIsError = $state(false);
  let isLoading = $state(false);
  // Header popover menus
  let syncMenuOpen = $state(false);
  let toolsMenuOpen = $state(false);

  // History sidebar filter
  let historySearch = $state("");

  // Active modal
  let activeModal = $state<ModalType>(null);
  let allBranches = $state(true);
  let selectedHistoryBranches = $state<string[]>([]);
  let conflictFilePath = $state<string | null>(null);
  let inspectFileState = $state<{ path: string; mode: "blame" | "history" } | null>(null);
  let workspaceToolsTab = $state<"reflog" | "submodules" | "worktrees" | null>(null);
  let terminalOpen = $state(false);
  let appHeaderHeight = $state(46);
  let appFooterHeight = $state(26);
  let sshSwitchModalState = $state<{
    open: boolean;
    repo: string;
    remoteName: string;
    currentUrl: string;
    suggestedUrl: string;
    errorMessage: string;
    retryAction?: () => Promise<void>;
  }>({
    open: false,
    repo: "",
    remoteName: "",
    currentUrl: "",
    suggestedUrl: "",
    errorMessage: "",
  });
  let aiModalState = $state<{
    task: "custom_ask" | "explain_diff" | "explain_commit" | "review_changes";
    prompt: string;
    context: string;
  } | null>(null);
  let paletteOpen = $state(false);

  // Shared modal states
  let recentRepos = $state<string[]>([]);
  let repoAliases = $state<Record<string, string>>({});
  let repoMissingStatus = $state<Record<string, boolean>>({});
  let repoSummaries = $state<Record<string, RepoQuickSummary>>({});
  let isLoadingRepoSummaries = $state(false);
  let branchDivergenceMap = $state<Record<string, AheadBehind>>({});
  let diagReport = $state<DiagnosticReport | null>(null);
  let repoConfig = $state<RepoConfig>({ user_name: "", user_email: "" });
  let globalConfig = $state<RepoConfig>({ user_name: "", user_email: "" });
  let isLoadingBranches = $state(false);
  let isLoadingDiag = $state(false);
  let isLoadingSettings = $state(false);
  let branchModalSeq = 0;
  let settingsModalSeq = 0;
  let diagModalSeq = 0;
  let loadRepoSeq = 0;
  const pendingBranchCompares = new Set<string>();
  const appErrorLogs = $derived(getAppErrorLogs());

  // Settings & preferences
  let activeLocale = $state<Locale>(getLocale());
  function readInitialTheme(): string {
    const saved = localStorage.getItem("forge_theme") || "dark";
    // Migration for the first neo option names.
    return saved.startsWith("neo-") ? `neobrutalism-${saved.slice(4)}` : saved;
  }
  let activeTheme = $state(readInitialTheme());
  function readInitialAutofetchSeconds(): number {
    const secRaw = localStorage.getItem("forge_autofetch_seconds");
    if (secRaw !== null) {
      const parsed = Number(secRaw);
      return Number.isFinite(parsed) && parsed >= 0 ? Math.round(parsed) : 300;
    }
    const minRaw = localStorage.getItem("forge_autofetch");
    if (minRaw !== null) {
      const mins = Number(minRaw);
      if (Number.isFinite(mins) && mins >= 0) return Math.round(mins * 60);
    }
    return 300;
  }
  let autofetchSeconds = $state(readInitialAutofetchSeconds());
  let confirmDiscard = $state(localStorage.getItem("forge_confirm_discard") !== "false");
  let confirmForcePush = $state(localStorage.getItem("forge_confirm_force_push") !== "false");
  let desktopNotifEnabled = $state(localStorage.getItem("forge_desktop_notif") !== "false");
  let externalEditor = $state(localStorage.getItem("forge_external_editor") || "system");
  const savedUpdateChannel = localStorage.getItem("forge_update_channel");
  let updateChannel = $state<UpdateChannel>(
    savedUpdateChannel === "stable" || savedUpdateChannel === "beta" || savedUpdateChannel === "auto"
      ? savedUpdateChannel
      : "auto"
  );
  const savedTargetPackageMode = localStorage.getItem("forge_target_package_mode");
  let targetPackageMode = $state<TargetPackageMode>(
    savedTargetPackageMode === "portable" || savedTargetPackageMode === "installer"
      ? savedTargetPackageMode
      : "auto"
  );
  let appRuntimeInfo = $state<AppRuntimeInfo | null>(null);
  let appUpdateInfo = $state<AppUpdateInfo | null>(null);
  let isCheckingUpdate = $state(false);
  let isInstallingUpdate = $state(false);
  let updateError = $state("");
  const EDITOR_LABELS: Record<string, string> = {
    antigravity: "Antigravity IDE",
    "antigravity-ide": "Antigravity IDE",
    agy: "Antigravity IDE",
    code: "Visual Studio Code",
    cursor: "Cursor",
    windsurf: "Windsurf",
    zed: "Zed",
    subl: "Sublime Text",
    "notepad++": "Notepad++",
    notepad: "Notepad",
    gedit: "GNOME Text Editor",
    kate: "Kate",
    idea: "IntelliJ IDEA",
    webstorm: "WebStorm",
    pycharm: "PyCharm",
    rustrover: "RustRover",
    goland: "GoLand",
    codium: "VSCodium",
    nvim: "Neovim",
    "android-studio": "Android Studio",
  };
  let externalEditorLabel = $derived(
    externalEditor === "system"
      ? t("wsModal.settings.editorSystemShort")
      : EDITOR_LABELS[externalEditor] || pathBaseName(externalEditor) || t("wsModal.settings.editorSystemShort")
  );

  const SIDEBAR_MIN = 230;
  const SIDEBAR_MAX = 440;
  const SIDEBAR_DEFAULT = 296;

  function clampSidebarWidth(w: number): number {
    if (!Number.isFinite(w)) return SIDEBAR_DEFAULT;
    return Math.min(SIDEBAR_MAX, Math.max(SIDEBAR_MIN, Math.round(w)));
  }

  let sidebarWidth = $state(
    clampSidebarWidth(Number(localStorage.getItem("forge_sidebar_width") || SIDEBAR_DEFAULT))
  );

  const COMMIT_MIN = 180;
  const COMMIT_MAX = 620;
  const COMMIT_DEFAULT = 300;
  function clampCommitHeight(h: number): number {
    if (!Number.isFinite(h)) return COMMIT_DEFAULT;
    return Math.min(COMMIT_MAX, Math.max(COMMIT_MIN, Math.round(h)));
  }
  let commitPanelHeight = $state(clampCommitHeight(Number(localStorage.getItem("forge_commit_panel_height") || COMMIT_DEFAULT)));
  let isResizingCommit = $state(false);
  function updateCommitPanelHeight(next: number) {
    const clamped = clampCommitHeight(next);
    commitPanelHeight = clamped;
    localStorage.setItem("forge_commit_panel_height", String(clamped));
  }
  function startCommitResize(e: PointerEvent) {
    e.preventDefault();
    const startY = e.clientY;
    const startH = commitPanelHeight;
    isResizingCommit = true;
    const commitIsAbove = orderedWorkspacePanels()[0] === "commit";
    const onMove = (ev: PointerEvent) => updateCommitPanelHeight(startH + (ev.clientY - startY) * (commitIsAbove ? 1 : -1));
    const onUp = () => {
      isResizingCommit = false;
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
      window.removeEventListener("pointercancel", onUp);
    };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
    window.addEventListener("pointercancel", onUp);
  }
  function handleCommitResizerKeydown(e: KeyboardEvent) {
    if (e.key === "ArrowUp" || e.key === "ArrowDown") {
      e.preventDefault();
      const commitIsAbove = orderedWorkspacePanels()[0] === "commit";
      updateCommitPanelHeight(commitPanelHeight + (e.key === "ArrowDown" ? 12 : -12) * (commitIsAbove ? 1 : -1) * (e.shiftKey ? 2 : 1));
    } else if (e.key === "Home") {
      e.preventDefault();
      updateCommitPanelHeight(COMMIT_MIN);
    } else if (e.key === "End") {
      e.preventDefault();
      updateCommitPanelHeight(COMMIT_MAX);
    }
  }

  // Confirm modal payload
  let confirmTitle = $state("");
  let confirmMessage = $state("");
  let confirmDanger = $state(false);
  let confirmAction = $state<(() => Promise<void>) | null>(null);
  let confirmReturnModal = $state<typeof activeModal>(null);

  // Tag / Squash / Rebase modal payloads
  let tagTargetSha = $state<string | null>(null);
  let renameTargetTag = $state<string | null>(null);
  let squashCount = $state(2);
  let squashInitialSummary = $state("");
  let rebaseBaseSha = $state("");
  let rebaseItems = $state<RebaseItem[]>([]);
  let rewordTargetCommit = $state<CommitInfo | null>(null);


  // GitHub multi-account & repo context
  let accountOverview = $state<AccountOverview>({
    accounts: [],
    default_by_host: {},
    active_repo_account_id: null,
    primary_account_id: null,
  });
  let accountHealthMap = $state<Record<string, AccountHealth>>({});
  let githubRepoInfo = $state<ParsedGitHubRepo | null>(null);
  let hostingRepoInfo = $state<ParsedHostingRepo | null>(null);
  let pullRequests = $state<PullRequestInfo[]>([]);
  let issues = $state<IssueInfo[]>([]);
  let prTab = $state<"prs" | "issues">("prs");
  let isLoadingGithub = $state(false);
  let githubError = $state("");
  let githubRateLimited = $state(false);
  let checkRunsMap = $state<Record<string, { status: string; conclusion?: string }>>({});
  let prReviewComments = $state<PullRequestReviewComment[]>([]);
  let activePrReviewNumber = $derived.by<number | null>(() => {
    if (!currentStatus?.branch) return null;
    const m = currentStatus.branch.match(/^pr-([0-9]+)$/);
    if (m) return parseInt(m[1], 10);
    const pr = pullRequests.find((p) => p.head_branch === currentStatus?.branch);
    return pr ? pr.number : null;
  });

  $effect(() => {
    if (githubRepoInfo && activePrReviewNumber && activeRepo) {
      gitService
        .listPrComments(githubRepoInfo.owner, githubRepoInfo.repo, activePrReviewNumber, githubRepoInfo.host, activeRepo)
        .then((comments) => {
          prReviewComments = comments;
        })
        .catch(() => {
          prReviewComments = [];
        });
    } else {
      prReviewComments = [];
    }
  });

  async function handleCreatePrComment(line: number, body: string, inReplyTo?: number) {
    if (!githubRepoInfo || !activePrReviewNumber || !selectedFile || !activeRepo) return;
    try {
      const newComment = await gitService.createPrComment(
        githubRepoInfo.owner,
        githubRepoInfo.repo,
        activePrReviewNumber,
        {
          body,
          commit_id: currentStatus?.branch || "HEAD",
          path: selectedFile.path,
          line,
          in_reply_to: inReplyTo,
        },
        githubRepoInfo.host,
        activeRepo
      );
      prReviewComments = [...prReviewComments, newComment];
      setStatus(t("diff.prCommentBtn"));
    } catch (err) {
      setStatus(String(err), true);
    }
  }
  const activeRepoAccount = $derived.by<AccountProfile | null>(() => {
    const list = accountOverview.accounts;
    if (list.length === 0) return null;
    const host = hostingRepoInfo?.host || githubRepoInfo?.host;
    if (currentStatus && host) {
      if (accountOverview.active_repo_account_id) {
        const bound = list.find(
          (a) => a.id === accountOverview.active_repo_account_id && a.host === host
        );
        if (bound) return bound;
      }
      const defId = accountOverview.default_by_host[host];
      if (defId) {
        const defAcc = list.find((a) => a.id === defId);
        if (defAcc) return defAcc;
      }
      const matching = list.filter((a) => a.host === host);
      if (matching.length === 1) return matching[0];
      return null;
    }
    if (accountOverview.active_repo_account_id) {
      const bound = list.find((a) => a.id === accountOverview.active_repo_account_id);
      if (bound) return bound;
    }
    if (accountOverview.primary_account_id) {
      const pri = list.find((a) => a.id === accountOverview.primary_account_id);
      if (pri) return pri;
    }
    return list[0];
  });
  let failedAvatarUrl = $state<string | null>(null);
  const activeAvatarUrl = $derived(
    activeRepoAccount?.avatar_url && activeRepoAccount.avatar_url !== failedAvatarUrl
      ? activeRepoAccount.avatar_url
      : null
  );


  const branchPushMeta = $derived.by(() => {
    if (!currentStatus) return null;
    if (!currentStatus.has_commits || aheadBehind.is_unborn) {
      return {
        text: t("app.branchMeta.zeroCommits"),
        tone: "is-neutral",
        title: t("app.branchMeta.zeroCommitsTitle"),
      };
    }
    if (!hasRemote) {
      const localCount = aheadBehind.unpushed_commits;
      return {
        text:
          localCount > 0
            ? t("app.branchMeta.localWithCount", { count: localCount })
            : t("app.branchMeta.local"),
        tone: "is-warn",
        title: t("app.branchMeta.localTitle"),
      };
    }
    if (!aheadBehind.has_upstream && !aheadBehind.remote_branch_exists) {
      return {
        text:
          aheadBehind.unpushed_commits > 0
            ? t("app.branchMeta.unpushedWithCount", { count: aheadBehind.unpushed_commits })
            : t("app.branchMeta.unpushed"),
        tone: "is-warn",
        title: t("app.branchMeta.unpushedTitle", {
          branch: formatBranchDisplay(currentStatus.branch),
        }),
      };
    }
    if (!aheadBehind.has_upstream && aheadBehind.remote_branch_exists) {
      return {
        text:
          aheadBehind.ahead > 0
            ? t("app.branchMeta.noUpstreamWithCount", { count: aheadBehind.ahead })
            : t("app.branchMeta.noUpstream"),
        tone: "is-warn",
        title: t("app.branchMeta.noUpstreamTitle"),
      };
    }
    if (aheadBehind.ahead > 0 || aheadBehind.behind > 0) {
      const parts = [
        aheadBehind.ahead > 0 ? `↑${aheadBehind.ahead}` : "",
        aheadBehind.behind > 0 ? `↓${aheadBehind.behind}` : "",
      ]
        .filter(Boolean)
        .join(" ");
      return {
        text: parts,
        tone: aheadBehind.ahead > 0 ? "is-warn" : "",
        title: t("app.branchMeta.upstreamTitle", { upstream: aheadBehind.upstream ?? "" }),
      };
    }
    return {
      text: t("app.branchMeta.synced"),
      tone: "is-synced",
      title: t("app.branchMeta.syncedTitle", { upstream: aheadBehind.upstream || "remote" }),
    };
  });
  let autofetchTimer: number | null = null;
  let localSyncTimer: number | null = null;
  let isWindowFocused = $state(true);
  let isSyncRunning = false;

  function setStatus(msg: string, isError = false) {
    statusMessage = msg;
    statusIsError = isError;
    if (isError && msg.trim()) {
      logAppError({ source: "ui", message: msg });
    }
  }

  function isGithubRateLimitError(err: unknown): boolean {
    const msg = String(err).toLowerCase();
    return msg.includes("rate limit") || (msg.includes("(403)") && !msg.includes("sso_required"));
  }

  function formatGithubError(err: unknown): string {
    const raw = String(err);
    if (isGithubRateLimitError(raw)) {
      githubRateLimited = true;
      if (!activeRepoAccount) {
        return t("app.status.githubRateLimitGuest");
      }
      return t("app.status.githubRateLimitAuth");
    }
    return raw;
  }

  async function checkAccountHealth(accountId: string): Promise<AccountHealth> {
    const health = await gitService.checkGithubAccountStatus(accountId);
    accountHealthMap = { ...accountHealthMap, [accountId]: health };
    return health;
  }

  async function refreshAccounts(repoPath?: string) {
    try {
      const ov = await gitService.listGithubAccounts(repoPath || undefined);
      accountOverview = ov;
      const targetId = ov.active_repo_account_id || ov.primary_account_id;
      if (targetId && !accountHealthMap[targetId]) {
        checkAccountHealth(targetId).catch(() => {});
      }
    } catch {
      // Non-Tauri browser preview
    }
  }

  async function handleAccountsChanged(
    overview: AccountOverview,
    syncedProfile?: AccountProfile
  ) {
    accountOverview = overview;
    githubRateLimited = false;
    githubError = "";
    if (syncedProfile) {
      accountHealthMap = {
        ...accountHealthMap,
        [syncedProfile.id]: { status: "valid", profile: syncedProfile },
      };
    }
    if (overview.accounts.length === 0) {
      pullRequests = [];
      issues = [];
      checkRunsMap = {};
    }
    if (activeRepo) {
      try {
        const remotes = await gitService.listRemotes(activeRepo);
        const origin = remotes.find((r) => r.name === "origin") || remotes[0];
        if (origin) {
          githubRepoInfo = await gitService.parseGithubRepository(origin.url);
        }
      } catch {
        // ignore
      }
      if (githubRepoInfo && activeRepoAccount) {
        gitService
          .listGithubIssues(
            githubRepoInfo.owner,
            githubRepoInfo.repo,
            githubRepoInfo.host,
            activeRepo
          )
          .then((list) => {
            issues = list;
          })
          .catch(() => {});
      }
    }
  }

  async function handleSyncRepoAuthor(name: string, email: string) {
    if (!activeRepo) return;
    await gitService.setRepoConfig(activeRepo, name, email);
    repoConfig = { user_name: name, user_email: email };
    setStatus(t("app.status.authorSynced", { name, email }));
  }

  function applyTheme(theme: string) {
    const cut = theme.lastIndexOf("-");
    const mode = cut > 0 ? theme.slice(cut + 1) : theme;
    const isStyleTheme = mode === "dark" || mode === "light";
    let resolved = isStyleTheme ? mode : theme;
    if (resolved === "system") {
      resolved = window.matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark";
    }
    const root = document.documentElement;
    root.setAttribute("data-theme", resolved);
    if (isStyleTheme && cut > 0) root.setAttribute("data-style", theme.slice(0, cut));
    else root.removeAttribute("data-style");
    const metaTheme = document.querySelector('meta[name="theme-color"]');
    if (metaTheme) {
      const bg = getComputedStyle(root).getPropertyValue("--bg-app").trim();
      if (bg) metaTheme.setAttribute("content", bg);
    }
  }

  function setupLocalSync() {
    if (localSyncTimer !== null) {
      clearInterval(localSyncTimer);
      localSyncTimer = null;
    }
    if (!activeRepo) return;
    const intervalMs = isWindowFocused ? 2500 : 15000;
    localSyncTimer = window.setInterval(() => {
      void syncRepoState(false);
    }, intervalMs);
  }

  function setupAutofetch(secs: number) {
    if (autofetchTimer !== null) {
      clearInterval(autofetchTimer);
      autofetchTimer = null;
    }
    const normalizedSecs = Number.isFinite(secs) ? Math.max(0, Math.round(secs)) : 0;
    if (normalizedSecs > 0 && activeRepo) {
      let isAutofetchRunning = false;
      autofetchTimer = window.setInterval(async () => {
        if (!activeRepo || isLoading || isAutofetchRunning || activeModal === "reword" || activeModal === "rebase" || activeModal === "squash") return;
        isAutofetchRunning = true;
        try {
          await gitService.gitFetch(activeRepo);
          await syncRepoState(false);
        } catch {
          // Silent background network fetch
        } finally {
          isAutofetchRunning = false;
        }
      }, normalizedSecs * 1000);
    }
  }

  async function syncRepoState(force = false) {
    if (!activeRepo || isLoading || isSyncRunning) return;
    if (activeModal === "reword" || activeModal === "rebase" || activeModal === "squash") return;

    isSyncRunning = true;
    try {
      const repo = activeRepo;
      const status = await gitService.inspectRepository(repo);
      if (activeRepo !== repo) return;

      const prevFiles = currentStatus?.files || [];
      const filesChanged =
        force ||
        !currentStatus ||
        prevFiles.length !== status.files.length ||
        prevFiles.some((cf, i) => {
          const nf = status.files[i];
          return !nf || cf.path !== nf.path || cf.status_code !== nf.status_code || cf.staged !== nf.staged;
        });

      const branchChanged = currentStatus?.branch !== status.branch;
      const commitStateChanged = currentStatus?.has_commits !== status.has_commits;
      const mergeOrRebaseChanged =
        currentStatus?.in_merge !== status.in_merge ||
        currentStatus?.in_rebase !== status.in_rebase ||
        currentStatus?.in_cherry_pick !== status.in_cherry_pick ||
        currentStatus?.in_revert !== status.in_revert;

      if (filesChanged || branchChanged || commitStateChanged || mergeOrRebaseChanged) {
        currentStatus = status;

        if (selectedFile) {
          const stillExists = status.files.find((f) => f.path === selectedFile!.path);
          if (stillExists) {
            selectedFile = stillExists;
            await loadDiffForFile(stillExists);
          } else {
            selectedFile = status.files.length > 0 ? status.files[0] : null;
            if (selectedFile) {
              await loadDiffForFile(selectedFile);
            } else {
              currentDiff = "";
              currentImageDiff = null;
            }
          }
        } else if (status.files.length > 0) {
          selectedFile = status.files[0];
          await loadDiffForFile(selectedFile);
        } else {
          currentDiff = "";
          currentImageDiff = null;
        }
      } else if (selectedFile && activeView === "changes") {
        // Even if the file list didn't change, re-check diff content for the active file
        try {
          const nextDiff = await gitService.getFileDiff(repo, selectedFile.path, selectedFile.staged, ignoreWhitespace);
          if (currentDiff !== nextDiff) {
            currentDiff = nextDiff;
          }
        } catch {
          // Silent diff error
        }
      }

      const isValidBranch =
        Boolean(status.branch) &&
        !status.is_detached &&
        status.branch !== "HEAD terlepas" &&
        !status.branch.includes("detached") &&
        !status.branch.startsWith("(");

      if (branchChanged || commitStateChanged || force) {
        const [nextAb, nextBranches, nextTags, nextCommits] = await Promise.all([
          isValidBranch
            ? gitService.getBranchAheadBehind(repo, status.branch)
            : Promise.resolve({ ...EMPTY_AHEAD_BEHIND, has_remote: hasRemote, is_unborn: !status.has_commits }),
          gitService.listBranches(repo),
          gitService.listTags(repo),
          gitService.getCommitHistory(
            repo,
            commitHistoryLimit,
            allBranches,
            selectedHistoryBranches.length > 0 ? selectedHistoryBranches : undefined
          ),
        ]);
        if (activeRepo !== repo) return;
        aheadBehind = nextAb;
        branches = nextBranches;
        tags = nextTags;
        commits = nextCommits;
      } else if (activeView === "history" || filesChanged) {
        const nextCommits = await gitService.getCommitHistory(
          repo,
          commitHistoryLimit,
          allBranches,
          selectedHistoryBranches.length > 0 ? selectedHistoryBranches : undefined
        );
        if (activeRepo !== repo) return;
        const commitsDiffer =
          nextCommits.length !== commits.length ||
          (nextCommits.length > 0 && commits.length > 0 && nextCommits[0].sha !== commits[0].sha);

        if (commitsDiffer) {
          commits = nextCommits;
          if (isValidBranch) {
            aheadBehind = await gitService.getBranchAheadBehind(repo, status.branch);
          }
        }
      }
    } catch {
      // Silent background sync
    } finally {
      isSyncRunning = false;
    }
  }

  function handleWindowFocus() {
    isWindowFocused = true;
    setupLocalSync();
    if (activeRepo) {
      void syncRepoState(true);
    }
  }

  function handleWindowBlur() {
    isWindowFocused = false;
    setupLocalSync();
  }

  function handleVisibilityChange() {
    if (document.visibilityState === "visible") {
      isWindowFocused = true;
      setupLocalSync();
      if (activeRepo) {
        void syncRepoState(true);
      }
    } else {
      isWindowFocused = false;
      setupLocalSync();
    }
  }

  function sendNotification(title: string, body: string) {
    if (!desktopNotifEnabled) return;
    if ("Notification" in window && Notification.permission === "granted") {
      new Notification(title, { body });
    } else if ("Notification" in window && Notification.permission !== "denied") {
      Notification.requestPermission().then((p) => {
        if (p === "granted") new Notification(title, { body });
      });
    }
  }

  function requestConfirmation(
    title: string,
    message: string,
    action: () => Promise<void>,
    danger = false
  ) {
    confirmReturnModal =
      activeModal === "auth" ||
      activeModal === "stash" ||
      activeModal === "remote" ||
      activeModal === "reword" ||
      activeModal === "rebase"
        ? activeModal
        : null;
    confirmTitle = title;
    confirmMessage = message;
    confirmDanger = danger;
    confirmAction = action;
    activeModal = "confirm";
  }

  async function handleConfirmSubmit() {
    const fn = confirmAction;
    const returnTo = confirmReturnModal;
    activeModal = null;
    confirmAction = null;
    confirmReturnModal = null;
    if (fn) await fn();
    if (returnTo) activeModal = returnTo;
  }

  function saveRecentRepo(path: string) {
    const updated = [path, ...recentRepos.filter((r) => r !== path)].slice(0, 12);
    recentRepos = updated;
    localStorage.setItem("forge_recent_repos", JSON.stringify(updated));
    localStorage.setItem("forge_last_repo", path);
  }

  function removeRecentRepo(path: string) {
    recentRepos = recentRepos.filter((r) => r !== path);
    localStorage.setItem("forge_recent_repos", JSON.stringify(recentRepos));
  }

  function setRepoAlias(path: string, alias: string) {
    const next = { ...repoAliases };
    if (alias.trim()) {
      next[path] = alias.trim();
    } else {
      delete next[path];
    }
    repoAliases = next;
    localStorage.setItem("forge_repo_aliases", JSON.stringify(next));
  }

  async function checkMissingRepos() {
    if (recentRepos.length === 0) {
      repoMissingStatus = {};
      repoSummaries = {};
      return;
    }
    isLoadingRepoSummaries = true;
    try {
      const summaryMap = await gitService.batchInspectRepoSummaries(recentRepos);
      repoSummaries = summaryMap;
      const statusMap: Record<string, boolean> = {};
      for (const r of recentRepos) {
        const sum = summaryMap[r];
        statusMap[r] = sum ? !sum.exists : false;
      }
      repoMissingStatus = statusMap;
    } catch {
      const statusMap: Record<string, boolean> = {};
      for (const r of recentRepos) {
        statusMap[r] = false;
      }
      repoMissingStatus = statusMap;
    } finally {
      isLoadingRepoSummaries = false;
    }
  }

  async function relocateMissingRepo(oldPath: string, newPath: string) {
    const trimmed = newPath.trim();
    if (!trimmed) return;
    try {
      const exists = await gitService.checkRepoExists(trimmed);
      if (!exists) {
        setStatus(t("app.status.relocateInvalid", { path: trimmed }), true);
        return;
      }
      recentRepos = recentRepos.map((r) => (r === oldPath ? trimmed : r));
      localStorage.setItem("forge_recent_repos", JSON.stringify(recentRepos));
      if (repoAliases[oldPath]) {
        const nextAliases = { ...repoAliases, [trimmed]: repoAliases[oldPath] };
        delete nextAliases[oldPath];
        repoAliases = nextAliases;
        localStorage.setItem("forge_repo_aliases", JSON.stringify(nextAliases));
      }
      await checkMissingRepos();
      await loadRepo(trimmed);
      setStatus(t("app.status.relocateSuccess", { path: trimmed }));
    } catch (err) {
      setStatus(t("app.status.relocateFailed", { err: String(err) }), true);
    }
  }

  async function loadDiffForFile(file: FileEntry) {
    const isImg = /\.(png|jpe?g|gif|webp|svg|ico|bmp)$/i.test(file.path);
    if (isImg) {
      try {
        const nextImgDiff = await gitService.getImageDiff(activeRepo, file.path);
        currentImageDiff = nextImgDiff;
        currentDiff = "";
        return;
      } catch {
        currentImageDiff = null;
      }
    }
    currentImageDiff = null;
    try {
      const nextDiff = await gitService.getFileDiff(activeRepo, file.path, file.staged, ignoreWhitespace);
      if (currentDiff !== nextDiff) {
        currentDiff = nextDiff;
      }
    } catch {
      currentDiff = "";
    }
  }

  async function refreshRemotesAndPushState() {
    if (!activeRepo || !currentStatus) return;
    try {
      const [remotes] = await Promise.all([
        gitService.listRemotes(activeRepo),
        refreshAccounts(activeRepo),
      ]);
      repoRemotes = remotes;
      hasRemote = remotes.length > 0;
      const isValidBranch =
        Boolean(currentStatus.branch) &&
        currentStatus.branch !== "HEAD terlepas" &&
        !currentStatus.branch.startsWith("(");
      if (isValidBranch) {
        aheadBehind = await gitService.getBranchAheadBehind(activeRepo, currentStatus.branch);
      } else {
        aheadBehind = { ...EMPTY_AHEAD_BEHIND, has_remote: hasRemote, is_unborn: !currentStatus.has_commits };
      }
      const origin = remotes.find((r) => r.name === "origin") || remotes[0];
      if (origin) {
        const [ghParsed, hostParsed] = await Promise.all([
          gitService.parseGithubRepository(origin.url).catch(() => null),
          gitService.parseHostingRepository(origin.url).catch(() => null),
        ]);
        githubRepoInfo = ghParsed;
        hostingRepoInfo = hostParsed;
      } else {
        githubRepoInfo = null;
        hostingRepoInfo = null;
      }
    } catch {
      repoRemotes = [];
      hasRemote = false;
      githubRepoInfo = null;
      hostingRepoInfo = null;
    }
  }

  async function handleInitPendingDir() {
    const target = pendingInitRepo.trim();
    if (!target) return;
    try {
      isLoading = true;
      setStatus(t("app.status.initializingGit", { name: pathBaseName(target) }));
      let giContent: string | undefined;
      if (initDirGitignore === "Node") {
        giContent = "node_modules/\ndist/\n.env\n*.log";
      } else if (initDirGitignore === "Rust") {
        giContent = "target/\n**/*.rs.bk\nCargo.lock";
      } else if (initDirGitignore === "Python") {
        giContent = "__pycache__/\n*.py[cod]\nvenv/\n.env";
      }
      await gitService.initRepository(target, initDirBranch.trim() || "main", giContent);
      const remUrl = initDirRemoteUrl.trim();
      if (remUrl) {
        await gitService.addRemote(target, "origin", remUrl);
      }
      let warnMsg = "";
      if (initDirCommit) {
        try {
          await gitService.stageAll(target);
          const postStage = await gitService.inspectRepository(target);
          if (postStage.files.some((f) => f.staged)) {
            await gitService.commitChanges(
              target,
              initDirCommitMsg.trim() || "chore: initial commit"
            );
          }
        } catch (commitErr) {
          warnMsg = t("app.status.initialCommitSkipped", { err: String(commitErr) });
        }
      }
      pendingInitRepo = "";
      await loadRepo(target);
      if (warnMsg) {
        setStatus(t("app.status.repoInitializedWithWarn", { warn: warnMsg }), true);
      } else {
        setStatus(
          remUrl
            ? t("app.status.repoInitializedWithRemote", { name: pathBaseName(target) })
            : t("app.status.repoInitialized", { name: pathBaseName(target) })
        );
      }
    } catch (err) {
      setStatus(t("app.status.initGitFailed", { err: String(err) }), true);
    } finally {
      isLoading = false;
    }
  }

  async function loadRepo(repoPath: string) {
    if (!repoPath.trim()) return;
    const reqSeq = ++loadRepoSeq;
    const isNewRepo = activeRepo !== repoPath;
    try {
      isLoading = true;
      setStatus(t("app.status.inspectingRepo", { name: pathBaseName(repoPath) }));
      try {
        const pathState = await gitService.getRepoPathState(repoPath);
        if (reqSeq !== loadRepoSeq) return;
        if (pathState === "missing") {
          pendingInitRepo = "";
          setStatus(t("app.status.folderNotFound", { path: repoPath }), true);
          return;
        }
        if (pathState === "non_git") {
          pendingInitRepo = repoPath;
          activeRepo = "";
          currentStatus = null;
          selectedFile = null;
          currentDiff = "";
          currentImageDiff = null;
          repoRemotes = [];
          hasRemote = false;
          githubRepoInfo = null;
          aheadBehind = { ...EMPTY_AHEAD_BEHIND };
          initDirBranch = "main";
          initDirGitignore = "None";
          initDirRemoteUrl = "";
          initDirCommit = false;
          initDirCommitMsg = "chore: initial commit";
          setStatus(t("app.status.folderNotGit", { name: pathBaseName(repoPath) }));
          if (localSyncTimer !== null) {
            clearInterval(localSyncTimer);
            localSyncTimer = null;
          }
          if (autofetchTimer !== null) {
            clearInterval(autofetchTimer);
            autofetchTimer = null;
          }
          return;
        }
      } catch {
        // Fallback for non-Tauri preview
      }

      pendingInitRepo = "";
      const status = await gitService.inspectRepository(repoPath);
      if (reqSeq !== loadRepoSeq) return;

      activeRepo = repoPath;
      currentStatus = status;
      const prevSelectedPath = !isNewRepo ? selectedFile?.path : null;
      const preservedFile = prevSelectedPath ? status.files.find((f) => f.path === prevSelectedPath) : null;
      selectedFile = preservedFile || (status.files.length > 0 ? status.files[0] : null);
      if (isNewRepo) {
        currentDiff = "";
        currentImageDiff = null;
      }
      if (isNewRepo) {
        commitHistoryLimit = 100;
        branches = [];
        pullRequests = [];
        issues = [];
        checkRunsMap = {};
        githubRepoInfo = null;
        hostingRepoInfo = null;
        repoRemotes = [];
        hasRemote = false;
      }
      branchDivergenceMap = {};
      pendingBranchCompares.clear();
      saveRecentRepo(repoPath);

      const isValidBranch =
        Boolean(status.branch) &&
        !status.is_detached &&
        !status.branch.includes("detached") &&
        status.branch !== "HEAD terlepas" &&
        !status.branch.startsWith("(");

      const [abRes, , tagsRes, historyRes, remotesRes, branchesRes] = await Promise.allSettled([
        isValidBranch
          ? gitService.getBranchAheadBehind(repoPath, status.branch)
          : Promise.resolve({ ...EMPTY_AHEAD_BEHIND, is_unborn: !status.has_commits }),
        selectedFile ? loadDiffForFile(selectedFile) : Promise.resolve(),
        gitService.listTags(repoPath),
        gitService.getCommitHistory(
          repoPath,
          commitHistoryLimit,
          allBranches,
          selectedHistoryBranches.length > 0 ? selectedHistoryBranches : undefined
        ),
        gitService.listRemotes(repoPath),
        gitService.listBranches(repoPath),
        refreshAccounts(repoPath),
      ]);
      if (reqSeq !== loadRepoSeq) return;

      aheadBehind =
        abRes.status === "fulfilled"
          ? abRes.value
          : { ...EMPTY_AHEAD_BEHIND, is_unborn: !status.has_commits };
      tags = tagsRes.status === "fulfilled" ? tagsRes.value : [];
      if (historyRes.status === "fulfilled" && historyRes.value !== null) {
        commits = historyRes.value;
      }
      if (branchesRes.status === "fulfilled") {
        branches = branchesRes.value;
      }
      if (remotesRes.status === "fulfilled") {
        const remotes = remotesRes.value;
        repoRemotes = remotes;
        hasRemote = remotes.length > 0;
        const origin = remotes.find((r) => r.name === "origin") || remotes[0];
        if (origin) {
          try {
            const [ghParsed, hostParsed] = await Promise.all([
              gitService.parseGithubRepository(origin.url).catch(() => null),
              gitService.parseHostingRepository(origin.url).catch(() => null),
            ]);
            if (reqSeq !== loadRepoSeq) return;
            githubRepoInfo = ghParsed;
            hostingRepoInfo = hostParsed;
            if (ghParsed && activeRepoAccount && !githubRateLimited) {
              gitService
                .listGithubIssues(ghParsed.owner, ghParsed.repo, ghParsed.host, repoPath)
                .then((list) => {
                  if (reqSeq === loadRepoSeq) issues = list;
                })
                .catch((err) => {
                  if (isGithubRateLimitError(err)) {
                    githubRateLimited = true;
                  }
                });
            } else if (!ghParsed) {
              issues = [];
            }
          } catch {
            githubRepoInfo = null;
            hostingRepoInfo = null;
          }
        } else {
          githubRepoInfo = null;
          hostingRepoInfo = null;
          issues = [];
        }
      } else {
        repoRemotes = [];
        hasRemote = false;
        githubRepoInfo = null;
        hostingRepoInfo = null;
        issues = [];
      }

      setupAutofetch(autofetchSeconds);
      setupLocalSync();
      setStatus(
        t("app.status.repoOpened", {
          name: pathBaseName(repoPath),
          branch: formatBranchDisplay(status.branch),
        })
      );
    } catch (err) {
      setStatus(t("app.status.loadRepoFailed", { err: String(err) }), true);
    } finally {
      if (reqSeq === loadRepoSeq) {
        isLoading = false;
      }
    }
  }

  async function selectFile(file: FileEntry) {
    selectedFile = file;
    await loadDiffForFile(file);
  }

  async function toggleWhitespace() {
    ignoreWhitespace = !ignoreWhitespace;
    if (selectedFile) {
      await loadDiffForFile(selectedFile);
    }
  }

  async function handleStageHunk(patchText: string, reverse: boolean) {
    try {
      await gitService.stageHunk(activeRepo, patchText, reverse);
      await loadRepo(activeRepo);
      setStatus(reverse ? t("app.status.hunkUnstaged") : t("app.status.hunkStaged"));
    } catch (err) {
      setStatus(t("app.status.hunkFailed", { err: String(err) }), true);
    }
  }

  async function handleStageLines(
    hunkPatch: string,
    selectedIndices: number[],
    reverse: boolean
  ) {
    try {
      await gitService.stageLines(activeRepo, hunkPatch, selectedIndices, reverse);
      await loadRepo(activeRepo);
      setStatus(reverse ? t("app.status.linesUnstaged") : t("app.status.linesStaged"));
    } catch (err) {
      setStatus(t("app.status.hunkFailed", { err: String(err) }), true);
    }
  }

  async function handleDiscardHunk(patchText: string) {
    try {
      await gitService.discardHunk(activeRepo, patchText);
      await loadRepo(activeRepo);
      setStatus(t("diff.hunkDiscarded"));
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  async function handleDiscardLines(hunkPatch: string, selectedIndices: number[]) {
    try {
      await gitService.discardLines(activeRepo, hunkPatch, selectedIndices);
      await loadRepo(activeRepo);
      setStatus(t("diff.linesDiscarded"));
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  async function handleAiGenerateCommit(): Promise<{
    summary?: string | null;
    description?: string | null;
  } | null> {
    if (!activeRepo) return null;
    try {
      setStatus(t("ai.generatingCommit"));
      const res = await gitService.aiRunTask("commit_message", "", activeRepo);
      const raw = res.content.trim();
      const summaryMatch = raw.match(/^SUMMARY:\s*(.+)$/im);
      const descMatch = raw.match(/^DESCRIPTION:\s*([\s\S]+)$/im);
      let summary = summaryMatch ? summaryMatch[1].trim() : "";
      let description = descMatch ? descMatch[1].trim() : "";
      if (!summary) {
        const lines = raw.split(/\r?\n/).map((l) => l.trim()).filter(Boolean);
        summary = (lines[0] || "").replace(/^[-*`#]+\s*/, "");
        description = lines.slice(1).join("\n");
      }
      setStatus(t("ai.generatedCommit", { model: res.model }));
      return { summary, description };
    } catch (err) {
      setStatus(String(err), true);
      return null;
    }
  }

  function handleAiExplainDiff(filePath: string, diffText: string) {
    aiModalState = {
      task: "explain_diff",
      prompt: `Explain changes in ${filePath}`,
      context: diffText.slice(0, 12000),
    };
  }

  function handleAiExplainCommit(sha: string, summary: string) {
    aiModalState = {
      task: "explain_commit",
      prompt: `Explain commit ${sha.slice(0, 8)}: ${summary}`,
      context: `Commit SHA: ${sha}\nSummary: ${summary}`,
    };
  }

  async function handleMergeBranchFromGraph(branchName: string) {
    if (!activeRepo) return;
    try {
      isLoading = true;
      const res = await gitService.mergeBranch(activeRepo, branchName);
      setStatus(res || t("app.status.mergedBranch", { branch: branchName }));
      await loadRepo(activeRepo);
    } catch (err) {
      setStatus(String(err), true);
    } finally {
      isLoading = false;
    }
  }

  async function handleRebaseBranchFromGraph(baseRef: string) {
    if (!activeRepo) return;
    try {
      isLoading = true;
      const res = await gitService.rebaseBranch(activeRepo, baseRef);
      setStatus(res || t("app.status.rebasedBranch", { branch: baseRef }));
      await loadRepo(activeRepo);
    } catch (err) {
      setStatus(String(err), true);
    } finally {
      isLoading = false;
    }
  }

  async function handleFilterBranches(filter: { all: boolean; selected: string[] }) {
    allBranches = filter.all;
    selectedHistoryBranches = filter.selected;
    if (activeRepo && activeView === "history") {
      try {
        commits = await gitService.getCommitHistory(
          activeRepo,
          commitHistoryLimit,
          filter.all,
          filter.selected.length > 0 ? filter.selected : undefined
        );
      } catch (err) {
        setStatus(String(err), true);
      }
    }
  }

  async function handleLoadMoreCommits() {
    if (!activeRepo || isLoading) return;
    commitHistoryLimit += 100;
    try {
      commits = await gitService.getCommitHistory(
        activeRepo,
        commitHistoryLimit,
        allBranches,
        selectedHistoryBranches.length > 0 ? selectedHistoryBranches : undefined
      );
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  async function toggleStage(file: FileEntry) {
    try {
      if (file.staged) {
        await gitService.unstageFile(activeRepo, file.path);
      } else {
        await gitService.stageFile(activeRepo, file.path);
      }
      await loadRepo(activeRepo);
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  async function handleStageAll() {
    try {
      await gitService.stageAll(activeRepo);
      await loadRepo(activeRepo);
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  async function handleUnstageAll() {
    try {
      await gitService.unstageAll(activeRepo);
      await loadRepo(activeRepo);
    } catch (err) {
      setStatus(String(err), true);
    }
  }
  function handleDiscardSelected(files: FileEntry[]) {
    if (files.length === 0) return;
    const execute = async () => {
      try {
        for (const file of files) await gitService.discardFileChanges(activeRepo, file.path);
        setStatus(t("app.status.allDiscarded"));
      } catch (err) {
        setStatus(String(err), true);
      }
      await loadRepo(activeRepo);
    };
    if (confirmDiscard) {
      requestConfirmation(
        t("changes.discardSelectedTitle"),
        t("changes.discardSelectedMsg", { count: files.length }),
        execute,
        true
      );
    } else {
      execute();
    }
  }

  async function handleCommit(summary: string, description?: string, amend?: boolean) {
    try {
      await gitService.commitChanges(activeRepo, summary, description, amend);
      setStatus(amend ? t("app.status.commitAmended") : t("app.status.commitCreated"));
      await loadRepo(activeRepo);
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  function handleUndoCommit() {
    requestConfirmation(
      t("app.status.undoCommitTitle"),
      t("app.status.undoCommitMsg"),
      async () => {
        try {
          await gitService.undoLastCommit(activeRepo);
          setStatus(t("app.status.undoCommitDone"));
          await loadRepo(activeRepo);
        } catch (err) {
          setStatus(String(err), true);
        }
      },
      false
    );
  }

  async function handleAddToGitignore(filePath: string) {
    try {
      await gitService.addToGitignore(activeRepo, filePath);
      setStatus(t("app.status.addedGitignore", { path: filePath }));
      await loadRepo(activeRepo);
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  async function handleAbortMerge() {
    try {
      await gitService.abortMerge(activeRepo);
      setStatus(t("app.status.mergeAborted"));
      await loadRepo(activeRepo);
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  async function handleAbortRebase() {
    try {
      await gitService.abortRebase(activeRepo);
      setStatus(t("app.status.rebaseAborted"));
      await loadRepo(activeRepo);
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  async function handleContinueMerge() {
    try {
      await gitService.continueMerge(activeRepo);
      setStatus(t("changes.mergeContinued"));
      await loadRepo(activeRepo);
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  async function handleContinueRebase() {
    try {
      await gitService.continueRebase(activeRepo);
      setStatus(t("changes.rebaseContinued"));
      await loadRepo(activeRepo);
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  async function handleAbortCherryPick() {
    try {
      await gitService.abortCherryPick(activeRepo);
      setStatus(t("changes.cherryPickAborted"));
      await loadRepo(activeRepo);
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  async function handleContinueCherryPick() {
    try {
      await gitService.continueCherryPick(activeRepo);
      setStatus(t("changes.cherryPickContinued"));
      await loadRepo(activeRepo);
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  async function handleAbortRevert() {
    try {
      await gitService.abortRevert(activeRepo);
      setStatus(t("changes.revertAborted"));
      await loadRepo(activeRepo);
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  async function handleContinueRevert() {
    try {
      await gitService.continueRevert(activeRepo);
      setStatus(t("changes.revertContinued"));
      await loadRepo(activeRepo);
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  function handleDiscardFile(file: FileEntry) {
    const execute = async () => {
      try {
        await gitService.discardFileChanges(activeRepo, file.path);
        setStatus(t("app.status.fileDiscarded", { path: file.path }));
        await loadRepo(activeRepo);
      } catch (err) {
        setStatus(String(err), true);
      }
    };
    if (confirmDiscard) {
      requestConfirmation(
        t("app.status.discardFileTitle"),
        t("app.status.discardFileMsg", { path: file.path }),
        execute,
        true
      );
    } else {
      execute();
    }
  }

  function handleDiscardAll() {
    const execute = async () => {
      try {
        await gitService.discardAllChanges(activeRepo);
        setStatus(t("app.status.allDiscarded"));
        await loadRepo(activeRepo);
      } catch (err) {
        setStatus(String(err), true);
      }
    };
    if (confirmDiscard) {
      requestConfirmation(
        t("app.status.discardAllTitle"),
        t("app.status.discardAllMsg"),
        execute,
        true
      );
    } else {
      execute();
    }
  }

  async function switchView(view: "changes" | "history") {
    activeView = view;
    if (view === "history" && activeRepo) {
      try {
        const [nextCommits, nextBranches, nextTags] = await Promise.all([
          gitService.getCommitHistory(
            activeRepo,
            commitHistoryLimit,
            allBranches,
            selectedHistoryBranches.length > 0 ? selectedHistoryBranches : undefined
          ),
          branches.length === 0 ? gitService.listBranches(activeRepo) : Promise.resolve(branches),
          gitService.listTags(activeRepo),
        ]);
        commits = nextCommits;
        branches = nextBranches;
        tags = nextTags;
        const activeInfo = hostingRepoInfo || (githubRepoInfo ? {
          provider: "github" as const,
          host: githubRepoInfo.host,
          namespace: githubRepoInfo.owner,
          repo: githubRepoInfo.repo,
          web_url: `https://${githubRepoInfo.host}/${githubRepoInfo.owner}/${githubRepoInfo.repo}`
        } : null);
        const isProviderRateLimited = activeInfo?.provider === "github" && githubRateLimited;
        if (activeInfo && !isProviderRateLimited && commits.length > 0) {
          const pushedCommits = commits.filter((c) => c.is_pushed).slice(0, 12);
          const checkPromises = pushedCommits.map(async (c) => {
            try {
              const runs = await gitService.getCommitCheckRuns(
                activeInfo.namespace,
                activeInfo.repo,
                c.sha,
                activeInfo.host,
                activeRepo
              );
              if (runs && runs.length > 0) {
                const failed = runs.find(
                  (r) => r.conclusion === "failure" || r.conclusion === "timed_out"
                );
                const pending = runs.find((r) => r.status !== "completed");
                if (failed) {
                  checkRunsMap = {
                    ...checkRunsMap,
                    [c.sha]: { status: "completed", conclusion: "failure" },
                  };
                  if (commits[0]?.sha === c.sha) {
                    sendNotification(
                      t("app.status.ciFailedTitle", { sha: c.short_sha }),
                      t("app.status.ciFailedBody", { summary: c.summary })
                    );
                  }
                } else if (pending) {
                  checkRunsMap = { ...checkRunsMap, [c.sha]: { status: pending.status } };
                } else {
                  checkRunsMap = {
                    ...checkRunsMap,
                    [c.sha]: { status: "completed", conclusion: "success" },
                  };
                }
              }
            } catch (err) {
              if (activeInfo.provider === "github" && isGithubRateLimitError(err)) {
                githubRateLimited = true;
              }
            }
          });
          Promise.allSettled(checkPromises);
        }
      } catch (err) {
        setStatus(String(err), true);
      }
    }
  }

  function inspectBranchDivergence(branchName: string) {
    if (!activeRepo || !currentStatus) return;
    const cur = currentStatus.branch;
    if (!cur || cur === branchName || cur === "HEAD terlepas" || cur.startsWith("(")) return;
    const key = `${cur}\0${branchName}`;
    if (branchDivergenceMap[branchName] || pendingBranchCompares.has(key)) return;
    pendingBranchCompares.add(key);
    const targetRepo = activeRepo;
    gitService
      .compareBranches(targetRepo, branchName, cur)
      .then((div) => {
        if (activeRepo === targetRepo && currentStatus?.branch === cur) {
          branchDivergenceMap = { ...branchDivergenceMap, [branchName]: div };
        }
      })
      .catch(() => {
        // ignore unreachable comparison
      })
      .finally(() => {
        pendingBranchCompares.delete(key);
      });
  }

  function openBranchModal() {
    if (!activeRepo) return;
    const seq = ++branchModalSeq;
    const targetRepo = activeRepo;
    branchDivergenceMap = {};
    pendingBranchCompares.clear();
    isLoadingBranches = true;
    activeModal = "branch";
    gitService
      .listBranches(targetRepo)
      .then((list) => {
        if (seq === branchModalSeq && activeRepo === targetRepo) {
          branches = list;
        }
      })
      .catch((err) => {
        if (seq === branchModalSeq) {
          setStatus(String(err), true);
        }
      })
      .finally(() => {
        if (seq === branchModalSeq) {
          isLoadingBranches = false;
        }
      });
  }

  function openOpenModal() {
    toolsMenuOpen = false;
    activeModal = "open";
    void checkMissingRepos();
  }

  async function openPrModal() {
    const hostInfo = hostingRepoInfo;
    if (!githubRepoInfo && !hostInfo) return;
    activeModal = "pr";
    prTab = "prs";
    isLoadingGithub = true;
    githubError = "";
    try {
      if (hostInfo && hostInfo.provider !== "github") {
        const originUrl = (repoRemotes.find((r) => r.name === "origin") || repoRemotes[0])?.url || "";
        const [prs, iss] = await Promise.all([
          gitService.listProviderPullRequests(originUrl, activeRepo),
          gitService.listProviderIssues(originUrl, activeRepo),
        ]);
        pullRequests = prs;
        issues = iss;
      } else if (githubRepoInfo) {
        const [prs, iss] = await Promise.all([
          gitService.listGithubPullRequests(
            githubRepoInfo.owner,
            githubRepoInfo.repo,
            githubRepoInfo.host,
            activeRepo
          ),
          gitService.listGithubIssues(
            githubRepoInfo.owner,
            githubRepoInfo.repo,
            githubRepoInfo.host,
            activeRepo
          ),
        ]);
        pullRequests = prs;
        issues = iss;
      }
    } catch (err) {
      const msg = formatGithubError(err);
      githubError = msg;
      setStatus(t("app.status.loadGithubDataFailed", { msg }), true);
    } finally {
      isLoadingGithub = false;
    }
  }

  async function handleSwitchPrTab(tab: "prs" | "issues") {
    prTab = tab;
    const hostInfo = hostingRepoInfo;
    if (!githubRepoInfo && !hostInfo) return;
    isLoadingGithub = true;
    githubError = "";
    try {
      if (hostInfo && hostInfo.provider !== "github") {
        const originUrl = (repoRemotes.find((r) => r.name === "origin") || repoRemotes[0])?.url || "";
        if (tab === "prs") {
          pullRequests = await gitService.listProviderPullRequests(originUrl, activeRepo);
        } else {
          issues = await gitService.listProviderIssues(originUrl, activeRepo);
        }
      } else if (githubRepoInfo) {
        if (tab === "prs") {
          pullRequests = await gitService.listGithubPullRequests(
            githubRepoInfo.owner,
            githubRepoInfo.repo,
            githubRepoInfo.host,
            activeRepo
          );
        } else {
          issues = await gitService.listGithubIssues(
            githubRepoInfo.owner,
            githubRepoInfo.repo,
            githubRepoInfo.host,
            activeRepo
          );
        }
      }
    } catch (err) {
      const msg = formatGithubError(err);
      githubError = msg;
      setStatus(t("app.status.loadGithubFailed", { msg }), true);
    } finally {
      isLoadingGithub = false;
    }
  }

  async function handleCheckoutPr(prNumber: number, headBranch = "") {
    activeModal = null;
    try {
      setStatus(t("app.status.checkingOutPr", { num: prNumber }));
      const res =
        hostingRepoInfo && hostingRepoInfo.provider !== "github"
          ? await gitService.checkoutProviderPullRequest(
              activeRepo,
              hostingRepoInfo.provider,
              prNumber,
              headBranch
            )
          : await gitService.checkoutGithubPullRequest(activeRepo, prNumber);
      setStatus(res || t("app.status.checkedOutPr", { num: prNumber }));
      await loadRepo(activeRepo);
    } catch (err) {
      setStatus(t("app.status.checkoutPrFailed", { err: String(err) }), true);
    }
  }

  function openSettingsModal() {
    toolsMenuOpen = false;
    const seq = ++settingsModalSeq;
    const targetRepo = activeRepo;
    isLoadingSettings = true;
    activeModal = "settings";
    Promise.all([
      targetRepo ? gitService.getRepoConfig(targetRepo) : Promise.resolve(repoConfig),
      gitService.getGlobalConfig(),
    ])
      .then(([rc, gc]) => {
        if (seq !== settingsModalSeq) return;
        if (targetRepo && activeRepo === targetRepo) {
          repoConfig = rc;
        }
        globalConfig = gc;
      })
      .catch((err) => {
        if (seq === settingsModalSeq) {
          setStatus(String(err), true);
        }
      })
      .finally(() => {
        if (seq === settingsModalSeq) {
          isLoadingSettings = false;
        }
      });
  }

  async function handleSaveSettings(cfg: {
    repoConfig: RepoConfig;
    globalConfig: RepoConfig;
    activeTheme: string;
    locale: Locale;
    autofetchSeconds: number;
    confirmDiscard: boolean;
    confirmForcePush: boolean;
    desktopNotifEnabled: boolean;
    externalEditor: string;
    updateChannel: UpdateChannel;
    targetPackageMode: TargetPackageMode;
  }) {
    try {
      if (activeRepo) {
        await gitService.setRepoConfig(
          activeRepo,
          cfg.repoConfig.user_name,
          cfg.repoConfig.user_email
        );
      }
      await gitService.setGlobalConfig(
        cfg.globalConfig.user_name,
        cfg.globalConfig.user_email
      );
      repoConfig = cfg.repoConfig;
      globalConfig = cfg.globalConfig;
      activeTheme = cfg.activeTheme;
      activeLocale = cfg.locale;
      setLocale(cfg.locale);
      autofetchSeconds = cfg.autofetchSeconds;
      confirmDiscard = cfg.confirmDiscard;
      confirmForcePush = cfg.confirmForcePush;
      desktopNotifEnabled = cfg.desktopNotifEnabled;
      externalEditor = cfg.externalEditor.trim() || "system";
      const prevChannel = updateChannel;
      const prevTargetMode = targetPackageMode;
      updateChannel = cfg.updateChannel;
      targetPackageMode = cfg.targetPackageMode;

      localStorage.setItem("forge_theme", activeTheme);
      localStorage.setItem("forge_autofetch_seconds", String(autofetchSeconds));
      localStorage.removeItem("forge_autofetch");
      localStorage.setItem("forge_confirm_discard", String(confirmDiscard));
      localStorage.setItem("forge_confirm_force_push", String(confirmForcePush));
      localStorage.setItem("forge_desktop_notif", String(desktopNotifEnabled));
      localStorage.setItem("forge_external_editor", externalEditor);
      localStorage.setItem("forge_update_channel", updateChannel);
      localStorage.setItem("forge_target_package_mode", targetPackageMode);
      applyTheme(activeTheme);
      setupAutofetch(autofetchSeconds);
      setupLocalSync();
      if (prevChannel !== updateChannel || prevTargetMode !== targetPackageMode) {
        void handleCheckUpdates(updateChannel, targetPackageMode);
      }

      activeModal = null;
      setStatus(t("app.status.settingsSaved"));
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  async function handleCheckUpdates(
    channelOverride?: UpdateChannel,
    targetModeOverride?: TargetPackageMode
  ) {
    const targetChannel = channelOverride ?? updateChannel;
    const targetMode = targetModeOverride ?? targetPackageMode;
    isCheckingUpdate = true;
    updateError = "";
    try {
      setStatus(t("app.status.checkingUpdates"));
      appUpdateInfo = await gitService.checkAppUpdate(targetChannel, targetMode);
      setStatus(
        appUpdateInfo.has_update
          ? t("app.status.updateFound", {
              version: appUpdateInfo.latest_version,
              tier: appUpdateInfo.is_prerelease ? "Beta" : "Stable",
            })
          : t("app.status.upToDate", { channel: appUpdateInfo.channel })
      );
    } catch (err) {
      const msg = String(err);
      updateError = msg;
      setStatus(t("app.status.checkUpdateFailed", { msg }), true);
    } finally {
      isCheckingUpdate = false;
    }
  }

  async function handleInstallUpdate(
    channelOverride?: UpdateChannel,
    targetModeOverride?: TargetPackageMode
  ) {
    const targetChannel = channelOverride ?? updateChannel;
    const targetMode = targetModeOverride ?? targetPackageMode;
    isInstallingUpdate = true;
    updateError = "";
    try {
      setStatus(t("app.status.installingUpdate"));
      const res = await gitService.installAppUpdate(targetChannel, targetMode);
      setStatus(res.message);
    } catch (err) {
      const msg = String(err);
      updateError = msg;
      setStatus(t("app.status.installUpdateFailed", { msg }), true);
    } finally {
      isInstallingUpdate = false;
    }
  }

  function openDiagModal() {
    toolsMenuOpen = false;
    const seq = ++diagModalSeq;
    const targetRepo = activeRepo || undefined;
    isLoadingDiag = true;
    activeModal = "diag";
    gitService
      .getDiagnostics(targetRepo)
      .then((report) => {
        if (seq === diagModalSeq) {
          diagReport = report;
        }
      })
      .catch((err) => {
        if (seq === diagModalSeq) {
          setStatus(String(err), true);
        }
      })
      .finally(() => {
        if (seq === diagModalSeq) {
          isLoadingDiag = false;
        }
      });
  }

  function openLogsModal() {
    toolsMenuOpen = false;
    activeModal = "logs";
  }

  async function checkAndOpenSshSwitchModal(
    errStr: string,
    retryAction?: () => Promise<void>
  ): Promise<boolean> {
    if (!activeRepo || !isSshConnectionError(errStr)) {
      return false;
    }
    try {
      const remotes =
        repoRemotes.length > 0 ? repoRemotes : await gitService.listRemotes(activeRepo);
      if (!remotes || remotes.length === 0) return false;
      const matched =
        remotes.find((r) => isSshRemoteUrl(r.url)) || remotes[0];
      const currentUrl = matched.url;
      const suggestedUrl = convertSshToHttps(currentUrl);
      if (!suggestedUrl || suggestedUrl === currentUrl) return false;

      sshSwitchModalState = {
        open: true,
        repo: activeRepo,
        remoteName: matched.name,
        currentUrl,
        suggestedUrl,
        errorMessage: errStr,
        retryAction,
      };
      return true;
    } catch {
      return false;
    }
  }

  async function handleFetch() {
    if (!activeRepo) return;
    try {
      isLoading = true;
      setStatus(t("app.status.fetching"));
      await gitService.gitFetch(activeRepo);
      await loadRepo(activeRepo);
      setStatus(t("app.status.fetchDone"));
    } catch (err) {
      const errStr = String(err);
      const opened = await checkAndOpenSshSwitchModal(errStr, handleFetch);
      if (!opened) {
        setStatus(errStr, true);
      }
    } finally {
      isLoading = false;
    }
  }

  async function handlePull() {
    if (!activeRepo) return;
    try {
      isLoading = true;
      setStatus(t("app.status.pulling"));
      const res = await gitService.gitPull(activeRepo);
      await loadRepo(activeRepo);
      setStatus(res || t("app.status.pullDone"));
    } catch (err) {
      await loadRepo(activeRepo);
      const errStr = String(err);
      const opened = await checkAndOpenSshSwitchModal(errStr, handlePull);
      if (!opened) {
        setStatus(errStr, true);
      }
    } finally {
      isLoading = false;
    }
  }

  async function handlePush() {
    if (!activeRepo) return;
    try {
      isLoading = true;
      setStatus(t("app.status.pushing"));
      const res = await gitService.gitPush(activeRepo);
      await loadRepo(activeRepo);
      setStatus(res || t("app.status.pushDone"));
    } catch (err) {
      const errStr = String(err);
      const opened = await checkAndOpenSshSwitchModal(errStr, handlePush);
      if (!opened) {
        setStatus(errStr, true);
      }
    } finally {
      isLoading = false;
    }
  }

  function handleForcePush() {
    syncMenuOpen = false;
    if (!activeRepo) return;
    const execute = async () => {
      try {
        isLoading = true;
        setStatus(t("app.status.forcePushing"));
        const res = await gitService.gitForcePush(activeRepo);
        await loadRepo(activeRepo);
        setStatus(res || t("app.status.forcePushDone"));
      } catch (err) {
        const errStr = String(err);
        const opened = await checkAndOpenSshSwitchModal(errStr, execute);
        if (!opened) {
          setStatus(errStr, true);
        }
      } finally {
        isLoading = false;
      }
    };
    if (confirmForcePush) {
      requestConfirmation(
        t("app.status.forcePushTitle"),
        t("app.status.forcePushMsg", {
          branch: currentStatus?.branch
            ? formatBranchDisplay(currentStatus.branch)
            : t("app.status.activeFallbackBranch"),
        }),
        execute,
        true
      );
    } else {
      execute();
    }
  }

  async function handleSyncFork() {
    syncMenuOpen = false;
    if (!activeRepo) return;
    try {
      isLoading = true;
      setStatus(t("app.status.syncingFork"));
      const res = await gitService.syncFork(activeRepo);
      await loadRepo(activeRepo);
      setStatus(res || t("app.status.syncForkDone"));
      sendNotification(
        t("app.status.syncForkNotifTitle"),
        res || t("app.status.syncForkNotifBody")
      );
    } catch (err) {
      setStatus(t("app.status.syncForkFailed", { err: String(err) }), true);
    } finally {
      isLoading = false;
    }
  }

  async function handleCherryPick(sha: string) {
    try {
      const res = await gitService.cherryPick(activeRepo, sha);
      setStatus(res || t("app.status.cherryPickDone"));
      await loadRepo(activeRepo);
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  function handleRevert(sha: string) {
    requestConfirmation(
      t("app.status.revertTitle"),
      t("app.status.revertMsg", { sha: sha.slice(0, 7) }),
      async () => {
        try {
          const res = await gitService.revertCommit(activeRepo, sha);
          setStatus(res || t("app.status.revertDone"));
          await loadRepo(activeRepo);
        } catch (err) {
          setStatus(String(err), true);
        }
      },
      false
    );
  }

  function openCreateTagModal(sha: string | null) {
    tagTargetSha = sha;
    activeModal = "tag";
  }

  function handleOpenRenameTag(tagName: string) {
    renameTargetTag = tagName;
    activeModal = "rename_tag";
  }

  function handleDeleteTag(tagName: string) {
    requestConfirmation(
      t("app.status.deleteTagTitle"),
      t("app.status.deleteTagMsg", { name: tagName }),
      async () => {
        try {
          await gitService.deleteTag(activeRepo, tagName);
          const [nextTags, nextCommits] = await Promise.all([
            gitService.listTags(activeRepo),
            activeView === "history"
              ? gitService.getCommitHistory(activeRepo, commitHistoryLimit, allBranches)
              : Promise.resolve(null),
          ]);
          tags = nextTags;
          if (nextCommits !== null) commits = nextCommits;
          setStatus(t("app.status.tagDeleted", { name: tagName }));
        } catch (err) {
          setStatus(String(err), true);
        }
      },
      true
    );
  }

  function handleReset(sha: string) {
    requestConfirmation(
      t("app.status.resetTitle"),
      t("app.status.resetMsg", { sha: sha.slice(0, 7) }),
      async () => {
        try {
          const res = await gitService.resetToCommit(activeRepo, sha, false);
          setStatus(res || t("app.status.resetDone"));
          await loadRepo(activeRepo);
        } catch (err) {
          setStatus(String(err), true);
        }
      },
      true
    );
  }

  function handleOpenSquash(commitIndex: number) {
    const count = commitIndex + 1;
    if (count < 2) return;
    squashCount = count;
    const selectedCommits = commits.slice(0, count);
    squashInitialSummary = selectedCommits[count - 1]?.summary || t("app.status.squashDefaultSummary");
    activeModal = "squash";
  }
  function handleOpenRewordModal(commit: CommitInfo) {
    rewordTargetCommit = commit;
    activeModal = "reword";
  }


  async function openInteractiveRebase(baseSha?: string) {
    if (!activeRepo) return;
    if (baseSha) {
      try {
        const rebaseCommits = await gitService.getRebaseCommits(activeRepo, baseSha);
        if (rebaseCommits.length === 0) {
          setStatus(t("interactiveRebase.noCommits"), true);
          return;
        }
        rebaseBaseSha = baseSha;
        rebaseItems = rebaseCommits.map((c) => ({
          sha: c.sha,
          short_sha: c.short_sha,
          summary: c.summary,
          author: c.author,
          action: "pick" as const,
          message: c.summary,
        }));
        activeModal = "rebase";
      } catch (err) {
        setStatus(String(err), true);
      }
      return;
    }

    try {
      const [baseSha, allRebaseCommits] = await gitService.getAllRebaseCommits(activeRepo);
      if (!baseSha || allRebaseCommits.length < 2) {
        setStatus(t("app.status.rebaseMinCommits"), true);
        return;
      }
      rebaseBaseSha = baseSha;
      rebaseItems = allRebaseCommits.map((c) => ({
        sha: c.sha,
        short_sha: c.short_sha,
        summary: c.summary,
        author: c.author,
        action: "pick" as const,
        message: c.summary,
      }));
      activeModal = "rebase";
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  async function openInEditorFile(filePath: string) {
    if (!activeRepo) return;
    try {
      await gitService.openInEditor(joinRepoPath(activeRepo, filePath), externalEditor);
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  async function openInFileManager() {
    if (!activeRepo) return;
    try {
      await gitService.openInFileManager(activeRepo);
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  async function openInFileManagerTarget(filePath: string) {
    if (!activeRepo) return;
    try {
      await gitService.openInFileManager(joinRepoPath(activeRepo, filePath));
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  function openRepoHeaderContextMenu(e: MouseEvent) {
    if (!activeRepo) return;
    const items: ContextMenuItem[] = [
      {
        label: t("ctx.repo.openInEditor", { editor: externalEditorLabel }),
        onSelect: () => {
          void openInEditor();
        },
      },
      {
        label: t("ctx.repo.openInTerminal"),
        onSelect: () => {
          void openInTerminal();
        },
      },
      {
        label: t("ctx.repo.openInFileManager"),
        onSelect: () => {
          void openInFileManager();
        },
      },
      { type: "divider" },
      {
        label: t("ctx.repo.copyPath"),
        onSelect: () => {
          void copyTextToClipboard(activeRepo, setStatus, t("ctx.status.copiedPath"));
        },
      },
    ];
    const webUrl = hostingRepoInfo?.web_url || (githubRepoInfo ? `https://${githubRepoInfo.host}/${githubRepoInfo.owner}/${githubRepoInfo.repo}` : null);
    if (webUrl) {
      items.push({
        label: t("ctx.repo.viewOnGithub"),
        onSelect: () => {
          void openExternalUrl(webUrl);
        },
      });
    }
    openContextMenu(e, items);
  }

  function openBranchHeaderContextMenu(e: MouseEvent) {
    if (!currentStatus) return;
    const branchName = currentStatus.branch;
    const items: ContextMenuItem[] = [
      {
        label: t("ctx.branch.manage"),
        onSelect: openBranchModal,
      },
      {
        label: t("ctx.branch.copyName"),
        onSelect: () => {
          void copyTextToClipboard(
            branchName,
            setStatus,
            t("ctx.status.copiedBranch", { branch: branchName })
          );
        },
      },
    ];
    if (hasRemote && currentStatus.has_commits) {
      items.push({ type: "divider" });
      items.push({
        label: t("app.fetch"),
        disabled: isLoading,
        onSelect: () => {
          void handleFetch();
        },
      });
      items.push({
        label: t("app.pull"),
        disabled: isLoading,
        onSelect: () => {
          void handlePull();
        },
      });
      items.push({
        label: t("app.push"),
        disabled: isLoading,
        onSelect: () => {
          void handlePush();
        },
      });
    }
    openContextMenu(e, items);
  }

  function openSidebarTagContextMenu(e: MouseEvent, tagName: string) {
    openContextMenu(e, [
      {
        label: t("ctx.tag.copyName"),
        onSelect: () => {
          void copyTextToClipboard(
            tagName,
            setStatus,
            t("ctx.status.copiedTag", { tag: tagName })
          );
        },
      },
      {
        label: t("ctx.tag.renameTag"),
        onSelect: () => handleOpenRenameTag(tagName),
      },
      { type: "divider" },
      {
        label: t("ctx.tag.deleteTag", { tag: tagName }),
        danger: true,
        onSelect: () => handleDeleteTag(tagName),
      },
    ]);
  }

  async function openInTerminal() {
    if (!activeRepo) return;
    try {
      await gitService.openInTerminal(activeRepo);
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  async function openInEditor() {
    if (!activeRepo) return;
    try {
      const normalized = externalEditor.trim().toLowerCase();
      const target =
        (normalized === "notepad" || normalized === "notepad.exe") && selectedFile
          ? joinRepoPath(activeRepo, selectedFile.path)
          : activeRepo;
      await gitService.openInEditor(target, externalEditor);
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  onMount(() => {
    try {
      const saved = localStorage.getItem("forge_recent_repos");
      if (saved) recentRepos = JSON.parse(saved);
      const savedAliases = localStorage.getItem("forge_repo_aliases");
      if (savedAliases) repoAliases = JSON.parse(savedAliases);
    } catch {
      recentRepos = [];
    }

    initLocale();
    applyTheme(activeTheme);

    const mediaQuery = window.matchMedia("(prefers-color-scheme: light)");
    const handleSystemTheme = () => {
      if (activeTheme === "system") applyTheme("system");
    };
    mediaQuery.addEventListener("change", handleSystemTheme);

    const handleGlobalKeydown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        syncMenuOpen = false;
        toolsMenuOpen = false;
        if (paletteOpen) {
          paletteOpen = false;
        } else if (aiModalState) {
          aiModalState = null;
        } else if (conflictFilePath) {
          conflictFilePath = null;
        } else if (inspectFileState) {
          inspectFileState = null;
        } else if (workspaceToolsTab) {
          workspaceToolsTab = null;
        } else if (activeModal) {
          activeModal = null;
        }
      } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        paletteOpen = !paletteOpen;
      } else if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key.toLowerCase() === "p") {
        e.preventDefault();
        paletteOpen = !paletteOpen;
      } else if ((e.ctrlKey || e.metaKey) && e.key === "`" && activeRepo) {
        e.preventDefault();
        terminalOpen = !terminalOpen;
      } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "o" && !activeModal) {
        e.preventDefault();
        openOpenModal();
      }
    };

    const handleGlobalClick = (e: MouseEvent) => {
      const target = e.target as HTMLElement | null;
      if (!target?.closest("[data-menu-root]")) {
        syncMenuOpen = false;
        toolsMenuOpen = false;
      }
    };

    const onDocContextMenu = (e: MouseEvent) => {
      handleGlobalContextMenu(e, setStatus);
    };

    window.addEventListener("keydown", handleGlobalKeydown);
    window.addEventListener("click", handleGlobalClick);
    window.addEventListener("keydown", handleWorkspaceSettingsKeydown);
    window.addEventListener("click", handleWorkspaceSettingsOutside);
    document.addEventListener("contextmenu", onDocContextMenu);
    window.addEventListener("focus", handleWindowFocus);
    window.addEventListener("blur", handleWindowBlur);
    document.addEventListener("visibilitychange", handleVisibilityChange);
    const initApp = async () => {
      let initialRepo = "";
      try {
        const cliArg = await gitService.getCliRepoArg();
        if (cliArg && cliArg.trim()) {
          initialRepo = cliArg.trim();
        }
      } catch {
        // Non-Tauri browser preview
      }

      if (!initialRepo) {
        initialRepo = localStorage.getItem("forge_last_repo") || recentRepos[0] || "";
      }

      const startupTasks: Promise<unknown>[] = [
        refreshAccounts(),
        gitService
          .getAppRuntimeInfo()
          .then((info) => {
            appRuntimeInfo = info;
          })
          .catch(() => {}),
      ];
      if (initialRepo) {
        startupTasks.push(loadRepo(initialRepo));
      } else {
        setStatus(t("app.selectOrCloneStart"));
        startupTasks.push(checkMissingRepos());
      }
      await Promise.allSettled(startupTasks);

      gitService
        .checkAppUpdate(updateChannel, targetPackageMode)
        .then((info) => {
          appUpdateInfo = info;
        })
        .catch(() => {
          // Offline or non-Tauri preview
        });
    };
    initApp();

    return () => {
      mediaQuery.removeEventListener("change", handleSystemTheme);
      window.removeEventListener("keydown", handleGlobalKeydown);
      window.removeEventListener("click", handleGlobalClick);
      window.removeEventListener("keydown", handleWorkspaceSettingsKeydown);
      window.removeEventListener("click", handleWorkspaceSettingsOutside);
      document.removeEventListener("contextmenu", onDocContextMenu);
      window.removeEventListener("focus", handleWindowFocus);
      window.removeEventListener("blur", handleWindowBlur);
      document.removeEventListener("visibilitychange", handleVisibilityChange);
      if (autofetchTimer !== null) {
        clearInterval(autofetchTimer);
        autofetchTimer = null;
      }
      if (localSyncTimer !== null) {
        clearInterval(localSyncTimer);
        localSyncTimer = null;
      }
    };
  });

  const activeWorkspaceModal = $derived(
    activeModal === "open" ||
      activeModal === "init" ||
      activeModal === "clone" ||
      activeModal === "publish" ||
      activeModal === "remote" ||
      activeModal === "stash" ||
      activeModal === "diag" ||
      activeModal === "logs" ||
      activeModal === "auth" ||
      activeModal === "settings"
      ? activeModal
      : null
  );

  const activeHistoryGithubModal = $derived(
    activeModal === "tag" ||
      activeModal === "rename_tag" ||
      activeModal === "squash" ||
      activeModal === "rebase" ||
      activeModal === "reword" ||
      activeModal === "pr" ||
      activeModal === "confirm"
      ? activeModal
      : null
  );
  const paletteCommands = $derived.by<PaletteCommand[]>(() => {
    const cmds: PaletteCommand[] = [
      { id: "open-repo", category: "Repo", label: t("app.menu.openRepo"), shortcut: "Ctrl+O", action: openOpenModal },
      { id: "clone-repo", category: "Repo", label: t("app.menu.cloneUrl"), action: () => { activeModal = "clone"; } },
      { id: "init-repo", category: "Repo", label: t("app.menu.initRepo"), action: () => { activeModal = "init"; } },
      { id: "ask-ai", category: "AI", label: t("palette.askAi"), action: () => { aiModalState = { task: "custom_ask", prompt: "", context: "" }; } },
      { id: "settings", category: "App", label: t("app.menu.settings"), action: openSettingsModal },
      { id: "accounts", category: "App", label: t("app.menu.manageGithubAccounts"), action: () => { activeModal = "auth"; } },
    ];
    if (activeRepo && currentStatus) {
      cmds.push(
        { id: "view-changes", category: "View", label: t("app.tabChanges"), action: () => switchView("changes") },
        { id: "view-history", category: "View", label: t("app.tabHistory"), action: () => switchView("history") },
        { id: "toggle-terminal", category: "Tools", label: t("terminal.toggleCommand"), shortcut: "Ctrl+`", action: () => { terminalOpen = !terminalOpen; } },
        { id: "open-reflog", category: "Tools", label: t("wsTools.tabReflog"), action: () => { workspaceToolsTab = "reflog"; } },
        { id: "open-submodules", category: "Tools", label: t("wsTools.tabSubmodules"), action: () => { workspaceToolsTab = "submodules"; } },
        { id: "open-worktrees", category: "Tools", label: t("tools.worktreeTitle"), action: () => { activeModal = "worktree"; } },
        { id: "open-branches", category: "Git", label: t("ctx.branch.manage"), action: openBranchModal },
        { id: "open-bisect", category: "Git", label: t("bisect.toolsMenu"), action: () => { activeModal = "bisect"; } },
        { id: "open-stash", category: "Git", label: t("app.manageStash"), action: () => { activeModal = "stash"; } },
        { id: "ai-review", category: "AI", label: t("palette.aiReview"), action: () => { aiModalState = { task: "review_changes", prompt: "Review staged & unstaged changes", context: "" }; } },
        { id: "ai-commit", category: "AI", label: t("palette.aiGenerateCommit"), action: () => void handleAiGenerateCommit() },
        { id: "git-fetch", category: "Git", label: t("app.fetch"), action: handleFetch },
        { id: "git-pull", category: "Git", label: t("app.pull"), action: handlePull },
        { id: "git-push", category: "Git", label: t("app.push"), action: handlePush }
      );
      const conflicted = currentStatus.files.find((f) => f.is_conflict);
      if (conflicted) {
        cmds.push({
          id: "resolve-conflict",
          category: "Git",
          label: `${t("conflict.resolveBtn")}: ${conflicted.path}`,
          action: () => { conflictFilePath = conflicted.path; },
        });
      }
      if (selectedFile) {
        const fp = selectedFile.path;
        cmds.push(
          { id: "file-blame", category: "Inspect", label: `Blame: ${fp}`, action: () => { inspectFileState = { path: fp, mode: "blame" }; } },
          { id: "file-history", category: "Inspect", label: `File History: ${fp}`, action: () => { inspectFileState = { path: fp, mode: "history" }; } }
        );
      }
      for (const b of branches.filter((x) => !x.is_current).slice(0, 15)) {
        cmds.push({
          id: `checkout-${b.name}`,
          category: "Branch",
          label: `${t("palette.checkoutPrefix")} ${b.name}`,
          action: async () => {
            await gitService.switchBranch(activeRepo, b.name);
            await loadRepo(activeRepo);
          },
        });
      }
    }
    return cmds;
  });
  type WorkspacePanelId = "file" | "commit" | "diff";
  type WorkspaceLayout = "group-left" | "group-right";
  type WorkspacePanel = { id: WorkspacePanelId; visible: boolean; order: number };
  const WORKSPACE_PANELS_STORAGE_KEY = "forge_workspace_panels_v2";
  const DEFAULT_WORKSPACE_PANELS: WorkspacePanel[] = [
    { id: "file", visible: true, order: 0 },
    { id: "commit", visible: true, order: 1 },
    { id: "diff", visible: true, order: 0 },
  ];
  const PANEL_LABEL_KEYS: Record<WorkspacePanelId, TranslationKey> = {
    file: "workspaceSettings.panelFile", commit: "workspaceSettings.panelCommit", diff: "workspaceSettings.panelDiff",
  };
  function readWorkspacePanels(): WorkspacePanel[] {
    try {
      const raw = JSON.parse(localStorage.getItem(WORKSPACE_PANELS_STORAGE_KEY) || "null");
      const saved: Array<{ id?: string; visible?: boolean; order?: number }> = Array.isArray(raw) ? raw : Array.isArray(raw?.panels) ? raw.panels : [];
      if (saved.length) return DEFAULT_WORKSPACE_PANELS.map((panel) => {
        const item = saved.find((entry: { id?: string }) => entry.id === panel.id);
        return item ? { ...panel, visible: typeof item.visible === "boolean" ? item.visible : panel.visible, order: typeof item.order === "number" && Number.isFinite(item.order) ? item.order : panel.order } : { ...panel };
      });
    } catch {}
    return DEFAULT_WORKSPACE_PANELS.map((panel) => ({ ...panel }));
  }
  function readWorkspaceLayout(): WorkspaceLayout {
    try {
      const raw = JSON.parse(localStorage.getItem(WORKSPACE_PANELS_STORAGE_KEY) || "null");
      const file = Array.isArray(raw) ? raw.find((item: { id?: string; dock?: string }) => item.id === "file") : raw?.panels?.find((item: { id?: string; dock?: string }) => item.id === "file");
      return file?.dock === "right" ? "group-right" : "group-left";
    } catch {}
    return "group-left";
  }
  let workspacePanels = $state<WorkspacePanel[]>(readWorkspacePanels());
  let workspaceLayout = $state<WorkspaceLayout>(readWorkspaceLayout());
  let workspaceSettingsOpen = $state(false);
  let workspaceSettingsButton = $state<HTMLButtonElement | undefined>();
  let draggedPanel: WorkspacePanelId | null = null;
  let workspaceDividerWidth = $state(50);
  let workspaceDividerDragging = $state(false);
  let workspaceDividerStartX = 0;
  let workspaceDividerStartWidth = 50;
  function readWorkspaceDividerWidth() {
    try { const raw = JSON.parse(localStorage.getItem(WORKSPACE_PANELS_STORAGE_KEY) || "null"); return typeof raw?.dividerWidth === "number" && raw.dividerWidth >= 20 && raw.dividerWidth <= 80 ? raw.dividerWidth : 50; }
    catch { return 50; }
  }
  workspaceDividerWidth = readWorkspaceDividerWidth();
  function startWorkspaceDividerResize(event: PointerEvent) {
    event.preventDefault();
    workspaceDividerDragging = true;
    workspaceDividerStartX = event.clientX;
    workspaceDividerStartWidth = workspaceDividerWidth;
    const onMove = (move: PointerEvent) => {
      const host = document.querySelector<HTMLElement>(".workspace-layout-panels");
      if (!host) return;
      const delta = (move.clientX - workspaceDividerStartX) / host.clientWidth * 100;
      workspaceDividerWidth = Math.min(80, Math.max(20, workspaceDividerStartWidth + delta * (workspaceLayout === "group-left" ? 1 : -1)));
    };
    const onUp = () => {
      workspaceDividerDragging = false;
      persistWorkspacePanels();
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
      window.removeEventListener("pointercancel", onUp);
    };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
    window.addEventListener("pointercancel", onUp);
  }
  function handleWorkspaceDividerKeydown(event: KeyboardEvent) {
    if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
    event.preventDefault();
    const step = event.shiftKey ? 5 : 2;
    const direction = (event.key === "ArrowRight" ? 1 : -1) * (workspaceLayout === "group-left" ? 1 : -1);
    workspaceDividerWidth = Math.min(80, Math.max(20, workspaceDividerWidth + step * direction));
    persistWorkspacePanels();
  }
  function persistWorkspacePanels() { localStorage.setItem(WORKSPACE_PANELS_STORAGE_KEY, JSON.stringify({ panels: workspacePanels, layout: workspaceLayout, dividerWidth: workspaceDividerWidth })); }
  function orderedWorkspacePanels() { return workspacePanels.filter((panel) => panel.visible && panel.id !== "diff").sort((a, b) => a.order - b.order).map((panel) => panel.id); }
  function updateWorkspacePanel(id: WorkspacePanelId, patch: Partial<WorkspacePanel>) { workspacePanels = workspacePanels.map((panel) => panel.id === id ? { ...panel, ...patch } : panel); persistWorkspacePanels(); }
  function toggleWorkspacePanel(id: WorkspacePanelId) { const panel = workspacePanels.find((item) => item.id === id); if (panel) updateWorkspacePanel(id, { visible: !panel.visible }); }
  function setWorkspaceLayout(layout: WorkspaceLayout) { workspaceLayout = layout; persistWorkspacePanels(); }
  function swapWorkspaceOrder() { const file = workspacePanels.find((panel) => panel.id === "file")?.order ?? 0; const commit = workspacePanels.find((panel) => panel.id === "commit")?.order ?? 1; workspacePanels = workspacePanels.map((panel) => panel.id === "file" ? { ...panel, order: commit } : panel.id === "commit" ? { ...panel, order: file } : panel); persistWorkspacePanels(); }
  function resetWorkspacePanels() { workspacePanels = DEFAULT_WORKSPACE_PANELS.map((panel) => ({ ...panel })); workspaceLayout = "group-left"; workspaceDividerWidth = 50; persistWorkspacePanels(); }
  function panelDragStart(id: WorkspacePanelId) { if (id !== "diff") draggedPanel = id; }
  function panelDrop(targetId: WorkspacePanelId) { if (draggedPanel && targetId !== "diff" && draggedPanel !== targetId) swapWorkspaceOrder(); draggedPanel = null; }
  function closeWorkspaceSettings() { workspaceSettingsOpen = false; workspaceSettingsButton?.focus(); }
  function handleWorkspaceSettingsKeydown(event: KeyboardEvent) { if (event.key === "Escape") closeWorkspaceSettings(); }
  function handleWorkspaceSettingsOutside(event: MouseEvent) { if (workspaceSettingsOpen && !(event.target as Element)?.closest("[data-workspace-settings]")) closeWorkspaceSettings(); }
  const visibleWorkspacePanels = $derived(workspacePanels.filter((panel) => panel.visible));
  const groupPanelsVisible = $derived(orderedWorkspacePanels().length > 0);
  const diffVisible = $derived(visibleWorkspacePanels.some((panel) => panel.id === "diff"));
  const workspaceGridStyle = $derived(`grid-template-columns: ${
    groupPanelsVisible && diffVisible
      ? (workspaceLayout === "group-left"
          ? `minmax(0, ${workspaceDividerWidth}fr) 5px minmax(0, ${100 - workspaceDividerWidth}fr)`
          : `minmax(0, ${100 - workspaceDividerWidth}fr) 5px minmax(0, ${workspaceDividerWidth}fr)`)
      : "minmax(0, 1fr)"
  }; grid-template-areas: ${
    groupPanelsVisible && diffVisible
      ? (workspaceLayout === "group-left" ? '"group . diff"' : '"diff . group"')
      : groupPanelsVisible ? '"group"' : diffVisible ? '"diff"' : '"empty"'
  };`);
 </script>

{#snippet workspacePanel(panelId: WorkspacePanelId, index: number)}
  {@const siblings = orderedWorkspacePanels()}
  {@const prev = index > 0 ? siblings[index - 1] : null}
  {@const next = index < siblings.length - 1 ? siblings[index + 1] : null}
  {@const connectedTop = prev === "file" && panelId === "commit"}
  {@const connectedBottom = panelId === "file" && next === "commit"}
  <article class="workspace-panel-card {panelId === 'commit' ? 'workspace-commit-card' : ''} {connectedTop ? 'is-connected-top' : ''} {connectedBottom ? 'is-connected-bottom' : ''}" style={panelId === "commit" ? `--commit-panel-h: ${commitPanelHeight}px;` : undefined} data-panel={panelId} role="listitem" draggable={panelId !== "diff"} ondragstart={() => panelDragStart(panelId)} ondragover={(event) => event.preventDefault()} ondrop={(event) => { event.stopPropagation(); panelDrop(panelId); }}>
    {#if panelId === "file"}
      <Changes repoPath={activeRepo} {externalEditorLabel} status={currentStatus!} panelMode="files" {selectedFile} onSelectFile={selectFile} onToggleStage={toggleStage} onStageAll={handleStageAll} onUnstageAll={handleUnstageAll} onDiscardSelected={handleDiscardSelected} onCommit={handleCommit} onUndoCommit={handleUndoCommit} onAbortMerge={handleAbortMerge} onContinueMerge={handleContinueMerge} onAbortRebase={handleAbortRebase} onContinueRebase={handleContinueRebase} onAbortCherryPick={handleAbortCherryPick} onContinueCherryPick={handleContinueCherryPick} onAbortRevert={handleAbortRevert} onContinueRevert={handleContinueRevert} onDiscardFile={handleDiscardFile} onDiscardAll={handleDiscardAll} onAddToGitignore={handleAddToGitignore} onRevealFile={openInFileManagerTarget} onOpenInEditor={openInEditorFile} onOpenConflictEditor={(fp) => (conflictFilePath = fp)} onInspectFile={(fp, mode) => (inspectFileState = { path: fp, mode })} onAiGenerateCommit={handleAiGenerateCommit} onOpenStash={() => (activeModal = "stash")} onStatus={setStatus} />
    {:else if panelId === "commit"}
      <div class="commit-panel-content" style={`height: ${commitPanelHeight}px; flex-basis: ${commitPanelHeight}px;`}><Changes repoPath={activeRepo} {externalEditorLabel} status={currentStatus!} panelMode="commit" {selectedFile} onSelectFile={selectFile} onToggleStage={toggleStage} onStageAll={handleStageAll} onUnstageAll={handleUnstageAll} onDiscardSelected={handleDiscardSelected} onCommit={handleCommit} onUndoCommit={handleUndoCommit} onAbortMerge={handleAbortMerge} onContinueMerge={handleContinueMerge} onAbortRebase={handleAbortRebase} onContinueRebase={handleContinueRebase} onAbortCherryPick={handleAbortCherryPick} onContinueCherryPick={handleContinueCherryPick} onAbortRevert={handleAbortRevert} onContinueRevert={handleContinueRevert} onDiscardFile={handleDiscardFile} onDiscardAll={handleDiscardAll} onAddToGitignore={handleAddToGitignore} onRevealFile={openInFileManagerTarget} onOpenInEditor={openInEditorFile} onOpenConflictEditor={(fp) => (conflictFilePath = fp)} onInspectFile={(fp, mode) => (inspectFileState = { path: fp, mode })} onAiGenerateCommit={handleAiGenerateCommit} onOpenStash={() => (activeModal = "stash")} onStatus={setStatus} /></div>
      {:else if panelId === "diff"}
        {#if selectedFile}<Diff repoPath={activeRepo} {externalEditorLabel} filePath={selectedFile.path} diffText={currentDiff} imageDiffData={currentImageDiff} {ignoreWhitespace} isLfs={selectedFile.is_lfs} isStaged={selectedFile.staged} onToggleWhitespace={toggleWhitespace} onAddToGitignore={handleAddToGitignore} onStageHunk={handleStageHunk} onStageLines={handleStageLines} onDiscardHunk={handleDiscardHunk} onDiscardLines={handleDiscardLines} onInspectFile={(fp: string, mode: "blame" | "history") => (inspectFileState = { path: fp, mode })} onAiExplainDiff={handleAiExplainDiff} onRevealFile={openInFileManagerTarget} onOpenInEditor={openInEditorFile} onStatus={setStatus} prComments={prReviewComments} activePrNumber={activePrReviewNumber} onCreatePrComment={handleCreatePrComment} />{:else}<div class="empty-state">{t("diff.emptyDiff")}</div>{/if}
      {/if}
    </article>
    {#if (panelId === "file" && next === "commit") || (panelId === "commit" && next === "file")}
      <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
      <div class="commit-panel-resizer {isResizingCommit ? 'is-resizing' : ''}" role="separator" aria-orientation="horizontal" aria-label={t("changes.commitMessageTitle")} aria-valuemin={COMMIT_MIN} aria-valuemax={COMMIT_MAX} aria-valuenow={commitPanelHeight} tabindex="0" onpointerdown={startCommitResize} onkeydown={handleCommitResizerKeydown}></div>
    {/if}
  {/snippet}
<div
  class="app-container"
  style="--app-header-h: {appHeaderHeight}px; --app-footer-h: {appFooterHeight}px;"
>
  <header class="app-header" bind:offsetHeight={appHeaderHeight}>
    <div class="header-left">
      <button
        type="button"
        class="picker-pill"
        onclick={openOpenModal}
        oncontextmenu={openRepoHeaderContextMenu}
        title={activeRepo || t("app.openRepoTooltip")}
      >
        <ProviderIcon
          provider={hostingRepoInfo?.provider || (githubRepoInfo ? "github" : "local")}
          size={14}
        />
        <span class="picker-pill-label">
          {activeRepo ? repoAliases[activeRepo] || pathBaseName(activeRepo) : t("app.openRepo")}
        </span>
        <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true" style="flex-shrink: 0; opacity: 0.65;">
          <path d="M4.427 7.427l3.396 3.396a.25.25 0 00.354 0l3.396-3.396A.25.25 0 0011.396 7H4.604a.25.25 0 00-.177.427z" />
        </svg>
      </button>

      {#if currentStatus}
        <button
          type="button"
          class="picker-pill"
          onclick={openBranchModal}
          oncontextmenu={openBranchHeaderContextMenu}
          title={t("app.activeBranchTooltip", { branch: formatBranchDisplay(currentStatus.branch) })}
        >
          <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true" style="flex-shrink: 0; color: var(--text-accent);">
            <path d="M9.5 3.25a2.25 2.25 0 1 1 3 2.122V6A2.5 2.5 0 0 1 10 8.5H6a1 1 0 0 0-1 1v1.128a2.251 2.251 0 1 1-1.5 0V5.372a2.25 2.25 0 1 1 1.5 0v1.836A2.493 2.493 0 0 1 6 7h4a1 1 0 0 0 1-1v-.628A2.25 2.25 0 0 1 9.5 3.25Zm-6 0a.75.75 0 1 0 1.5 0 .75.75 0 0 0-1.5 0Zm8.25-.75a.75.75 0 1 0 0 1.5.75.75 0 0 0 0-1.5ZM4.25 12a.75.75 0 1 0 0 1.5.75.75 0 0 0 0-1.5Z" />
          </svg>
          <span class="picker-pill-label">{formatBranchDisplay(currentStatus.branch)}</span>
          {#if branchPushMeta}
            <span class="picker-pill-meta tabular-nums {branchPushMeta.tone}" title={branchPushMeta.title}>
              {branchPushMeta.text}
            </span>
          {/if}
        </button>
      {/if}
    </div>

    <div class="header-center">
      <div class="nav-tabs" role="tablist" aria-label={t("app.mainViewAria")}>
        <button
          type="button"
          role="tab"
          aria-selected={activeView === "changes"}
          class="nav-tab {activeView === 'changes' ? 'active' : ''}"
          onclick={() => switchView("changes")}
        >
          <span>{t("app.tabChanges")}</span>
          {#if currentStatus}
            <span class="count-pill">{currentStatus.files.length}</span>
          {/if}
        </button>
        <button
          type="button"
          role="tab"
          aria-selected={activeView === "history"}
          class="nav-tab {activeView === 'history' ? 'active' : ''}"
          onclick={() => switchView("history")}
        >
          <span>{t("app.tabHistory")}</span>
        </button>
      </div>
    </div>

    <div class="header-right">
      {#if currentStatus}
        {#if !hasRemote}
          <button
            type="button"
            class="btn-secondary"
            style="height: 28px; padding: 0 10px; font-size: 12px;"
            onclick={() => (activeModal = "remote")}
            title={t("app.connectRemoteTooltip")}
          >
            {t("app.connectRemote")}
          </button>
          <button
            type="button"
            class="btn-primary"
            style="height: 28px; padding: 0 10px; font-size: 12px;"
            onclick={() => (activeModal = "publish")}
          >
            {t("app.publishToGithub")}
          </button>
        {:else}
          {@const pushCount = aheadBehind.has_upstream ? aheadBehind.ahead : aheadBehind.unpushed_commits}
          {@const needsPublishBranch = currentStatus.has_commits && !aheadBehind.has_upstream}
          <div class="menu-wrapper" data-menu-root>
            <div class="btn-group">
              <button type="button" onclick={handleFetch} disabled={isLoading} title={t("app.fetchTooltip")}>
                {t("app.fetch")}
              </button>
              <button
                type="button"
                onclick={handlePull}
                disabled={isLoading || !currentStatus.has_commits}
                class={aheadBehind.behind > 0 ? "is-highlighted" : ""}
                title={currentStatus.has_commits ? t("app.pullTooltip") : t("app.noCommitsYet")}
              >
                {t("app.pull")}{#if aheadBehind.behind > 0}<span class="tabular-nums">↓{aheadBehind.behind}</span>{/if}
              </button>
              <button
                type="button"
                onclick={handlePush}
                disabled={isLoading || !currentStatus.has_commits}
                class={(aheadBehind.ahead > 0 || needsPublishBranch) && currentStatus.has_commits ? "is-highlighted" : ""}
                title={!currentStatus.has_commits
                  ? t("app.makeFirstCommitBeforePush")
                  : needsPublishBranch
                    ? t("app.pushBranchTooltip", { branch: formatBranchDisplay(currentStatus.branch) })
                    : t("app.pushTooltip")}
              >
                {needsPublishBranch ? t("app.pushBranch") : t("app.push")}{#if pushCount > 0}<span class="tabular-nums">↑{pushCount}</span>{/if}
              </button>
              <button
                type="button"
                aria-label={t("app.advancedSyncAria")}
                aria-expanded={syncMenuOpen}
                style="padding: 0 6px;"
                onclick={() => {
                  syncMenuOpen = !syncMenuOpen;
                  toolsMenuOpen = false;
                }}
              >
                <svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
                  <path d="M4.427 7.427l3.396 3.396a.25.25 0 00.354 0l3.396-3.396A.25.25 0 0011.396 7H4.604a.25.25 0 00-.177.427z" />
                </svg>
              </button>
            </div>

            {#if syncMenuOpen}
              <div class="popover-menu" role="menu">
                <button
                  type="button"
                  role="menuitem"
                  class="popover-item"
                  onclick={() => {
                    syncMenuOpen = false;
                    activeModal = "remote";
                  }}
                >
                  <span>{t("app.manageRemotesMenu")}</span>
                  <span class="popover-item-hint">
                    {repoRemotes.length === 1
                      ? t("app.remoteCountOne", { count: repoRemotes.length })
                      : t("app.remoteCountMany", { count: repoRemotes.length })}
                  </span>
                </button>
                <div class="popover-divider"></div>
                <button
                  type="button"
                  role="menuitem"
                  class="popover-item danger"
                  disabled={!currentStatus.has_commits}
                  onclick={handleForcePush}
                >
                  <span>{t("app.forcePushMenu")}</span>
                  <span class="popover-item-hint">--force-with-lease</span>
                </button>
                <button
                  type="button"
                  role="menuitem"
                  class="popover-item"
                  disabled={!currentStatus.has_commits}
                  onclick={handleSyncFork}
                >
                  <span>{t("app.syncForkMenu")}</span>
                  <span class="popover-item-hint">upstream</span>
                </button>
              </div>
            {/if}
          </div>
        {/if}

        {#if githubRepoInfo || hostingRepoInfo}
          <button
            type="button"
            class="btn-secondary header-pr-btn"
            style="height: 28px; padding: 0 9px; font-size: 12px;"
            onclick={openPrModal}
            title={t("app.prIssuesTooltip")}
          >
            {t("app.prIssuesBtn")}
          </button>
        {/if}
        <button
          type="button"
          class="btn-secondary"
          style="height: 28px; padding: 0 9px; font-size: 12px;"
          onclick={() => (aiModalState = { task: "custom_ask", prompt: "", context: "" })}
          title={t("ai.assistantTitle")}
        >
          ✦ AI
        </button>
        <button
          type="button"
          class="btn-secondary {terminalOpen ? 'is-highlighted' : ''}"
          style="height: 28px; padding: 0 9px; font-size: 12px;"
          onclick={() => (terminalOpen = !terminalOpen)}
          title={t("terminal.toggleCommand")}
        >
          &gt;_
        </button>
        <button
          type="button"
          class="btn-secondary"
          style="height: 28px; padding: 0 8px; font-size: 11px; font-family: var(--font-mono);"
          onclick={() => (paletteOpen = true)}
          title={t("palette.title")}
        >
          ⌘K
        </button>

        <div class="header-divider header-quick-divider"></div>

        <div class="header-quick-icons">
          <button
            type="button"
            class="icon-btn"
            onclick={openInEditor}
            title={t("app.openInEditorTitle", { editor: externalEditorLabel })}
            aria-label={t("app.openInEditorAria")}
          >
            <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
              <path d="M4.72 3.22a.75.75 0 0 1 1.06 1.06L2.06 8l3.72 3.72a.75.75 0 1 1-1.06 1.06L.47 8.53a.75.75 0 0 1 0-1.06l4.25-4.25Zm6.56 0a.75.75 0 1 0-1.06 1.06L13.94 8l-3.72 3.72a.75.75 0 1 0 1.06 1.06l4.25-4.25a.75.75 0 0 0 0-1.06l-4.25-4.25Z" />
            </svg>
          </button>
          <button type="button" class="icon-btn" onclick={openInTerminal} title={t("app.openInTerminal")} aria-label={t("app.openInTerminal")}>
            <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
              <path d="M0 2.75C0 1.784.784 1 1.75 1h12.5c.966 0 1.75.784 1.75 1.75v10.5A1.75 1.75 0 0 1 14.25 15H1.75A1.75 1.75 0 0 1 0 13.25Zm1.75-.25a.25.25 0 0 0-.25.25v10.5c0 .138.112.25.25.25h12.5a.25.25 0 0 0 .25-.25V2.75a.25.25 0 0 0-.25-.25ZM7.25 8a.749.749 0 0 1-.22.53l-2.25 2.25a.749.749 0 0 1-1.275-.326.749.749 0 0 1 .215-.734L5.44 8 3.72 6.28a.749.749 0 0 1 .326-1.275.749.749 0 0 1 .734.215l2.25 2.25c.141.14.22.331.22.53Zm1.5 1.5h3a.75.75 0 0 1 0 1.5h-3a.75.75 0 0 1 0-1.5Z" />
            </svg>
          </button>
          <button type="button" class="icon-btn" onclick={openInFileManager} title={t("app.openInFileManager")} aria-label={t("app.openInFileManager")}>
            <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
              <path d="M1.75 1A1.75 1.75 0 0 0 0 2.75v10.5C0 14.216.784 15 1.75 15h12.5A1.75 1.75 0 0 0 16 13.25v-8.5A1.75 1.75 0 0 0 14.25 3H7.5a.25.25 0 0 1-.2-.1l-.9-1.2C6.07 1.26 5.55 1 5 1H1.75Z" />
            </svg>
          </button>
          <button type="button" class="icon-btn" onclick={() => (activeModal = "stash")} title={t("app.manageStash")} aria-label={t("app.manageStash")}>
            <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
              <path d="M1.75 2h12.5c.966 0 1.75.784 1.75 1.75v8.5A1.75 1.75 0 0 1 14.25 14H1.75A1.75 1.75 0 0 1 0 12.25v-8.5C0 2.784.784 2 1.75 2Zm0 1.5a.25.25 0 0 0-.25.25v2.75h13V3.75a.25.25 0 0 0-.25-.25H1.75ZM1.5 8v4.25c0 .138.112.25.25.25h12.5a.25.25 0 0 0 .25-.25V8H1.5Zm4 1.5h5a.75.75 0 0 1 0 1.5h-5a.75.75 0 0 1 0-1.5Z" />
            </svg>
          </button>
          <div class="menu-wrapper" data-workspace-settings>
            <button
              type="button"
              class="icon-btn"
              bind:this={workspaceSettingsButton}
              onclick={() => {
                workspaceSettingsOpen = !workspaceSettingsOpen;
                syncMenuOpen = false;
                toolsMenuOpen = false;
              }}
              title={t("workspaceSettings.title")}
              aria-label={t("workspaceSettings.title")}
              aria-expanded={workspaceSettingsOpen}
              aria-haspopup="true"
            >
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" />
                <circle cx="12" cy="12" r="3" />
              </svg>
            </button>
            {#if workspaceSettingsOpen}
              <div class="popover-menu workspace-settings-popover" role="menu" aria-label={t("workspaceSettings.title")}>
                <div class="workspace-settings-head">
                  <span class="workspace-settings-title">{t("workspaceSettings.title")}</span>
                  <button type="button" class="link-btn workspace-settings-reset" onclick={resetWorkspacePanels}>
                    {t("workspaceSettings.reset")}
                  </button>
                </div>
                <div class="workspace-settings-list">
                  {#each workspacePanels as panel (panel.id)}
                    <div class="workspace-settings-row" data-panel-id={panel.id}>
                      <span class="workspace-settings-label">{t(PANEL_LABEL_KEYS[panel.id])}</span>
                      <button type="button" class="eye-btn {panel.visible ? 'is-visible' : ''}" onclick={() => toggleWorkspacePanel(panel.id)} title={panel.visible ? t("workspaceSettings.hide") : t("workspaceSettings.show")} aria-label={panel.visible ? t("workspaceSettings.hide") : t("workspaceSettings.show")} aria-pressed={panel.visible}>
                        <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4" aria-hidden="true"><path d="M1.2 8s2.2-4 6.8-4 6.8 4 6.8 4-2.2 4-6.8 4S1.2 8 1.2 8Z" /><circle cx="8" cy="8" r="1.8" fill="currentColor" stroke="none" />{#if !panel.visible}<path d="M2 2 14 14" stroke-width="1.8" stroke-linecap="round" />{/if}</svg>
                      </button>
                    </div>
                  {/each}
                </div>
                <fieldset class="workspace-settings-choice">
                  <legend>{t("workspaceSettings.layout")}</legend>
                  <label><input type="radio" name="workspace-layout" checked={workspaceLayout === "group-left"} onchange={() => setWorkspaceLayout("group-left")} /> {t("workspaceSettings.layoutGroupLeft")}</label>
                  <label><input type="radio" name="workspace-layout" checked={workspaceLayout === "group-right"} onchange={() => setWorkspaceLayout("group-right")} /> {t("workspaceSettings.layoutGroupRight")}</label>
                </fieldset>
                <fieldset class="workspace-settings-choice">
                  <legend>{t("workspaceSettings.stackOrder")}</legend>
                  <label><input type="radio" name="workspace-order" checked={workspacePanels.find((panel) => panel.id === "file")?.order === 0} onchange={() => { if (workspacePanels.find((panel) => panel.id === "file")?.order !== 0) swapWorkspaceOrder(); }} /> {t("workspaceSettings.fileAboveCommit")}</label>
                  <label><input type="radio" name="workspace-order" checked={workspacePanels.find((panel) => panel.id === "commit")?.order === 0} onchange={() => { if (workspacePanels.find((panel) => panel.id === "commit")?.order !== 0) swapWorkspaceOrder(); }} /> {t("workspaceSettings.commitAboveFile")}</label>
                </fieldset>
                <p class="workspace-settings-hint">{t("workspaceSettings.hint")}</p>
              </div>
            {/if}
          </div>
        </div>
        <button type="button" class="icon-btn" onclick={() => loadRepo(activeRepo)} disabled={isLoading} title={t("app.refreshRepo")} aria-label={t("app.refreshRepo")}>
          <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
            <path d="M1.705 8.005a.75.75 0 0 1 .834.656 5.5 5.5 0 0 0 9.592 2.97l-1.204-1.204a.25.25 0 0 1 .177-.427h3.646a.25.25 0 0 1 .25.25v3.646a.25.25 0 0 1-.427.177l-1.38-1.38A7.002 7.002 0 0 1 1.05 8.84a.75.75 0 0 1 .656-.834ZM8 2.5a5.487 5.487 0 0 0-4.131 1.869l1.204 1.204A.25.25 0 0 1 4.896 6H1.25A.25.25 0 0 1 1 5.75V2.104a.25.25 0 0 1 .427-.177l1.38 1.38A7.002 7.002 0 0 1 14.95 7.16a.75.75 0 0 1-1.49.178A5.5 5.5 0 0 0 8 2.5Z" />
          </svg>
        </button>
      {/if}

      {#if currentStatus}
        {@const headerAccHealth = activeRepoAccount ? accountHealthMap[activeRepoAccount.id] : undefined}
        {@const isHeaderAccWarn =
          headerAccHealth?.status === "expired" || headerAccHealth?.status === "sso_required"}
        <button
          type="button"
          class="header-account-chip"
          onclick={() => (activeModal = "auth")}
          title={activeRepoAccount
            ? t("app.githubAccountActiveTooltip", { login: activeRepoAccount.login, host: activeRepoAccount.host })
            : t("app.connectGithubAccountTooltip")}
        >
          {#if activeRepoAccount}
            <span class="header-account-avatar-wrap">
              {#if activeAvatarUrl}
                <img
                  class="header-account-avatar"
                  src={activeAvatarUrl}
                  alt=""
                  onerror={() => (failedAvatarUrl = activeRepoAccount.avatar_url ?? null)}
                />
              {:else}
                <span class="header-account-avatar-fallback">
                  {activeRepoAccount.login.slice(0, 1).toUpperCase()}
                </span>
              {/if}
              <span class="header-account-status-dot {isHeaderAccWarn ? 'is-warn' : ''}"></span>
            </span>
            <span style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
              @{activeRepoAccount.login}{#if activeRepoAccount.host !== "github.com"}<span class="header-account-host"> · {activeRepoAccount.host}</span>{/if}
            </span>
          {:else}
            <span class="header-account-dot"></span>
            <span style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
              {t("app.signInGithub")}
            </span>
          {/if}
        </button>
      {/if}

      <div class="menu-wrapper" data-menu-root>
        <button
          type="button"
          class="icon-btn"
          aria-label={t("app.toolsMenuAria")}
          aria-expanded={toolsMenuOpen}
          onclick={() => {
            toolsMenuOpen = !toolsMenuOpen;
            syncMenuOpen = false;
          }}
        >
          {#if appErrorLogs.length > 0}
            <span class="update-dot is-error" title={t("app.errorLogDotTitle", { count: appErrorLogs.length })}></span>
          {:else if appUpdateInfo?.has_update}
            <span class="update-dot" title={t("app.updateAvailableDotTitle", { version: appUpdateInfo.latest_version })}></span>
          {/if}
          <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
            <path d="M8 9a1.5 1.5 0 1 0 0-3 1.5 1.5 0 0 0 0 3ZM1.5 9a1.5 1.5 0 1 0 0-3 1.5 1.5 0 0 0 0 3Zm13 0a1.5 1.5 0 1 0 0-3 1.5 1.5 0 0 0 0 3Z" />
          </svg>
        </button>

        {#if toolsMenuOpen}
          <div class="popover-menu" role="menu">
            <button type="button" role="menuitem" class="popover-item" onclick={openOpenModal}>
              <span>{t("app.menu.openRepo")}</span>
              <span class="popover-item-hint">Ctrl+O</span>
            </button>
            <button
              type="button"
              role="menuitem"
              class="popover-item"
              onclick={() => {
                toolsMenuOpen = false;
                activeModal = "clone";
              }}
            >
              <span>{t("app.menu.cloneUrl")}</span>
            </button>
            <button
              type="button"
              role="menuitem"
              class="popover-item"
              onclick={() => {
                toolsMenuOpen = false;
                activeModal = "init";
              }}
            >
              <span>{t("app.menu.initRepo")}</span>
            </button>

            {#if currentStatus}
              <div class="popover-divider"></div>
              <button
                type="button"
                role="menuitem"
                class="popover-item"
                onclick={() => {
                  toolsMenuOpen = false;
                  activeModal = "remote";
                }}
              >
                <span>{t("app.menu.manageRemotes")}</span>
                <span class="popover-item-hint">{repoRemotes.length}</span>
              </button>
              <button
                type="button"
                role="menuitem"
                class="popover-item"
                onclick={() => {
                  toolsMenuOpen = false;
                  openCreateTagModal(null);
                }}
              >
                <span>{t("app.menu.createTagHead")}</span>
              </button>
            {/if}
              <div class="popover-divider"></div>
              <button
                type="button"
                role="menuitem"
                class="popover-item"
                onclick={() => {
                  toolsMenuOpen = false;
                  paletteOpen = true;
                }}
              >
                <span>{t("palette.title")}</span>
                <span class="popover-item-hint">Ctrl+K</span>
              </button>
              <button
                type="button"
                role="menuitem"
                class="popover-item"
                onclick={() => {
                  toolsMenuOpen = false;
                  terminalOpen = !terminalOpen;
                }}
              >
                <span>{t("terminal.toggleCommand")}</span>
                <span class="popover-item-hint">Ctrl+`</span>
              </button>
              <button
                type="button"
                role="menuitem"
                class="popover-item"
                onclick={() => {
                  toolsMenuOpen = false;
                  workspaceToolsTab = "reflog";
                }}
              >
                <span>{t("wsTools.menuLabel")}</span>
              </button>
              <button
                type="button"
                role="menuitem"
                class="popover-item"
                onclick={() => {
                  toolsMenuOpen = false;
                  activeModal = "worktree";
                }}
              >
                <span>{t("tools.worktreeTitle")}</span>
              </button>
              <button
                type="button"
                role="menuitem"
                class="popover-item"
                onclick={() => {
                  toolsMenuOpen = false;
                  activeModal = "bisect";
                }}
              >
                <span>{t("bisect.toolsMenu")}</span>
              </button>
              <button
                type="button"
                role="menuitem"
                class="popover-item"
                onclick={() => {
                  toolsMenuOpen = false;
                  aiModalState = { task: "custom_ask", prompt: "", context: "" };
                }}
              >
                <span>✦ {t("ai.assistantTitle")}</span>
              </button>

            <div class="popover-divider"></div>
            <button
              type="button"
              role="menuitem"
              class="popover-item menu-account-item"
              onclick={() => {
                toolsMenuOpen = false;
                activeModal = "auth";
              }}
            >
              <span class="menu-account-label">
                {#if activeRepoAccount}
                  {#if activeAvatarUrl}
                    <img
                      class="menu-account-avatar"
                      src={activeAvatarUrl}
                      alt=""
                      onerror={() => (failedAvatarUrl = activeRepoAccount.avatar_url ?? null)}
                    />
                  {:else}
                    <span class="menu-account-avatar-fallback">
                      {activeRepoAccount.login.slice(0, 1).toUpperCase()}
                    </span>
                  {/if}
                  <span class="menu-account-login">
                    {t("app.menu.githubAccountNamed", { login: activeRepoAccount.login })}
                  </span>
                  {#if activeRepoAccount.label}
                    <span class="menu-account-tag">{activeRepoAccount.label}</span>
                  {/if}
                {:else}
                  <svg class="menu-account-empty-icon" width="14" height="14" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
                    <path d="M10.561 8.073a6.005 6.005 0 0 1 3.432 5.142.75.75 0 1 1-1.498.07 4.5 4.5 0 0 0-8.99 0 .75.75 0 0 1-1.498-.07 6.004 6.004 0 0 1 3.431-5.142 3.999 3.999 0 1 1 5.123 0ZM10.5 5a2.5 2.5 0 1 0-5 0 2.5 2.5 0 0 0 5 0Z" />
                  </svg>
                  <span>{t("app.menu.manageGithubAccounts")}</span>
                {/if}
              </span>
              {#if accountOverview.accounts.length > 0}
                <span class="menu-account-count-badge">
                  {accountOverview.accounts.length === 1
                    ? t("app.menu.accountsCountOne", { count: accountOverview.accounts.length })
                    : t("app.menu.accountsCountMany", { count: accountOverview.accounts.length })}
                </span>
              {/if}
            </button>
            <button type="button" role="menuitem" class="popover-item" onclick={openDiagModal}>
              <span>{t("app.menu.diagnostics")}</span>
            </button>
            <button type="button" role="menuitem" class="popover-item" onclick={openLogsModal}>
              <span>{t("app.menu.appLogs")}</span>
              {#if appErrorLogs.length > 0}
                <span class="popover-item-hint is-error">{appErrorLogs.length}</span>
              {/if}
            </button>
            <button type="button" role="menuitem" class="popover-item" onclick={openSettingsModal}>
              <span>{t("app.menu.settings")}</span>
            </button>
          </div>
        {/if}
      </div>
    </div>
  </header>

  {#if !currentStatus}
    <main class="content-panel">
      <div class="empty-state">
        {#if pendingInitRepo}
          <div class="welcome-panel">
            <div>
              <div style="display: flex; align-items: center; gap: 8px; margin-bottom: 4px;">
                <span class="badge-warning">{t("app.onboarding.noGitBadge")}</span>
                <span style="font-family: var(--font-mono); font-size: 11px; color: var(--text-muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
                  {pendingInitRepo}
                </span>
              </div>
              <h1 class="welcome-title">{t("app.onboarding.title")}</h1>
              <p class="welcome-sub">
                {t("app.onboarding.subtitle")}
              </p>
            </div>

            <div class="form-grid-2">
              <div class="form-field">
                <label class="form-label" for="onboard-branch">{t("app.onboarding.defaultBranch")}</label>
                <input id="onboard-branch" type="text" class="form-input" bind:value={initDirBranch} placeholder="main" />
              </div>
              <div class="form-field">
                <label class="form-label" for="onboard-gitignore">{t("app.onboarding.gitignoreTemplate")}</label>
                <select id="onboard-gitignore" class="form-select" bind:value={initDirGitignore}>
                  <option value="None">{t("app.onboarding.gitignoreNone")}</option>
                  <option value="Node">Node.js</option>
                  <option value="Rust">Rust</option>
                  <option value="Python">Python</option>
                </select>
              </div>
            </div>

            <div class="form-field">
              <label class="form-label" for="onboard-remote">{t("app.onboarding.remoteUrlOptional")}</label>
              <input
                id="onboard-remote"
                type="text"
                class="form-input"
                bind:value={initDirRemoteUrl}
                placeholder={t("app.onboarding.remotePlaceholder")}
              />
            </div>

            <label class="checkbox-label">
              <input type="checkbox" bind:checked={initDirCommit} />
              <span>{t("app.onboarding.autoInitialCommit")}</span>
            </label>

            {#if initDirCommit}
              <div class="form-field">
                <label class="form-label" for="onboard-commit-msg">{t("app.onboarding.initialCommitMsgLabel")}</label>
                <input id="onboard-commit-msg" type="text" class="form-input" bind:value={initDirCommitMsg} />
              </div>
            {/if}

            <div class="welcome-actions">
              <button type="button" class="btn-primary" disabled={isLoading} onclick={handleInitPendingDir}>
                {isLoading ? t("app.onboarding.processing") : t("app.onboarding.initHereBtn")}
              </button>
              <button type="button" class="btn-secondary" onclick={openOpenModal}>{t("app.onboarding.openAnotherFolder")}</button>
              <button
                type="button"
                class="btn-secondary"
                onclick={() => {
                  pendingInitRepo = "";
                  setStatus(t("app.ready"));
                }}
              >
                {t("app.onboarding.back")}
              </button>
            </div>
          </div>
        {:else}
          <div class="welcome-panel">
            <div style="display: flex; align-items: center; gap: 14px;">
              <img src="/icon.svg" alt={t("app.welcome.logoAlt")} width="56" height="56" style="width: 56px; height: 56px; flex-shrink: 0; border-radius: 12px;" />
              <div>
                <h1 class="welcome-title">Forge Desktop</h1>
                <p class="welcome-sub">
                  {t("app.welcome.tagline")}
                </p>
              </div>
            </div>
            <div class="welcome-actions">
              <button type="button" class="btn-primary" onclick={openOpenModal}>{t("app.welcome.openRepo")}</button>
              <button type="button" class="btn-secondary" onclick={() => (activeModal = "clone")}>{t("app.welcome.cloneUrl")}</button>
              <button type="button" class="btn-secondary" onclick={() => (activeModal = "init")}>{t("app.welcome.newRepo")}</button>
              <button type="button" class="btn-secondary welcome-account-btn" onclick={() => (activeModal = "auth")}>
                {#if activeRepoAccount}
                  {#if activeAvatarUrl}
                    <img
                      class="menu-account-avatar"
                      src={activeAvatarUrl}
                      alt=""
                      onerror={() => (failedAvatarUrl = activeRepoAccount.avatar_url ?? null)}
                    />
                  {:else}
                    <span class="menu-account-avatar-fallback">
                      {activeRepoAccount.login.slice(0, 1).toUpperCase()}
                    </span>
                  {/if}
                  <span>{t("app.welcome.githubAccount", { login: activeRepoAccount.login })}</span>
                {:else}
                  <span>{t("app.welcome.signInGithub")}</span>
                {/if}
              </button>
              <button type="button" class="btn-secondary" onclick={openSettingsModal}>{t("app.welcome.settings")}</button>
            </div>

            {#if recentRepos.length > 0}
              <div class="form-field">
                <div style="display: flex; align-items: center; justify-content: space-between; gap: 8px;">
                  <span class="form-label">{t("app.welcome.recentRepos")}</span>
                  {#if isLoadingRepoSummaries}
                    <span style="font-size: 10px; color: var(--text-muted);">{t("wsModal.open.loadingSummary")}</span>
                  {/if}
                </div>
                <div class="repo-list-box">
                  {#each recentRepos.slice(0, 6) as r (r)}
                    {@const sum = repoSummaries[r]}
                    {@const isMissing = sum ? !sum.exists : Boolean(repoMissingStatus[r])}
                    {@const iconKind = isMissing ? "missing" : sum?.provider || "local"}
                    {@const pushCount = sum ? (sum.has_upstream ? sum.ahead : sum.unpushed_commits) : 0}
                    <button
                      type="button"
                      class="repo-card {isMissing ? 'is-missing' : ''}"
                      style="width: 100%; text-align: left;"
                      disabled={isMissing}
                      onclick={() => loadRepo(r)}
                    >
                      <div class="repo-card-top">
                        <span class="repo-card-icon">
                          <ProviderIcon provider={iconKind} size={17} />
                        </span>
                        <span class="repo-card-body">
                          <span class="repo-card-title-row">
                            <span class="repo-card-name">{repoAliases[r] || pathBaseName(r)}</span>
                            {#if isMissing}
                              <span class="badge-warn">{t("wsModal.open.missingBadge")}</span>
                            {/if}
                          </span>
                          <span class="repo-card-path">{r}</span>
                          {#if sum && !isMissing && sum.is_git}
                            <span class="repo-status-row">
                              {#if sum.branch}
                                <span class="repo-chip repo-chip-branch">{sum.branch}</span>
                              {/if}
                              {#if sum.dirty_count > 0}
                                <span class="repo-chip repo-chip-dirty">{t("wsModal.open.dirtyBadge", { count: sum.dirty_count })}</span>
                              {/if}
                              {#if pushCount > 0}
                                <span class="repo-chip repo-chip-push">↑ {t("wsModal.open.pushBadge", { count: pushCount })}</span>
                              {/if}
                              {#if sum.behind > 0}
                                <span class="repo-chip repo-chip-pull">↓ {t("wsModal.open.pullBadge", { count: sum.behind })}</span>
                              {/if}
                              {#if sum.has_remote && sum.has_upstream && sum.dirty_count === 0 && pushCount === 0 && sum.behind === 0}
                                <span class="repo-chip repo-chip-synced">✓ {t("wsModal.open.syncedBadge")}</span>
                              {:else if !sum.has_remote}
                                <span class="badge-neutral">{t("wsModal.open.localBadge")}</span>
                              {/if}
                            </span>
                          {/if}
                        </span>
                        {#if !isMissing}
                          <span class="repo-card-open-cue">{t("wsModal.open.openCardHint")} →</span>
                        {/if}
                      </div>
                    </button>
                  {/each}
                </div>
              </div>
            {/if}
          </div>
        {/if}
      </div>
    </main>
  {:else}
    <main class="app-body {activeView === 'history' ? 'history-mode' : ''}" style="--sidebar-w: {sidebarWidth}px;">
      {#if activeView === "changes"}
        <div class="workspace-layout-panels {workspaceLayout} {workspaceDividerDragging ? 'is-resizing' : ''} {groupPanelsVisible && diffVisible ? 'has-two-regions' : groupPanelsVisible ? 'has-group-only' : diffVisible ? 'has-diff-only' : 'has-no-panels'}" style={workspaceGridStyle}>
          {#if groupPanelsVisible}
            <section class="workspace-group" role="list" aria-label={t("workspaceSettings.groupLabel")}>
              {#each orderedWorkspacePanels() as panelId, index (panelId)}
                {@render workspacePanel(panelId, index)}
              {/each}
            </section>
          {/if}
          {#if groupPanelsVisible && diffVisible}
            <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
            <div class="workspace-region-divider" role="separator" aria-orientation="vertical" aria-label={t("workspaceSettings.resizeRegions")} aria-valuemin={20} aria-valuemax={80} aria-valuenow={Math.round(workspaceDividerWidth)} tabindex="0" onpointerdown={startWorkspaceDividerResize} onkeydown={handleWorkspaceDividerKeydown}></div>
          {/if}
          {#if diffVisible}
            <section class="workspace-diff" role="list" aria-label={t("workspaceSettings.panelDiff")}>
              {@render workspacePanel("diff", 0)}
            </section>
          {/if}
          {#if !groupPanelsVisible && !diffVisible}
            <div class="workspace-all-empty"><span class="workspace-center-empty-title">{t("workspaceSettings.empty")}</span></div>
          {/if}
        </div>
      {/if}
      {#if activeView === "history"}
        <section class="content-panel is-full-width">
          <History repoPath={activeRepo} {externalEditorLabel} remoteWebUrl={hostingRepoInfo?.web_url || null} accounts={accountOverview.accounts} {commits} searchQuery={historySearch} onSearchChange={(q) => (historySearch = q)} {allBranches} {branches} currentBranch={currentStatus?.branch || ""} selectedBranches={selectedHistoryBranches} onFilterBranches={handleFilterBranches} {tags} onCreateTag={() => openCreateTagModal(null)} onInteractiveRebase={openInteractiveRebase} onTagContextMenu={openSidebarTagContextMenu} {checkRunsMap} onCherryPick={handleCherryPick} onRevert={handleRevert} onTag={(sha) => openCreateTagModal(sha)} onRenameTag={handleOpenRenameTag} onDeleteTag={handleDeleteTag} onReset={handleReset} onSquash={(idx) => handleOpenSquash(idx)} onEditCommitMessage={handleOpenRewordModal} onMergeBranch={handleMergeBranchFromGraph} onRebaseBranch={handleRebaseBranchFromGraph} onInspectFile={(fp, mode) => (inspectFileState = { path: fp, mode })} onAiExplainCommit={handleAiExplainCommit} onRevealFile={openInFileManagerTarget} onOpenInEditor={openInEditorFile} onLoadMoreCommits={handleLoadMoreCommits} {hasMoreCommits} />
        </section>
      {/if}
    </main>
  {/if}

  {#if terminalOpen && activeRepo}
    <TerminalDrawer
      repoPath={activeRepo}
      currentBranch={currentStatus?.branch || "HEAD"}
      onClose={() => (terminalOpen = false)}
      onRefreshRepo={() => loadRepo(activeRepo)}
    />
  {/if}

  <footer class="app-footer" bind:offsetHeight={appFooterHeight}>
    <div class="footer-status {statusIsError ? 'is-error' : ''}" title={statusMessage}>
      <span
        style="width: 7px; height: 7px; border-radius: 50%; flex-shrink: 0; background-color: {statusIsError ? 'var(--diff-del-text)' : isLoading ? 'var(--warn-text)' : 'var(--diff-add-text)'};"
      ></span>
      <span class="footer-status-text">{statusMessage}</span>
    </div>
    <span class="footer-path" title={activeRepo}>{activeRepo || t("app.noActiveRepo")}</span>
  </footer>
</div>

{#if activeModal === "branch" && currentStatus}
  <BranchModal
    {activeRepo}
    currentBranch={currentStatus.branch}
    bind:branches
    {branchDivergenceMap}
    {isLoadingBranches}
    onInspectBranch={inspectBranchDivergence}
    hasDirtyFiles={currentStatus.files.length > 0}
    dirtyFileCount={currentStatus.files.length}
    {hasRemote}
    hasCommits={currentStatus.has_commits}
    onClose={() => (activeModal = null)}
    onReloadRepo={() => loadRepo(activeRepo)}
    onStatus={setStatus}
    onRequestConfirm={requestConfirmation}
  />
{/if}

{#if activeModal === "worktree" && activeRepo}
  <WorktreeModal
    repoPath={activeRepo}
    onClose={() => (activeModal = null)}
    onRefreshRepo={() => loadRepo(activeRepo)}
    onOpenWorktreeRepo={loadRepo}
    onStatus={setStatus}
  />
{/if}

{#if activeModal === "bisect" && activeRepo}
  <BisectWizardModal
    repoPath={activeRepo}
    {commits}
    onClose={() => (activeModal = null)}
    onStatus={setStatus}
    onReloadRepo={() => loadRepo(activeRepo)}
    onInspectCommit={(sha) => {
      activeModal = null;
      activeView = "history";
      historySearch = sha;
    }}
  />
{/if}

{#if activeWorkspaceModal}
  <WorkspaceModals
    modal={activeWorkspaceModal}
    {activeRepo}
    {recentRepos}
    {repoAliases}
    {repoMissingStatus}
    {repoSummaries}
    {isLoadingRepoSummaries}
    {diagReport}
    {isLoadingDiag}
    {repoConfig}
    {globalConfig}
    {isLoadingSettings}
    {activeTheme}
    {activeLocale}
    {autofetchSeconds}
    {confirmDiscard}
    {confirmForcePush}
    {desktopNotifEnabled}
    {externalEditor}
    {accountOverview}
    {activeRepoAccount}
    {githubRepoInfo}
    {accountHealthMap}
    remotes={repoRemotes}
    hasCommits={currentStatus?.has_commits ?? false}
    {appRuntimeInfo}
    {appUpdateInfo}
    {updateChannel}
    {targetPackageMode}
    {isCheckingUpdate}
    {isInstallingUpdate}
    {updateError}
    onClose={() => (activeModal = null)}
    onLoadRepo={loadRepo}
    onRemotesChanged={refreshRemotesAndPushState}
    onRelocateRepo={relocateMissingRepo}
    onSetAlias={setRepoAlias}
    onRemoveRecent={removeRecentRepo}
    onSaveSettings={handleSaveSettings}
    onCheckUpdates={handleCheckUpdates}
    onInstallUpdate={handleInstallUpdate}
    onStatus={setStatus}
    onAccountsChanged={handleAccountsChanged}
    onCheckAccountHealth={checkAccountHealth}
    onSyncRepoAuthor={handleSyncRepoAuthor}
    onOpenAuth={() => (activeModal = "auth")}
    onRequestConfirm={requestConfirmation}
  />
{/if}

{#if activeHistoryGithubModal}
  <HistoryGithubModals
    modal={activeHistoryGithubModal}
    {activeRepo}
    currentBranch={currentStatus?.branch || ""}
    {githubRepoInfo}
    {hostingRepoInfo}
    {pullRequests}
    {issues}
    {prTab}
    {isLoadingGithub}
    {githubError}
    hasGithubAccount={Boolean(activeRepoAccount)}
    onOpenAuth={() => (activeModal = "auth")}
    {tagTargetSha}
    {renameTargetTag}
    {squashCount}
    {squashInitialSummary}
    {rebaseBaseSha}
    bind:rebaseItems
    {rewordTargetCommit}
    status={currentStatus}
    {confirmTitle}
    {confirmMessage}
    {confirmDanger}
    onRequestConfirm={requestConfirmation}
    onConfirmSubmit={handleConfirmSubmit}
    onSwitchPrTab={handleSwitchPrTab}
    onCheckoutPr={handleCheckoutPr}
    onReloadRepo={() => loadRepo(activeRepo)}
    onReloadHistory={async () => {
      commits = await gitService.getCommitHistory(activeRepo, commitHistoryLimit, allBranches);
    }}
    onReloadTags={async () => {
      const [nextTags, nextCommits] = await Promise.all([
        gitService.listTags(activeRepo),
        activeView === "history"
          ? gitService.getCommitHistory(activeRepo, commitHistoryLimit, allBranches)
          : Promise.resolve(null),
      ]);
      tags = nextTags;
      if (nextCommits !== null) commits = nextCommits;
    }}
    onStatus={setStatus}
    onClose={() => {
      const returnTo = activeModal === "confirm" ? confirmReturnModal : null;
      if (activeModal === "confirm") {
        confirmAction = null;
        confirmReturnModal = null;
      }
      activeModal = returnTo;
    }}
  />
{/if}


{#if conflictFilePath && activeRepo}
  <ConflictEditorModal
    repoPath={activeRepo}
    conflictedFiles={(currentStatus?.files.filter((f) => f.is_conflict).map((f) => f.path)) || [conflictFilePath]}
    initialFilePath={conflictFilePath}
    onClose={() => (conflictFilePath = null)}
    onResolved={async () => {
      conflictFilePath = null;
      await loadRepo(activeRepo);
    }}
    onStatus={setStatus}
  />
{/if}

{#if inspectFileState && activeRepo}
  <FileInspectModal
    repoPath={activeRepo}
    filePath={inspectFileState.path}
    initialMode={inspectFileState.mode}
    onClose={() => (inspectFileState = null)}
    onStatus={setStatus}
  />
{/if}

{#if workspaceToolsTab && activeRepo}
  <WorkspaceToolsModal
    repoPath={activeRepo}
    initialTab={workspaceToolsTab}
    onClose={() => (workspaceToolsTab = null)}
    onRefreshRepo={() => loadRepo(activeRepo)}
    onOpenWorktreeRepo={loadRepo}
    onStatus={setStatus}
  />
{/if}

{#if aiModalState}
  <AiAssistantModal
    repoPath={activeRepo}
    initialTask={aiModalState.task}
    initialPrompt={aiModalState.prompt}
    initialContext={aiModalState.context}
    onClose={() => (aiModalState = null)}
    onOpenSettings={openSettingsModal}
    onStatus={setStatus}
  />
{/if}

{#if paletteOpen}
  <CommandPalette commands={paletteCommands} onClose={() => (paletteOpen = false)} />
{/if}

<SshRemoteSwitchModal
  open={sshSwitchModalState.open}
  repo={sshSwitchModalState.repo}
  remoteName={sshSwitchModalState.remoteName}
  currentUrl={sshSwitchModalState.currentUrl}
  suggestedUrl={sshSwitchModalState.suggestedUrl}
  errorMessage={sshSwitchModalState.errorMessage}
  accountLogin={activeRepoAccount?.login}
  onClose={() => {
    sshSwitchModalState.open = false;
  }}
  onSwitchSuccess={async () => {
    sshSwitchModalState.open = false;
    setStatus(t("sshModal.switchSuccess"));
    await loadRepo(activeRepo);
    await refreshRemotesAndPushState();
    if (sshSwitchModalState.retryAction) {
      void sshSwitchModalState.retryAction();
    }
  }}
/>

<ContextMenu />
