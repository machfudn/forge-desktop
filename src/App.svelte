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
    ImageDiffData,
    IssueInfo,
    ParsedGitHubRepo,
    PullRequestInfo,
    RemoteInfo,
    RepoConfig,
    RepoStatus,
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
  import { joinRepoPath, openExternalUrl, pathBaseName } from "./utils/path";
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
  } from "./i18n/index.svelte";

  type ModalType = "branch" | WorkspaceModalType | HistoryGithubModalType | null;

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

  // Shared modal states
  let recentRepos = $state<string[]>([]);
  let repoAliases = $state<Record<string, string>>({});
  let repoMissingStatus = $state<Record<string, boolean>>({});
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

  // Settings & preferences
  let activeLocale = $state<Locale>(getLocale());
  let activeTheme = $state(localStorage.getItem("forge_theme") || "dark");
  let autofetchMinutes = $state(Number(localStorage.getItem("forge_autofetch") || "5"));
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
  let appRuntimeInfo = $state<AppRuntimeInfo | null>(null);
  let appUpdateInfo = $state<AppUpdateInfo | null>(null);
  let isCheckingUpdate = $state(false);
  let isInstallingUpdate = $state(false);
  let updateError = $state("");
  const EDITOR_LABELS: Record<string, string> = {
    code: "Visual Studio Code",
    cursor: "Cursor",
    windsurf: "Windsurf",
    zed: "Zed",
    subl: "Sublime Text",
    "notepad++": "Notepad++",
    notepad: "Notepad",
    gedit: "gedit",
    kate: "Kate",
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
  let isResizingSidebar = $state(false);

  function updateSidebarWidth(next: number) {
    const clamped = clampSidebarWidth(next);
    sidebarWidth = clamped;
    localStorage.setItem("forge_sidebar_width", String(clamped));
  }

  function startSidebarResize(e: PointerEvent) {
    e.preventDefault();
    const startX = e.clientX;
    const startW = sidebarWidth;
    isResizingSidebar = true;

    const onMove = (ev: PointerEvent) => {
      updateSidebarWidth(startW + (ev.clientX - startX));
    };
    const onUp = () => {
      isResizingSidebar = false;
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
      window.removeEventListener("pointercancel", onUp);
    };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
    window.addEventListener("pointercancel", onUp);
  }

  function handleResizerKeydown(e: KeyboardEvent) {
    if (e.key === "ArrowLeft") {
      e.preventDefault();
      updateSidebarWidth(sidebarWidth - (e.shiftKey ? 24 : 12));
    } else if (e.key === "ArrowRight") {
      e.preventDefault();
      updateSidebarWidth(sidebarWidth + (e.shiftKey ? 24 : 12));
    } else if (e.key === "Home") {
      e.preventDefault();
      updateSidebarWidth(SIDEBAR_MIN);
    } else if (e.key === "End") {
      e.preventDefault();
      updateSidebarWidth(SIDEBAR_MAX);
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
  let squashCount = $state(2);
  let squashInitialSummary = $state("");
  let rebaseBaseSha = $state("");
  let rebaseItems = $state<RebaseItem[]>([]);

  // GitHub multi-account & repo context
  let accountOverview = $state<AccountOverview>({
    accounts: [],
    default_by_host: {},
    active_repo_account_id: null,
    primary_account_id: null,
  });
  let accountHealthMap = $state<Record<string, AccountHealth>>({});
  let githubRepoInfo = $state<ParsedGitHubRepo | null>(null);
  let pullRequests = $state<PullRequestInfo[]>([]);
  let issues = $state<IssueInfo[]>([]);
  let prTab = $state<"prs" | "issues">("prs");
  let isLoadingGithub = $state(false);
  let githubError = $state("");
  let githubRateLimited = $state(false);
  let checkRunsMap = $state<Record<string, { status: string; conclusion?: string }>>({});
  const activeRepoAccount = $derived.by<AccountProfile | null>(() => {
    const list = accountOverview.accounts;
    if (list.length === 0) return null;
    if (currentStatus && githubRepoInfo) {
      const host = githubRepoInfo.host;
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

  function setStatus(msg: string, isError = false) {
    statusMessage = msg;
    statusIsError = isError;
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
    let resolved = theme;
    if (theme === "system") {
      resolved = window.matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark";
    }
    document.documentElement.setAttribute("data-theme", resolved);
    const metaTheme = document.querySelector('meta[name="theme-color"]');
    if (metaTheme) {
      metaTheme.setAttribute("content", resolved === "light" ? "#ffffff" : "#0d1117");
    }
  }

  function setupAutofetch(mins: number) {
    if (autofetchTimer !== null) {
      clearInterval(autofetchTimer);
      autofetchTimer = null;
    }
    if (mins > 0 && activeRepo) {
      autofetchTimer = window.setInterval(async () => {
        if (!activeRepo || isLoading) return;
        try {
          await gitService.gitFetch(activeRepo);
          const status = await gitService.inspectRepository(activeRepo);
          currentStatus = status;
          if (status.branch && status.branch !== "HEAD terlepas" && !status.branch.startsWith("(")) {
            aheadBehind = await gitService.getBranchAheadBehind(activeRepo, status.branch);
          }
        } catch {
          // Silent background fetch
        }
      }, mins * 60 * 1000);
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
      activeModal === "auth" || activeModal === "stash" || activeModal === "remote"
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
      return;
    }
    try {
      const existsMap = await gitService.checkReposExists(recentRepos);
      const statusMap: Record<string, boolean> = {};
      for (const r of recentRepos) {
        statusMap[r] = !existsMap[r];
      }
      repoMissingStatus = statusMap;
    } catch {
      const statusMap: Record<string, boolean> = {};
      for (const r of recentRepos) {
        statusMap[r] = false;
      }
      repoMissingStatus = statusMap;
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
    currentImageDiff = null;
    const isImg = /\.(png|jpe?g|gif|webp|svg|ico|bmp)$/i.test(file.path);
    if (isImg) {
      try {
        currentImageDiff = await gitService.getImageDiff(activeRepo, file.path);
        currentDiff = "";
        return;
      } catch {
        currentImageDiff = null;
      }
    }
    try {
      currentDiff = await gitService.getFileDiff(activeRepo, file.path, file.staged, ignoreWhitespace);
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
      githubRepoInfo = origin ? await gitService.parseGithubRepository(origin.url) : null;
    } catch {
      repoRemotes = [];
      hasRemote = false;
      githubRepoInfo = null;
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
      selectedFile = status.files.length > 0 ? status.files[0] : null;
      currentDiff = "";
      currentImageDiff = null;
      if (isNewRepo) {
        branches = [];
        pullRequests = [];
        issues = [];
        checkRunsMap = {};
        githubRepoInfo = null;
        repoRemotes = [];
        hasRemote = false;
      }
      branchDivergenceMap = {};
      pendingBranchCompares.clear();
      saveRecentRepo(repoPath);

      const isValidBranch =
        Boolean(status.branch) &&
        status.branch !== "HEAD terlepas" &&
        !status.branch.startsWith("(");

      const [abRes, , tagsRes, historyRes, remotesRes] = await Promise.allSettled([
        isValidBranch
          ? gitService.getBranchAheadBehind(repoPath, status.branch)
          : Promise.resolve({ ...EMPTY_AHEAD_BEHIND, is_unborn: !status.has_commits }),
        status.files.length > 0 ? loadDiffForFile(status.files[0]) : Promise.resolve(),
        gitService.listTags(repoPath),
        activeView === "history" ? gitService.getCommitHistory(repoPath, 80) : Promise.resolve(null),
        gitService.listRemotes(repoPath),
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

      if (remotesRes.status === "fulfilled") {
        const remotes = remotesRes.value;
        repoRemotes = remotes;
        hasRemote = remotes.length > 0;
        const origin = remotes.find((r) => r.name === "origin") || remotes[0];
        if (origin) {
          try {
            const ghParsed = await gitService.parseGithubRepository(origin.url);
            if (reqSeq !== loadRepoSeq) return;
            githubRepoInfo = ghParsed;
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
            }
          } catch {
            githubRepoInfo = null;
          }
        } else {
          githubRepoInfo = null;
        }
      } else {
        repoRemotes = [];
        hasRemote = false;
        githubRepoInfo = null;
      }

      setupAutofetch(autofetchMinutes);
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

  async function handleCommit(summary: string, description?: string, amend?: boolean) {
    try {
      await gitService.commitChanges(activeRepo, summary, description, amend);
      setStatus(amend ? t("app.status.commitAmended") : t("app.status.commitCreated"));
      await loadRepo(activeRepo);
      if (activeView === "history") {
        commits = await gitService.getCommitHistory(activeRepo, 80);
      }
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
          if (activeView === "history") {
            commits = await gitService.getCommitHistory(activeRepo, 80);
          }
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
        commits = await gitService.getCommitHistory(activeRepo, 80);
        tags = await gitService.listTags(activeRepo);
        if (githubRepoInfo && activeRepoAccount && !githubRateLimited && commits.length > 0) {
          const checkPromises = commits.slice(0, 12).map(async (c) => {
            try {
              const runs = await gitService.getCommitCheckRuns(
                githubRepoInfo!.owner,
                githubRepoInfo!.repo,
                c.sha,
                githubRepoInfo!.host,
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
              if (isGithubRateLimitError(err)) {
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
    if (!githubRepoInfo) return;
    activeModal = "pr";
    prTab = "prs";
    isLoadingGithub = true;
    githubError = "";
    try {
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
    if (!githubRepoInfo) return;
    isLoadingGithub = true;
    githubError = "";
    try {
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
    } catch (err) {
      const msg = formatGithubError(err);
      githubError = msg;
      setStatus(t("app.status.loadGithubFailed", { msg }), true);
    } finally {
      isLoadingGithub = false;
    }
  }

  async function handleCheckoutPr(prNumber: number) {
    activeModal = null;
    try {
      setStatus(t("app.status.checkingOutPr", { num: prNumber }));
      const res = await gitService.checkoutGithubPullRequest(activeRepo, prNumber);
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
    autofetchMinutes: number;
    confirmDiscard: boolean;
    confirmForcePush: boolean;
    desktopNotifEnabled: boolean;
    externalEditor: string;
    updateChannel: UpdateChannel;
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
      autofetchMinutes = cfg.autofetchMinutes;
      confirmDiscard = cfg.confirmDiscard;
      confirmForcePush = cfg.confirmForcePush;
      desktopNotifEnabled = cfg.desktopNotifEnabled;
      externalEditor = cfg.externalEditor.trim() || "system";
      const prevChannel = updateChannel;
      updateChannel = cfg.updateChannel;

      localStorage.setItem("forge_theme", activeTheme);
      localStorage.setItem("forge_autofetch", String(autofetchMinutes));
      localStorage.setItem("forge_confirm_discard", String(confirmDiscard));
      localStorage.setItem("forge_confirm_force_push", String(confirmForcePush));
      localStorage.setItem("forge_desktop_notif", String(desktopNotifEnabled));
      localStorage.setItem("forge_external_editor", externalEditor);
      localStorage.setItem("forge_update_channel", updateChannel);
      applyTheme(activeTheme);
      setupAutofetch(autofetchMinutes);
      if (prevChannel !== updateChannel) {
        void handleCheckUpdates(updateChannel);
      }

      activeModal = null;
      setStatus(t("app.status.settingsSaved"));
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  async function handleCheckUpdates(channelOverride?: UpdateChannel) {
    const targetChannel = channelOverride ?? updateChannel;
    isCheckingUpdate = true;
    updateError = "";
    try {
      setStatus(t("app.status.checkingUpdates"));
      appUpdateInfo = await gitService.checkAppUpdate(targetChannel);
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

  async function handleInstallUpdate(channelOverride?: UpdateChannel) {
    const targetChannel = channelOverride ?? updateChannel;
    isInstallingUpdate = true;
    updateError = "";
    try {
      setStatus(t("app.status.installingUpdate"));
      const res = await gitService.installAppUpdate(targetChannel);
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

  async function handleFetch() {
    if (!activeRepo) return;
    try {
      isLoading = true;
      setStatus(t("app.status.fetching"));
      await gitService.gitFetch(activeRepo);
      await loadRepo(activeRepo);
      setStatus(t("app.status.fetchDone"));
    } catch (err) {
      setStatus(String(err), true);
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
      setStatus(String(err), true);
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
      setStatus(String(err), true);
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
        setStatus(String(err), true);
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
          if (activeView === "history") {
            commits = await gitService.getCommitHistory(activeRepo, 80);
          }
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
              ? gitService.getCommitHistory(activeRepo, 80)
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
          if (activeView === "history") {
            commits = await gitService.getCommitHistory(activeRepo, 80);
          }
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

  function openInteractiveRebase() {
    if (commits.length < 2) {
      setStatus(t("app.status.rebaseMinCommits"), true);
      return;
    }
    const targetCommits = commits.slice(0, Math.min(10, commits.length));
    const oldestDisplayedIdx = targetCommits.length - 1;
    const baseCommit = commits[oldestDisplayedIdx + 1];
    if (!baseCommit) {
      setStatus(t("app.status.rebaseNeedBase"), true);
      return;
    }
    rebaseBaseSha = baseCommit.sha;
    rebaseItems = [...targetCommits].reverse().map((c) => ({
      sha: c.sha,
      short_sha: c.short_sha,
      summary: c.summary,
      author: c.author,
      action: "pick" as const,
    }));
    activeModal = "rebase";
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
    if (githubRepoInfo) {
      const ghUrl = `https://${githubRepoInfo.host}/${githubRepoInfo.owner}/${githubRepoInfo.repo}`;
      items.push({
        label: t("ctx.repo.viewOnGithub"),
        onSelect: () => {
          void openExternalUrl(ghUrl);
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
        if (activeModal) activeModal = null;
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
    document.addEventListener("contextmenu", onDocContextMenu);
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
      }
      await Promise.allSettled(startupTasks);

      gitService
        .checkAppUpdate(updateChannel)
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
      document.removeEventListener("contextmenu", onDocContextMenu);
      if (autofetchTimer !== null) {
        clearInterval(autofetchTimer);
        autofetchTimer = null;
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
      activeModal === "auth" ||
      activeModal === "settings"
      ? activeModal
      : null
  );

  const activeHistoryGithubModal = $derived(
    activeModal === "tag" ||
      activeModal === "squash" ||
      activeModal === "rebase" ||
      activeModal === "pr" ||
      activeModal === "confirm"
      ? activeModal
      : null
  );
</script>

<div class="app-container">
  <header class="app-header">
    <div class="header-left">
      <button
        type="button"
        class="picker-pill"
        onclick={openOpenModal}
        oncontextmenu={openRepoHeaderContextMenu}
        title={activeRepo || t("app.openRepoTooltip")}
      >
        <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true" style="flex-shrink: 0; color: var(--text-secondary);">
          <path d="M2 2.5A2.5 2.5 0 0 1 4.5 0h8.75a.75.75 0 0 1 .75.75v12.5a.75.75 0 0 1-.75.75h-2.5a.75.75 0 0 1 0-1.5h1.75v-2h-8a1 1 0 0 0-.714 1.7.75.75 0 1 1-1.072 1.05A2.495 2.495 0 0 1 2 11.5Zm10.5-1h-8a1 1 0 0 0-1 1v6.708A2.486 2.486 0 0 1 4.5 9h8ZM5 12.25a.25.25 0 0 1 .25-.25h3.5a.25.25 0 0 1 .25.25v3.25a.25.25 0 0 1-.4.2l-1.45-1.087a.249.249 0 0 0-.3 0L5.4 15.7a.25.25 0 0 1-.4-.2Z" />
        </svg>
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

        {#if githubRepoInfo}
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
          <span class="header-account-dot {isHeaderAccWarn ? 'is-warn' : ''}"></span>
          <span style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
            {#if activeRepoAccount}
              @{activeRepoAccount.login}{#if activeRepoAccount.host !== "github.com"}<span class="header-account-host"> · {activeRepoAccount.host}</span>{/if}
            {:else}
              {t("app.signInGithub")}
            {/if}
          </span>
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
          {#if appUpdateInfo?.has_update}
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
                activeModal = "auth";
              }}
            >
              <span>
                {activeRepoAccount
                  ? t("app.menu.githubAccountNamed", { login: activeRepoAccount.login })
                  : t("app.menu.manageGithubAccounts")}
              </span>
              {#if accountOverview.accounts.length > 0}
                <span class="popover-item-hint" style="color: var(--diff-add-text);">
                  {accountOverview.accounts.length === 1
                    ? t("app.menu.accountsCountOne", { count: accountOverview.accounts.length })
                    : t("app.menu.accountsCountMany", { count: accountOverview.accounts.length })}
                </span>
              {/if}
            </button>
            <button type="button" role="menuitem" class="popover-item" onclick={openDiagModal}>
              <span>{t("app.menu.diagnostics")}</span>
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
              <button type="button" class="btn-secondary" onclick={() => (activeModal = "auth")}>
                {activeRepoAccount
                  ? t("app.welcome.githubAccount", { login: activeRepoAccount.login })
                  : t("app.welcome.signInGithub")}
              </button>
              <button type="button" class="btn-secondary" onclick={openSettingsModal}>{t("app.welcome.settings")}</button>
            </div>

            {#if recentRepos.length > 0}
              <div class="form-field">
                <span class="form-label">{t("app.welcome.recentRepos")}</span>
                <div class="list-box">
                  {#each recentRepos.slice(0, 6) as r (r)}
                    <button
                      type="button"
                      class="list-row"
                      style="width: 100%; text-align: left;"
                      onclick={() => loadRepo(r)}
                    >
                      <span style="font-weight: 600;">{repoAliases[r] || pathBaseName(r)}</span>
                      <span style="font-family: var(--font-mono); font-size: 10px; color: var(--text-muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
                        {r}
                      </span>
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
    <main class="app-body" style="--sidebar-w: {sidebarWidth}px;">
      <aside class="sidebar-panel">
        {#if activeView === "changes"}
          <Changes
            repoPath={activeRepo}
            {externalEditorLabel}
            status={currentStatus}
            {selectedFile}
            onSelectFile={selectFile}
            onToggleStage={toggleStage}
            onStageAll={handleStageAll}
            onUnstageAll={handleUnstageAll}
            onCommit={handleCommit}
            onUndoCommit={handleUndoCommit}
            onAbortMerge={handleAbortMerge}
            onAbortRebase={handleAbortRebase}
            onDiscardFile={handleDiscardFile}
            onDiscardAll={handleDiscardAll}
            onAddToGitignore={handleAddToGitignore}
            onRevealFile={openInFileManagerTarget}
            onOpenInEditor={openInEditorFile}
            onOpenStash={() => (activeModal = "stash")}
            onStatus={setStatus}
            issues={issues.map((i) => ({ number: i.number, title: i.title }))}
          />
        {:else}
          <div class="sidebar-header">
            <h3 class="tabular-nums">{t("app.historySidebar.title", { count: commits.length })}</h3>
            {#if commits.length > 1}
              <button type="button" class="btn-secondary btn-xs" onclick={openInteractiveRebase}>
                {t("app.historySidebar.interactiveRebase")}
              </button>
            {/if}
          </div>
          <div class="history-sidebar-body">
            <div class="form-field">
              <label class="form-label" for="history-search-input">{t("app.historySidebar.filterLabel")}</label>
              <input
                id="history-search-input"
                type="search"
                class="form-input"
                placeholder={t("app.historySidebar.filterPlaceholder")}
                bind:value={historySearch}
              />
            </div>

            <div>
              <div class="sidebar-section-title">
                <span class="tabular-nums">{t("app.historySidebar.tagsHeader", { count: tags.length })}</span>
                <button type="button" class="link-btn" onclick={() => openCreateTagModal(null)}>
                  {t("app.historySidebar.createTagBtn")}
                </button>
              </div>
              {#if tags.length === 0}
                <div style="font-size: 11px; color: var(--text-muted); padding: 6px 0;">
                  {t("app.historySidebar.noTags")}
                </div>
              {:else}
                <div class="tag-badge-list">
                  {#each tags as tagName (tagName)}
                    <button
                      type="button"
                      class="tag-badge"
                      title={t("app.historySidebar.tagTooltip", { tag: tagName })}
                      aria-label={t("app.historySidebar.tagAria", { tag: tagName })}
                      onclick={() => handleDeleteTag(tagName)}
                      oncontextmenu={(e) => openSidebarTagContextMenu(e, tagName)}
                    >
                      <span class="tag-badge-label">{tagName}</span>
                    </button>
                  {/each}
                </div>
              {/if}
            </div>
          </div>
        {/if}
      </aside>

      <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
      <div
        class="sidebar-resizer {isResizingSidebar ? 'is-resizing' : ''}"
        role="separator"
        aria-orientation="vertical"
        aria-label={t("app.sidebarResizer.aria")}
        aria-valuemin={SIDEBAR_MIN}
        aria-valuemax={SIDEBAR_MAX}
        aria-valuenow={sidebarWidth}
        tabindex="0"
        title={t("app.sidebarResizer.title")}
        onpointerdown={startSidebarResize}
        onkeydown={handleResizerKeydown}
        ondblclick={() => updateSidebarWidth(SIDEBAR_DEFAULT)}
      ></div>

      <section class="content-panel">
        {#if activeView === "changes"}
          {#if selectedFile}
            <Diff
              repoPath={activeRepo}
              {externalEditorLabel}
              filePath={selectedFile.path}
              diffText={currentDiff}
              imageDiffData={currentImageDiff}
              {ignoreWhitespace}
              isLfs={selectedFile.is_lfs}
              isStaged={selectedFile.staged}
              onToggleWhitespace={toggleWhitespace}
              onAddToGitignore={handleAddToGitignore}
              onStageHunk={handleStageHunk}
              onRevealFile={openInFileManagerTarget}
              onOpenInEditor={openInEditorFile}
              onStatus={setStatus}
            />
          {:else}
            <div class="empty-state">
              <div class="empty-state-icon" aria-hidden="true">
                <svg width="18" height="18" viewBox="0 0 16 16" fill="currentColor">
                  {#if currentStatus.files.length === 0}
                    <path d="M13.78 4.22a.75.75 0 0 1 0 1.06l-7.25 7.25a.75.75 0 0 1-1.06 0L2.22 9.28a.751.751 0 0 1 .018-1.042.751.751 0 0 1 1.042-.018L6 10.94l6.72-6.72a.75.75 0 0 1 1.06 0Z" />
                  {:else}
                    <path d="M2 1.75C2 .784 2.784 0 3.75 0h6.586c.464 0 .909.184 1.237.513l2.914 2.914c.329.328.513.773.513 1.237v9.586A1.75 1.75 0 0 1 13.25 16h-9.5A1.75 1.75 0 0 1 2 14.25Zm1.75-.25a.25.25 0 0 0-.25.25v12.5c0 .138.112.25.25.25h9.5a.25.25 0 0 0 .25-.25V6h-2.75A1.75 1.75 0 0 1 9 4.25V1.5Zm6.75.062V4.25c0 .138.112.25.25.25h2.688l-.011-.013-2.914-2.914-.013-.011Z" />
                  {/if}
                </svg>
              </div>
              {#if currentStatus.files.length === 0}
                {#if !currentStatus.has_commits}
                  <div style="font-weight: 600; color: var(--text-primary);">{t("app.emptyChanges.emptyRepoTitle")}</div>
                  <span style="font-size: 12px; max-width: 360px;">
                    {t("app.emptyChanges.emptyRepoDesc", { branch: formatBranchDisplay(currentStatus.branch) })}
                  </span>
                  <div class="empty-state-actions">
                    {#if !hasRemote}
                      <button type="button" class="btn-secondary btn-xs" onclick={() => (activeModal = "remote")}>
                        {t("app.connectRemote")}
                      </button>
                    {/if}
                    <button type="button" class="btn-secondary btn-xs" onclick={openInEditor}>
                      {t("app.emptyChanges.openInEditor")}
                    </button>
                  </div>
                {:else}
                  <div style="font-weight: 600; color: var(--text-primary);">{t("app.emptyChanges.cleanTreeTitle")}</div>
                  <span style="font-size: 12px; max-width: 340px;">
                    {t("app.emptyChanges.cleanTreeDesc", { branch: formatBranchDisplay(currentStatus.branch) })}
                  </span>
                  <div class="empty-state-actions">
                    <button type="button" class="btn-secondary btn-xs" onclick={() => switchView("history")}>
                      {t("app.emptyChanges.viewHistory")}
                    </button>
                    {#if hasRemote}
                      <button type="button" class="btn-secondary btn-xs" onclick={handleFetch} disabled={isLoading}>
                        {t("app.emptyChanges.fetchRemote")}
                      </button>
                    {/if}
                  </div>
                {/if}
              {:else}
                <div style="font-weight: 600; color: var(--text-primary);">{t("app.emptyChanges.selectFileTitle")}</div>
                <span style="font-size: 12px; max-width: 340px;">
                  {t("app.emptyChanges.selectFileDesc", { count: currentStatus.files.length })}
                </span>
              {/if}
            </div>
          {/if}
        {:else}
          <History
            repoPath={activeRepo}
            {externalEditorLabel}
            githubRepoUrl={githubRepoInfo ? `https://${githubRepoInfo.host}/${githubRepoInfo.owner}/${githubRepoInfo.repo}` : null}
            {commits}
            searchQuery={historySearch}
            {checkRunsMap}
            onStatus={setStatus}
            onCherryPick={handleCherryPick}
            onRevert={handleRevert}
            onTag={(sha) => openCreateTagModal(sha)}
            onDeleteTag={handleDeleteTag}
            onReset={handleReset}
            onSquash={(idx) => handleOpenSquash(idx)}
            onRevealFile={openInFileManagerTarget}
            onOpenInEditor={openInEditorFile}
          />
        {/if}
      </section>
    </main>
  {/if}

  <footer class="app-footer">
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

{#if activeWorkspaceModal}
  <WorkspaceModals
    modal={activeWorkspaceModal}
    {activeRepo}
    {recentRepos}
    {repoAliases}
    {repoMissingStatus}
    {diagReport}
    {isLoadingDiag}
    {repoConfig}
    {globalConfig}
    {isLoadingSettings}
    {activeTheme}
    {activeLocale}
    {autofetchMinutes}
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
    {pullRequests}
    {issues}
    {prTab}
    {isLoadingGithub}
    {githubError}
    hasGithubAccount={Boolean(activeRepoAccount)}
    onOpenAuth={() => (activeModal = "auth")}
    {tagTargetSha}
    {squashCount}
    {squashInitialSummary}
    {rebaseBaseSha}
    bind:rebaseItems
    {confirmTitle}
    {confirmMessage}
    {confirmDanger}
    onConfirmSubmit={handleConfirmSubmit}
    onSwitchPrTab={handleSwitchPrTab}
    onCheckoutPr={handleCheckoutPr}
    onReloadRepo={() => loadRepo(activeRepo)}
    onReloadHistory={async () => {
      commits = await gitService.getCommitHistory(activeRepo, 80);
    }}
    onReloadTags={async () => {
      const [nextTags, nextCommits] = await Promise.all([
        gitService.listTags(activeRepo),
        activeView === "history"
          ? gitService.getCommitHistory(activeRepo, 80)
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

<ContextMenu />
