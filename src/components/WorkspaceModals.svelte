<script lang="ts">
  import { onDestroy, untrack } from "svelte";
  import * as gitService from "../services/git";
  import type {
    AccountHealth,
    AccountOverview,
    AccountProfile,
    AppRuntimeInfo,
    AppUpdateInfo,
    DetectedSession,
    DeviceFlowInit,
    DiagnosticReport,
    LocalSessionSource,
    ParsedGitHubRepo,
    RemoteInfo,
    RepoConfig,
    UpdateChannel,
  } from "../types/git";
  import {
    deriveRepoNameFromUrl,
    joinRepoPath,
    openExternalUrl,
    pathBaseName,
    pickDirectory,
    pickFile,
  } from "../utils/path";

  export type WorkspaceModalType =
    | "open"
    | "init"
    | "clone"
    | "publish"
    | "remote"
    | "stash"
    | "diag"
    | "auth"
    | "settings";
  interface Props {
    modal: WorkspaceModalType;
    activeRepo: string;
    recentRepos: string[];
    repoAliases: Record<string, string>;
    repoMissingStatus: Record<string, boolean>;
    diagReport: DiagnosticReport | null;
    repoConfig: RepoConfig;
    globalConfig: RepoConfig;
    activeTheme: string;
    autofetchMinutes: number;
    confirmDiscard: boolean;
    confirmForcePush: boolean;
    desktopNotifEnabled: boolean;
    externalEditor: string;
    appRuntimeInfo: AppRuntimeInfo | null;
    appUpdateInfo: AppUpdateInfo | null;
    updateChannel: UpdateChannel;
    isCheckingUpdate: boolean;
    isInstallingUpdate: boolean;
    updateError: string;
    accountOverview: AccountOverview;
    activeRepoAccount: AccountProfile | null;
    githubRepoInfo: ParsedGitHubRepo | null;
    accountHealthMap: Record<string, AccountHealth>;
    remotes: RemoteInfo[];
    hasCommits: boolean;
    onClose: () => void;
    onLoadRepo: (path: string) => Promise<void>;
    onRemotesChanged: () => Promise<void>;
    onRelocateRepo: (oldPath: string, newPath: string) => Promise<void>;
    onSetAlias: (path: string, alias: string) => void;
    onRemoveRecent: (path: string) => void;
    onSaveSettings: (cfg: {
      repoConfig: RepoConfig;
      globalConfig: RepoConfig;
      activeTheme: string;
      autofetchMinutes: number;
      confirmDiscard: boolean;
      confirmForcePush: boolean;
      desktopNotifEnabled: boolean;
      externalEditor: string;
      updateChannel: UpdateChannel;
    }) => Promise<void>;
    onCheckUpdates: (channel?: UpdateChannel) => Promise<void>;
    onInstallUpdate: (channel?: UpdateChannel) => Promise<void>;
    onStatus: (msg: string, isError?: boolean) => void;
    onAccountsChanged: (overview: AccountOverview, syncedProfile?: AccountProfile) => void;
    onCheckAccountHealth: (accountId: string) => Promise<AccountHealth>;
    onSyncRepoAuthor: (name: string, email: string) => Promise<void>;
    onOpenAuth?: () => void;
    onRequestConfirm: (
      title: string,
      message: string,
      action: () => Promise<void>,
      danger?: boolean
    ) => void;
  }

  let {
    modal,
    activeRepo,
    recentRepos,
    repoAliases,
    repoMissingStatus,
    diagReport,
    repoConfig: initialRepoConfig,
    globalConfig: initialGlobalConfig,
    activeTheme: initialTheme,
    autofetchMinutes: initialAutofetch,
    confirmDiscard: initialConfirmDiscard,
    confirmForcePush: initialConfirmForcePush,
    desktopNotifEnabled: initialDesktopNotif,
    externalEditor: initialExternalEditor,
    appRuntimeInfo,
    appUpdateInfo,
    updateChannel: initialUpdateChannel,
    isCheckingUpdate,
    isInstallingUpdate,
    updateError,
    accountOverview,
    activeRepoAccount,
    githubRepoInfo,
    accountHealthMap,
    remotes,
    hasCommits,
    onClose,
    onLoadRepo,
    onRemotesChanged,
    onRelocateRepo,
    onSetAlias,
    onRemoveRecent,
    onSaveSettings,
    onCheckUpdates,
    onInstallUpdate,
    onStatus,
    onAccountsChanged,
    onCheckAccountHealth,
    onSyncRepoAuthor,
    onOpenAuth,
    onRequestConfirm,
  }: Props = $props();

  let inlineError = $state("");

  // Open modal states
  let openRepoPath = $state("");
  let editingAliasFor = $state<string | null>(null);
  let aliasInput = $state("");
  let relocatingPathFor = $state<string | null>(null);
  let relocateInput = $state("");

  // Init modal states
  let initRepoPath = $state("");
  let initDefaultBranch = $state("main");
  let initGitignore = $state("Node");
  let initLicense = $state("MIT");
  let initRemoteUrl = $state("");

  // Clone modal states
  let cloneUrl = $state("");
  let cloneTarget = $state("");
  let cloneAccountId = $state("");

  // Publish modal states
  let publishName = $state("");
  let publishDesc = $state("");
  let publishPrivate = $state(false);
  let publishAccountId = $state("");

  // Remote modal states
  let newRemoteName = $state("origin");
  let newRemoteUrl = $state("");
  let editingRemoteName = $state<string | null>(null);
  let editingRemoteUrl = $state("");
  let isSubmittingRemote = $state(false);
  let stashMsg = $state("");
  let stashList = $state<string[]>([]);
  let selectedStash = $state<string | null>(null);
  let stashDiff = $state("");
  let isLoadingStashDiff = $state(false);

  // Settings local drafts
  let draftRepoConfig = $state<RepoConfig>({ user_name: "", user_email: "" });
  let draftGlobalConfig = $state<RepoConfig>({ user_name: "", user_email: "" });
  let draftTheme = $state("dark");
  let draftAutofetch = $state(5);
  let draftConfirmDiscard = $state(true);
  let draftConfirmForcePush = $state(true);
  let draftDesktopNotif = $state(true);
  let draftEditorPreset = $state("system");
  let draftCustomEditor = $state("");
  let draftUpdateChannel = $state<UpdateChannel>("auto");
  const EXTERNAL_EDITOR_PRESETS = [
    { value: "system", label: "Default Sistem OS (Aplikasi Bawaan)" },
    { value: "code", label: "Visual Studio Code (code)" },
    { value: "cursor", label: "Cursor (cursor)" },
    { value: "windsurf", label: "Windsurf (windsurf)" },
    { value: "zed", label: "Zed (zed)" },
    { value: "subl", label: "Sublime Text (subl)" },
    { value: "notepad++", label: "Notepad++ (notepad++)" },
    { value: "notepad", label: "Notepad (Windows)" },
    { value: "gedit", label: "GNOME Text Editor / gedit" },
    { value: "kate", label: "Kate" },
    { value: "custom", label: "Kustom (Perintah / Path Aplikasi)" },
  ];

  async function handlePickCustomEditor() {
    try {
      const picked = await pickFile({ title: "Pilih Aplikasi Text Editor Eksternal" });
      if (picked) {
        draftCustomEditor = picked;
      }
    } catch (err) {
      inlineError = String(err);
    }
  }
  // Multi-account Auth states
  let authView = $state<"manager" | "add">("manager");
  let hostKind = $state<"github.com" | "enterprise">("github.com");
  let enterpriseHostInput = $state("");
  let accountLabelInput = $state("");
  let authTab = $state<"token" | "oauth" | "import">("token");
  let patInput = $state("");
  let showPat = $state(false);
  let isVerifyingAuth = $state(false);
  let editingLabelFor = $state<string | null>(null);
  let labelDraft = $state("");
  let checkingHealthFor = $state<string | null>(null);
  let oauthClientId = $state(localStorage.getItem("forge_oauth_client_id:github.com") || "");
  let deviceFlow = $state<DeviceFlowInit | null>(null);
  let deviceExpiresLeft = $state(0);
  let copiedCode = $state(false);
  let pollStatus = $state("");
  let pollStatusClass = $state<"" | "success" | "error">("");
  let detectedSessions = $state<DetectedSession[]>([]);
  let isDetectingSessions = $state(false);
  let keyringUnavailableAction = $state<(() => Promise<void>) | null>(null);
  let pendingSsoUrl = $state<string | null>(null);
  let pollTimer: number | null = null;
  let countdownTimer: number | null = null;

  const effectiveHost = $derived.by(() => {
    if (hostKind === "github.com") return "github.com";
    return (
      enterpriseHostInput
        .trim()
        .replace(/^https?:\/\//i, "")
        .replace(/\/+$/, "")
        .replace(/\/api\/v3$/i, "")
        .toLowerCase() || ""
    );
  });

  const authorMismatch = $derived.by(() => {
    if (!activeRepo || !activeRepoAccount) return null;
    const targetName = activeRepoAccount.name || activeRepoAccount.login;
    const targetEmail = activeRepoAccount.email || "";
    if (!targetEmail && !targetName) return null;
    const diffName = targetName && initialRepoConfig.user_name.trim() !== targetName;
    const diffEmail = targetEmail && initialRepoConfig.user_email.trim() !== targetEmail;
    if (!diffName && !diffEmail) return null;
    return {
      name: targetName,
      email: targetEmail || initialRepoConfig.user_email.trim(),
    };
  });
  function clearPollTimer() {
    if (pollTimer !== null) {
      clearInterval(pollTimer);
      pollTimer = null;
    }
    if (countdownTimer !== null) {
      clearInterval(countdownTimer);
      countdownTimer = null;
    }
  }

  onDestroy(() => {
    clearPollTimer();
  });

  $effect(() => {
    const m = modal;
    untrack(() => {
      inlineError = "";
      if (m === "open") {
        openRepoPath = activeRepo;
        editingAliasFor = null;
        relocatingPathFor = null;
      } else if (m === "init") {
        initRemoteUrl = "";
      } else if (m === "clone") {
        cloneAccountId = "";
      } else if (m === "remote") {
        const list = remotes ?? [];
        newRemoteName = list.some((r) => r.name === "origin") ? "" : "origin";
        newRemoteUrl = "";
        editingRemoteName = null;
        editingRemoteUrl = "";
      } else if (m === "publish") {
        publishName = pathBaseName(activeRepo);
        publishDesc = "";
        publishPrivate = false;
        publishAccountId =
          activeRepoAccount?.id ||
          accountOverview.primary_account_id ||
          accountOverview.accounts[0]?.id ||
          "";
      } else if (m === "stash") {
        stashMsg = "";
        selectedStash = null;
        stashDiff = "";
        if (activeRepo) {
          gitService
            .gitListStashes(activeRepo)
            .then((list) => (stashList = list))
            .catch((err) => (inlineError = String(err)));
        }
      } else if (m === "settings") {
        draftRepoConfig = { ...initialRepoConfig };
        draftGlobalConfig = { ...initialGlobalConfig };
        draftTheme = initialTheme;
        draftAutofetch = initialAutofetch;
        draftConfirmDiscard = initialConfirmDiscard;
        draftConfirmForcePush = initialConfirmForcePush;
        draftDesktopNotif = initialDesktopNotif;
        const savedEditor = (initialExternalEditor || "system").trim();
        const isPreset = EXTERNAL_EDITOR_PRESETS.some(
          (p) => p.value !== "custom" && p.value === savedEditor
        );
        if (isPreset) {
          draftEditorPreset = savedEditor;
          draftCustomEditor = "";
        } else {
          draftEditorPreset = "custom";
          draftCustomEditor = savedEditor;
        }
        draftUpdateChannel = initialUpdateChannel;
      } else if (m === "auth") {
        clearPollTimer();
        authView = accountOverview.accounts.length > 0 ? "manager" : "add";
        hostKind = "github.com";
        enterpriseHostInput = "";
        accountLabelInput = "";
        patInput = "";
        showPat = false;
        isVerifyingAuth = false;
        editingLabelFor = null;
        labelDraft = "";
        deviceFlow = null;
        deviceExpiresLeft = 0;
        copiedCode = false;
        pollStatus = "";
        pollStatusClass = "";
        keyringUnavailableAction = null;
        pendingSsoUrl = null;
        oauthClientId = localStorage.getItem("forge_oauth_client_id:github.com") || "";
        if (authView === "add" && authTab === "import") {
          refreshDetectedSessions();
        }
      }
    });
  });

  let isBrowsingDir = $state(false);

  async function browseDirectory(
    currentValue: string,
    title: string,
    canCreateDirectories = false
  ): Promise<string | null> {
    if (isBrowsingDir) return null;
    inlineError = "";
    isBrowsingDir = true;
    try {
      return await pickDirectory({
        title,
        defaultPath: currentValue.trim() || activeRepo || undefined,
        canCreateDirectories,
      });
    } catch (err) {
      inlineError = err instanceof Error ? err.message : String(err);
      return null;
    } finally {
      isBrowsingDir = false;
    }
  }

  function extractStashRef(stashLine: string): string | undefined {
    const m = stashLine.match(/^stash@\{[0-9]+\}/);
    return m ? m[0] : undefined;
  }

  async function selectStash(stashLine: string) {
    selectedStash = stashLine;
    const ref = extractStashRef(stashLine);
    isLoadingStashDiff = true;
    try {
      stashDiff = await gitService.gitStashShow(activeRepo, ref);
    } catch (err) {
      stashDiff = `Gagal memuat diff stash: ${err}`;
    } finally {
      isLoadingStashDiff = false;
    }
  }

  function handleDropStash(stashLine: string) {
    const ref = extractStashRef(stashLine);
    onRequestConfirm(
      "Hapus Stash",
      `Apakah Anda yakin ingin menghapus ${ref || stashLine}? Perubahan di stash ini akan hilang secara permanen.`,
      async () => {
        try {
          await gitService.gitStashDrop(activeRepo, ref);
          onStatus(`Stash ${ref || stashLine} berhasil dihapus`);
          stashList = await gitService.gitListStashes(activeRepo);
          selectedStash = null;
          stashDiff = "";
        } catch (err) {
          onStatus(String(err), true);
        }
      },
      true
    );
  }

  async function handleStashSave() {
    onClose();
    try {
      await gitService.gitStashSave(activeRepo, stashMsg.trim() || undefined);
      onStatus("Perubahan berhasil disimpan ke Stash");
      await onLoadRepo(activeRepo);
    } catch (err) {
      onStatus(String(err), true);
    }
  }

  async function handleStashPop() {
    onClose();
    try {
      await gitService.gitStashPop(activeRepo);
      onStatus("Stash teratas diterapkan ke working tree");
      await onLoadRepo(activeRepo);
    } catch (err) {
      onStatus(String(err), true);
    }
  }

  async function handleClone() {
    const url = cloneUrl.trim();
    const target = cloneTarget.trim();
    if (!url || !target) {
      inlineError = "URL repositori dan folder tujuan wajib diisi.";
      return;
    }
    onClose();
    try {
      onStatus(`Cloning ${url}...`);
      await gitService.cloneRepository(url, target, cloneAccountId || undefined);
      onStatus("Clone sukses");
      await onLoadRepo(target);
    } catch (err) {
      onStatus(String(err), true);
    }
  }

  async function handleInitRepo() {
    const target = initRepoPath.trim();
    if (!target) {
      inlineError = "Path folder repositori wajib diisi.";
      return;
    }
    onClose();
    try {
      onStatus(`Menginisialisasi repositori baru di ${target}...`);
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
      const remoteUrl = initRemoteUrl.trim();
      if (remoteUrl) {
        await gitService.addRemote(target, "origin", remoteUrl);
      }
      onStatus(remoteUrl ? "Repositori dibuat dan remote origin ditautkan" : "Repositori berhasil dibuat");
      await onLoadRepo(target);
    } catch (err) {
      onStatus(String(err), true);
    }
  }

  async function handleAddRemote() {
    const name = newRemoteName.trim();
    const url = newRemoteUrl.trim();
    if (!name || !url) {
      inlineError = "Nama remote dan URL wajib diisi.";
      return;
    }
    inlineError = "";
    isSubmittingRemote = true;
    try {
      const msg = await gitService.addRemote(activeRepo, name, url);
      newRemoteName = "";
      newRemoteUrl = "";
      await onRemotesChanged();
      onStatus(msg);
    } catch (err) {
      inlineError = String(err);
    } finally {
      isSubmittingRemote = false;
    }
  }

  async function handleSaveRemoteUrl(name: string) {
    const url = editingRemoteUrl.trim();
    if (!url) {
      inlineError = "URL remote tidak boleh kosong.";
      return;
    }
    inlineError = "";
    isSubmittingRemote = true;
    try {
      const msg = await gitService.setRemoteUrl(activeRepo, name, url);
      editingRemoteName = null;
      editingRemoteUrl = "";
      await onRemotesChanged();
      onStatus(msg);
    } catch (err) {
      inlineError = String(err);
    } finally {
      isSubmittingRemote = false;
    }
  }

  function handleRemoveRemote(name: string) {
    onRequestConfirm(
      "Hapus Remote",
      `Hapus remote '${name}' dari repositori ini? Branch lokal tetap aman, hanya tautan ke server remote yang dilepas.`,
      async () => {
        inlineError = "";
        isSubmittingRemote = true;
        try {
          const msg = await gitService.removeRemote(activeRepo, name);
          if (editingRemoteName === name) editingRemoteName = null;
          if (!newRemoteName.trim() && (remotes ?? []).filter((r) => r.name !== name).length === 0) {
            newRemoteName = "origin";
          }
          await onRemotesChanged();
          onStatus(msg);
        } catch (err) {
          inlineError = String(err);
        } finally {
          isSubmittingRemote = false;
        }
      },
      true
    );
  }

  async function handlePublishSubmit() {
    if (!publishName.trim()) {
      inlineError = "Nama repositori wajib diisi.";
      return;
    }
    onClose();
    const targetAcc = accountOverview.accounts.find((a) => a.id === publishAccountId);
    try {
      onStatus(`Mempublikasikan repositori ${publishName} ke GitHub...`);
      const cloneUrlResult = await gitService.publishRepositoryToGithub(
        activeRepo,
        {
          name: publishName.trim(),
          description: publishDesc.trim() || undefined,
          private: publishPrivate,
        },
        publishAccountId || undefined,
        targetAcc?.host
      );
      onStatus(`Repositori dipublikasikan ke GitHub: ${cloneUrlResult}`);
      await onLoadRepo(activeRepo);
    } catch (err) {
      onStatus(String(err), true);
    }
  }

  function parseAuthError(rawErr: unknown, retryWithMemoryFallback: () => Promise<void>) {
    const msg = String(rawErr);
    keyringUnavailableAction = null;
    pendingSsoUrl = null;
    if (msg.includes("[KEYRING_UNAVAILABLE]")) {
      inlineError = msg.replace("[KEYRING_UNAVAILABLE]", "").trim();
      keyringUnavailableAction = retryWithMemoryFallback;
      return;
    }
    const ssoMatch = msg.match(/\[SSO_REQUIRED:([^\]]*)\]\s*(.*)/);
    if (ssoMatch) {
      pendingSsoUrl = ssoMatch[1] || `https://${effectiveHost || "github.com"}/settings/tokens`;
      inlineError = ssoMatch[2] || "Organisasi mewajibkan otorisasi SAML SSO untuk token ini.";
      return;
    }
    inlineError = msg.replace("[AUTH_EXPIRED]", "").trim();
  }

  async function refreshDetectedSessions() {
    const h = effectiveHost || "github.com";
    isDetectingSessions = true;
    try {
      detectedSessions = await gitService.detectLocalGithubSessions(h);
    } catch {
      detectedSessions = [];
    } finally {
      isDetectingSessions = false;
    }
  }

  function openAddAccountSheet(prefillHost?: string, prefillLabel?: string | null) {
    clearPollTimer();
    inlineError = "";
    keyringUnavailableAction = null;
    pendingSsoUrl = null;
    deviceFlow = null;
    pollStatus = "";
    patInput = "";
    showPat = false;
    if (prefillHost && prefillHost !== "github.com") {
      hostKind = "enterprise";
      enterpriseHostInput = prefillHost;
    } else {
      hostKind = "github.com";
      enterpriseHostInput = "";
    }
    accountLabelInput = prefillLabel || "";
    const targetHost = prefillHost || "github.com";
    oauthClientId =
      localStorage.getItem(`forge_oauth_client_id:${targetHost}`) ||
      localStorage.getItem("forge_oauth_client_id") ||
      "";
    authView = "add";
    if (authTab === "import") {
      refreshDetectedSessions();
    }
  }

  async function handleConnectPat(allowMemoryFallback = false) {
    const tok = patInput.trim();
    const host = effectiveHost;
    if (!host) {
      inlineError = "Domain GitHub Enterprise wajib diisi.";
      return;
    }
    if (!tok) {
      inlineError = "Personal Access Token wajib diisi.";
      return;
    }
    inlineError = "";
    keyringUnavailableAction = null;
    pendingSsoUrl = null;
    isVerifyingAuth = true;
    try {
      const profile = await gitService.verifyAndConnectPat(
        host,
        tok,
        accountLabelInput.trim() || undefined,
        allowMemoryFallback
      );
      const overview = await gitService.listGithubAccounts(activeRepo || undefined);
      patInput = "";
      onAccountsChanged(overview, profile);
      authView = "manager";
      onStatus(
        profile.memory_only
          ? `Terhubung sementara (sesi memori) sebagai @${profile.login}`
          : `Terhubung sebagai @${profile.login} (${profile.host})`
      );
    } catch (err) {
      parseAuthError(err, () => handleConnectPat(true));
    } finally {
      isVerifyingAuth = false;
    }
  }

  async function handleImportSession(source: LocalSessionSource, allowMemoryFallback = false) {
    const host = effectiveHost;
    if (!host) {
      inlineError = "Domain host wajib diisi.";
      return;
    }
    inlineError = "";
    keyringUnavailableAction = null;
    pendingSsoUrl = null;
    isVerifyingAuth = true;
    try {
      const profile = await gitService.importLocalGithubSession(
        host,
        source,
        accountLabelInput.trim() || undefined,
        allowMemoryFallback
      );
      const overview = await gitService.listGithubAccounts(activeRepo || undefined);
      onAccountsChanged(overview, profile);
      authView = "manager";
      onStatus(`Berhasil mengimpor akun @${profile.login} (${profile.host})`);
    } catch (err) {
      parseAuthError(err, () => handleImportSession(source, true));
    } finally {
      isVerifyingAuth = false;
    }
  }

  async function handleBindAccount(accountId: string) {
    if (!activeRepo) return;
    inlineError = "";
    try {
      const overview = await gitService.bindAccountToRepo(activeRepo, accountId);
      onAccountsChanged(overview);
      onStatus("Akun aktif diperbarui untuk repositori ini");
    } catch (err) {
      inlineError = String(err);
    }
  }

  async function handleSetDefaultAccount(accountId: string) {
    inlineError = "";
    try {
      const overview = await gitService.setDefaultGithubAccount(
        accountId,
        activeRepo || undefined
      );
      onAccountsChanged(overview);
      onStatus("Akun default host diperbarui");
    } catch (err) {
      inlineError = String(err);
    }
  }

  async function handleSaveLabel(accountId: string) {
    inlineError = "";
    try {
      const overview = await gitService.updateGithubAccountLabel(
        accountId,
        labelDraft.trim() || undefined,
        activeRepo || undefined
      );
      editingLabelFor = null;
      onAccountsChanged(overview);
    } catch (err) {
      inlineError = String(err);
    }
  }

  async function handleCheckHealth(accountId: string) {
    checkingHealthFor = accountId;
    try {
      await onCheckAccountHealth(accountId);
    } finally {
      checkingHealthFor = null;
    }
  }

  function handleRemoveAccount(acc: AccountProfile) {
    onRequestConfirm(
      "Hapus Akun GitHub",
      `Hapus akun @${acc.login} (${acc.host}) dari penyimpanan kredensial? Binding repositori untuk akun ini juga akan dilepas.`,
      async () => {
        try {
          const overview = await gitService.removeGithubAccount(
            acc.id,
            activeRepo || undefined
          );
          onAccountsChanged(overview);
          onStatus(`Akun @${acc.login} (${acc.host}) dihapus`);
          if (overview.accounts.length === 0) {
            authView = "add";
          }
        } catch (err) {
          inlineError = String(err);
        }
      },
      true
    );
  }

  function formatCountdown(secs: number): string {
    const m = Math.floor(secs / 60);
    const s = secs % 60;
    return `${m}:${s.toString().padStart(2, "0")}`;
  }

  async function handleStartDeviceFlow(allowMemoryFallback = false) {
    const host = effectiveHost;
    const cid = oauthClientId.trim();
    if (!host) {
      inlineError = "Domain GitHub Enterprise wajib diisi.";
      return;
    }
    if (!cid) {
      inlineError = "GitHub OAuth Client ID wajib diisi.";
      return;
    }
    inlineError = "";
    keyringUnavailableAction = null;
    pendingSsoUrl = null;
    clearPollTimer();
    localStorage.setItem(`forge_oauth_client_id:${host}`, cid);
    try {
      pollStatus = "Meminta kode perangkat...";
      pollStatusClass = "";
      deviceFlow = await gitService.githubStartDeviceFlow(cid, host);
      deviceExpiresLeft = deviceFlow.expires_in || 900;
      pollStatus = "Menunggu konfirmasi otorisasi di browser...";
      openExternalUrl(deviceFlow.verification_uri);

      countdownTimer = window.setInterval(() => {
        if (deviceExpiresLeft > 0) {
          deviceExpiresLeft -= 1;
        } else {
          clearPollTimer();
          pollStatus = "Kode perangkat kedaluwarsa. Minta kode baru.";
          pollStatusClass = "error";
        }
      }, 1000);

      const sec = Math.max(deviceFlow.interval || 5, 5);
      pollTimer = window.setInterval(async () => {
        try {
          const res = await gitService.pollAndConnectDeviceFlow(
            host,
            cid,
            deviceFlow!.device_code,
            accountLabelInput.trim() || undefined,
            allowMemoryFallback
          );
          if (res.status === "authorized" && res.profile) {
            clearPollTimer();
            const overview = await gitService.listGithubAccounts(activeRepo || undefined);
            deviceFlow = null;
            deviceExpiresLeft = 0;
            onAccountsChanged(overview, res.profile);
            authView = "manager";
            onStatus(`Terhubung ke ${res.profile.host} sebagai @${res.profile.login}`);
          } else if (res.status === "error") {
            clearPollTimer();
            pollStatusClass = "error";
            pollStatus = res.error || "Otorisasi gagal";
            if (res.error?.includes("[KEYRING_UNAVAILABLE]")) {
              parseAuthError(res.error, () => handleStartDeviceFlow(true));
            }
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

  function copyDeviceCode() {
    if (!deviceFlow) return;
    navigator.clipboard?.writeText(deviceFlow.user_code);
    copiedCode = true;
    window.setTimeout(() => (copiedCode = false), 2000);
  }

  const modalTitle = $derived.by(() => {
    switch (modal) {
      case "open": return "Buka Repositori Git";
      case "init": return "Inisialisasi Repositori Baru";
      case "clone": return "Clone Repositori";
      case "publish": return "Publikasikan Repositori ke GitHub";
      case "remote": return "Kelola Remote Repositori";
      case "stash": return "Git Stash & Pratinjau Perubahan";
      case "diag": return "Laporan Diagnostik Sistem";
      case "auth":
        return authView === "add"
          ? "Tambah / Hubungkan Akun GitHub"
          : "Manajer Akun GitHub & Enterprise";
      case "settings": return "Pengaturan Repositori & Aplikasi";
    }
  });

  const modalWidth = $derived(
    modal === "stash"
      ? "720px"
      : modal === "open" || modal === "diag" || modal === "auth" || modal === "remote"
        ? "540px"
        : "480px"
  );

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
    class="modal-dialog"
    style="--modal-w: {modalWidth};"
    role="dialog"
    aria-modal="true"
    aria-labelledby="ws-modal-title"
  >
    <div class="modal-header">
      <span id="ws-modal-title">{modalTitle}</span>
      <button type="button" class="icon-btn compact" onclick={onClose} aria-label="Tutup dialog">
        <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
          <path d="M3.72 3.72a.75.75 0 0 1 1.06 0L8 6.94l3.22-3.22a.749.749 0 0 1 1.275.326.749.749 0 0 1-.215.734L9.06 8l3.22 3.22a.749.749 0 0 1-.326 1.275.749.749 0 0 1-.734-.215L8 9.06l-3.22 3.22a.751.751 0 0 1-1.042-.018.751.751 0 0 1-.018-1.042L6.94 8 3.72 4.78a.75.75 0 0 1 0-1.06Z" />
        </svg>
      </button>
    </div>

    {#if inlineError}
      <div class="alert-banner" style="border-radius: var(--radius-sm); border: 1px solid var(--danger-border);">
        <span>{inlineError}</span>
      </div>
    {/if}

    <div class="modal-body">
      {#if modal === "open"}
        <div class="form-field">
          <label class="form-label" for="open-repo-path">Path Folder Repositori</label>
          <div style="display: flex; gap: 6px; align-items: center;">
            <input
              id="open-repo-path"
              type="text"
              class="form-input"
              style="flex: 1;"
              bind:value={openRepoPath}
              placeholder="C:\Development\my-repo atau /path/ke/repo"
              onkeydown={(e) => {
                if (e.key === "Enter" && openRepoPath.trim()) {
                  onClose();
                  onLoadRepo(openRepoPath.trim());
                }
              }}
            />
            <button
              type="button"
              class="btn-secondary"
              disabled={isBrowsingDir}
              onclick={async () => {
                const picked = await browseDirectory(openRepoPath, "Pilih Folder Repositori Git", false);
                if (picked) openRepoPath = picked;
              }}
            >
              Browse...
            </button>
          </div>
        </div>

        {#if recentRepos.length > 0}
          <div class="form-field">
            <span class="form-label">Repositori Terakhir & Alias</span>
            <div class="list-box">
              {#each recentRepos as r (r)}
                {@const isMissing = repoMissingStatus[r]}
                {@const alias = repoAliases[r]}
                <div class="list-row {r === activeRepo ? 'is-active' : ''}" style="flex-direction: column; align-items: stretch; gap: 6px;">
                  <div style="display: flex; align-items: center; justify-content: space-between; gap: 8px;">
                    <button
                      type="button"
                      disabled={isMissing}
                      onclick={() => { openRepoPath = r; }}
                      style="flex: 1; min-width: 0; text-align: left;"
                    >
                      <div style="font-weight: 600; display: flex; align-items: center; gap: 6px; font-size: 12px;">
                        <span>{alias || pathBaseName(r) || r}</span>
                        {#if alias}
                          <span style="font-size: 10px; font-weight: 400; color: var(--text-muted); font-family: var(--font-mono);">
                            ({pathBaseName(r)})
                          </span>
                        {/if}
                        {#if isMissing}
                          <span style="font-size: 9px; padding: 1px 5px; border-radius: 3px; background: var(--danger-bg); color: var(--diff-del-text); font-weight: 700;">
                            HILANG
                          </span>
                        {/if}
                      </div>
                      <div style="font-size: 10px; color: var(--text-muted); font-family: var(--font-mono); overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
                        {r}
                      </div>
                    </button>

                    <div style="display: flex; gap: 4px; flex-shrink: 0;">
                      {#if isMissing}
                        <button
                          type="button"
                          class="btn-secondary btn-xs"
                          onclick={() => {
                            relocatingPathFor = relocatingPathFor === r ? null : r;
                            relocateInput = r;
                          }}
                        >
                          Relokasi
                        </button>
                      {/if}
                      <button
                        type="button"
                        class="btn-secondary btn-xs"
                        onclick={() => {
                          editingAliasFor = editingAliasFor === r ? null : r;
                          aliasInput = alias || "";
                        }}
                      >
                        Alias
                      </button>
                      <button
                        type="button"
                        class="btn-secondary btn-xs btn-danger"
                        aria-label="Hapus {r} dari daftar terakhir"
                        onclick={() => onRemoveRecent(r)}
                      >
                        Hapus
                      </button>
                    </div>
                  </div>

                  {#if editingAliasFor === r}
                    <div style="display: flex; gap: 6px; padding-top: 2px;">
                      <input
                        type="text"
                        class="form-input"
                        aria-label="Alias tampilan untuk {r}"
                        placeholder="Alias repositori (kosongkan untuk reset)"
                        bind:value={aliasInput}
                        style="flex: 1; font-size: 11px;"
                      />
                      <button
                        type="button"
                        class="btn-primary btn-xs"
                        onclick={() => {
                          onSetAlias(r, aliasInput);
                          editingAliasFor = null;
                        }}
                      >
                        Simpan
                      </button>
                    </div>
                  {/if}

                  {#if relocatingPathFor === r}
                    <div style="display: flex; gap: 6px; padding-top: 2px;">
                      <input
                        type="text"
                        class="form-input"
                        aria-label="Lokasi baru untuk {r}"
                        placeholder="Path lokasi baru repositori"
                        bind:value={relocateInput}
                        style="flex: 1; font-size: 11px;"
                      />
                      <button
                        type="button"
                        class="btn-secondary btn-xs"
                        disabled={isBrowsingDir}
                        onclick={async () => {
                          const picked = await browseDirectory(relocateInput, "Pilih Lokasi Baru Repositori", false);
                          if (picked) relocateInput = picked;
                        }}
                      >
                        Browse...
                      </button>
                      <button
                        type="button"
                        class="btn-primary btn-xs"
                        onclick={async () => {
                          await onRelocateRepo(r, relocateInput);
                          relocatingPathFor = null;
                        }}
                      >
                        Pindah
                      </button>
                    </div>
                  {/if}
                </div>
              {/each}
            </div>
          </div>
        {/if}

      {:else if modal === "init"}
        <div class="form-field">
          <label class="form-label" for="init-repo-path">Lokasi Direktori</label>
          <div style="display: flex; gap: 6px; align-items: center;">
            <input
              id="init-repo-path"
              type="text"
              class="form-input"
              style="flex: 1;"
              bind:value={initRepoPath}
              placeholder="C:\Development\folder-baru atau /home/user/proj"
            />
            <button
              type="button"
              class="btn-secondary"
              disabled={isBrowsingDir}
              onclick={async () => {
                const picked = await browseDirectory(initRepoPath, "Pilih Folder Repositori Baru", true);
                if (picked) initRepoPath = picked;
              }}
            >
              Browse...
            </button>
          </div>
        </div>
        <div class="form-field">
          <label class="form-label" for="init-default-branch">Default Branch</label>
          <input id="init-default-branch" type="text" class="form-input" bind:value={initDefaultBranch} placeholder="main" />
        </div>
        <div class="form-field">
          <label class="form-label" for="init-gitignore">Template .gitignore</label>
          <select id="init-gitignore" class="form-select" bind:value={initGitignore}>
            <option value="None">Tidak Ada</option>
            <option value="Node">Node.js</option>
            <option value="Rust">Rust</option>
            <option value="Python">Python</option>
          </select>
        </div>
        <div class="form-field">
          <label class="form-label" for="init-license">Lisensi</label>
          <select id="init-license" class="form-select" bind:value={initLicense}>
            <option value="None">Tidak Ada</option>
            <option value="MIT">MIT License</option>
            <option value="Apache-2.0">Apache 2.0</option>
          </select>
        </div>
        <div class="form-field">
          <label class="form-label" for="init-remote-url">URL Remote origin (Opsional)</label>
          <input id="init-remote-url" type="text" class="form-input" bind:value={initRemoteUrl} placeholder="https://github.com/user/repo-kosong.git" />
        </div>

      {:else if modal === "clone"}
        <div class="form-field">
          <label class="form-label" for="clone-url">URL Repositori (HTTPS / SSH)</label>
          <input id="clone-url" type="text" class="form-input" bind:value={cloneUrl} placeholder="https://github.com/user/repo.git" />
        </div>
        <div class="form-field">
          <label class="form-label" for="clone-target">Folder Tujuan</label>
          <div style="display: flex; gap: 6px; align-items: center;">
            <input
              id="clone-target"
              type="text"
              class="form-input"
              style="flex: 1;"
              bind:value={cloneTarget}
              placeholder="C:\Development\my-repo atau /home/user/my-repo"
            />
            <button
              type="button"
              class="btn-secondary"
              disabled={isBrowsingDir}
              onclick={async () => {
                const picked = await browseDirectory(cloneTarget, "Pilih Folder Tujuan Clone", true);
                if (!picked) return;
                const repoName = deriveRepoNameFromUrl(cloneUrl);
                if (repoName && pathBaseName(picked).toLowerCase() !== repoName.toLowerCase()) {
                  cloneTarget = joinRepoPath(picked, repoName);
                } else {
                  cloneTarget = picked;
                }
              }}
            >
              Browse...
            </button>
          </div>
        </div>
        {#if accountOverview.accounts.length > 0}
          <div class="form-field">
            <label class="form-label" for="clone-account">Akun untuk Clone Privat (Opsional)</label>
            <select id="clone-account" class="form-select" bind:value={cloneAccountId}>
              <option value="">Otomatis (Default host URL)</option>
              {#each accountOverview.accounts as acc (acc.id)}
                <option value={acc.id}>
                  @{acc.login} ({acc.host}){acc.label ? ` — ${acc.label}` : ""}
                </option>
              {/each}
            </select>
          </div>
        {/if}

      {:else if modal === "publish"}
        {#if accountOverview.accounts.length === 0 && onOpenAuth}
          <div class="info-callout" style="display: flex; justify-content: space-between; align-items: center; gap: 10px;">
            <span>Belum ada akun GitHub terdaftar. Hubungkan akun dulu sebelum publikasi.</span>
            <button type="button" class="btn-secondary btn-xs btn-accent" onclick={onOpenAuth}>
              Hubungkan Akun
            </button>
          </div>
        {:else if accountOverview.accounts.length > 0}
          <div class="form-field">
            <label class="form-label" for="publish-account">Akun &amp; Server Tujuan</label>
            <select id="publish-account" class="form-select" bind:value={publishAccountId}>
              {#each accountOverview.accounts as acc (acc.id)}
                <option value={acc.id}>
                  @{acc.login} ({acc.host}){acc.label ? ` — ${acc.label}` : ""}
                </option>
              {/each}
            </select>
          </div>
        {/if}
        <p style="font-size: 12px; color: var(--text-secondary);">
          Repositori lokal ini belum terhubung ke remote. Buat repositori baru di akun GitHub Anda dan push branch utama.
        </p>
        {#if !hasCommits}
          <div class="info-callout">
            Repositori lokal ini belum memiliki komit. Publikasi akan membuat repo di GitHub dan menautkan remote <code>origin</code>; buat komit pertama lalu klik Push.
          </div>
        {/if}
        <div class="form-field">
          <label class="form-label" for="publish-name">Nama Repositori</label>
          <input id="publish-name" type="text" class="form-input" bind:value={publishName} placeholder="Nama repositori" />
        </div>
        <div class="form-field">
          <label class="form-label" for="publish-desc">Deskripsi (Opsional)</label>
          <textarea id="publish-desc" class="form-input" bind:value={publishDesc} rows="2" placeholder="Deskripsi repositori GitHub" style="resize: vertical;"></textarea>
        </div>
        <label class="checkbox-label">
          <input type="checkbox" bind:checked={publishPrivate} />
          <span>Buat sebagai repositori privat (Private Repository)</span>
        </label>


      {:else if modal === "remote"}
        {#if (remotes ?? []).length === 0}
          <div class="info-callout">
            Belum ada remote terhubung. Masukkan nama (biasanya <code style="font-family: var(--font-mono);">origin</code>) dan URL repositori kosong dari GitHub/GitLab/server Git Anda di bawah ini.
          </div>
        {:else}
          <div class="form-field">
            <span class="form-label">Remote Terhubung ({(remotes ?? []).length})</span>
            <div style="display: flex; flex-direction: column; gap: 8px; max-height: 240px; overflow-y: auto;">
              {#each remotes ?? [] as r, idx (`${idx}:${r.name}`)}
                <div class="account-item-card">
                  <div style="display: flex; align-items: center; justify-content: space-between; gap: 10px;">
                    <div style="min-width: 0; flex: 1;">
                      <div style="display: flex; align-items: center; gap: 6px;">
                        <span class="badge-accent" style="font-family: var(--font-mono);">{r.name}</span>
                      </div>
                      <div style="font-size: 11px; color: var(--text-secondary); font-family: var(--font-mono); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; margin-top: 4px; user-select: text;" title={r.url}>
                        {r.url}
                      </div>
                    </div>
                    <div style="display: flex; gap: 5px; flex-shrink: 0;">
                      <button
                        type="button"
                        class="btn-secondary btn-xs"
                        disabled={isSubmittingRemote}
                        onclick={() => {
                          editingRemoteName = editingRemoteName === r.name ? null : r.name;
                          editingRemoteUrl = r.url;
                          inlineError = "";
                        }}
                      >
                        Ubah URL
                      </button>
                      <button
                        type="button"
                        class="btn-secondary btn-xs btn-danger"
                        disabled={isSubmittingRemote}
                        onclick={() => handleRemoveRemote(r.name)}
                      >
                        Hapus
                      </button>
                    </div>
                  </div>
                  {#if editingRemoteName === r.name}
                    <div style="display: flex; gap: 6px; padding-top: 4px; border-top: 1px solid var(--border-muted);">
                      <input
                        type="text"
                        class="form-input"
                        style="flex: 1; font-size: 11px; font-family: var(--font-mono);"
                        aria-label="URL baru untuk remote {r.name}"
                        bind:value={editingRemoteUrl}
                        placeholder="https://github.com/user/repo.git"
                        onkeydown={(e) => {
                          if (e.key === "Enter") handleSaveRemoteUrl(r.name);
                          else if (e.key === "Escape") editingRemoteName = null;
                        }}
                      />
                      <button
                        type="button"
                        class="btn-primary btn-xs"
                        disabled={isSubmittingRemote || !editingRemoteUrl.trim()}
                        onclick={() => handleSaveRemoteUrl(r.name)}
                      >
                        Simpan
                      </button>
                      <button
                        type="button"
                        class="btn-secondary btn-xs"
                        disabled={isSubmittingRemote}
                        onclick={() => (editingRemoteName = null)}
                      >
                        Batal
                      </button>
                    </div>
                  {/if}
                </div>
              {/each}
            </div>
          </div>
        {/if}

        <div class="modal-section-heading">Tambah Remote Baru</div>
        <div class="remote-add-grid">
          <div class="form-field">
            <label class="form-label" for="new-remote-name">Nama Remote</label>
            <input
              id="new-remote-name"
              type="text"
              class="form-input"
              style="font-family: var(--font-mono);"
              bind:value={newRemoteName}
              placeholder="origin"
              onkeydown={(e) => {
                if (e.key === "Enter" && newRemoteName.trim() && newRemoteUrl.trim()) handleAddRemote();
              }}
            />
          </div>
          <div class="form-field">
            <label class="form-label" for="new-remote-url">URL Remote (HTTPS / SSH)</label>
            <input
              id="new-remote-url"
              type="text"
              class="form-input"
              style="font-family: var(--font-mono);"
              bind:value={newRemoteUrl}
              placeholder="https://github.com/user/repo.git"
              onkeydown={(e) => {
                if (e.key === "Enter" && newRemoteName.trim() && newRemoteUrl.trim()) handleAddRemote();
              }}
            />
          </div>
        </div>
        <div style="display: flex; justify-content: flex-end;">
          <button type="button" class="btn-primary" disabled={isSubmittingRemote || !newRemoteName.trim() || !newRemoteUrl.trim()} onclick={handleAddRemote}>
            {isSubmittingRemote ? "Menyimpan..." : "Tambah Remote"}
          </button>
        </div>
      {:else if modal === "stash"}
        <div style="display: flex; gap: 6px;">
          <input
            type="text"
            class="form-input"
            aria-label="Pesan stash baru"
            bind:value={stashMsg}
            placeholder="Pesan stash baru (opsional)"
            style="flex: 1;"
          />
          <button type="button" onclick={handleStashSave} class="btn-primary">Simpan Stash</button>
        </div>

        <div class="stash-layout">
          <div style="display: flex; flex-direction: column; gap: 6px; min-height: 0;">
            <div style="display: flex; justify-content: space-between; align-items: center;">
              <span class="form-label tabular-nums">Daftar Stash ({stashList.length})</span>
              <button
                type="button"
                onclick={handleStashPop}
                class="btn-secondary btn-xs"
                disabled={stashList.length === 0}
              >
                Pop Teratas
              </button>
            </div>
            <div class="list-box" style="flex: 1; max-height: none;">
              {#if stashList.length === 0}
                <div style="color: var(--text-muted); text-align: center; padding: 24px 12px; font-size: 11px;">
                  Tidak ada stash tersimpan
                </div>
              {:else}
                {#each stashList as s (s)}
                  <div class="list-row {selectedStash === s ? 'is-active' : ''}" style="padding: 5px 8px;">
                    <button
                      type="button"
                      onclick={() => selectStash(s)}
                      title={s}
                      style="flex: 1; min-width: 0; text-align: left; font-family: var(--font-mono); font-size: 11px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;"
                    >
                      {s}
                    </button>
                    <button
                      type="button"
                      class="btn-secondary btn-xs btn-danger"
                      title="Hapus stash ini"
                      onclick={() => handleDropStash(s)}
                    >
                      Drop
                    </button>
                  </div>
                {/each}
              {/if}
            </div>
          </div>

          <div style="display: flex; flex-direction: column; min-height: 0;">
            <span class="form-label" style="margin-bottom: 6px;">Pratinjau Perubahan (Diff)</span>
            <div style="flex: 1; overflow: auto; background: var(--bg-canvas); border: 1px solid var(--border-default); border-radius: var(--radius-sm); padding: 8px; font-family: var(--font-mono); font-size: 11px; white-space: pre-wrap; user-select: text;">
              {#if isLoadingStashDiff}
                <div style="color: var(--text-muted); text-align: center; padding: 24px;">Memuat diff stash...</div>
              {:else if stashDiff}
                {stashDiff}
              {:else}
                <div style="color: var(--text-muted); text-align: center; padding: 24px;">
                  Pilih entri stash di sebelah kiri untuk melihat isi perubahannya.
                </div>
              {/if}
            </div>
          </div>
        </div>

      {:else if modal === "diag"}
        <div style="display: flex; flex-direction: column; gap: 8px; font-size: 12px;">
          <div><strong>Aplikasi:</strong> Forge Desktop v{diagReport?.app_version || "-"}</div>
          <div><strong>OS / Arch:</strong> {diagReport?.os_info || "-"}</div>
          <div><strong>Versi Git:</strong> {diagReport?.git_version || "-"}</div>
          <div class="form-label" style="margin-top: 4px;">Konfigurasi Git (Disanitasi)</div>
          <pre style="background: var(--bg-app); border: 1px solid var(--border-default); padding: 10px; border-radius: var(--radius-sm); max-height: 200px; overflow: auto; font-family: var(--font-mono); font-size: 11px; user-select: text;">{diagReport?.sanitized_config.join("\n") || "Tidak ada config"}</pre>
        </div>

      {:else if modal === "auth"}
        {#if authView === "manager" && accountOverview.accounts.length > 0}
          <div style="display: flex; justify-content: space-between; align-items: center; gap: 8px;">
            <span style="font-size: 12px; color: var(--text-secondary);">
              {accountOverview.accounts.length} akun tersimpan di OS Keyring / sesi
            </span>
            <button
              type="button"
              class="btn-primary btn-xs"
              onclick={() => openAddAccountSheet(githubRepoInfo?.host)}
            >
              + Tambah Akun
            </button>
          </div>

          {#if authorMismatch}
            <div class="info-callout" style="display: flex; justify-content: space-between; align-items: center; gap: 10px;">
              <span>
                Identitas author repo ini berbeda dengan akun aktif (<strong>{authorMismatch.name}</strong>{authorMismatch.email ? ` <${authorMismatch.email}>` : ""}).
              </span>
              <button
                type="button"
                class="btn-secondary btn-xs btn-accent"
                style="flex-shrink: 0;"
                onclick={() => onSyncRepoAuthor(authorMismatch!.name, authorMismatch!.email)}
              >
                Terapkan ke Repo
              </button>
            </div>
          {/if}

          <div style="display: flex; flex-direction: column; gap: 8px; max-height: 360px; overflow-y: auto;">
            {#each accountOverview.accounts as acc (acc.id)}
              {@const isActiveForRepo = activeRepoAccount?.id === acc.id}
              {@const isDefault = accountOverview.default_by_host[acc.host] === acc.id}
              {@const health = accountHealthMap[acc.id]}
              {@const hostMatchesRepo = !githubRepoInfo || githubRepoInfo.host === acc.host}

              <div class="account-item-card {isActiveForRepo ? 'is-active' : ''}">
                <div style="display: flex; align-items: center; gap: 10px;">
                  {#if acc.avatar_url}
                    <img src={acc.avatar_url} alt="Avatar {acc.login}" class="account-avatar" />
                  {:else}
                    <div class="account-avatar-fallback" aria-hidden="true">
                      {acc.login.charAt(0).toUpperCase()}
                    </div>
                  {/if}
                  <div style="flex: 1; min-width: 0;">
                    <div style="display: flex; align-items: center; gap: 6px; flex-wrap: wrap;">
                      <span style="font-weight: 600; font-size: 13px; color: var(--text-primary);">
                        {acc.name || acc.login}
                      </span>
                      {#if acc.label}
                        <span class="badge-accent">{acc.label}</span>
                      {/if}
                      {#if isActiveForRepo}
                        <span class="status-badge-ok">Aktif di Repo Ini</span>
                      {:else if isDefault}
                        <span class="badge-neutral">Default {acc.host === "github.com" ? "" : acc.host}</span>
                      {/if}
                      {#if acc.memory_only}
                        <span class="badge-neutral">Memori Sesi</span>
                      {/if}
                      {#if health?.status === "expired"}
                        <span class="badge-warn">Kedaluwarsa</span>
                      {:else if health?.status === "sso_required"}
                        <span class="badge-warn">Perlu SSO</span>
                      {/if}
                    </div>
                    <div style="font-family: var(--font-mono); font-size: 11px; color: var(--text-secondary);">
                      @{acc.login} · {acc.host === "github.com" ? "GitHub.com" : `Enterprise: ${acc.host}`}
                    </div>
                  </div>
                </div>

                {#if health?.status === "expired"}
                  <div class="warn-callout">
                    <span>{health.message}</span>
                    <div style="display: flex; gap: 6px;">
                      <button
                        type="button"
                        class="btn-primary btn-xs"
                        onclick={() => openAddAccountSheet(acc.host, acc.label)}
                      >
                        Perbarui Token
                      </button>
                    </div>
                  </div>
                {:else if health?.status === "sso_required"}
                  <div class="warn-callout">
                    <span>{health.message}</span>
                    <div style="display: flex; gap: 6px;">
                      <button
                        type="button"
                        class="btn-primary btn-xs"
                        onclick={() =>
                          openExternalUrl(health.sso_url || `https://${acc.host}/settings/tokens`)}
                      >
                        Otorisasi SSO di Browser
                      </button>
                      <button
                        type="button"
                        class="btn-secondary btn-xs"
                        onclick={() => handleCheckHealth(acc.id)}
                      >
                        Cek Ulang
                      </button>
                    </div>
                  </div>
                {/if}

                {#if editingLabelFor === acc.id}
                  <div style="display: flex; gap: 6px; align-items: center;">
                    <input
                      type="text"
                      class="form-input"
                      style="flex: 1;"
                      bind:value={labelDraft}
                      placeholder="Contoh: Pribadi atau Kantor"
                    />
                    <button type="button" class="btn-primary btn-xs" onclick={() => handleSaveLabel(acc.id)}>
                      Simpan
                    </button>
                    <button type="button" class="btn-secondary btn-xs" onclick={() => (editingLabelFor = null)}>
                      Batal
                    </button>
                  </div>
                {/if}

                <div class="account-actions-row">
                  <div style="display: flex; flex-wrap: wrap; gap: 5px;">
                    {#if activeRepo && hostMatchesRepo && !isActiveForRepo}
                      <button
                        type="button"
                        class="btn-secondary btn-xs btn-accent"
                        onclick={() => handleBindAccount(acc.id)}
                      >
                        Pakai di Repo Ini
                      </button>
                    {/if}
                    {#if !isDefault}
                      <button
                        type="button"
                        class="btn-secondary btn-xs"
                        onclick={() => handleSetDefaultAccount(acc.id)}
                      >
                        Jadikan Default
                      </button>
                    {/if}
                    <button
                      type="button"
                      class="btn-secondary btn-xs"
                      onclick={() => {
                        editingLabelFor = acc.id;
                        labelDraft = acc.label || "";
                      }}
                    >
                      Label
                    </button>
                    <button
                      type="button"
                      class="btn-secondary btn-xs"
                      disabled={checkingHealthFor === acc.id}
                      onclick={() => handleCheckHealth(acc.id)}
                    >
                      {checkingHealthFor === acc.id ? "Mengecek..." : "Cek Sesi"}
                    </button>
                  </div>

                  <div style="display: flex; gap: 5px;">
                    <button
                      type="button"
                      class="btn-secondary btn-xs"
                      onclick={() =>
                        openExternalUrl(acc.html_url || `https://${acc.host}/${acc.login}`)}
                    >
                      Profil
                    </button>
                    <button
                      type="button"
                      class="btn-secondary btn-xs btn-danger"
                      onclick={() => handleRemoveAccount(acc)}
                    >
                      Hapus
                    </button>
                  </div>
                </div>
              </div>
            {/each}
          </div>
        {:else}
          {#if accountOverview.accounts.length > 0}
            <div style="display: flex; justify-content: space-between; align-items: center;">
              <button
                type="button"
                class="btn-secondary btn-xs"
                onclick={() => {
                  clearPollTimer();
                  inlineError = "";
                  keyringUnavailableAction = null;
                  pendingSsoUrl = null;
                  authView = "manager";
                }}
              >
                ← Kembali ke Daftar Akun
              </button>
            </div>
          {/if}

          <div class="form-grid-2">
            <div class="form-field">
              <span class="form-label">Server GitHub</span>
              <div class="seg-group" role="radiogroup" aria-label="Pilih tipe server GitHub">
                <button
                  type="button"
                  class="seg-btn {hostKind === 'github.com' ? 'active' : ''}"
                  onclick={() => {
                    hostKind = "github.com";
                    inlineError = "";
                    oauthClientId = localStorage.getItem("forge_oauth_client_id:github.com") || "";
                    if (authTab === "import") refreshDetectedSessions();
                  }}
                >
                  GitHub.com
                </button>
                <button
                  type="button"
                  class="seg-btn {hostKind === 'enterprise' ? 'active' : ''}"
                  onclick={() => {
                    hostKind = "enterprise";
                    inlineError = "";
                    if (authTab === "import") refreshDetectedSessions();
                  }}
                >
                  Enterprise
                </button>
              </div>
            </div>

            <div class="form-field">
              <label class="form-label" for="account-label-input">Label (Opsional)</label>
              <input
                id="account-label-input"
                type="text"
                class="form-input"
                bind:value={accountLabelInput}
                placeholder="Mis. Pribadi / Kantor"
              />
            </div>
          </div>

          {#if hostKind === "enterprise"}
            <div class="form-field">
              <label class="form-label" for="enterprise-host-input">Domain GitHub Enterprise</label>
              <input
                id="enterprise-host-input"
                type="text"
                class="form-input"
                style="font-family: var(--font-mono);"
                bind:value={enterpriseHostInput}
                placeholder="git.perusahaan.com"
                onblur={() => {
                  if (effectiveHost) {
                    oauthClientId =
                      localStorage.getItem(`forge_oauth_client_id:${effectiveHost}`) || "";
                    if (authTab === "import") refreshDetectedSessions();
                  }
                }}
              />
              <span style="font-size: 11px; color: var(--text-muted); font-family: var(--font-mono);">
                Endpoint API: https://{effectiveHost || "git.perusahaan.com"}/api/v3
              </span>
            </div>
          {/if}

          <div class="seg-group" style="align-self: flex-start;" role="tablist" aria-label="Metode autentikasi">
            <button
              type="button"
              role="tab"
              aria-selected={authTab === "token"}
              class="seg-btn {authTab === 'token' ? 'active' : ''}"
              onclick={() => {
                authTab = "token";
                inlineError = "";
              }}
            >
              Token (PAT)
            </button>
            <button
              type="button"
              role="tab"
              aria-selected={authTab === "oauth"}
              class="seg-btn {authTab === 'oauth' ? 'active' : ''}"
              onclick={() => {
                authTab = "oauth";
                inlineError = "";
              }}
            >
              Browser OAuth
            </button>
            <button
              type="button"
              role="tab"
              aria-selected={authTab === "import"}
              class="seg-btn {authTab === 'import' ? 'active' : ''}"
              onclick={() => {
                authTab = "import";
                inlineError = "";
                refreshDetectedSessions();
              }}
            >
              Impor Sesi
            </button>
          </div>

          {#if keyringUnavailableAction}
            <div class="warn-callout">
              <span>OS Keyring / Secret Service tidak tersedia. Anda dapat menyimpan token sementara di memori sesi saja (hilang saat aplikasi ditutup).</span>
              <div>
                <button
                  type="button"
                  class="btn-primary btn-xs"
                  onclick={() => keyringUnavailableAction?.()}
                >
                  Simpan di Memori Sesi Saja
                </button>
              </div>
            </div>
          {/if}

          {#if pendingSsoUrl}
            <div class="warn-callout">
              <span>Token perlu otorisasi SAML Single Sign-On oleh organisasi Anda.</span>
              <div>
                <button
                  type="button"
                  class="btn-primary btn-xs"
                  onclick={() => openExternalUrl(pendingSsoUrl!)}
                >
                  Otorisasi SSO di Browser
                </button>
              </div>
            </div>
          {/if}

          {#if authTab === "token"}
            <div class="info-callout" style="display: flex; flex-direction: column; gap: 6px;">
              <span><strong>Buat &amp; Verifikasi Token:</strong></span>
              <span>1. Buat Personal Access Token dengan scope <code style="font-family: var(--font-mono);">repo</code>, <code style="font-family: var(--font-mono);">read:user</code>, &amp; <code style="font-family: var(--font-mono);">user:email</code>.</span>
              <span>2. Token disimpan terenkripsi ke OS Keyring (bukan di disk plaintext).</span>
              <div style="padding-top: 2px;">
                <button
                  type="button"
                  class="btn-secondary btn-xs btn-accent"
                  disabled={!effectiveHost}
                  onclick={() =>
                    openExternalUrl(
                      `https://${effectiveHost || "github.com"}/settings/tokens/new?description=Forge+Desktop&scopes=repo,read:user,user:email`
                    )}
                >
                  Buat Token di {effectiveHost || "github.com"}...
                </button>
              </div>
            </div>

            <div class="form-field">
              <label class="form-label" for="github-pat-input">Personal Access Token (PAT)</label>
              <div style="display: flex; gap: 6px; align-items: center;">
                <input
                  id="github-pat-input"
                  type={showPat ? "text" : "password"}
                  class="form-input"
                  style="flex: 1; font-family: var(--font-mono);"
                  bind:value={patInput}
                  placeholder="ghp_... atau github_pat_..."
                  autocomplete="off"
                  onkeydown={(e) => {
                    if (e.key === "Enter" && !isVerifyingAuth) handleConnectPat(false);
                  }}
                />
                <button
                  type="button"
                  class="btn-secondary"
                  onclick={() => (showPat = !showPat)}
                  aria-pressed={showPat}
                >
                  {showPat ? "Sembunyikan" : "Lihat"}
                </button>
              </div>
            </div>

            <div style="display: flex; justify-content: flex-end;">
              <button
                type="button"
                class="btn-primary"
                disabled={isVerifyingAuth || !patInput.trim() || !effectiveHost}
                onclick={() => handleConnectPat(false)}
              >
                {isVerifyingAuth ? "Memverifikasi..." : "Verifikasi & Simpan Akun"}
              </button>
            </div>
          {:else if authTab === "oauth"}
            {#if !deviceFlow}
              <div class="info-callout">
                Gunakan GitHub OAuth App dengan <strong>Enable Device Flow</strong> aktif (mendukung GitHub.com &amp; Enterprise).
              </div>
              <div class="form-field">
                <label class="form-label" for="oauth-client-id">OAuth App Client ID</label>
                <input
                  id="oauth-client-id"
                  type="text"
                  class="form-input"
                  style="font-family: var(--font-mono);"
                  bind:value={oauthClientId}
                  placeholder="Iv1.xxxxxxxxxxxx"
                  onkeydown={(e) => {
                    if (e.key === "Enter") handleStartDeviceFlow(false);
                  }}
                />
              </div>
              <div style="display: flex; justify-content: space-between; align-items: center; gap: 8px;">
                <button
                  type="button"
                  class="link-btn"
                  disabled={!effectiveHost}
                  onclick={() =>
                    openExternalUrl(`https://${effectiveHost || "github.com"}/settings/developers`)}
                >
                  Buka Developer Settings
                </button>
                <button
                  type="button"
                  onclick={() => handleStartDeviceFlow(false)}
                  class="btn-primary"
                  disabled={!oauthClientId.trim() || !effectiveHost}
                >
                  Minta Kode &amp; Buka Browser
                </button>
              </div>
            {:else}
              <div style="display: flex; flex-direction: column; gap: 10px; font-size: 12px;">
                <div style="display: flex; justify-content: space-between; align-items: center;">
                  <span>Masukkan kode berikut di browser:</span>
                  {#if deviceExpiresLeft > 0}
                    <span style="font-family: var(--font-mono); font-size: 11px; color: var(--text-muted);">
                      Sisa {formatCountdown(deviceExpiresLeft)}
                    </span>
                  {/if}
                </div>
                <div class="oauth-code-box">
                  <span class="oauth-code-value">{deviceFlow.user_code}</span>
                  <button type="button" class="btn-secondary btn-xs" onclick={copyDeviceCode}>
                    {copiedCode ? "Tersalin!" : "Salin Kode"}
                  </button>
                </div>
                <div style="display: flex; justify-content: space-between; align-items: center;">
                  <button
                    type="button"
                    class="link-btn"
                    onclick={() => openExternalUrl(deviceFlow!.verification_uri)}
                  >
                    Buka ulang {deviceFlow.verification_uri}
                  </button>
                  <button
                    type="button"
                    class="btn-secondary btn-xs"
                    onclick={() => {
                      clearPollTimer();
                      deviceFlow = null;
                      deviceExpiresLeft = 0;
                      pollStatus = "";
                    }}
                  >
                    Batalkan
                  </button>
                </div>
                {#if pollStatus}
                  <div style="text-align: center; font-size: 11px; color: {pollStatusClass === 'success' ? 'var(--diff-add-text)' : pollStatusClass === 'error' ? 'var(--diff-del-text)' : 'var(--text-muted)'}; font-weight: {pollStatusClass ? '600' : '400'};">
                    {pollStatus}
                  </div>
                {/if}
              </div>
            {/if}
          {:else}
            <div style="display: flex; flex-direction: column; gap: 8px;">
              <div style="display: flex; justify-content: space-between; align-items: center;">
                <span style="font-size: 12px; color: var(--text-secondary);">
                  Deteksi otomatis sesi <code style="font-family: var(--font-mono);">gh</code> CLI dan <code style="font-family: var(--font-mono);">~/.git-credentials</code> untuk <strong>{effectiveHost || "github.com"}</strong>:
                </span>
                <button
                  type="button"
                  class="btn-secondary btn-xs"
                  disabled={isDetectingSessions || !effectiveHost}
                  onclick={refreshDetectedSessions}
                >
                  {isDetectingSessions ? "Memindai..." : "Pindai Ulang"}
                </button>
              </div>

              {#if isDetectingSessions}
                <div class="info-callout">Mencari sesi login lokal yang aktif...</div>
              {:else if detectedSessions.length === 0}
                <div class="info-callout">
                  Tidak ditemukan sesi lokal untuk {effectiveHost || "github.com"}. Gunakan tab <strong>Token (PAT)</strong> atau <strong>Browser OAuth</strong>.
                </div>
              {:else}
                <div class="list-box">
                  {#each detectedSessions as sess (sess.source + sess.host)}
                    <div class="list-row">
                      <div style="display: flex; flex-direction: column; min-width: 0;">
                        <span style="font-weight: 600; font-size: 12px;">{sess.display}</span>
                        <span style="font-size: 11px; color: var(--text-muted);">
                          Token diverifikasi langsung oleh backend ke {sess.host}
                        </span>
                      </div>
                      <button
                        type="button"
                        class="btn-primary btn-xs"
                        disabled={isVerifyingAuth}
                        onclick={() => handleImportSession(sess.source, false)}
                      >
                        {isVerifyingAuth ? "Mengimpor..." : "Impor & Verifikasi"}
                      </button>
                    </div>
                  {/each}
                </div>
              {/if}
            </div>
          {/if}
        {/if}
      {:else if modal === "settings"}
        <div class="modal-section-heading">Konfigurasi Lokal Repositori</div>
        <div class="form-field">
          <label class="form-label" for="repo-user-name">Nama Author (user.name)</label>
          <input id="repo-user-name" type="text" class="form-input" bind:value={draftRepoConfig.user_name} placeholder="Nama Anda (lokal)" />
        </div>
        <div class="form-field">
          <label class="form-label" for="repo-user-email">Email Author (user.email)</label>
          <input id="repo-user-email" type="text" class="form-input" bind:value={draftRepoConfig.user_email} placeholder="email@example.com (lokal)" />
        </div>

        <div class="modal-section-heading">Konfigurasi Global Git (~/.gitconfig)</div>
        <div class="form-field">
          <label class="form-label" for="global-user-name">Global Nama (user.name)</label>
          <input id="global-user-name" type="text" class="form-input" bind:value={draftGlobalConfig.user_name} placeholder="Nama Anda (global)" />
        </div>
        <div class="form-field">
          <label class="form-label" for="global-user-email">Global Email (user.email)</label>
          <input id="global-user-email" type="text" class="form-input" bind:value={draftGlobalConfig.user_email} placeholder="email@example.com (global)" />
        </div>

        <div class="modal-section-heading">Tampilan & Sinkronisasi</div>
        <div class="settings-row">
          <label class="form-label" for="theme-select">Tema Antarmuka</label>
          <select id="theme-select" class="form-select" bind:value={draftTheme}>
            <option value="dark">Dark Theme (Gelap)</option>
            <option value="light">Light Theme (Terang)</option>
            <option value="system">Mengikuti Sistem OS</option>
          </select>
        </div>
        <div class="settings-row">
          <label class="form-label" for="autofetch-select">Auto-fetch Otomatis</label>
          <select id="autofetch-select" class="form-select" bind:value={draftAutofetch}>
            <option value={0}>Nonaktif</option>
            <option value={1}>Setiap 1 menit</option>
            <option value={5}>Setiap 5 menit (Default)</option>
            <option value={10}>Setiap 10 menit</option>
            <option value={15}>Setiap 15 menit</option>
          </select>
        </div>

        <div class="modal-section-heading">Editor Eksternal</div>
        <div class="settings-row">
          <label class="form-label" for="external-editor-select">Text Editor</label>
          <select id="external-editor-select" class="form-select" bind:value={draftEditorPreset}>
            {#each EXTERNAL_EDITOR_PRESETS as preset (preset.value)}
              <option value={preset.value}>{preset.label}</option>
            {/each}
          </select>
        </div>
        {#if draftEditorPreset === "custom"}
          <div class="form-field">
            <label class="form-label" for="custom-editor-input">Perintah / Path Editor Kustom</label>
            <div style="display: flex; gap: 6px;">
              <input
                id="custom-editor-input"
                type="text"
                class="form-input"
                bind:value={draftCustomEditor}
                placeholder="Contoh: nvim-qt atau C:\Program Files\...\editor.exe"
                style="flex: 1;"
              />
              <button type="button" class="btn-secondary btn-xs" onclick={handlePickCustomEditor}>
                Pilih...
              </button>
            </div>
          </div>
        {/if}
        <div style="font-size: 11px; color: var(--text-muted); line-height: 1.45;">
          {#if draftEditorPreset === "system"}
            Membuka file/folder menggunakan aplikasi default sistem operasi saat ini (Windows Default App / macOS open / Linux xdg-open).
          {:else}
            Digunakan saat menekan tombol Editor Eksternal di header atau tombol Edit pada file konflik.
          {/if}
        </div>

        <div class="modal-section-heading">Aplikasi & Pembaruan</div>
        <div style="display: flex; justify-content: space-between; align-items: center; gap: 8px; font-size: 12px;">
          <span style="color: var(--text-secondary);">
            Versi Saat Ini:
            <strong>v{appUpdateInfo?.current_version || appRuntimeInfo?.current_version || "1.1.0"}</strong>
            {#if appUpdateInfo?.install_mode || appRuntimeInfo?.install_mode}
              <span style="margin-left: 6px; padding: 1px 6px; border-radius: 999px; font-size: 10px; background: var(--bg-subtle); color: var(--text-muted); border: 1px solid var(--border-subtle);">
                {(appUpdateInfo?.install_mode || appRuntimeInfo?.install_mode) === "portable" ? "Portable" : "Installed"}
                ({appUpdateInfo?.package_kind || appRuntimeInfo?.package_kind})
              </span>
            {/if}
          </span>
          <button
            type="button"
            class="btn-secondary btn-xs"
            disabled={isCheckingUpdate || isInstallingUpdate}
            onclick={() => onCheckUpdates(draftUpdateChannel)}
          >
            {isCheckingUpdate ? "Memeriksa..." : "Periksa Pembaruan"}
          </button>
        </div>
        <div class="settings-row">
          <label class="form-label" for="update-channel-select">Saluran Rilis (Channel)</label>
          <select id="update-channel-select" class="form-select" bind:value={draftUpdateChannel}>
            <option value="auto">Otomatis (Ikuti build saat ini)</option>
            <option value="stable">Stable (Rilis stabil saja)</option>
            <option value="beta">Beta (Sertakan pra-rilis / beta)</option>
          </select>
        </div>
        {#if updateError}
          <div style="font-size: 11px; color: var(--diff-del-text); background: var(--bg-subtle); padding: 8px; border-radius: var(--radius-sm);">
            {updateError}
          </div>
        {/if}
        {#if appUpdateInfo}
          <div style="font-size: 11px; color: {appUpdateInfo.has_update ? 'var(--text-accent)' : 'var(--diff-add-text)'}; background: var(--bg-subtle); padding: 8px; border-radius: var(--radius-sm); display: flex; flex-direction: column; gap: 6px;">
            {#if appUpdateInfo.has_update}
              <div>
                Versi baru <strong>v{appUpdateInfo.latest_version}</strong>
                ({appUpdateInfo.is_prerelease ? "Beta" : "Stable"}) tersedia di GitHub Releases.
                {#if appUpdateInfo.asset_name}
                  <div style="color: var(--text-muted); margin-top: 2px;">
                    Paket: {appUpdateInfo.asset_name}
                    {#if appUpdateInfo.asset_size}
                      ({(appUpdateInfo.asset_size / (1024 * 1024)).toFixed(1)} MB)
                    {/if}
                  </div>
                {/if}
              </div>
              {#if appUpdateInfo.release_notes?.trim()}
                <div style="background: var(--bg-panel); border: 1px solid var(--border-subtle); border-radius: var(--radius-sm); padding: 8px; color: var(--text-secondary);">
                  <div style="font-weight: 600; color: var(--text-primary); margin-bottom: 4px;">
                    Catatan Rilis {appUpdateInfo.is_prerelease ? "Beta" : "Stable"}
                  </div>
                  <div style="max-height: 150px; overflow-y: auto; white-space: pre-wrap; line-height: 1.45; font-size: 11px;">
                    {appUpdateInfo.release_notes.trim()}
                  </div>
                </div>
              {/if}
              <div style="display: flex; align-items: center; gap: 8px; flex-wrap: wrap;">
                {#if appUpdateInfo.can_self_update}
                  <button
                    type="button"
                    class="btn-primary btn-xs"
                    disabled={isInstallingUpdate || isCheckingUpdate}
                    onclick={() => onInstallUpdate(draftUpdateChannel)}
                  >
                    {isInstallingUpdate ? "Mengunduh & Memasang..." : "Unduh & Perbarui"}
                  </button>
                {:else}
                  <span style="color: var(--text-muted);">Pembaruan otomatis belum tersedia untuk paket ini; unduh manual dari halaman rilis.</span>
                {/if}
                {#if appUpdateInfo.html_url}
                  <button type="button" class="link-btn" onclick={() => openExternalUrl(appUpdateInfo!.html_url!)}>
                    Buka Halaman Rilis
                  </button>
                {/if}
              </div>
            {:else}
              <div>
                Aplikasi sudah menggunakan versi terbaru (v{appUpdateInfo.current_version}, channel {appUpdateInfo.channel}).
              </div>
              {#if appUpdateInfo.release_notes?.trim()}
                <details style="margin-top: 2px;">
                  <summary style="cursor: pointer; color: var(--text-secondary);">
                    Lihat Catatan Rilis {appUpdateInfo.is_prerelease ? "Beta" : "Stable"} (v{appUpdateInfo.latest_version})
                  </summary>
                  <div style="margin-top: 6px; max-height: 140px; overflow-y: auto; white-space: pre-wrap; line-height: 1.45; font-size: 11px; color: var(--text-secondary); background: var(--bg-panel); border: 1px solid var(--border-subtle); border-radius: var(--radius-sm); padding: 8px;">
                    {appUpdateInfo.release_notes.trim()}
                  </div>
                </details>
              {/if}
            {/if}
          </div>
        {/if}
        <div class="modal-section-heading">Preferensi Keamanan</div>
        <label class="checkbox-label">
          <input type="checkbox" bind:checked={draftConfirmDiscard} />
          <span>Konfirmasi sebelum membatalkan perubahan file (Discard)</span>
        </label>
        <label class="checkbox-label">
          <input type="checkbox" bind:checked={draftConfirmForcePush} />
          <span>Konfirmasi sebelum Force Push (--force-with-lease)</span>
        </label>
        <label class="checkbox-label">
          <input type="checkbox" bind:checked={draftDesktopNotif} />
          <span>Notifikasi Desktop (CI Build Gagal & Sync Fork)</span>
        </label>
      {/if}
    </div>

    <div class="modal-actions">
      {#if modal === "open"}
        <button type="button" class="btn-secondary" onclick={onClose}>Batal</button>
        <button
          type="button"
          class="btn-primary"
          disabled={!openRepoPath.trim()}
          onclick={() => {
            onClose();
            onLoadRepo(openRepoPath.trim());
          }}
        >
          Buka
        </button>
      {:else if modal === "init"}
        <button type="button" class="btn-secondary" onclick={onClose}>Batal</button>
        <button type="button" class="btn-primary" onclick={handleInitRepo}>Buat Repositori</button>
      {:else if modal === "clone"}
        <button type="button" class="btn-secondary" onclick={onClose}>Batal</button>
        <button type="button" class="btn-primary" onclick={handleClone}>Clone</button>
      {:else if modal === "publish"}
        <button type="button" class="btn-secondary" onclick={onClose}>Batal</button>
        <button type="button" class="btn-primary" disabled={!publishName.trim()} onclick={handlePublishSubmit}>
          Publikasikan
        </button>
      {:else if modal === "diag"}
        <button
          type="button"
          class="btn-secondary"
          onclick={() => {
            if (diagReport) {
              const text = `App: Forge Desktop v${diagReport.app_version}\nOS: ${diagReport.os_info}\nGit: ${diagReport.git_version}\nConfig:\n${diagReport.sanitized_config.join("\n")}`;
              navigator.clipboard.writeText(text);
              onStatus("Diagnostik disalin ke clipboard");
            }
          }}
        >
          Salin Info
        </button>
        <button type="button" class="btn-primary" onclick={onClose}>Tutup</button>
      {:else if modal === "settings"}
        <button type="button" class="btn-secondary" onclick={onClose}>Batal</button>
        <button
          type="button"
          class="btn-primary"
          onclick={() => {
            if (draftEditorPreset === "custom" && !draftCustomEditor.trim()) {
              inlineError = "Masukkan perintah atau path text editor kustom.";
              return;
            }
            inlineError = "";
            onSaveSettings({
              repoConfig: draftRepoConfig,
              globalConfig: draftGlobalConfig,
              activeTheme: draftTheme,
              autofetchMinutes: draftAutofetch,
              confirmDiscard: draftConfirmDiscard,
              confirmForcePush: draftConfirmForcePush,
              desktopNotifEnabled: draftDesktopNotif,
              externalEditor:
                draftEditorPreset === "custom" ? draftCustomEditor.trim() : draftEditorPreset,
              updateChannel: draftUpdateChannel,
            });
          }}
        >
          Simpan Semua
        </button>
      {:else}
        <button type="button" class="btn-secondary" onclick={onClose}>Tutup</button>
      {/if}
    </div>
  </div>
</div>
