<script lang="ts">
  import { onMount } from "svelte";
  import type {
    AheadBehind,
    BranchInfo,
    CommitInfo,
    DeviceFlowInit,
    DiagnosticReport,
    FileEntry,
    IssueInfo,
    PullRequestInfo,
    RemoteInfo,
    RepoConfig,
    RepoStatus,
  } from "./types/git";
  import * as gitService from "./services/git";
  import Changes from "./components/Changes.svelte";
  import Diff from "./components/Diff.svelte";
  import History from "./components/History.svelte";

  let activeRepo = $state(
    localStorage.getItem("forge:active-repo") ||
      "/home/machfudin/Development/github-desktop"
  );
  let activeTab: "changes" | "history" = $state("changes");
  let currentStatus: RepoStatus | null = $state(null);
  let selectedFile: FileEntry | null = $state(null);
  let currentDiff = $state("");
  let commitHistory: CommitInfo[] = $state([]);
  let branches: BranchInfo[] = $state([]);
  let currentRemotes: RemoteInfo[] = $state([]);
  let hasRemotes = $state(false);
  let aheadBehind: AheadBehind = $state({ ahead: 0, behind: 0 });

  let footerStatus = $state("Siap");
  let isStatusError = $state(false);

  // Modal states
  type ModalType =
    | "branch"
    | "open"
    | "clone"
    | "init"
    | "settings"
    | "stash"
    | "diag"
    | "auth"
    | "tag"
    | "pr"
    | "squash"
    | "confirm"
    | "publish"
    | null;
  let activeModal: ModalType = $state(null);

  // Publish repo states
  let publishName = $state("");
  let publishDesc = $state("");
  let publishPrivate = $state(false);

  // Branch divergence comparison map
  let branchDivergenceMap = $state<Record<string, AheadBehind>>({});

  // Theme & Auto-fetch preferences
  let activeTheme = $state(
    typeof localStorage !== "undefined"
      ? localStorage.getItem("forge:theme") || "dark"
      : "dark"
  );
  let autofetchMinutes = $state(
    typeof localStorage !== "undefined"
      ? Number(localStorage.getItem("forge:autofetch-minutes") || "5")
      : 5
  );
  let autofetchTimer: number | null = null;

  // Desktop Notifications & Fork Sync
  let desktopNotifEnabled = $state(
    typeof localStorage !== "undefined"
      ? localStorage.getItem("forge:desktop-notifs") !== "false"
      : true
  );
  let hasUpstream = $derived(currentRemotes.some((r) => r.name === "upstream"));

  function sendDesktopNotification(title: string, body: string) {
    if (!desktopNotifEnabled) return;
    if (typeof window === "undefined" || !("Notification" in window)) return;
    if (Notification.permission === "granted") {
      try {
        new Notification(title, { body });
      } catch {}
    } else if (Notification.permission !== "denied") {
      Notification.requestPermission().then((perm) => {
        if (perm === "granted") {
          try {
            new Notification(title, { body });
          } catch {}
        }
      });
    }
  }

  async function handleSyncFork() {
    try {
      setStatus("Menyinkronkan dengan remote upstream...");
      const res = await gitService.syncFork(
        activeRepo,
        "upstream",
        currentStatus?.branch || "main"
      );
      setStatus(`Fork berhasil disinkronkan: ${res || "Up-to-date"}`);
      sendDesktopNotification(
        "Forge Desktop",
        "Fork berhasil disinkronkan dengan branch upstream."
      );
      await loadRepo(activeRepo);
    } catch (err) {
      setStatus(`Gagal sinkronisasi fork: ${err}`, true);
    }
  }

  // Safety Preferences
  let confirmDiscard = $state(
    typeof localStorage !== "undefined"
      ? localStorage.getItem("forge:confirm-discard") !== "false"
      : true
  );
  let confirmForcePush = $state(
    typeof localStorage !== "undefined"
      ? localStorage.getItem("forge:confirm-force-push") !== "false"
      : true
  );

  // Confirm dialog modal state
  let confirmModalTitle = $state("");
  let confirmModalMessage = $state("");
  let confirmModalAction: (() => Promise<void>) | null = null;

  // GitHub PR & Issues states
  let githubRepoInfo = $state<{ owner: string; repo: string } | null>(null);
  let pullRequests: PullRequestInfo[] = $state([]);
  let issues: IssueInfo[] = $state([]);
  let prTab: "prs" | "issues" = $state("prs");
  let isLoadingGithub = $state(false);
  let checkRunsMap = $state<Record<string, { status: string; conclusion?: string }>>({});

  // Tag modal states
  let tagModalTargetSha = $state<string | null>(null);
  let tagNameInput = $state("");

  // Squash modal states
  let squashCount = $state(2);
  let squashSummary = $state("");
  let squashDescription = $state("");

  // Diff whitespace state
  let ignoreWhitespace = $state(false);

  // Branch modal sub-states
  let branchSearch = $state("");
  let newBranchName = $state("");
  let branchToRename = $state<string | null>(null);
  let renameBranchInput = $state("");

  // Open & Init modal sub-states
  let openRepoPath = $state("");
  let recentRepos: string[] = $state([]);
  let initRepoPath = $state("");
  let initDefaultBranch = $state("main");
  let initGitignore = $state("Node");
  let initLicense = $state("MIT");

  // Clone sub-states
  let cloneUrl = $state("");
  let cloneTarget = $state("");

  // Stash sub-states
  let stashMsg = $state("");
  let stashList: string[] = $state([]);
  let selectedStash: string | null = $state(null);
  let stashDiff = $state("");
  let isLoadingStashDiff = $state(false);

  // Diagnostics & Settings sub-states
  let diagReport: DiagnosticReport | null = $state(null);
  let repoConfig: RepoConfig = $state({ user_name: "", user_email: "" });
  let globalConfig: RepoConfig = $state({ user_name: "", user_email: "" });

  // Auth flow states
  let oauthClientId = $state("");
  let deviceFlow: DeviceFlowInit | null = $state(null);
  let pollStatus = $state("");
  let pollStatusClass = $state("");
  let pollTimer: number | null = null;

  function setStatus(msg: string, isError = false) {
    footerStatus = msg;
    isStatusError = isError;
  }

  function getStoredAliases(): Record<string, string> {
    try {
      return JSON.parse(localStorage.getItem("forge:repo-aliases") || "{}");
    } catch {
      return {};
    }
  }

  let repoAliases: Record<string, string> = $state(getStoredAliases());
  let repoMissingStatus: Record<string, boolean> = $state({});

  function setRepoAlias(path: string, alias: string) {
    const updated = { ...repoAliases };
    if (alias.trim()) {
      updated[path] = alias.trim();
    } else {
      delete updated[path];
    }
    repoAliases = updated;
    localStorage.setItem("forge:repo-aliases", JSON.stringify(updated));
  }

  function removeRecentRepo(path: string) {
    const list = getStoredRecents().filter((p) => p !== path);
    localStorage.setItem("forge:recent-repos", JSON.stringify(list));
    recentRepos = list;
  }

  async function relocateRepo(oldPath: string, newPath: string) {
    if (!newPath.trim()) return;
    try {
      const exists = await gitService.checkRepoExists(newPath);
      if (!exists) {
        setStatus(`Direktori ${newPath} bukan repositori git yang valid`, true);
        return;
      }
      removeRecentRepo(oldPath);
      if (repoAliases[oldPath]) {
        setRepoAlias(newPath, repoAliases[oldPath]);
        setRepoAlias(oldPath, "");
      }
      await loadRepo(newPath);
      closeModal();
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  async function checkRecentReposMissing() {
    const statusMap: Record<string, boolean> = {};
    for (const r of recentRepos) {
      try {
        const ok = await gitService.checkRepoExists(r);
        statusMap[r] = !ok;
      } catch {
        statusMap[r] = true;
      }
    }
    repoMissingStatus = statusMap;
  }

  function getStoredRecents(): string[] {
    try {
      return JSON.parse(localStorage.getItem("forge:recent-repos") || "[]");
    } catch {
      return [];
    }
  }

  function saveRecent(path: string) {
    const list = getStoredRecents().filter((p) => p !== path);
    list.unshift(path);
    const sliced = list.slice(0, 10);
    localStorage.setItem("forge:recent-repos", JSON.stringify(sliced));
    recentRepos = sliced;
  }

  async function loadRepo(path: string) {
    try {
      setStatus("Memuat repositori...");
      const status = await gitService.inspectRepository(path);
      activeRepo = path;
      localStorage.setItem("forge:active-repo", path);
      saveRecent(path);
      currentStatus = status;

      try {
        currentRemotes = await gitService.listRemotes(path);
        hasRemotes = currentRemotes.length > 0;
        const originRemote = currentRemotes.find((r) => r.name === "origin") || currentRemotes[0];
        if (originRemote) {
          const parsed = await gitService.parseGithubRepository(originRemote.url);
          if (parsed) {
            githubRepoInfo = { owner: parsed[0], repo: parsed[1] };
          } else {
            githubRepoInfo = null;
          }
        } else {
          githubRepoInfo = null;
        }
      } catch {
        currentRemotes = [];
        hasRemotes = false;
        githubRepoInfo = null;
      }

      if (status.branch && !status.branch.startsWith("(")) {
        try {
          aheadBehind = await gitService.getBranchAheadBehind(path, status.branch);
        } catch {
          aheadBehind = { ahead: 0, behind: 0 };
        }
      } else {
        aheadBehind = { ahead: 0, behind: 0 };
      }

      if (status.files.length > 0) {
        if (!selectedFile || !status.files.some((f) => f.path === selectedFile?.path)) {
          selectedFile = status.files[0];
        }
        await loadDiff(selectedFile);
      } else {
        selectedFile = null;
        currentDiff = "";
      }

      if (activeTab === "history") {
        await loadHistory();
      }

      setStatus("Siap");
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  async function loadDiff(file: FileEntry | null) {
    if (!file) {
      currentDiff = "";
      return;
    }
    try {
      currentDiff = await gitService.getFileDiff(
        activeRepo,
        file.path,
        file.staged,
        ignoreWhitespace
      );
    } catch (err) {
      currentDiff = `Error loading diff: ${String(err)}`;
    }
  }

  async function handleToggleWhitespace() {
    ignoreWhitespace = !ignoreWhitespace;
    if (selectedFile) {
      await loadDiff(selectedFile);
    }
  }

  async function handleAddToGitignore(filePath: string) {
    try {
      await gitService.addToGitignore(activeRepo, filePath);
      setStatus(`Menambahkan ${filePath} ke .gitignore`);
      await loadRepo(activeRepo);
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  async function loadHistory() {
    try {
      setStatus("Memuat riwayat komit...");
      commitHistory = await gitService.getCommitHistory(activeRepo, 50);
      setStatus("Siap");

      if (githubRepoInfo && commitHistory.length > 0) {
        const topSha = commitHistory[0].sha;
        try {
          const runs = await gitService.getCommitCheckRuns(
            githubRepoInfo.owner,
            githubRepoInfo.repo,
            topSha
          );
          if (runs.length > 0) {
            const latest = runs[0];
            checkRunsMap = {
              ...checkRunsMap,
              [topSha]: {
                status: latest.status,
                conclusion: latest.conclusion,
              },
            };
            if (latest.conclusion === "failure" || latest.conclusion === "timed_out") {
              sendDesktopNotification(
                "Forge Desktop: CI Build Gagal",
                `Pemeriksaan CI gagal pada commit ${commitHistory[0].short_sha} (${commitHistory[0].summary})`
              );
            }
          }
        } catch {
          // Non-blocking
        }
      }
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  async function handleToggleStage(file: FileEntry) {
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
      setStatus("Melakukan komit...");
      await gitService.commitChanges(activeRepo, summary, description, amend);
      setStatus("Komit berhasil dibuat");
      await loadRepo(activeRepo);
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  async function handleUndoCommit() {
    if (confirm("Batalkan komit terakhir? Perubahan akan kembali ke status staging.")) {
      try {
        setStatus("Membatalkan komit terakhir...");
        await gitService.undoLastCommit(activeRepo);
        setStatus("Komit terakhir dibatalkan (soft reset)");
        await loadRepo(activeRepo);
      } catch (err) {
        setStatus(String(err), true);
      }
    }
  }

  async function handleAbortMerge() {
    try {
      setStatus("Membatalkan merge...");
      await gitService.abortMerge(activeRepo);
      setStatus("Merge dibatalkan");
      await loadRepo(activeRepo);
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  async function handleAbortRebase() {
    try {
      setStatus("Membatalkan rebase...");
      await gitService.abortRebase(activeRepo);
      setStatus("Rebase dibatalkan");
      await loadRepo(activeRepo);
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  function requestConfirmation(title: string, message: string, action: () => Promise<void>) {
    confirmModalTitle = title;
    confirmModalMessage = message;
    confirmModalAction = action;
    activeModal = "confirm";
  }

  async function handleConfirmModalSubmit() {
    const action = confirmModalAction;
    closeModal();
    if (action) {
      await action();
    }
  }

  async function handleDiscardFile(file: FileEntry) {
    const doDiscard = async () => {
      try {
        setStatus(`Membatalkan perubahan pada ${file.path}...`);
        await gitService.discardFileChanges(activeRepo, file.path);
        setStatus(`Perubahan ${file.path} dibatalkan`);
        await loadRepo(activeRepo);
      } catch (err) {
        setStatus(String(err), true);
      }
    };

    if (confirmDiscard) {
      requestConfirmation(
        "Batalkan Perubahan File",
        `Apakah Anda yakin ingin membatalkan semua perubahan pada file "${file.path}"? Tindakan ini tidak dapat diurungkan.`,
        doDiscard
      );
    } else {
      await doDiscard();
    }
  }

  async function handleDiscardAll() {
    const doDiscardAll = async () => {
      try {
        setStatus("Membatalkan seluruh perubahan working tree...");
        await gitService.discardAllChanges(activeRepo);
        setStatus("Seluruh perubahan berhasil dibatalkan");
        await loadRepo(activeRepo);
      } catch (err) {
        setStatus(String(err), true);
      }
    };

    if (confirmDiscard) {
      requestConfirmation(
        "Batalkan Semua Perubahan",
        "Apakah Anda yakin ingin membatalkan SEMUA perubahan pada repositori ini? Semua modifikasi dan file belum terlacak akan dihapus secara permanen.",
        doDiscardAll
      );
    } else {
      await doDiscardAll();
    }
  }

  // Remote operations
  async function handleFetch() {
    try {
      setStatus("Melakukan fetch dari remote...");
      const res = await gitService.gitFetch(activeRepo);
      setStatus(res || "Fetch selesai");
      await loadRepo(activeRepo);
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  async function handlePull() {
    try {
      setStatus("Melakukan pull dari remote...");
      const res = await gitService.gitPull(activeRepo);
      setStatus(res || "Pull selesai");
      await loadRepo(activeRepo);
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  async function handlePush() {
    try {
      setStatus("Melakukan push ke remote...");
      const res = await gitService.gitPush(activeRepo);
      setStatus(res || "Push selesai");
      await loadRepo(activeRepo);
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  // Modals opening
  async function openBranchModal() {
    try {
      branches = await gitService.listBranches(activeRepo);
      newBranchName = "";
      branchSearch = "";
      branchToRename = null;
      branchDivergenceMap = {};
      activeModal = "branch";

      const current = branches.find((b) => b.is_current);
      if (current) {
        for (const b of branches) {
          if (!b.is_current) {
            gitService
              .compareBranches(activeRepo, current.name, b.name)
              .then((div) => {
                branchDivergenceMap = { ...branchDivergenceMap, [b.name]: div };
              })
              .catch(() => {});
          }
        }
      }
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  function openPublishModal() {
    publishName = activeRepo.split("/").pop() || "";
    publishDesc = "";
    publishPrivate = false;
    activeModal = "publish";
  }

  async function handlePublishSubmit() {
    if (!publishName.trim()) return;
    closeModal();
    try {
      setStatus(`Mempublikasikan repositori ${publishName} ke GitHub...`);
      const cloneUrl = await gitService.publishRepositoryToGithub(
        activeRepo,
        {
          name: publishName.trim(),
          description: publishDesc.trim() || undefined,
          private: publishPrivate,
        }
      );
      setStatus(`Repositori berhasil dipublikasikan ke GitHub: ${cloneUrl}`);
      await loadRepo(activeRepo);
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  function applyTheme(theme: string) {
    let resolved = theme;
    if (theme === "system") {
      resolved =
        typeof window !== "undefined" &&
        window.matchMedia &&
        window.matchMedia("(prefers-color-scheme: light)").matches
          ? "light"
          : "dark";
    }
    if (typeof document !== "undefined") {
      document.documentElement.setAttribute("data-theme", resolved);
    }
  }

  function setupAutofetch() {
    if (autofetchTimer) {
      clearInterval(autofetchTimer);
      autofetchTimer = null;
    }
    if (autofetchMinutes > 0 && typeof window !== "undefined") {
      autofetchTimer = window.setInterval(async () => {
        if (hasRemotes && activeRepo) {
          try {
            await gitService.gitFetch(activeRepo);
            const status = await gitService.inspectRepository(activeRepo);
            currentStatus = status;
          } catch {
            // background silent failure
          }
        }
      }, autofetchMinutes * 60 * 1000);
    }
  }

  async function openOpenRepoModal() {
    recentRepos = getStoredRecents();
    openRepoPath = activeRepo;
    activeModal = "open";
    await checkRecentReposMissing();
  }

  function openInitModal() {
    initRepoPath = "";
    initDefaultBranch = "main";
    initGitignore = "Node";
    initLicense = "MIT";
    activeModal = "init";
  }

  function openCloneModal() {
    cloneUrl = "";
    cloneTarget = "";
    activeModal = "clone";
  }

  async function openSettingsModal() {
    try {
      repoConfig = await gitService.getRepoConfig(activeRepo);
    } catch {
      repoConfig = { user_name: "", user_email: "" };
    }
    try {
      globalConfig = await gitService.getGlobalConfig();
    } catch {
      globalConfig = { user_name: "", user_email: "" };
    }
    activeModal = "settings";
  }

  async function openStashModal() {
    try {
      stashList = await gitService.gitListStashes(activeRepo);
      stashMsg = "";
      selectedStash = null;
      stashDiff = "";
      activeModal = "stash";
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  async function openDiagModal() {
    try {
      diagReport = await gitService.getDiagnostics(activeRepo);
      activeModal = "diag";
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  function openAuthModal() {
    deviceFlow = null;
    pollStatus = "";
    oauthClientId = "";
    if (pollTimer) clearInterval(pollTimer);
    activeModal = "auth";
  }

  function closeModal() {
    activeModal = null;
    if (pollTimer) clearInterval(pollTimer);
  }

  // Branch modal actions
  async function handleSwitchBranch(name: string) {
    if (currentStatus && currentStatus.files.length > 0) {
      requestConfirmation(
        "Smart Stash & Pindah Branch",
        `Terdapat ${currentStatus.files.length} file dengan perubahan belum dikomit. Simpan perubahan ke Stash sebelum berpindah ke "${name}"?`,
        async () => {
          try {
            setStatus("Menyimpan perubahan ke stash...");
            await gitService.gitStashSave(activeRepo, `Auto-stash sebelum checkout ke ${name}`);
            setStatus(`Berpindah ke branch ${name}...`);
            await gitService.switchBranch(activeRepo, name);
            setStatus(`Berhasil pindah ke branch ${name} (perubahan tersimpan di Stash)`);
            await loadRepo(activeRepo);
          } catch (err) {
            setStatus(String(err), true);
          }
        }
      );
      return;
    }

    closeModal();
    try {
      await gitService.switchBranch(activeRepo, name);
      await loadRepo(activeRepo);
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  async function handleCreateBranch() {
    const name = newBranchName.trim();
    if (!name) return;
    closeModal();
    try {
      await gitService.createBranch(activeRepo, name);
      await loadRepo(activeRepo);
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  async function handleDeleteBranch(name: string, isCurrent: boolean) {
    if (isCurrent) {
      alert("Tidak dapat menghapus branch yang sedang aktif.");
      return;
    }
    if (confirm(`Hapus branch '${name}'?`)) {
      try {
        await gitService.deleteBranch(activeRepo, name, true);
        branches = await gitService.listBranches(activeRepo);
        setStatus(`Branch ${name} berhasil dihapus`);
      } catch (err) {
        setStatus(String(err), true);
      }
    }
  }

  async function handleStartRename(name: string) {
    branchToRename = name;
    renameBranchInput = name;
  }

  async function handleConfirmRename() {
    if (!branchToRename) return;
    const newName = renameBranchInput.trim();
    if (!newName || newName === branchToRename) {
      branchToRename = null;
      return;
    }
    try {
      await gitService.renameBranch(activeRepo, branchToRename, newName);
      branchToRename = null;
      branches = await gitService.listBranches(activeRepo);
      await loadRepo(activeRepo);
      setStatus(`Branch berhasil diubah menjadi ${newName}`);
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  async function handleMergeBranch(targetName: string) {
    if (confirm(`Gabungkan (merge) branch '${targetName}' ke '${currentStatus?.branch}'?`)) {
      closeModal();
      try {
        setStatus(`Menggabungkan ${targetName}...`);
        const res = await gitService.mergeBranch(activeRepo, targetName);
        setStatus(res || "Merge berhasil");
        await loadRepo(activeRepo);
      } catch (err) {
        setStatus(String(err), true);
      }
    }
  }

  async function handleRebase(targetName: string) {
    if (confirm(`Rebase branch '${currentStatus?.branch}' ke atas '${targetName}'?`)) {
      closeModal();
      try {
        setStatus(`Me-rebase ke ${targetName}...`);
        const res = await gitService.rebaseBranch(activeRepo, targetName);
        setStatus(res || "Rebase berhasil");
        await loadRepo(activeRepo);
      } catch (err) {
        setStatus(String(err), true);
      }
    }
  }

  async function handleCherryPick(sha: string) {
    try {
      setStatus(`Melakukan cherry-pick ${sha.slice(0, 7)}...`);
      await gitService.cherryPick(activeRepo, sha);
      setStatus(`Berhasil cherry-pick komit ${sha.slice(0, 7)}`);
      await loadRepo(activeRepo);
      await loadHistory();
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  async function handleRevertCommit(sha: string) {
    try {
      setStatus(`Membalikkan komit ${sha.slice(0, 7)}...`);
      await gitService.revertCommit(activeRepo, sha);
      setStatus(`Berhasil revert komit ${sha.slice(0, 7)}`);
      await loadRepo(activeRepo);
      await loadHistory();
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  function openTagModal(sha: string) {
    tagModalTargetSha = sha;
    tagNameInput = "";
    activeModal = "tag";
  }

  async function handleCreateTagSubmit() {
    if (!tagNameInput.trim()) return;
    try {
      setStatus(`Membuat tag ${tagNameInput.trim()}...`);
      await gitService.createTag(
        activeRepo,
        tagNameInput.trim(),
        tagModalTargetSha || undefined
      );
      setStatus(`Tag ${tagNameInput.trim()} berhasil dibuat`);
      activeModal = null;
      tagNameInput = "";
      tagModalTargetSha = null;
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  async function handleResetToCommit(sha: string) {
    if (
      !confirm(
        `Apakah Anda yakin ingin me-reset branch aktif ke commit ${sha.slice(
          0,
          7
        )}? Perubahan yang belum di-commit akan tetap ada di working tree (soft reset).`
      )
    ) {
      return;
    }
    try {
      setStatus(`Me-reset branch ke ${sha.slice(0, 7)}...`);
      await gitService.resetToCommit(activeRepo, sha, false);
      setStatus(`Branch berhasil di-reset ke ${sha.slice(0, 7)}`);
      await loadRepo(activeRepo);
      await loadHistory();
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  async function handleForcePush() {
    const doForcePush = async () => {
      try {
        setStatus("Melakukan force push (--force-with-lease)...");
        await gitService.gitForcePush(activeRepo);
        setStatus("Force push berhasil diselesaikan");
        await loadRepo(activeRepo);
      } catch (err) {
        setStatus(String(err), true);
      }
    };

    if (confirmForcePush) {
      requestConfirmation(
        "Konfirmasi Force Push",
        "Peringatan: Force Push with lease akan menimpa commit di remote repository jika ada perbedaan riwayat. Lanjutkan proses?",
        doForcePush
      );
    } else {
      await doForcePush();
    }
  }

  async function openPRModal() {
    activeModal = "pr";
    if (githubRepoInfo) {
      await loadGithubData();
    }
  }

  async function loadGithubData() {
    if (!githubRepoInfo) return;
    isLoadingGithub = true;
    try {
      if (prTab === "prs") {
        pullRequests = await gitService.listGithubPullRequests(
          githubRepoInfo.owner,
          githubRepoInfo.repo
        );
      } else {
        issues = await gitService.listGithubIssues(
          githubRepoInfo.owner,
          githubRepoInfo.repo
        );
      }
    } catch (err) {
      setStatus(String(err), true);
    } finally {
      isLoadingGithub = false;
    }
  }

  async function handleCheckoutPR(prNumber: number) {
    closeModal();
    try {
      setStatus(`Mengambil branch PR #${prNumber}...`);
      await gitService.checkoutGithubPullRequest(activeRepo, prNumber);
      setStatus(`Berhasil checkout branch PR #${prNumber}`);
      await loadRepo(activeRepo);
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  function handleCreatePR() {
    if (!githubRepoInfo || !currentStatus?.branch) return;
    const url = `https://github.com/${githubRepoInfo.owner}/${githubRepoInfo.repo}/compare/${currentStatus.branch}?expand=1`;
    window.open(url, "_blank");
  }

  // Stash modal actions
  async function selectStash(stashLine: string) {
    selectedStash = stashLine;
    const match = stashLine.match(/^stash@\{[0-9]+\}/);
    const ref = match ? match[0] : undefined;
    isLoadingStashDiff = true;
    try {
      stashDiff = await gitService.gitStashShow(activeRepo, ref);
    } catch (err) {
      stashDiff = `Gagal memuat diff stash: ${err}`;
    } finally {
      isLoadingStashDiff = false;
    }
  }

  async function handleDropStash(stashLine: string) {
    const match = stashLine.match(/^stash@\{[0-9]+\}/);
    const ref = match ? match[0] : undefined;
    requestConfirmation(
      "Hapus Stash",
      `Apakah Anda yakin ingin menghapus ${ref || stashLine}? Perubahan di stash ini akan hilang secara permanen.`,
      async () => {
        try {
          await gitService.gitStashDrop(activeRepo, ref);
          setStatus(`Stash ${ref || stashLine} berhasil dihapus`);
          stashList = await gitService.gitListStashes(activeRepo);
          selectedStash = null;
          stashDiff = "";
        } catch (err) {
          setStatus(String(err), true);
        }
      }
    );
  }

  async function handleStashSave() {
    closeModal();
    try {
      await gitService.gitStashSave(activeRepo, stashMsg || undefined);
      await loadRepo(activeRepo);
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  async function handleStashPop() {
    closeModal();
    try {
      await gitService.gitStashPop(activeRepo);
      await loadRepo(activeRepo);
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  // Clone actions
  async function handleClone() {
    const url = cloneUrl.trim();
    const target = cloneTarget.trim();
    if (!url || !target) {
      alert("URL dan folder tujuan wajib diisi");
      return;
    }
    closeModal();
    try {
      setStatus(`Cloning ${url}...`);
      await gitService.cloneRepository(url, target);
      setStatus("Clone sukses");
      await loadRepo(target);
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  // Init repo actions
  async function handleInitRepo() {
    const target = initRepoPath.trim();
    if (!target) {
      alert("Path folder repositori wajib diisi");
      return;
    }
    closeModal();
    try {
      setStatus(`Menginisialisasi repositori baru di ${target}...`);
      let giContent: string | undefined;
      if (initGitignore === "Node") {
        giContent = "node_modules/\ndist/\n.env\n*.log";
      } else if (initGitignore === "Rust") {
        giContent = "target/\n**/*.rs.bk\nCargo.lock";
      } else if (initGitignore === "Python") {
        giContent = "__pycache__/\n*.py[cod]\nvenv/\n.env";
      }

      let licContent: string | undefined;
      if (initLicense === "MIT") {
        licContent = `MIT License\n\nCopyright (c) ${new Date().getFullYear()}\n\nPermission is hereby granted, free of charge...`;
      } else if (initLicense === "Apache-2.0") {
        licContent = `Apache License Version 2.0, January 2004\nhttp://www.apache.org/licenses/`;
      }

      await gitService.initRepository(target, initDefaultBranch, giContent, licContent);
      setStatus("Repositori berhasil dibuat");
      await loadRepo(target);
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  // Settings actions
  async function handleSaveSettings() {
    closeModal();
    try {
      await gitService.setRepoConfig(
        activeRepo,
        repoConfig.user_name,
        repoConfig.user_email
      );
      await gitService.setGlobalConfig(
        globalConfig.user_name,
        globalConfig.user_email
      );
      if (typeof localStorage !== "undefined") {
        localStorage.setItem("forge:confirm-discard", String(confirmDiscard));
        localStorage.setItem("forge:confirm-force-push", String(confirmForcePush));
        localStorage.setItem("forge:theme", activeTheme);
        localStorage.setItem("forge:autofetch-minutes", String(autofetchMinutes));
        localStorage.setItem("forge:desktop-notifs", String(desktopNotifEnabled));
      }
      applyTheme(activeTheme);
      setupAutofetch();
      setStatus("Pengaturan konfigurasi lokal, global, tema, dan preferensi keamanan berhasil disimpan");
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  function openSquashModal(index: number, sha: string) {
    squashCount = index + 1;
    squashSummary = `Squash ${squashCount} komit ke ${sha.slice(0, 7)}`;
    squashDescription = "";
    activeModal = "squash";
  }

  async function handleSquashSubmit() {
    if (!squashSummary.trim()) return;
    closeModal();
    try {
      setStatus(`Me-nyquash ${squashCount} komit...`);
      await gitService.squashCommits(
        activeRepo,
        squashCount,
        squashSummary.trim(),
        squashDescription.trim() || undefined
      );
      setStatus(`Berhasil squash ${squashCount} komit`);
      await loadRepo(activeRepo);
      await loadHistory();
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  // External tools
  async function handleOpenInTerminal() {
    try {
      await gitService.openInTerminal(activeRepo);
      setStatus("Membuka terminal...");
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  async function handleOpenInFileManager() {
    try {
      await gitService.openInFileManager(activeRepo);
      setStatus("Membuka file manager...");
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  async function handleOpenInEditor() {
    try {
      await gitService.openInEditor(activeRepo);
      setStatus("Membuka editor eksternal...");
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  // Auth Device Flow
  async function handleStartDeviceFlow() {
    const cid = oauthClientId.trim();
    if (!cid) {
      alert("Client ID wajib diisi");
      return;
    }
    try {
      pollStatus = "Meminta kode...";
      pollStatusClass = "";
      deviceFlow = await gitService.githubStartDeviceFlow(cid);
      pollStatus = "Menunggu konfirmasi di browser...";

      const sec = Math.max(deviceFlow.interval || 5, 5);
      pollTimer = window.setInterval(async () => {
        try {
          const res = await gitService.githubPollToken(cid, deviceFlow!.device_code);
          if (res.access_token) {
            if (pollTimer) clearInterval(pollTimer);
            await gitService.configureGitCredentials("oauth", res.access_token);
            pollStatus = "Berhasil terhubung ke akun GitHub!";
            pollStatusClass = "success";
          } else if (res.error && res.error !== "authorization_pending") {
            if (pollTimer) clearInterval(pollTimer);
            pollStatus = `Gagal: ${res.error_description || res.error}`;
            pollStatusClass = "error";
          }
        } catch {
          // continue polling
        }
      }, sec * 1000);
    } catch (err) {
      pollStatus = String(err);
      pollStatusClass = "error";
    }
  }

  let filteredBranches = $derived(
    branches.filter((b) =>
      b.name.toLowerCase().includes(branchSearch.toLowerCase())
    )
  );

  onMount(async () => {
    applyTheme(activeTheme);
    setupAutofetch();
    try {
      const cliRepo = await gitService.getCliRepoArg();
      if (cliRepo) {
        activeRepo = cliRepo;
      }
    } catch {
      // non-blocking
    }
    loadRepo(activeRepo);
  });
</script>

<div class="app-container">
  <header class="app-header">
    <div class="header-left">
      <button onclick={openOpenRepoModal} class="btn-secondary" style="font-weight: 600;">
        Buka Repo
      </button>
      <button onclick={openInitModal} class="btn-secondary" title="Inisialisasi Git Baru">
        Repo Baru
      </button>
      <button onclick={openCloneModal} class="btn-secondary">
        Clone
      </button>
      <div class="repo-badge">
        <span>{repoAliases[activeRepo] || activeRepo.split("/").pop() || activeRepo}</span>
      </div>
      <button
        onclick={openBranchModal}
        class="branch-select-btn"
        disabled={!currentStatus}
      >
        Branch: <span>{currentStatus?.branch || "-"}</span>
      </button>
    </div>

    <nav class="nav-tabs" aria-label="Navigasi">
      <button
        class="nav-tab {activeTab === 'changes' ? 'active' : ''}"
        onclick={() => {
          activeTab = "changes";
          if (selectedFile) loadDiff(selectedFile);
        }}
      >
        Changes
      </button>
      <button
        class="nav-tab {activeTab === 'history' ? 'active' : ''}"
        onclick={() => {
          activeTab = "history";
          loadHistory();
        }}
      >
        History
      </button>
    </nav>

    <div class="header-right">
      {#if !hasRemotes}
        <button
          onclick={openPublishModal}
          class="btn-primary"
          style="font-size: 11px; padding: 4px 10px;"
          title="Publikasikan repositori lokal ini ke GitHub"
        >
          Publish ke GitHub
        </button>
      {/if}
      {#if hasUpstream}
        <button
          onclick={handleSyncFork}
          class="btn-secondary"
          style="font-size: 11px; padding: 4px 8px; border-color: var(--accent-blue); color: var(--accent-blue);"
          title="Tarik dan gabungkan komit terbaru dari remote upstream fork"
        >
          Sync Fork
        </button>
      {/if}
      <button
        onclick={handleFetch}
        class="btn-secondary"
        title="Fetch dari Remote"
        disabled={!hasRemotes}
      >
        Fetch
      </button>
      <button
        onclick={handlePull}
        class="btn-secondary"
        title="Pull dari Remote"
        disabled={!hasRemotes}
      >
        Pull {aheadBehind.behind > 0 ? `↓${aheadBehind.behind}` : ""}
      </button>
      <button
        onclick={handlePush}
        class="btn-secondary"
        title="Push ke Remote"
        disabled={!hasRemotes}
      >
        Push {aheadBehind.ahead > 0 ? `↑${aheadBehind.ahead}` : ""}
      </button>
      <button
        onclick={handleForcePush}
        class="btn-secondary"
        title="Force Push dengan lease (--force-with-lease)"
        disabled={!hasRemotes}
        style="font-size: 11px;"
      >
        Force Push
      </button>

      {#if githubRepoInfo}
        <button
          onclick={openPRModal}
          class="btn-secondary"
          title="Lihat Pull Requests dan Issues GitHub"
          style="font-size: 11px; border-color: var(--accent-blue);"
        >
          PR & Issues
        </button>
      {/if}

      <button onclick={handleOpenInEditor} class="btn-secondary" title="Buka folder di VS Code / Editor">
        Editor
      </button>
      <button onclick={handleOpenInTerminal} class="btn-secondary" title="Buka folder di Terminal">
        Terminal
      </button>
      <button onclick={handleOpenInFileManager} class="btn-secondary" title="Buka folder di File Manager">
        Folder
      </button>

      <button onclick={openSettingsModal} class="btn-secondary" title="Pengaturan Repositori">
        Settings
      </button>
      <button onclick={openAuthModal} class="btn-secondary" title="Akun GitHub">GitHub</button>
      <button onclick={openStashModal} class="btn-secondary" title="Simpan perubahan ke Stash">Stash</button>
      <button onclick={openDiagModal} class="btn-secondary" title="Laporan Diagnostik Sistem">Diagnostik</button>
      <button onclick={() => loadRepo(activeRepo)} class="btn-secondary" title="Muat Ulang">Refresh</button>
    </div>
  </header>

  <main class="app-body">
    <aside class="sidebar-panel">
      {#if activeTab === "changes"}
        {#if currentStatus}
          <Changes
            status={currentStatus}
            {selectedFile}
            issues={issues.map((i) => ({ number: i.number, title: i.title }))}
            onSelectFile={(f) => {
              selectedFile = f;
              loadDiff(f);
            }}
            onToggleStage={handleToggleStage}
            onStageAll={handleStageAll}
            onUnstageAll={handleUnstageAll}
            onCommit={handleCommit}
            onUndoCommit={handleUndoCommit}
            onAbortMerge={handleAbortMerge}
            onAbortRebase={handleAbortRebase}
            onDiscardFile={handleDiscardFile}
            onDiscardAll={handleDiscardAll}
            onOpenInEditor={(filePath) => gitService.openInEditor(activeRepo + "/" + filePath)}
          />
        {/if}
      {:else}
        <div class="sidebar-header">
          <h3>Riwayat Komit</h3>
        </div>
        <div style="padding: 12px; font-size: 11px; color: var(--text-secondary);">
          Branch saat ini: <strong>{currentStatus?.branch || "-"}</strong>
        </div>
      {/if}
    </aside>

    <section class="content-panel">
      {#if activeTab === "changes"}
        {#if selectedFile}
          <Diff
            filePath={selectedFile.path}
            diffText={currentDiff}
            {ignoreWhitespace}
            isLfs={selectedFile.is_lfs}
            onToggleWhitespace={handleToggleWhitespace}
            onAddToGitignore={handleAddToGitignore}
          />
        {:else}
          <div style="display: grid; place-items: center; height: 100%; color: var(--text-muted);">
            {currentStatus?.files.length === 0 ? "Working tree bersih. Tidak ada perubahan." : "Pilih file untuk melihat diff."}
          </div>
        {/if}
      {:else}
        <History
          commits={commitHistory}
          {checkRunsMap}
          onCherryPick={handleCherryPick}
          onRevert={handleRevertCommit}
          onTag={openTagModal}
          onReset={handleResetToCommit}
          onSquash={openSquashModal}
        />
      {/if}
    </section>
  </main>

  <footer class="app-footer">
    <span style="color: {isStatusError ? 'var(--diff-del-text)' : 'var(--text-muted)'}">{footerStatus}</span>
    <span>{activeRepo}</span>
  </footer>
</div>

<!-- Modals -->
{#if activeModal === "branch"}
  <div class="modal-overlay">
    <div class="modal-dialog" style="width: 480px;">
      <div class="modal-header">Pilih atau Kelola Branch</div>
      <div style="display: flex; gap: 6px;">
        <input
          type="text"
          class="commit-input"
          placeholder="Filter nama branch..."
          style="flex: 1;"
          bind:value={branchSearch}
        />
        <input
          type="text"
          class="commit-input"
          placeholder="Nama branch baru"
          style="flex: 1;"
          bind:value={newBranchName}
        />
        <button onclick={handleCreateBranch} class="btn-primary" style="font-size: 11px;">Buat</button>
      </div>

      {#if branchToRename}
        <div style="display: flex; gap: 6px; padding: 8px; background: var(--bg-subtle); border-radius: var(--radius-sm); align-items: center;">
          <span style="font-size: 11px; color: var(--text-secondary);">Ganti nama '{branchToRename}':</span>
          <input type="text" class="commit-input" bind:value={renameBranchInput} style="flex: 1; font-size: 11px;" />
          <button onclick={handleConfirmRename} class="btn-primary" style="font-size: 11px;">Simpan</button>
          <button onclick={() => (branchToRename = null)} class="btn-secondary" style="font-size: 11px;">Batal</button>
        </div>
      {/if}

      <div style="max-height: 240px; overflow-y: auto; border: 1px solid var(--border-default); border-radius: var(--radius-sm);">
        {#if filteredBranches.length === 0}
          <div style="padding: 16px; text-align: center; color: var(--text-muted); font-size: 11px;">Tidak ada branch ditemukan</div>
        {:else}
          {#each filteredBranches as b (b.name)}
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
              class="branch-item"
              style="padding: 6px 10px; display: flex; justify-content: space-between; align-items: center; font-size: 12px; border-bottom: 1px solid var(--border-muted); background: {b.is_current ? 'var(--bg-subtle)' : 'transparent'};"
            >
              <span
                onclick={() => handleSwitchBranch(b.name)}
                style="cursor: pointer; flex: 1; {b.is_current ? 'font-weight: 600;' : ''} display: flex; align-items: center; gap: 6px;"
              >
                <span>{b.name}</span>
                {#if b.is_current}
                  <span style="color: var(--accent-blue); font-size: 10px;">(aktif)</span>
                {:else if branchDivergenceMap[b.name]}
                  {@const div = branchDivergenceMap[b.name]}
                  <span style="font-size: 10px; color: var(--text-muted); font-family: var(--font-mono);">
                    {div.ahead > 0 ? `+${div.ahead}` : ""}{div.behind > 0 ? ` -${div.behind}` : ""}{div.ahead === 0 && div.behind === 0 ? "identik" : ""}
                  </span>
                {/if}
              </span>

              <div style="display: flex; gap: 4px; align-items: center;">
                {#if !b.is_current}
                  <button
                    onclick={() => handleMergeBranch(b.name)}
                    class="btn-secondary"
                    style="font-size: 10px; padding: 2px 6px;"
                    title="Merge branch ini ke branch aktif"
                  >
                    Merge
                  </button>
                  <button
                    onclick={() => handleRebase(b.name)}
                    class="btn-secondary"
                    style="font-size: 10px; padding: 2px 6px;"
                    title="Rebase branch aktif ke atas branch ini"
                  >
                    Rebase
                  </button>
                  <button
                    onclick={() => handleStartRename(b.name)}
                    class="btn-secondary"
                    style="font-size: 10px; padding: 2px 6px;"
                    title="Ganti nama branch"
                  >
                    Rename
                  </button>
                  <button
                    onclick={() => handleDeleteBranch(b.name, b.is_current)}
                    class="btn-secondary"
                    style="font-size: 10px; padding: 2px 6px; color: var(--diff-del-text);"
                    title="Hapus branch lokal"
                  >
                    Hapus
                  </button>
                {/if}
              </div>
            </div>
          {/each}
        {/if}
      </div>
      <div class="modal-actions">
        <button class="btn-secondary" onclick={closeModal}>Tutup</button>
      </div>
    </div>
  </div>
{:else if activeModal === "open"}
  <div class="modal-overlay">
    <div class="modal-dialog" style="width: 520px;">
      <div class="modal-header">Buka Repositori Git</div>
      <span style="font-size: 11px; color: var(--text-secondary); margin-bottom: 4px;">Path Folder Repositori:</span>
      <input type="text" class="commit-input" bind:value={openRepoPath} placeholder="/path/ke/repo" />
      {#if recentRepos.length > 0}
        <span style="font-size: 11px; color: var(--text-secondary); margin-top: 10px;">Repositori Terakhir & Alias:</span>
        <div style="max-height: 220px; overflow-y: auto; border: 1px solid var(--border-default); border-radius: var(--radius-sm); margin-top: 4px;">
          {#each recentRepos as r (r)}
            {@const isMissing = repoMissingStatus[r]}
            {@const alias = repoAliases[r]}
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
              style="padding: 6px 10px; font-size: 11px; border-bottom: 1px solid var(--border-muted); display: flex; align-items: center; justify-content: space-between; gap: 8px; background: {r === activeRepo ? 'rgba(56, 139, 253, 0.1)' : 'transparent'};"
            >
              <div
                onclick={() => { if (!isMissing) openRepoPath = r; }}
                style="cursor: {isMissing ? 'default' : 'pointer'}; flex: 1; overflow: hidden;"
              >
                <div style="font-weight: 600; display: flex; align-items: center; gap: 6px;">
                  <span>{alias || r.split("/").pop() || r}</span>
                  {#if alias}
                    <span style="font-size: 10px; font-weight: normal; color: var(--text-muted); font-family: var(--font-mono);">({r.split("/").pop()})</span>
                  {/if}
                  {#if isMissing}
                    <span style="font-size: 9px; padding: 1px 4px; border-radius: 2px; background: rgba(248,81,73,0.2); color: var(--diff-del-text); font-weight: 700;">HILANG</span>
                  {/if}
                </div>
                <div style="font-size: 10px; color: var(--text-muted); font-family: var(--font-mono); text-overflow: ellipsis; overflow: hidden; white-space: nowrap;">
                  {r}
                </div>
              </div>
              <div style="display: flex; gap: 4px; align-items: center;">
                {#if isMissing}
                  <button
                    class="btn-secondary"
                    style="font-size: 10px; padding: 1px 6px;"
                    title="Cari lokasi baru untuk repositori ini"
                    onclick={() => {
                      const newP = prompt("Masukkan path folder repositori yang baru:", r);
                      if (newP) relocateRepo(r, newP);
                    }}
                  >
                    Relokasi
                  </button>
                {/if}
                <button
                  class="btn-secondary"
                  style="font-size: 10px; padding: 1px 6px;"
                  title="Ubah alias / nama tampilan repositori"
                  onclick={() => {
                    const newAlias = prompt("Ubah alias tampilan untuk repositori ini:", alias || "");
                    if (newAlias !== null) setRepoAlias(r, newAlias);
                  }}
                >
                  Alias
                </button>
                <button
                  class="btn-secondary"
                  style="font-size: 10px; padding: 1px 6px; color: var(--diff-del-text);"
                  title="Hapus dari daftar repositori terakhir"
                  onclick={() => removeRecentRepo(r)}
                >
                  x
                </button>
              </div>
            </div>
          {/each}
        </div>
      {/if}
      <div class="modal-actions" style="margin-top: 12px;">
        <button class="btn-secondary" onclick={closeModal}>Batal</button>
        <button class="btn-primary" onclick={() => { closeModal(); loadRepo(openRepoPath); }}>Buka</button>
      </div>
    </div>
  </div>
{:else if activeModal === "init"}
  <div class="modal-overlay">
    <div class="modal-dialog" style="width: 440px;">
      <div class="modal-header">Inisialisasi Repositori Baru</div>
      <div style="display: flex; flex-direction: column; gap: 8px;">
        <span style="font-size: 11px; color: var(--text-secondary);">Lokasi Direktori</span>
        <input type="text" class="commit-input" bind:value={initRepoPath} placeholder="/path/ke/folder-baru" />

        <span style="font-size: 11px; color: var(--text-secondary);">Default Branch</span>
        <input type="text" class="commit-input" bind:value={initDefaultBranch} placeholder="main" />

        <span style="font-size: 11px; color: var(--text-secondary);">Template .gitignore</span>
        <select class="commit-input" bind:value={initGitignore} style="background: var(--bg-canvas); color: var(--text-default);">
          <option value="None">Tidak Ada</option>
          <option value="Node">Node.js</option>
          <option value="Rust">Rust</option>
          <option value="Python">Python</option>
        </select>

        <span style="font-size: 11px; color: var(--text-secondary);">Lisensi</span>
        <select class="commit-input" bind:value={initLicense} style="background: var(--bg-canvas); color: var(--text-default);">
          <option value="None">Tidak Ada</option>
          <option value="MIT">MIT License</option>
          <option value="Apache-2.0">Apache 2.0</option>
        </select>
      </div>
      <div class="modal-actions" style="margin-top: 12px;">
        <button class="btn-secondary" onclick={closeModal}>Batal</button>
        <button class="btn-primary" onclick={handleInitRepo}>Buat Repositori</button>
      </div>
    </div>
  </div>
{:else if activeModal === "settings"}
  <div class="modal-overlay">
    <div class="modal-dialog" style="width: 460px;">
      <div class="modal-header">Pengaturan Repositori & Git</div>
      <div style="display: flex; flex-direction: column; gap: 8px;">
        <div style="font-weight: 600; font-size: 11px; color: var(--text-primary); border-bottom: 1px solid var(--border-default); padding-bottom: 4px;">
          Konfigurasi Lokal
        </div>
        <span style="font-size: 11px; color: var(--text-secondary);">Nama Author (user.name)</span>
        <input type="text" class="commit-input" bind:value={repoConfig.user_name} placeholder="Nama Anda (lokal)" />

        <span style="font-size: 11px; color: var(--text-secondary);">Email Author (user.email)</span>
        <input type="text" class="commit-input" bind:value={repoConfig.user_email} placeholder="email@example.com (lokal)" />

        <div style="font-weight: 600; font-size: 11px; color: var(--text-primary); border-bottom: 1px solid var(--border-default); padding-bottom: 4px; margin-top: 8px;">
          Konfigurasi Global Git (~/.gitconfig)
        </div>
        <span style="font-size: 11px; color: var(--text-secondary);">Global Nama (user.name)</span>
        <input type="text" class="commit-input" bind:value={globalConfig.user_name} placeholder="Nama Anda (global)" />

        <span style="font-size: 11px; color: var(--text-secondary);">Global Email (user.email)</span>
        <input type="text" class="commit-input" bind:value={globalConfig.user_email} placeholder="email@example.com (global)" />

        <div style="font-weight: 600; font-size: 11px; color: var(--text-primary); border-bottom: 1px solid var(--border-default); padding-bottom: 4px; margin-top: 8px;">
          Tampilan & Latar Belakang
        </div>
        <div style="display: flex; gap: 8px; align-items: center;">
          <span style="font-size: 11px; color: var(--text-secondary); width: 140px;">Tema Antarmuka:</span>
          <select class="commit-input" bind:value={activeTheme} style="flex: 1; background: var(--bg-canvas); color: var(--text-primary);">
            <option value="dark">Dark Theme (Gelap)</option>
            <option value="light">Light Theme (Terang)</option>
            <option value="system">Mengikuti Sistem OS</option>
          </select>
        </div>
        <div style="display: flex; gap: 8px; align-items: center;">
          <span style="font-size: 11px; color: var(--text-secondary); width: 140px;">Auto-fetch Otomatis:</span>
          <select class="commit-input" bind:value={autofetchMinutes} style="flex: 1; background: var(--bg-canvas); color: var(--text-primary);">
            <option value={0}>Nonaktif</option>
            <option value={1}>Setiap 1 menit</option>
            <option value={5}>Setiap 5 menit (Default)</option>
            <option value={10}>Setiap 10 menit</option>
            <option value={15}>Setiap 15 menit</option>
          </select>
        </div>

        <div style="font-weight: 600; font-size: 11px; color: var(--text-primary); border-bottom: 1px solid var(--border-default); padding-bottom: 4px; margin-top: 8px;">
          Preferensi Keamanan
        </div>
        <label style="display: flex; align-items: center; gap: 8px; font-size: 11px; color: var(--text-primary); cursor: pointer;">
          <input type="checkbox" bind:checked={confirmDiscard} />
          <span>Konfirmasi sebelum membatalkan perubahan file (Discard)</span>
        </label>
        <label style="display: flex; align-items: center; gap: 8px; font-size: 11px; color: var(--text-primary); cursor: pointer;">
          <input type="checkbox" bind:checked={confirmForcePush} />
          <span>Konfirmasi sebelum Force Push (--force-with-lease)</span>
        </label>
        <label style="display: flex; align-items: center; gap: 8px; font-size: 11px; color: var(--text-primary); cursor: pointer;">
          <input type="checkbox" bind:checked={desktopNotifEnabled} />
          <span>Notifikasi Desktop (CI Build Gagal & Sync Fork)</span>
        </label>
      </div>
      <div class="modal-actions" style="margin-top: 14px;">
        <button class="btn-secondary" onclick={closeModal}>Batal</button>
        <button class="btn-primary" onclick={handleSaveSettings}>Simpan Semua</button>
      </div>
    </div>
  </div>
{:else if activeModal === "clone"}
  <div class="modal-overlay">
    <div class="modal-dialog">
      <div class="modal-header">Clone Repositori</div>
      <div style="display: flex; flex-direction: column; gap: 6px;">
        <span style="font-size: 11px; color: var(--text-secondary);">URL Repositori (HTTPS / SSH)</span>
        <input type="text" class="commit-input" bind:value={cloneUrl} placeholder="https://github.com/user/repo.git" />
        <span style="font-size: 11px; color: var(--text-secondary); margin-top: 6px;">Folder Tujuan</span>
        <input type="text" class="commit-input" bind:value={cloneTarget} placeholder="/home/user/Development/my-repo" />
      </div>
      <div class="modal-actions">
        <button class="btn-secondary" onclick={closeModal}>Batal</button>
        <button class="btn-primary" onclick={handleClone}>Clone</button>
      </div>
    </div>
  </div>
{:else if activeModal === "stash"}
  <div class="modal-overlay">
    <div class="modal-dialog" style="width: 700px; max-height: 80vh; display: flex; flex-direction: column;">
      <div class="modal-header">Git Stash & Pratinjau Perubahan</div>
      <div style="display: flex; gap: 6px; margin-bottom: 8px;">
        <input type="text" class="commit-input" bind:value={stashMsg} placeholder="Pesan stash baru (opsional)" style="flex: 1;" />
        <button onclick={handleStashSave} class="btn-primary" style="font-size: 11px;">Simpan Stash</button>
      </div>

      <div style="display: grid; grid-template-columns: 280px 1fr; gap: 12px; min-height: 240px; max-height: 380px; overflow: hidden;">
        <!-- Left: Stash list -->
        <div style="display: flex; flex-direction: column; gap: 6px; border-right: 1px solid var(--border-default); padding-right: 10px;">
          <div style="display: flex; justify-content: space-between; align-items: center;">
            <span style="font-size: 11px; color: var(--text-secondary); font-weight: 600;">Daftar Stash ({stashList.length})</span>
            <button onclick={handleStashPop} class="btn-secondary" style="font-size: 10px; padding: 2px 6px;" disabled={stashList.length === 0} title="Terapkan stash teratas ke working tree">
              Pop Teratas
            </button>
          </div>
          <div style="overflow-y: auto; flex: 1; border: 1px solid var(--border-default); border-radius: var(--radius-sm); font-size: 11px;">
            {#if stashList.length === 0}
              <div style="color: var(--text-muted); text-align: center; padding: 20px;">Tidak ada stash tersimpan</div>
            {:else}
              {#each stashList as s (s)}
                <!-- svelte-ignore a11y_click_events_have_key_events -->
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <div
                  onclick={() => selectStash(s)}
                  style="padding: 6px 8px; border-bottom: 1px solid var(--border-muted); cursor: pointer; display: flex; justify-content: space-between; align-items: center; background: {selectedStash === s ? 'rgba(56, 139, 253, 0.15)' : 'transparent'}; font-family: var(--font-mono); font-size: 11px;"
                >
                  <span style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap; flex: 1;" title={s}>{s}</span>
                  <button
                    class="btn-secondary"
                    style="font-size: 9px; padding: 1px 4px; color: var(--diff-del-text); border: none; margin-left: 4px;"
                    title="Hapus stash ini (Drop)"
                    onclick={(e) => { e.stopPropagation(); handleDropStash(s); }}
                  >
                    x
                  </button>
                </div>
              {/each}
            {/if}
          </div>
        </div>

        <!-- Right: Stash diff preview -->
        <div style="display: flex; flex-direction: column; overflow: hidden;">
          <span style="font-size: 11px; color: var(--text-secondary); font-weight: 600; margin-bottom: 6px;">Pratinjau Perubahan (Diff)</span>
          <div style="flex: 1; overflow-y: auto; background: var(--bg-canvas); border: 1px solid var(--border-default); border-radius: var(--radius-sm); padding: 8px; font-family: var(--font-mono); font-size: 11px; white-space: pre-wrap;">
            {#if isLoadingStashDiff}
              <div style="color: var(--text-muted); text-align: center; padding: 24px;">Memuat diff stash...</div>
            {:else if stashDiff}
              {stashDiff}
            {:else}
              <div style="color: var(--text-muted); text-align: center; padding: 24px;">Pilih entri stash di sebelah kiri untuk melihat isi perubahannya</div>
            {/if}
          </div>
        </div>
      </div>

      <div class="modal-actions" style="margin-top: 12px;">
        <button class="btn-secondary" onclick={closeModal}>Tutup</button>
      </div>
    </div>
  </div>
{:else if activeModal === "diag"}
  <div class="modal-overlay">
    <div class="modal-dialog" style="width: 500px; max-height: 80vh; display: flex; flex-direction: column;">
      <div class="modal-header">Laporan Diagnostik Sistem</div>
      <div style="overflow-y: auto; flex: 1; font-size: 11px; display: flex; flex-direction: column; gap: 8px;">
        <div><strong>Aplikasi:</strong> Forge Desktop v{diagReport?.app_version}</div>
        <div><strong>OS / Arch:</strong> {diagReport?.os_info}</div>
        <div><strong>Versi Git:</strong> {diagReport?.git_version}</div>
        <div><strong>Konfigurasi Git (Aman / Redacted):</strong></div>
        <pre style="background: var(--bg-subtle); padding: 8px; border-radius: var(--radius-sm); max-height: 180px; overflow-y: auto; font-family: var(--font-mono); font-size: 10px;">{diagReport?.sanitized_config.join("\n") || "Tidak ada config"}</pre>
      </div>
      <div class="modal-actions" style="margin-top: 12px;">
        <button
          class="btn-secondary"
          onclick={() => {
            if (diagReport) {
              const text = `App: Forge Desktop v${diagReport.app_version}\nOS: ${diagReport.os_info}\nGit: ${diagReport.git_version}\nConfig:\n${diagReport.sanitized_config.join("\n")}`;
              navigator.clipboard.writeText(text);
              setStatus("Diagnostik disalin ke clipboard");
            }
          }}
        >
          Salin Info
        </button>
        <button class="btn-primary" onclick={closeModal}>Tutup</button>
      </div>
    </div>
  </div>
{:else if activeModal === "auth"}
  <div class="modal-overlay">
    <div class="modal-dialog" style="width: 420px;">
      <div class="modal-header">Masuk dengan GitHub</div>
      {#if !deviceFlow}
        <p style="font-size: 12px; color: var(--text-secondary); margin-bottom: 8px;">
          Gunakan Device Code Authorization untuk menghubungkan repositori ke akun GitHub Anda.
        </p>
        <span style="font-size: 11px; color: var(--text-secondary);">GitHub OAuth Client ID</span>
        <input type="text" class="commit-input" bind:value={oauthClientId} placeholder="Client ID dari GitHub Developer" />
        <div style="margin-top: 12px; display: flex; justify-content: flex-end;">
          <button onclick={handleStartDeviceFlow} class="btn-primary" style="font-size: 12px;">Minta Kode Perangkat</button>
        </div>
      {:else}
        <div style="display: flex; flex-direction: column; gap: 10px; font-size: 12px;">
          <div>Buka link berikut di browser:</div>
          <a href={deviceFlow.verification_uri} target="_blank" rel="noreferrer" style="color: var(--accent-blue); font-family: var(--font-mono);">{deviceFlow.verification_uri}</a>
          <div>Dan masukkan kode perangkat ini:</div>
          <div style="font-size: 20px; font-weight: 700; font-family: var(--font-mono); text-align: center; padding: 10px; background: var(--bg-subtle); border-radius: var(--radius-sm); letter-spacing: 2px; color: var(--accent-blue);">
            {deviceFlow.user_code}
          </div>
          <div
            style="text-align: center; font-size: 11px; color: {pollStatusClass === 'success' ? 'var(--diff-add-text)' : pollStatusClass === 'error' ? 'var(--diff-del-text)' : 'var(--text-muted)'}; font-weight: {pollStatusClass ? '600' : 'normal'};"
          >
            {pollStatus}
          </div>
        </div>
      {/if}
      <div class="modal-actions" style="margin-top: 8px;">
        <button class="btn-secondary" onclick={closeModal}>Tutup</button>
      </div>
    </div>
  </div>
{/if}

{#if activeModal === "tag"}
  <div class="modal-overlay">
    <div class="modal-dialog" style="width: 400px;">
      <div class="modal-header">Buat Git Tag</div>
      <div style="font-size: 12px; color: var(--text-secondary); margin-bottom: 8px;">
        Target komit: <span style="font-family: var(--font-mono); font-weight: 600;">{tagModalTargetSha?.slice(0, 7) || "HEAD"}</span>
      </div>
      <input
        type="text"
        class="commit-input"
        placeholder="v1.0.0 atau release-2026"
        bind:value={tagNameInput}
        style="margin-bottom: 12px;"
      />
      <div class="modal-actions">
        <button onclick={closeModal} class="btn-secondary">Batal</button>
        <button
          onclick={handleCreateTagSubmit}
          class="btn-primary"
          disabled={!tagNameInput.trim()}
        >
          Buat Tag
        </button>
      </div>
    </div>
  </div>
{/if}

{#if activeModal === "pr"}
  <div class="modal-overlay">
    <div class="modal-dialog" style="width: 550px; max-height: 80vh; display: flex; flex-direction: column;">
      <div class="modal-header" style="display: flex; justify-content: space-between; align-items: center;">
        <span>GitHub: {githubRepoInfo ? `${githubRepoInfo.owner}/${githubRepoInfo.repo}` : "Repository"}</span>
        <button
          onclick={handleCreatePR}
          class="btn-primary"
          style="font-size: 11px; padding: 2px 8px;"
          title="Buka browser untuk membuat Pull Request"
        >
          + Buat PR di Browser
        </button>
      </div>

      <!-- Tab toggle: Pull Requests vs Issues -->
      <div style="display: flex; gap: 8px; border-bottom: 1px solid var(--border-default); margin-bottom: 8px;">
        <button
          onclick={() => { prTab = "prs"; loadGithubData(); }}
          style="background: none; border: none; border-bottom: 2px solid {prTab === 'prs' ? 'var(--accent-blue)' : 'transparent'}; color: {prTab === 'prs' ? 'var(--text-primary)' : 'var(--text-muted)'}; font-weight: {prTab === 'prs' ? '600' : 'normal'}; padding: 6px 12px; cursor: pointer; font-size: 12px;"
        >
          Pull Requests ({pullRequests.length})
        </button>
        <button
          onclick={() => { prTab = "issues"; loadGithubData(); }}
          style="background: none; border: none; border-bottom: 2px solid {prTab === 'issues' ? 'var(--accent-blue)' : 'transparent'}; color: {prTab === 'issues' ? 'var(--text-primary)' : 'var(--text-muted)'}; font-weight: {prTab === 'issues' ? '600' : 'normal'}; padding: 6px 12px; cursor: pointer; font-size: 12px;"
        >
          Issues ({issues.length})
        </button>
      </div>

      {#if isLoadingGithub}
        <div style="padding: 24px; text-align: center; color: var(--text-muted); font-size: 12px;">
          Memuat data GitHub...
        </div>
      {:else if prTab === "prs"}
        <div style="overflow-y: auto; max-height: 350px; display: flex; flex-direction: column; gap: 4px;">
          {#if pullRequests.length === 0}
            <div style="padding: 20px; text-align: center; color: var(--text-muted); font-size: 12px;">
              Tidak ada Pull Request yang sedang terbuka.
            </div>
          {:else}
            {#each pullRequests as pr (pr.id)}
              <div
                style="padding: 8px 10px; border: 1px solid var(--border-muted); border-radius: var(--radius-sm); display: flex; justify-content: space-between; align-items: center; font-size: 12px;"
              >
                <div style="flex: 1; min-width: 0; margin-right: 8px;">
                  <div style="font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;">
                    <span style="color: var(--accent-blue);">#{pr.number}</span> {pr.title}
                  </div>
                  <div style="font-size: 10px; color: var(--text-muted);">
                    oleh {pr.author} • {pr.head_branch} &rarr; {pr.base_branch}
                  </div>
                </div>
                <div style="display: flex; gap: 4px;">
                  <button
                    onclick={() => handleCheckoutPR(pr.number)}
                    class="btn-primary"
                    style="font-size: 10px; padding: 2px 8px;"
                    title="Checkout branch PR ini ke lokal"
                  >
                    Checkout
                  </button>
                  <a
                    href={pr.html_url}
                    target="_blank"
                    rel="noreferrer"
                    class="btn-secondary"
                    style="font-size: 10px; padding: 2px 6px; text-decoration: none;"
                    title="Buka PR di github.com"
                  >
                    Web
                  </a>
                </div>
              </div>
            {/each}
          {/if}
        </div>
      {:else}
        <div style="overflow-y: auto; max-height: 350px; display: flex; flex-direction: column; gap: 4px;">
          {#if issues.length === 0}
            <div style="padding: 20px; text-align: center; color: var(--text-muted); font-size: 12px;">
              Tidak ada Issue yang sedang terbuka.
            </div>
          {:else}
            {#each issues as issue (issue.id)}
              <div
                style="padding: 8px 10px; border: 1px solid var(--border-muted); border-radius: var(--radius-sm); display: flex; justify-content: space-between; align-items: center; font-size: 12px;"
              >
                <div style="flex: 1; min-width: 0; margin-right: 8px;">
                  <div style="font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;">
                    <span style="color: var(--diff-add-text);">#{issue.number}</span> {issue.title}
                  </div>
                  <div style="font-size: 10px; color: var(--text-muted);">
                    oleh {issue.author} • {issue.comments_count} komentar
                  </div>
                </div>
                <a
                  href={issue.html_url}
                  target="_blank"
                  rel="noreferrer"
                  class="btn-secondary"
                  style="font-size: 10px; padding: 2px 6px; text-decoration: none;"
                  title="Buka issue di github.com"
                >
                  Web
                </a>
              </div>
            {/each}
          {/if}
        </div>
      {/if}

      <div class="modal-actions" style="margin-top: 12px;">
        <button onclick={closeModal} class="btn-secondary">Tutup</button>
      </div>
    </div>
  </div>
{/if}

{#if activeModal === "squash"}
  <div class="modal-overlay">
    <div class="modal-dialog" style="width: 440px;">
      <div class="modal-header">Squash {squashCount} Komit</div>
      <div style="font-size: 12px; color: var(--text-secondary); margin-bottom: 8px;">
        {squashCount} komit dari HEAD akan digabung menjadi satu komit baru.
      </div>
      <div style="display: flex; flex-direction: column; gap: 8px;">
        <span style="font-size: 11px; color: var(--text-secondary);">Ringkasan Komit Baru (Summary)</span>
        <input
          type="text"
          class="commit-input"
          placeholder="Ringkasan komit hasil squash"
          bind:value={squashSummary}
        />
        <span style="font-size: 11px; color: var(--text-secondary);">Deskripsi Komit (Opsional)</span>
        <textarea
          class="commit-input"
          placeholder="Rincian deskripsi gabungan"
          rows="3"
          bind:value={squashDescription}
          style="resize: vertical; font-size: 12px;"
        ></textarea>
      </div>
      <div class="modal-actions" style="margin-top: 14px;">
        <button class="btn-secondary" onclick={closeModal}>Batal</button>
        <button
          class="btn-primary"
          onclick={handleSquashSubmit}
          disabled={!squashSummary.trim()}
        >
          Konfirmasi Squash
        </button>
      </div>
    </div>
  </div>
{/if}

{#if activeModal === "confirm"}
  <div class="modal-overlay">
    <div class="modal-dialog" style="width: 420px;">
      <div class="modal-header" style="color: var(--diff-del-text);">{confirmModalTitle}</div>
      <div style="font-size: 12px; color: var(--text-secondary); margin-bottom: 16px; line-height: 1.4;">
        {confirmModalMessage}
      </div>
      <div class="modal-actions">
        <button class="btn-secondary" onclick={closeModal}>Batal</button>
        <button
          class="btn-primary"
          style="background: var(--diff-del-text); border-color: var(--diff-del-text); color: #fff;"
          onclick={handleConfirmModalSubmit}
        >
          Ya, Lanjutkan
        </button>
      </div>
    </div>
  </div>
{/if}

{#if activeModal === "publish"}
  <div class="modal-overlay">
    <div class="modal-dialog" style="width: 440px;">
      <div class="modal-header">Publikasikan Repositori ke GitHub</div>
      <div style="font-size: 12px; color: var(--text-secondary); margin-bottom: 8px;">
        Repositori lokal ini belum terhubung ke remote. Buat repositori baru di akun GitHub Anda dan lakukan push branch utama.
      </div>
      <div style="display: flex; flex-direction: column; gap: 8px;">
        <span style="font-size: 11px; color: var(--text-secondary);">Nama Repositori</span>
        <input
          type="text"
          class="commit-input"
          placeholder="Nama repositori"
          bind:value={publishName}
        />
        <span style="font-size: 11px; color: var(--text-secondary);">Deskripsi (Opsional)</span>
        <textarea
          class="commit-input"
          placeholder="Deskripsi repositori GitHub"
          rows="2"
          bind:value={publishDesc}
          style="resize: vertical; font-size: 12px;"
        ></textarea>
        <label style="display: flex; align-items: center; gap: 8px; font-size: 11px; color: var(--text-primary); cursor: pointer; margin-top: 4px;">
          <input type="checkbox" bind:checked={publishPrivate} />
          <span>Buat sebagai repositori privat (Private Repository)</span>
        </label>
      </div>
      <div class="modal-actions" style="margin-top: 14px;">
        <button class="btn-secondary" onclick={closeModal}>Batal</button>
        <button
          class="btn-primary"
          onclick={handlePublishSubmit}
          disabled={!publishName.trim()}
        >
          Publikasikan
        </button>
      </div>
    </div>
  </div>
{/if}
