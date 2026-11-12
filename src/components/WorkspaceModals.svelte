<script lang="ts">
  import { onDestroy, untrack } from "svelte";
  import * as gitService from "../services/git";
  import type {
    AccountHealth,
    AccountOverview,
    AccountProfile,
    AiConfigPublic,
    AppRuntimeInfo,
    AppUpdateInfo,
    DetectedSession,
    DeviceFlowInit,
    DiagnosticReport,
    HostingProvider,
    LocalSessionSource,
    ParsedGitHubRepo,
    RemoteInfo,
    RepoConfig,
    RepoQuickSummary,
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
  import { SUPPORTED_LOCALES, t, type Locale } from "../i18n/index.svelte";
  import {
    clearAppErrorLogs,
    formatAppErrorLogsText,
    getAppErrorLogs,
  } from "../utils/appLog.svelte";
  import { renderMarkdownHtml } from "../utils/markdown";
  import ProviderIcon from "./ProviderIcon.svelte";

  export type WorkspaceModalType =
    | "open"
    | "init"
    | "clone"
    | "publish"
    | "remote"
    | "stash"
    | "diag"
    | "logs"
    | "auth"
    | "settings";
  interface Props {
    modal: WorkspaceModalType;
    activeRepo: string;
    recentRepos: string[];
    repoAliases: Record<string, string>;
    repoMissingStatus: Record<string, boolean>;
    repoSummaries?: Record<string, RepoQuickSummary>;
    isLoadingRepoSummaries?: boolean;
    diagReport: DiagnosticReport | null;
    isLoadingDiag?: boolean;
    repoConfig: RepoConfig;
    globalConfig: RepoConfig;
    isLoadingSettings?: boolean;
    activeTheme: string;
    activeLocale: Locale;
    autofetchSeconds: number;
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
      locale: Locale;
      autofetchSeconds: number;
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
    repoSummaries = {},
    isLoadingRepoSummaries = false,
    diagReport,
    isLoadingDiag = false,
    repoConfig: initialRepoConfig,
    globalConfig: initialGlobalConfig,
    isLoadingSettings = false,
    activeTheme: initialTheme,
    activeLocale: initialLocale,
    autofetchSeconds: initialAutofetchSeconds,
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
  let openRepoFilter = $state("");
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

  type SettingsTabId = "git" | "appearance" | "safety" | "updates" | "ai";
  let settingsTab = $state<SettingsTabId>("git");
  const SETTINGS_TAB_ORDER: SettingsTabId[] = ["git", "appearance", "safety", "updates", "ai"];
  function handleSettingsTabKeydown(e: KeyboardEvent, current: SettingsTabId) {
    const idx = SETTINGS_TAB_ORDER.indexOf(current);
    if (idx < 0) return;
    let nextIdx = -1;
    if (e.key === "ArrowDown" || e.key === "ArrowRight") nextIdx = (idx + 1) % SETTINGS_TAB_ORDER.length;
    else if (e.key === "ArrowUp" || e.key === "ArrowLeft") nextIdx = (idx - 1 + SETTINGS_TAB_ORDER.length) % SETTINGS_TAB_ORDER.length;
    else if (e.key === "Home") nextIdx = 0;
    else if (e.key === "End") nextIdx = SETTINGS_TAB_ORDER.length - 1;
    if (nextIdx >= 0) {
      e.preventDefault();
      const nextId = SETTINGS_TAB_ORDER[nextIdx];
      settingsTab = nextId;
      document.getElementById(`settings-tab-${nextId}`)?.focus();
    }
  }
  // Settings local drafts
  let draftRepoConfig = $state<RepoConfig>({ user_name: "", user_email: "" });
  let draftGlobalConfig = $state<RepoConfig>({ user_name: "", user_email: "" });
  let settingsAuthorsDirty = $state(false);
  let draftTheme = $state("dark");
  let draftLocale = $state<Locale>("id");
  let draftAutofetchSeconds = $state<number | null>(300);
  let draftConfirmDiscard = $state(true);
  let draftConfirmForcePush = $state(true);
  let draftDesktopNotif = $state(true);
  let draftEditorPreset = $state("system");
  let draftCustomEditor = $state("");
  let draftUpdateChannel = $state<UpdateChannel>("auto");
  // Custom AI Engine drafts
  let aiConfig = $state<AiConfigPublic>({
    provider_kind: "openai_compatible",
    endpoint: "https://api.openai.com/v1",
    model: "gpt-4o-mini",
    system_prompt: "",
    max_tokens: 1200,
    has_api_key: false,
    api_key_preview: "",
    memory_only: false,
  });
  let aiApiKeyInput = $state("");
  let showAiApiKey = $state(false);
  let isTestingAi = $state(false);
  let aiTestResult = $state<{ ok: boolean; msg: string } | null>(null);
  let aiKeyringFallbackPending = $state(false);
  let isLoadingAi = $state(false);
  const EXTERNAL_EDITOR_PRESETS = $derived([
    { value: "system", label: t("wsModal.settings.editorSystem") },
    { value: "code", label: "Visual Studio Code (code)" },
    { value: "cursor", label: "Cursor (cursor)" },
    { value: "windsurf", label: "Windsurf (windsurf)" },
    { value: "zed", label: "Zed (zed)" },
    { value: "subl", label: "Sublime Text (subl)" },
    { value: "notepad++", label: "Notepad++ (notepad++)" },
    { value: "notepad", label: "Notepad (Windows)" },
    { value: "gedit", label: "GNOME Text Editor / gedit" },
    { value: "kate", label: "Kate" },
    { value: "custom", label: t("wsModal.settings.editorCustom") },
  ]);

  async function handlePickCustomEditor() {
    try {
      const picked = await pickFile({ title: t("wsModal.settings.pickCustomEditorTitle") });
      if (picked) {
        draftCustomEditor = picked;
      }
    } catch (err) {
      inlineError = String(err);
    }
  }
  // Multi-account Auth states
  let authView = $state<"manager" | "add">("manager");
  let hostingProvider = $state<HostingProvider>("github");
  let hostKind = $state<"github.com" | "enterprise">("github.com");
  let providerHostInput = $state("");
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
    if (hostingProvider === "github") {
      if (hostKind === "github.com") return "github.com";
      return (
        enterpriseHostInput
          .trim()
          .replace(/^https?:\/\//i, "")
          .replace(/\/+$/, "")
          .replace(/\/api\/v3$/i, "")
          .toLowerCase() || ""
      );
    }
    const raw = providerHostInput
      .trim()
      .replace(/^https?:\/\//i, "")
      .replace(/\/+$/, "");
    if (raw) return raw.toLowerCase();
    if (hostingProvider === "gitlab") return "gitlab.com";
    if (hostingProvider === "bitbucket") return "bitbucket.org";
    return "dev.azure.com";
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
        settingsTab = "git";
        settingsAuthorsDirty = false;
        draftRepoConfig = { ...initialRepoConfig };
        draftGlobalConfig = { ...initialGlobalConfig };
        draftTheme = initialTheme;
        draftLocale = initialLocale;
        draftAutofetchSeconds = initialAutofetchSeconds;
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
        aiApiKeyInput = "";
        showAiApiKey = false;
        aiTestResult = null;
        aiKeyringFallbackPending = false;
        isLoadingAi = true;
        gitService
          .getAiConfig()
          .then((cfg) => {
            aiConfig = cfg;
          })
          .catch(() => {})
          .finally(() => {
            isLoadingAi = false;
          });
      } else if (m === "auth") {
        clearPollTimer();
        authView = accountOverview.accounts.length > 0 ? "manager" : "add";
        hostingProvider = "github";
        hostKind = "github.com";
        providerHostInput = "";
        enterpriseHostInput = "";
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

  $effect(() => {
    const rc = initialRepoConfig;
    const gc = initialGlobalConfig;
    if (modal === "settings" && !settingsAuthorsDirty) {
      draftRepoConfig = { ...rc };
      draftGlobalConfig = { ...gc };
    }
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
      stashDiff = t("wsModal.stash.loadDiffFailed", { err: String(err) });
    } finally {
      isLoadingStashDiff = false;
    }
  }

  function handleDropStash(stashLine: string) {
    const ref = extractStashRef(stashLine);
    onRequestConfirm(
      t("wsModal.stash.deleteTitle"),
      t("wsModal.stash.deleteMsg", { ref: ref || stashLine }),
      async () => {
        try {
          await gitService.gitStashDrop(activeRepo, ref);
          onStatus(t("wsModal.stash.deletedStatus", { ref: ref || stashLine }));
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
      onStatus(t("wsModal.stash.savedStatus"));
      await onLoadRepo(activeRepo);
    } catch (err) {
      onStatus(String(err), true);
    }
  }

  async function handleStashPop() {
    onClose();
    try {
      await gitService.gitStashPop(activeRepo);
      onStatus(t("wsModal.stash.poppedStatus"));
      await onLoadRepo(activeRepo);
    } catch (err) {
      onStatus(String(err), true);
    }
  }

  async function handleClone() {
    const url = cloneUrl.trim();
    const target = cloneTarget.trim();
    if (!url || !target) {
      inlineError = t("wsModal.clone.requiredFields");
      return;
    }
    onClose();
    try {
      onStatus(t("wsModal.clone.cloningStatus", { url }));
      await gitService.cloneRepository(url, target, cloneAccountId || undefined);
      onStatus(t("wsModal.clone.cloneSuccess"));
    } catch (err) {
      onStatus(String(err), true);
    }
  }

  async function handleInitRepo() {
    const target = initRepoPath.trim();
    if (!target) {
      inlineError = t("wsModal.init.pathRequired");
      return;
    }
    onClose();
    try {
      onStatus(t("wsModal.init.initializingStatus", { path: target }));
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
      onStatus(remoteUrl ? t("wsModal.init.createdWithRemote") : t("wsModal.init.created"));
      await onLoadRepo(target);
    } catch (err) {
      onStatus(String(err), true);
    }
  }

  async function handleAddRemote() {
    const name = newRemoteName.trim();
    const url = newRemoteUrl.trim();
    if (!name || !url) {
      inlineError = t("wsModal.remote.requiredFields");
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
      inlineError = t("wsModal.remote.emptyUrl");
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
      t("wsModal.remote.removeTitle"),
      t("wsModal.remote.removeMsg", { name }),
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
      inlineError = t("wsModal.publish.nameRequired");
      return;
    }
    onClose();
    const targetAcc = accountOverview.accounts.find((a) => a.id === publishAccountId);
    try {
      onStatus(t("wsModal.publish.publishingStatus", { name: publishName }));
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
      onStatus(t("wsModal.publish.publishedStatus", { url: cloneUrlResult }));
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
      inlineError = ssoMatch[2] || t("wsModal.auth.ssoDefaultErr");
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
      inlineError = t("wsModal.auth.enterpriseRequired");
      return;
    }
    if (!tok) {
      inlineError = t("wsModal.auth.patRequired");
    }
    inlineError = "";
    keyringUnavailableAction = null;
    pendingSsoUrl = null;
    isVerifyingAuth = true;
    try {
      const profile =
        hostingProvider === "github"
          ? await gitService.verifyAndConnectPat(
              host,
              tok,
              accountLabelInput.trim() || undefined,
              allowMemoryFallback
            )
          : await gitService.connectProviderPat(
              hostingProvider,
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
          ? t("wsModal.auth.connectedMemory", { login: profile.login })
          : t("wsModal.auth.connectedHost", { login: profile.login, host: profile.host })
      );
    } catch (err) {
      parseAuthError(err, () => handleConnectPat(true));
    } finally {
      isVerifyingAuth = false;
    }
  }
  async function persistAiConfig(allowMemoryFallback = false): Promise<boolean> {
    aiKeyringFallbackPending = false;
    try {
      const saved = await gitService.saveAiConfig({
        provider_kind: aiConfig.provider_kind,
        endpoint: aiConfig.endpoint,
        model: aiConfig.model,
        system_prompt: aiConfig.system_prompt,
        max_tokens: aiConfig.max_tokens,
        api_key: aiApiKeyInput.trim() ? aiApiKeyInput.trim() : undefined,
        allow_memory_fallback: allowMemoryFallback,
      });
      aiConfig = saved;
      aiApiKeyInput = "";
      return true;
    } catch (err) {
      const msg = String(err);
      if (msg.includes("[KEYRING_UNAVAILABLE]")) {
        aiKeyringFallbackPending = true;
        aiTestResult = { ok: false, msg: msg.replace("[KEYRING_UNAVAILABLE]", "").trim() };
        return false;
      }
      aiTestResult = { ok: false, msg };
      return false;
    }
  }

  async function handleTestAiConnection(allowMemoryFallback = false) {
    isTestingAi = true;
    aiTestResult = null;
    try {
      const ok = await persistAiConfig(allowMemoryFallback);
      if (!ok) return;
      const res = await gitService.testAiConnection();
      aiTestResult = {
        ok: true,
        msg: t("ai.testSuccess", { model: res.model, ms: res.latency_ms }),
      };
    } catch (err) {
      aiTestResult = { ok: false, msg: String(err) };
    } finally {
      isTestingAi = false;
    }
  }

  async function handleImportSession(source: LocalSessionSource, allowMemoryFallback = false) {
    const host = effectiveHost;
    if (!host) {
      inlineError = t("wsModal.auth.hostRequired");
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
      onStatus(t("wsModal.auth.importedAccount", { login: profile.login, host: profile.host }));
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
      onStatus(t("wsModal.auth.boundRepo"));
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
      onStatus(t("wsModal.auth.defaultUpdated"));
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
      t("wsModal.auth.removeAccountTitle"),
      t("wsModal.auth.removeAccountMsg", { login: acc.login, host: acc.host }),
      async () => {
        try {
          const overview = await gitService.removeGithubAccount(
            acc.id,
            activeRepo || undefined
          );
          onAccountsChanged(overview);
          onStatus(t("wsModal.auth.removedAccountStatus", { login: acc.login, host: acc.host }));
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
      inlineError = t("wsModal.auth.enterpriseRequired");
      return;
    }
    if (!cid) {
      inlineError = t("wsModal.auth.oauthClientRequired");
    }
    inlineError = "";
    keyringUnavailableAction = null;
    pendingSsoUrl = null;
    clearPollTimer();
    localStorage.setItem(`forge_oauth_client_id:${host}`, cid);
    try {
      pollStatus = t("wsModal.auth.requestingCode");
      pollStatusClass = "";
      deviceFlow = await gitService.githubStartDeviceFlow(cid, host);
      deviceExpiresLeft = deviceFlow.expires_in || 900;
      pollStatus = t("wsModal.auth.waitingAuth");
      openExternalUrl(deviceFlow.verification_uri);

      countdownTimer = window.setInterval(() => {
        if (deviceExpiresLeft > 0) {
          deviceExpiresLeft -= 1;
        } else {
          clearPollTimer();
          pollStatus = t("wsModal.auth.codeExpired");
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
            onStatus(t("wsModal.auth.authorizedHost", { host: res.profile.host, login: res.profile.login }));
          } else if (res.status === "error") {
            clearPollTimer();
            pollStatusClass = "error";
            pollStatus = res.error || t("wsModal.auth.authFailed");
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

  const appErrorLogs = $derived(getAppErrorLogs());
  function formatLogTime(iso: string): string {
    const d = new Date(iso);
    if (Number.isNaN(d.getTime())) return iso;
    return d.toLocaleTimeString([], { hour12: false, hour: "2-digit", minute: "2-digit", second: "2-digit" });
  }

  function handleReleaseNoteClick(e: MouseEvent) {
    const target = e.target as HTMLElement | null;
    const anchor = target?.closest("a");
    const href = anchor?.getAttribute("href");
    if (href && /^https?:\/\//i.test(href)) {
      e.preventDefault();
      void openExternalUrl(href);
    }
  }

  const modalTitle = $derived.by(() => {
    switch (modal) {
      case "open": return t("wsModal.titleOpen");
      case "init": return t("wsModal.titleInit");
      case "clone": return t("wsModal.titleClone");
      case "publish": return t("wsModal.titlePublish");
      case "remote": return t("wsModal.titleRemote");
      case "stash": return t("wsModal.titleStash");
      case "diag": return t("wsModal.titleDiag");
      case "logs": return t("wsModal.titleLogs");
      case "auth":
        return authView === "add"
          ? t("wsModal.titleAuthAdd")
          : t("wsModal.titleAuthManager");
      case "settings": return t("wsModal.titleSettings");
    }
  });

  const modalWidth = $derived(
    modal === "settings"
      ? "760px"
      : modal === "stash" || modal === "logs"
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
      <button type="button" class="icon-btn compact" onclick={onClose} aria-label={t("common.closeDialog")}>
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

    <div class="modal-body {modal === 'settings' ? 'settings-modal-body' : ''}">
      {#if modal === "open"}
        {@const q = openRepoFilter.trim().toLowerCase()}
        {@const filteredRecents = q
          ? recentRepos.filter((r) => {
              const alias = (repoAliases[r] || "").toLowerCase();
              const name = (pathBaseName(r) || "").toLowerCase();
              const sum = repoSummaries[r];
              const branch = (sum?.branch || "").toLowerCase();
              return r.toLowerCase().includes(q) || alias.includes(q) || name.includes(q) || branch.includes(q);
            })
          : recentRepos}
        <div class="form-field">
          <label class="form-label" for="open-repo-path">{t("wsModal.open.pathLabel")}</label>
          <div style="display: flex; gap: 6px; align-items: center;">
            <input
              id="open-repo-path"
              type="text"
              class="form-input"
              style="flex: 1;"
              bind:value={openRepoPath}
              placeholder={t("wsModal.open.pathPlaceholder")}
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
                const picked = await browseDirectory(openRepoPath, t("wsModal.open.pickRepoTitle"), false);
                if (picked) {
                  openRepoPath = picked;
                  onClose();
                  onLoadRepo(picked);
                }
              }}
            >
              {t("common.browse")}
            </button>
          </div>
        </div>

        {#if recentRepos.length > 0}
          <div class="form-field">
            <div style="display: flex; align-items: center; justify-content: space-between; gap: 8px;">
              <span class="form-label">{t("wsModal.open.recentAndAliases")}</span>
              {#if isLoadingRepoSummaries}
                <span style="font-size: 10px; color: var(--text-muted);">{t("wsModal.open.loadingSummary")}</span>
              {/if}
            </div>
            <input
              type="search"
              class="form-input"
              aria-label={t("wsModal.open.searchPlaceholder")}
              placeholder={t("wsModal.open.searchPlaceholder")}
              bind:value={openRepoFilter}
            />
            {#if filteredRecents.length === 0}
              <div style="padding: 14px; text-align: center; font-size: 12px; color: var(--text-muted); border: 1px solid var(--border-default); border-radius: var(--radius-sm);">
                {t("wsModal.open.emptyFilter")}
              </div>
            {:else}
              <div class="repo-list-box">
                {#each filteredRecents as r (r)}
                  {@const sum = repoSummaries[r]}
                  {@const isMissing = sum ? !sum.exists : Boolean(repoMissingStatus[r])}
                  {@const alias = repoAliases[r]}
                  {@const iconKind = isMissing ? "missing" : sum?.provider || "local"}
                  {@const pushCount = sum ? (sum.has_upstream ? sum.ahead : sum.unpushed_commits) : 0}
                  <div class="repo-card {r === activeRepo ? 'is-active' : ''} {isMissing ? 'is-missing' : ''}">
                    <div class="repo-card-top">
                      <button
                        type="button"
                        class="repo-card-trigger"
                        disabled={isMissing}
                        onclick={() => {
                          onClose();
                          onLoadRepo(r);
                        }}
                      >
                        <span class="repo-card-icon">
                          <ProviderIcon provider={iconKind} size={17} />
                        </span>
                        <span class="repo-card-body">
                          <span class="repo-card-title-row">
                            <span class="repo-card-name">{alias || pathBaseName(r) || r}</span>
                            {#if alias}
                              <span style="font-size: 10px; color: var(--text-muted); font-family: var(--font-mono);">
                                ({pathBaseName(r)})
                              </span>
                            {/if}
                            {#if r === activeRepo}
                              <span class="badge-accent">{t("wsModal.open.activeBadge")}</span>
                            {/if}
                            {#if isMissing}
                              <span class="badge-warn">{t("wsModal.open.missingBadge")}</span>
                            {/if}
                          </span>
                          <span class="repo-card-path">{r}</span>
                          {#if sum && !isMissing && sum.is_git}
                            <span class="repo-status-row">
                              {#if sum.branch}
                                <span class="repo-chip repo-chip-branch">
                                  <svg width="10" height="10" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
                                    <path d="M9.5 3.25a2.25 2.25 0 1 1 3 2.122V6A2.5 2.5 0 0 1 10 8.5H6a1 1 0 0 0-1 1v1.128a2.251 2.251 0 1 1-1.5 0V5.372a2.25 2.25 0 1 1 1.5 0v1.836A2.493 2.493 0 0 1 6 7h4a1 1 0 0 0 1-1v-.628A2.25 2.25 0 0 1 9.5 3.25Z" />
                                  </svg>
                                  {sum.branch}
                                </span>
                              {/if}
                              {#if sum.dirty_count > 0}
                                <span class="repo-chip repo-chip-dirty">
                                  <svg width="10" height="10" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
                                    <path d="M11.013 1.427a1.75 1.75 0 0 1 2.474 0l1.086 1.086a1.75 1.75 0 0 1 0 2.474l-8.61 8.61c-.21.21-.47.364-.756.445l-3.251.93a.75.75 0 0 1-.927-.928l.929-3.25c.081-.286.235-.547.445-.758l8.61-8.61Z" />
                                  </svg>
                                  {t("wsModal.open.dirtyBadge", { count: sum.dirty_count })}
                                </span>
                              {/if}
                              {#if pushCount > 0}
                                <span class="repo-chip repo-chip-push">
                                  <svg width="10" height="10" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
                                    <path d="M3.47 7.78a.75.75 0 0 1 0-1.06l4.25-4.25a.75.75 0 0 1 1.06 0l4.25 4.25a.751.751 0 0 1-.018 1.042.751.751 0 0 1-1.042.018L9 4.81v7.44a.75.75 0 0 1-1.5 0V4.81L4.53 7.78a.75.75 0 0 1-1.06 0Z" />
                                  </svg>
                                  {t("wsModal.open.pushBadge", { count: pushCount })}
                                </span>
                              {/if}
                              {#if sum.behind > 0}
                                <span class="repo-chip repo-chip-pull">
                                  <svg width="10" height="10" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
                                    <path d="M12.53 8.22a.75.75 0 0 1 0 1.06l-4.25 4.25a.75.75 0 0 1-1.06 0L2.97 9.28a.751.751 0 0 1 .018-1.042.751.751 0 0 1 1.042-.018L7 11.19V3.75a.75.75 0 0 1 1.5 0v7.44l2.97-2.97a.75.75 0 0 1 1.06 0Z" />
                                  </svg>
                                  {t("wsModal.open.pullBadge", { count: sum.behind })}
                                </span>
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
                      </button>

                      <div class="repo-card-actions">
                        {#if isMissing}
                          <button
                            type="button"
                            class="btn-secondary btn-xs"
                            onclick={(e) => {
                              e.stopPropagation();
                              relocatingPathFor = relocatingPathFor === r ? null : r;
                              relocateInput = r;
                            }}
                          >
                            {t("wsModal.open.relocateBtn")}
                          </button>
                        {/if}
                        <button
                          type="button"
                          class="btn-secondary btn-xs"
                          onclick={(e) => {
                            e.stopPropagation();
                            editingAliasFor = editingAliasFor === r ? null : r;
                            aliasInput = alias || "";
                          }}
                        >
                          {t("wsModal.open.aliasBtn")}
                        </button>
                        <button
                          type="button"
                          class="btn-secondary btn-xs btn-danger"
                          aria-label={t("wsModal.open.removeAria", { path: r })}
                          onclick={(e) => {
                            e.stopPropagation();
                            onRemoveRecent(r);
                          }}
                        >
                          {t("common.delete")}
                        </button>
                      </div>
                    </div>

                    {#if editingAliasFor === r}
                      <div style="display: flex; gap: 6px; padding-top: 2px;">
                        <input
                          type="text"
                          class="form-input"
                          aria-label={t("wsModal.open.aliasAria", { path: r })}
                          placeholder={t("wsModal.open.aliasPlaceholder")}
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
                          {t("common.save")}
                        </button>
                      </div>
                    {/if}

                    {#if relocatingPathFor === r}
                      <div style="display: flex; gap: 6px; padding-top: 2px;">
                        <input
                          type="text"
                          class="form-input"
                          aria-label={t("wsModal.open.relocateAria", { path: r })}
                          placeholder={t("wsModal.open.relocatePlaceholder")}
                          bind:value={relocateInput}
                          style="flex: 1; font-size: 11px;"
                        />
                        <button
                          type="button"
                          class="btn-secondary btn-xs"
                          disabled={isBrowsingDir}
                          onclick={async () => {
                            const picked = await browseDirectory(relocateInput, t("wsModal.open.relocatePickerTitle"), false);
                            if (picked) relocateInput = picked;
                          }}
                        >
                          {t("common.browse")}
                        </button>
                        <button
                          type="button"
                          class="btn-primary btn-xs"
                          onclick={async () => {
                            await onRelocateRepo(r, relocateInput);
                            relocatingPathFor = null;
                          }}
                        >
                          {t("wsModal.open.moveBtn")}
                        </button>
                      </div>
                    {/if}
                  </div>
                {/each}
              </div>
            {/if}
          </div>
        {/if}

      {:else if modal === "init"}
        <div class="form-field">
          <label class="form-label" for="init-repo-path">{t("wsModal.init.dirLabel")}</label>
          <div style="display: flex; gap: 6px; align-items: center;">
            <input
              id="init-repo-path"
              type="text"
              class="form-input"
              style="flex: 1;"
              bind:value={initRepoPath}
              placeholder={t("wsModal.init.dirPlaceholder")}
            />
            <button
              type="button"
              class="btn-secondary"
              disabled={isBrowsingDir}
              onclick={async () => {
                const picked = await browseDirectory(initRepoPath, t("wsModal.init.pickDirTitle"), true);
                if (picked) initRepoPath = picked;
              }}
            >
              {t("common.browse")}
            </button>
          </div>
        </div>
        <div class="form-field">
          <label class="form-label" for="init-default-branch">{t("wsModal.init.defaultBranchLabel")}</label>
          <input id="init-default-branch" type="text" class="form-input" bind:value={initDefaultBranch} placeholder="main" />
        </div>
        <div class="form-field">
          <label class="form-label" for="init-gitignore">{t("wsModal.init.gitignoreLabel")}</label>
          <select id="init-gitignore" class="form-select" bind:value={initGitignore}>
            <option value="None">{t("common.none")}</option>
            <option value="Node">Node.js</option>
            <option value="Rust">Rust</option>
            <option value="Python">Python</option>
          </select>
        </div>
        <div class="form-field">
          <label class="form-label" for="init-license">{t("wsModal.init.licenseLabel")}</label>
          <select id="init-license" class="form-select" bind:value={initLicense}>
            <option value="None">{t("common.none")}</option>
            <option value="MIT">{t("wsModal.init.licenseMit")}</option>
            <option value="Apache-2.0">{t("wsModal.init.licenseApache")}</option>
          </select>
        </div>
        <div class="form-field">
          <label class="form-label" for="init-remote-url">{t("wsModal.init.remoteUrlLabel")}</label>
          <input id="init-remote-url" type="text" class="form-input" bind:value={initRemoteUrl} placeholder={t("wsModal.init.remoteUrlPlaceholder")} />
        </div>

      {:else if modal === "clone"}
        <div class="form-field">
          <label class="form-label" for="clone-url">{t("wsModal.clone.urlLabel")}</label>
          <input id="clone-url" type="text" class="form-input" bind:value={cloneUrl} placeholder="https://github.com/user/repo.git" />
        </div>
        <div class="form-field">
          <label class="form-label" for="clone-target">{t("wsModal.clone.targetLabel")}</label>
          <div style="display: flex; gap: 6px; align-items: center;">
            <input
              id="clone-target"
              type="text"
              class="form-input"
              style="flex: 1;"
              bind:value={cloneTarget}
              placeholder={t("wsModal.clone.targetPlaceholder")}
            />
            <button
              type="button"
              class="btn-secondary"
              disabled={isBrowsingDir}
              onclick={async () => {
                const picked = await browseDirectory(cloneTarget, t("wsModal.clone.pickTargetTitle"), true);
                if (!picked) return;
                const repoName = deriveRepoNameFromUrl(cloneUrl);
                if (repoName && pathBaseName(picked).toLowerCase() !== repoName.toLowerCase()) {
                  cloneTarget = joinRepoPath(picked, repoName);
                } else {
                  cloneTarget = picked;
                }
              }}
            >
              {t("common.browse")}
            </button>
          </div>
        </div>
        {#if accountOverview.accounts.length > 0}
          <div class="form-field">
            <label class="form-label" for="clone-account">{t("wsModal.clone.accountLabel")}</label>
            <select id="clone-account" class="form-select" bind:value={cloneAccountId}>
              <option value="">{t("wsModal.clone.accountAuto")}</option>
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
            <span>{t("wsModal.publish.noAccountNotice")}</span>
            <button type="button" class="btn-secondary btn-xs btn-accent" onclick={onOpenAuth}>
              {t("wsModal.publish.connectAccountBtn")}
            </button>
          </div>
        {:else if accountOverview.accounts.length > 0}
          <div class="form-field">
            <label class="form-label" for="publish-account">{t("wsModal.publish.targetAccountLabel")}</label>
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
          {t("wsModal.publish.intro")}
        </p>
        {#if !hasCommits}
          <div class="info-callout">
            {t("wsModal.publish.noCommitsPrefix")}<code>origin</code>{t("wsModal.publish.noCommitsSuffix")}
          </div>
        {/if}
        <div class="form-field">
          <label class="form-label" for="publish-name">{t("wsModal.publish.nameLabel")}</label>
          <input id="publish-name" type="text" class="form-input" bind:value={publishName} placeholder={t("wsModal.publish.namePlaceholder")} />
        </div>
        <div class="form-field">
          <label class="form-label" for="publish-desc">{t("wsModal.publish.descLabel")}</label>
          <textarea id="publish-desc" class="form-input" bind:value={publishDesc} rows="2" placeholder={t("wsModal.publish.descPlaceholder")} style="resize: vertical;"></textarea>
        </div>
        <label class="checkbox-label">
          <input type="checkbox" bind:checked={publishPrivate} />
          <span>{t("wsModal.publish.privateLabel")}</span>
        </label>


      {:else if modal === "remote"}
        {#if (remotes ?? []).length === 0}
          <div class="info-callout">
            {t("wsModal.remote.emptyNoticePrefix")}<code style="font-family: var(--font-mono);">origin</code>{t("wsModal.remote.emptyNoticeSuffix")}
          </div>
        {:else}
          <div class="form-field">
            <span class="form-label">{t("wsModal.remote.connectedList", { count: (remotes ?? []).length })}</span>
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
                        {t("wsModal.remote.editUrlBtn")}
                      </button>
                      <button
                        type="button"
                        class="btn-secondary btn-xs btn-danger"
                        disabled={isSubmittingRemote}
                        onclick={() => handleRemoveRemote(r.name)}
                      >
                        {t("common.delete")}
                      </button>
                    </div>
                  </div>
                  {#if editingRemoteName === r.name}
                    <div style="display: flex; gap: 6px; padding-top: 4px; border-top: 1px solid var(--border-muted);">
                      <input
                        type="text"
                        class="form-input"
                        style="flex: 1; font-size: 11px; font-family: var(--font-mono);"
                        aria-label={t("wsModal.remote.editUrlAria", { name: r.name })}
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
                        {t("common.save")}
                      </button>
                      <button
                        type="button"
                        class="btn-secondary btn-xs"
                        disabled={isSubmittingRemote}
                        onclick={() => (editingRemoteName = null)}
                      >
                        {t("common.cancel")}
                      </button>
                    </div>
                  {/if}
                </div>
              {/each}
            </div>
          </div>
        {/if}

        <div class="modal-section-heading">{t("wsModal.remote.addSection")}</div>
        <div class="remote-add-grid">
          <div class="form-field">
            <label class="form-label" for="new-remote-name">{t("wsModal.remote.nameLabel")}</label>
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
            <label class="form-label" for="new-remote-url">{t("wsModal.remote.urlLabel")}</label>
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
            {isSubmittingRemote ? t("common.saving") : t("wsModal.remote.addBtn")}
          </button>
        </div>
      {:else if modal === "stash"}
        <div style="display: flex; gap: 6px;">
          <input
            type="text"
            class="form-input"
            aria-label={t("wsModal.stash.msgAria")}
            bind:value={stashMsg}
            placeholder={t("wsModal.stash.msgPlaceholder")}
            style="flex: 1;"
          />
          <button type="button" onclick={handleStashSave} class="btn-primary">{t("wsModal.stash.saveBtn")}</button>
        </div>

        <div class="stash-layout">
          <div style="display: flex; flex-direction: column; gap: 6px; min-height: 0;">
            <div style="display: flex; justify-content: space-between; align-items: center;">
              <span class="form-label tabular-nums">{t("wsModal.stash.listTitle", { count: stashList.length })}</span>
              <button
                type="button"
                onclick={handleStashPop}
                class="btn-secondary btn-xs"
                disabled={stashList.length === 0}
              >
                {t("wsModal.stash.popTopBtn")}
              </button>
            </div>
            <div class="list-box" style="flex: 1; max-height: none;">
              {#if stashList.length === 0}
                <div style="color: var(--text-muted); text-align: center; padding: 24px 12px; font-size: 11px;">
                  {t("wsModal.stash.emptyList")}
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
                      title={t("wsModal.stash.dropTitle")}
                      onclick={() => handleDropStash(s)}
                    >
                      {t("wsModal.stash.dropBtn")}
                    </button>
                  </div>
                {/each}
              {/if}
            </div>
          </div>

          <div style="display: flex; flex-direction: column; min-height: 0;">
            <span class="form-label" style="margin-bottom: 6px;">{t("wsModal.stash.previewLabel")}</span>
            <div style="flex: 1; overflow: auto; background: var(--bg-canvas); border: 1px solid var(--border-default); border-radius: var(--radius-sm); padding: 8px; font-family: var(--font-mono); font-size: 11px; white-space: pre-wrap; user-select: text;">
              {#if isLoadingStashDiff}
                <div style="color: var(--text-muted); text-align: center; padding: 24px;">{t("wsModal.stash.loadingDiff")}</div>
              {:else if stashDiff}
                {stashDiff}
              {:else}
                <div style="color: var(--text-muted); text-align: center; padding: 24px;">
                  {t("wsModal.stash.selectPrompt")}
                </div>
              {/if}
            </div>
          </div>
        </div>

      {:else if modal === "diag"}
        {#if isLoadingDiag && !diagReport}
          <div class="info-callout">{t("wsModal.diag.collecting")}</div>
        {:else}
          <div style="display: flex; flex-direction: column; gap: 8px; font-size: 12px;">
            <div><strong>{t("wsModal.diag.appLabel")}</strong> Forge Desktop v{diagReport?.app_version || "-"}</div>
            <div><strong>{t("wsModal.diag.osLabel")}</strong> {diagReport?.os_info || "-"}</div>
            <div><strong>{t("wsModal.diag.gitVersionLabel")}</strong> {diagReport?.git_version || "-"}</div>
            <div class="form-label" style="margin-top: 4px;">{t("wsModal.diag.sanitizedConfigLabel")}</div>
            <pre style="background: var(--bg-app); border: 1px solid var(--border-default); padding: 10px; border-radius: var(--radius-sm); max-height: 200px; overflow: auto; font-family: var(--font-mono); font-size: 11px; user-select: text;">{diagReport?.sanitized_config.join("\n") || t("wsModal.diag.noConfig")}</pre>
          </div>
        {/if}

      {:else if modal === "logs"}
        <div class="app-logs-toolbar">
          <span class="app-logs-summary">
            {appErrorLogs.length === 0
              ? t("wsModal.logs.emptyCount")
              : appErrorLogs.length === 1
                ? t("wsModal.logs.countOne", { count: appErrorLogs.length })
                : t("wsModal.logs.countMany", { count: appErrorLogs.length })}
          </span>
          {#if appErrorLogs.length > 0}
            <span class="app-logs-hint">{t("wsModal.logs.sessionHint")}</span>
          {/if}
        </div>
        {#if appErrorLogs.length === 0}
          <div class="app-logs-empty">
            <svg width="20" height="20" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
              <path d="M8 16A8 8 0 1 1 8 0a8 8 0 0 1 0 16Zm3.78-9.72a.751.751 0 0 0-.018-1.042.751.751 0 0 0-1.042-.018L6.75 9.19 5.28 7.72a.751.751 0 0 0-1.042.018.751.751 0 0 0-.018 1.042l2 2a.75.75 0 0 0 1.06 0Z" />
            </svg>
            <span>{t("wsModal.logs.emptyState")}</span>
          </div>
        {:else}
          <div class="app-logs-list" role="log" aria-live="polite">
            {#each appErrorLogs as entry (entry.id)}
              <div class="app-log-item">
                <div class="app-log-meta">
                  <span class="app-log-time" title={entry.timestamp}>{formatLogTime(entry.timestamp)}</span>
                  <span class="app-log-source">{entry.source}</span>
                </div>
                <div class="app-log-message">{entry.message}</div>
                {#if entry.detail}
                  <pre class="app-log-detail">{entry.detail}</pre>
                {/if}
              </div>
            {/each}
          </div>
        {/if}

      {:else if modal === "auth"}
        {#if authView === "manager" && accountOverview.accounts.length > 0}
          <div style="display: flex; justify-content: space-between; align-items: center; gap: 8px;">
            <span style="font-size: 12px; color: var(--text-secondary);">
              {accountOverview.accounts.length === 1
                ? t("wsModal.auth.accountsCountOne", { count: accountOverview.accounts.length })
                : t("wsModal.auth.accountsCountMany", { count: accountOverview.accounts.length })}
            </span>
            <button
              type="button"
              class="btn-primary btn-xs"
              onclick={() => openAddAccountSheet(githubRepoInfo?.host)}
            >
              {t("wsModal.auth.addAccountBtn")}
            </button>
          </div>

          {#if authorMismatch}
            <div class="info-callout" style="display: flex; justify-content: space-between; align-items: center; gap: 10px;">
              <span>
                {t("wsModal.auth.authorMismatchPrefix")}<strong>{authorMismatch.name}</strong>{authorMismatch.email ? ` <${authorMismatch.email}>` : ""}{t("wsModal.auth.authorMismatchSuffix")}
              </span>
              <button
                type="button"
                class="btn-secondary btn-xs btn-accent"
                style="flex-shrink: 0;"
                onclick={() => onSyncRepoAuthor(authorMismatch!.name, authorMismatch!.email)}
              >
                {t("wsModal.auth.applyToRepoBtn")}
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
                    <img src={acc.avatar_url} alt={t("wsModal.auth.avatarAlt", { login: acc.login })} class="account-avatar" />
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
                        <span class="status-badge-ok">{t("wsModal.auth.activeInRepoBadge")}</span>
                      {:else if isDefault}
                        <span class="badge-neutral">{t("wsModal.auth.defaultBadge")} {acc.host === "github.com" ? "" : acc.host}</span>
                      {/if}
                      {#if acc.memory_only}
                        <span class="badge-neutral">{t("wsModal.auth.memorySessionBadge")}</span>
                      {/if}
                      {#if health?.status === "expired"}
                        <span class="badge-warn">{t("wsModal.auth.expiredBadge")}</span>
                      {:else if health?.status === "sso_required"}
                        <span class="badge-warn">{t("wsModal.auth.ssoRequiredBadge")}</span>
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
                        {t("wsModal.auth.updateTokenBtn")}
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
                        {t("wsModal.auth.authorizeSsoBtn")}
                      </button>
                      <button
                        type="button"
                        class="btn-secondary btn-xs"
                        onclick={() => handleCheckHealth(acc.id)}
                      >
                        {t("wsModal.auth.recheckBtn")}
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
                      placeholder={t("wsModal.auth.labelPlaceholder")}
                    />
                    <button type="button" class="btn-primary btn-xs" onclick={() => handleSaveLabel(acc.id)}>
                      {t("common.save")}
                    </button>
                    <button type="button" class="btn-secondary btn-xs" onclick={() => (editingLabelFor = null)}>
                      {t("common.cancel")}
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
                        {t("wsModal.auth.useInRepoBtn")}
                      </button>
                    {/if}
                    {#if !isDefault}
                      <button
                        type="button"
                        class="btn-secondary btn-xs"
                        onclick={() => handleSetDefaultAccount(acc.id)}
                      >
                        {t("wsModal.auth.setDefaultBtn")}
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
                      {t("wsModal.auth.labelBtn")}
                    </button>
                    <button
                      type="button"
                      class="btn-secondary btn-xs"
                      disabled={checkingHealthFor === acc.id}
                      onclick={() => handleCheckHealth(acc.id)}
                    >
                      {checkingHealthFor === acc.id ? t("wsModal.auth.checkingSessionBtn") : t("wsModal.auth.checkSessionBtn")}
                    </button>
                  </div>

                  <div style="display: flex; gap: 5px;">
                    <button
                      type="button"
                      class="btn-secondary btn-xs"
                      onclick={() =>
                        openExternalUrl(acc.html_url || `https://${acc.host}/${acc.login}`)}
                    >
                      {t("wsModal.auth.profileBtn")}
                    </button>
                    <button
                      type="button"
                      class="btn-secondary btn-xs btn-danger"
                      onclick={() => handleRemoveAccount(acc)}
                    >
                      {t("common.delete")}
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
                {t("wsModal.auth.backToAccountsBtn")}
              </button>
            </div>
          {/if}

          <div class="form-field">
            <span class="form-label">{t("wsModal.auth.providerLabel")}</span>
            <div class="seg-group" role="radiogroup" aria-label={t("wsModal.auth.providerLabel")}>
              <button
                type="button"
                class="seg-btn {hostingProvider === 'github' ? 'active' : ''}"
                onclick={() => { hostingProvider = "github"; inlineError = ""; authTab = "token"; }}
              >GitHub</button>
              <button
                type="button"
                class="seg-btn {hostingProvider === 'gitlab' ? 'active' : ''}"
                onclick={() => { hostingProvider = "gitlab"; inlineError = ""; authTab = "token"; }}
              >GitLab</button>
              <button
                type="button"
                class="seg-btn {hostingProvider === 'bitbucket' ? 'active' : ''}"
                onclick={() => { hostingProvider = "bitbucket"; inlineError = ""; authTab = "token"; }}
              >Bitbucket</button>
              <button
                type="button"
                class="seg-btn {hostingProvider === 'azure_devops' ? 'active' : ''}"
                onclick={() => { hostingProvider = "azure_devops"; inlineError = ""; authTab = "token"; }}
              >Azure DevOps</button>
            </div>
          </div>

          <div class="form-grid-2">
            {#if hostingProvider === "github"}
              <div class="form-field">
                <span class="form-label">{t("wsModal.auth.githubServerLabel")}</span>
                <div class="seg-group" role="radiogroup" aria-label={t("wsModal.auth.serverTypeAria")}>
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
                    {t("wsModal.auth.serverGithub")}
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
                    {t("wsModal.auth.serverEnterprise")}
                  </button>
                </div>
              </div>
            {:else}
              <div class="form-field">
                <label class="form-label" for="provider-host-input">{t("wsModal.auth.providerHostLabel")}</label>
                <input
                  id="provider-host-input"
                  type="text"
                  class="form-input"
                  style="font-family: var(--font-mono);"
                  bind:value={providerHostInput}
                  placeholder={hostingProvider === "gitlab" ? "gitlab.com" : hostingProvider === "bitbucket" ? "bitbucket.org" : "dev.azure.com"}
                />
              </div>
            {/if}

            <div class="form-field">
              <label class="form-label" for="account-label-input">{t("wsModal.auth.labelOptional")}</label>
              <input
                id="account-label-input"
                type="text"
                class="form-input"
                bind:value={accountLabelInput}
                placeholder={t("wsModal.auth.labelExamplePlaceholder")}
              />
            </div>
          </div>

          {#if hostingProvider === "github" && hostKind === "enterprise"}
            <div class="form-field">
              <label class="form-label" for="enterprise-host-input">{t("wsModal.auth.enterpriseHostLabel")}</label>
              <input
                id="enterprise-host-input"
                type="text"
                class="form-input"
                style="font-family: var(--font-mono);"
                bind:value={enterpriseHostInput}
                placeholder={t("wsModal.auth.enterpriseHostPlaceholder")}
                onblur={() => {
                  if (effectiveHost) {
                    oauthClientId =
                      localStorage.getItem(`forge_oauth_client_id:${effectiveHost}`) || "";
                    if (authTab === "import") refreshDetectedSessions();
                  }
                }}
              />
              <span style="font-size: 11px; color: var(--text-muted); font-family: var(--font-mono);">
                {t("wsModal.auth.apiEndpointPrefix")} https://{effectiveHost || t("wsModal.auth.enterpriseHostPlaceholder")}/api/v3
              </span>
            </div>
          {/if}

          {#if hostingProvider === "github"}
            <div class="seg-group" style="align-self: flex-start;" role="tablist" aria-label={t("wsModal.auth.authMethodAria")}>
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
                {t("wsModal.auth.tabToken")}
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
                {t("wsModal.auth.tabOauth")}
              </button>
              <button
                type="button"
                role="tab"
                aria-selected={authTab === 'import'}
                class="seg-btn {authTab === 'import' ? 'active' : ''}"
                onclick={() => {
                  authTab = "import";
                  inlineError = "";
                  refreshDetectedSessions();
                }}
              >
                {t("wsModal.auth.tabImport")}
              </button>
            </div>
          {/if}

          {#if keyringUnavailableAction}
            <div class="warn-callout">
              <span>{t("wsModal.auth.keyringNotice")}</span>
              <div>
                <button
                  type="button"
                  class="btn-primary btn-xs"
                  onclick={() => keyringUnavailableAction?.()}
                >
                  {t("wsModal.auth.saveMemoryOnlyBtn")}
                </button>
              </div>
            </div>
          {/if}

          {#if pendingSsoUrl}
            <div class="warn-callout">
              <span>{t("wsModal.auth.ssoNotice")}</span>
              <div>
                <button
                  type="button"
                  class="btn-primary btn-xs"
                  onclick={() => openExternalUrl(pendingSsoUrl!)}
                >
                  {t("wsModal.auth.authorizeSsoBtn")}
                </button>
              </div>
            </div>
          {/if}

          {#if authTab === "token"}
            {#if hostingProvider === "github"}
              <div class="info-callout" style="display: flex; flex-direction: column; gap: 6px;">
                <span><strong>{t("wsModal.auth.patGuideTitle")}</strong></span>
                <span>{t("wsModal.auth.patGuideStep1Prefix")}<code style="font-family: var(--font-mono);">repo</code>, <code style="font-family: var(--font-mono);">read:user</code>, &amp; <code style="font-family: var(--font-mono);">user:email</code>{t("wsModal.auth.patGuideStep1Suffix")}</span>
                <span>{t("wsModal.auth.patGuideStep2")}</span>
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
                    {t("wsModal.auth.createTokenOnBtn", { host: effectiveHost || "github.com" })}
                  </button>
                </div>
              </div>
            {:else}
              <div class="info-callout" style="display: flex; flex-direction: column; gap: 6px;">
                <span><strong>{t("wsModal.auth.providerPatGuideTitle", { provider: hostingProvider })}</strong></span>
                <div style="padding-top: 2px;">
                  <button
                    type="button"
                    class="btn-secondary btn-xs btn-accent"
                    disabled={!effectiveHost}
                    onclick={() => {
                      const h = effectiveHost || "gitlab.com";
                      const url =
                        hostingProvider === "gitlab"
                          ? `https://${h}/-/user_settings/personal_access_tokens`
                          : hostingProvider === "bitbucket"
                            ? `https://${h}/account/settings/app-passwords`
                            : `https://${h}/_usersSettings/tokens`;
                      openExternalUrl(url);
                    }}
                  >
                    {t("wsModal.auth.createTokenOnBtn", { host: effectiveHost })}
                  </button>
                </div>
              </div>
            {/if}

            <div class="form-field">
              <label class="form-label" for="github-pat-input">{t("wsModal.auth.patLabel")}</label>
              <div style="display: flex; gap: 6px; align-items: center;">
                <input
                  id="github-pat-input"
                  type={showPat ? "text" : "password"}
                  class="form-input"
                  style="flex: 1; font-family: var(--font-mono);"
                  bind:value={patInput}
                  placeholder={t("wsModal.auth.patPlaceholder")}
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
                  {showPat ? t("wsModal.auth.hidePat") : t("wsModal.auth.showPat")}
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
                {isVerifyingAuth ? t("wsModal.auth.verifying") : t("wsModal.auth.verifyAndSaveBtn")}
              </button>
            </div>
          {:else if authTab === "oauth"}
            {#if !deviceFlow}
              <div class="info-callout">
                {t("wsModal.auth.oauthInfoPrefix")}<strong>Enable Device Flow</strong>{t("wsModal.auth.oauthInfoSuffix")}
              </div>
              <div class="form-field">
                <label class="form-label" for="oauth-client-id">{t("wsModal.auth.oauthClientIdLabel")}</label>
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
                  {t("wsModal.auth.openDevSettingsBtn")}
                </button>
                <button
                  type="button"
                  onclick={() => handleStartDeviceFlow(false)}
                  class="btn-primary"
                  disabled={!oauthClientId.trim() || !effectiveHost}
                >
                  {t("wsModal.auth.requestCodeBtn")}
                </button>
              </div>
            {:else}
              <div style="display: flex; flex-direction: column; gap: 10px; font-size: 12px;">
                <div style="display: flex; justify-content: space-between; align-items: center;">
                  <span>{t("wsModal.auth.enterCodePrompt")}</span>
                  {#if deviceExpiresLeft > 0}
                    <span style="font-family: var(--font-mono); font-size: 11px; color: var(--text-muted);">
                      {t("wsModal.auth.timeRemaining", { time: formatCountdown(deviceExpiresLeft) })}
                    </span>
                  {/if}
                </div>
                <div class="oauth-code-box">
                  <span class="oauth-code-value">{deviceFlow.user_code}</span>
                  <button type="button" class="btn-secondary btn-xs" onclick={copyDeviceCode}>
                    {copiedCode ? t("wsModal.auth.copied") : t("wsModal.auth.copyCode")}
                  </button>
                </div>
                <div style="display: flex; justify-content: space-between; align-items: center;">
                  <button
                    type="button"
                    class="link-btn"
                    onclick={() => openExternalUrl(deviceFlow!.verification_uri)}
                  >
                    {t("wsModal.auth.reopenUrl", { url: deviceFlow.verification_uri })}
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
                    {t("common.cancel")}
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
                  {t("wsModal.auth.importIntroPrefix")}<code style="font-family: var(--font-mono);">gh</code>{t("wsModal.auth.importIntroMid")}<code style="font-family: var(--font-mono);">~/.git-credentials</code>{t("wsModal.auth.importIntroPost")}<strong>{effectiveHost || "github.com"}</strong>:
                </span>
                <button
                  type="button"
                  class="btn-secondary btn-xs"
                  disabled={isDetectingSessions || !effectiveHost}
                  onclick={refreshDetectedSessions}
                >
                  {isDetectingSessions ? t("wsModal.auth.scanning") : t("wsModal.auth.rescan")}
                </button>
              </div>

              {#if isDetectingSessions}
                <div class="info-callout">{t("wsModal.auth.scanningLocal")}</div>
              {:else if detectedSessions.length === 0}
                <div class="info-callout">
                  {t("wsModal.auth.noLocalSessionPrefix", { host: effectiveHost || "github.com" })}<strong>{t("wsModal.auth.tabToken")}</strong>{t("wsModal.auth.noLocalSessionMid")}<strong>{t("wsModal.auth.tabOauth")}</strong>{t("wsModal.auth.noLocalSessionSuffix")}
                </div>
              {:else}
                <div class="list-box">
                  {#each detectedSessions as sess (sess.source + sess.host)}
                    <div class="list-row">
                      <div style="display: flex; flex-direction: column; min-width: 0;">
                        <span style="font-weight: 600; font-size: 12px;">{sess.display}</span>
                        <span style="font-size: 11px; color: var(--text-muted);">
                          {t("wsModal.auth.verifiedByBackend", { host: sess.host })}
                        </span>
                      </div>
                      <button
                        type="button"
                        class="btn-primary btn-xs"
                        disabled={isVerifyingAuth}
                        onclick={() => handleImportSession(sess.source, false)}
                      >
                        {isVerifyingAuth ? t("wsModal.auth.importing") : t("wsModal.auth.importAndVerify")}
                      </button>
                    </div>
                  {/each}
                </div>
              {/if}
            </div>
          {/if}
        {/if}
      {:else if modal === "settings"}
        <div class="settings-shell">
          <div class="settings-nav" role="tablist" aria-orientation="vertical" aria-label={t("wsModal.settings.navAria")}>
            <button
              id="settings-tab-git"
              type="button"
              role="tab"
              aria-selected={settingsTab === "git"}
              aria-controls="settings-panel-git"
              tabindex={settingsTab === "git" ? 0 : -1}
              class="settings-nav-item {settingsTab === 'git' ? 'active' : ''}"
              onclick={() => (settingsTab = "git")}
              onkeydown={(e) => handleSettingsTabKeydown(e, "git")}
            >
              <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
                <path d="M10.561 8.073a6.005 6.005 0 0 1 3.432 5.142.75.75 0 1 1-1.498.07 4.5 4.5 0 0 0-8.99 0 .75.75 0 0 1-1.498-.07 6.004 6.004 0 0 1 3.431-5.142 3.999 3.999 0 1 1 5.123 0ZM10.5 5a2.5 2.5 0 1 0-5 0 2.5 2.5 0 0 0 5 0Z" />
              </svg>
              <span>{t("wsModal.settings.tabGit")}</span>
            </button>
            <button
              id="settings-tab-appearance"
              type="button"
              role="tab"
              aria-selected={settingsTab === "appearance"}
              aria-controls="settings-panel-appearance"
              tabindex={settingsTab === "appearance" ? 0 : -1}
              class="settings-nav-item {settingsTab === 'appearance' ? 'active' : ''}"
              onclick={() => (settingsTab = "appearance")}
              onkeydown={(e) => handleSettingsTabKeydown(e, "appearance")}
            >
              <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
                <path d="M1.75 2.5a.25.25 0 0 0-.25.25v8.5c0 .138.112.25.25.25h12.5a.25.25 0 0 0 .25-.25v-8.5a.25.25 0 0 0-.25-.25H1.75ZM0 2.75C0 1.784.784 1 1.75 1h12.5c.966 0 1.75.784 1.75 1.75v8.5A1.75 1.75 0 0 1 14.25 13H9.06l1.397 1.397a.75.75 0 1 1-1.06 1.06L7.94 14H4.75a.75.75 0 0 1 0-1.5h9.5V2.5H1.75v10h1a.75.75 0 0 1 0 1.5h-1A1.75 1.75 0 0 1 0 11.25v-8.5Z" />
              </svg>
              <span>{t("wsModal.settings.tabAppearance")}</span>
            </button>
            <button
              id="settings-tab-safety"
              type="button"
              role="tab"
              aria-selected={settingsTab === "safety"}
              aria-controls="settings-panel-safety"
              tabindex={settingsTab === "safety" ? 0 : -1}
              class="settings-nav-item {settingsTab === 'safety' ? 'active' : ''}"
              onclick={() => (settingsTab = "safety")}
              onkeydown={(e) => handleSettingsTabKeydown(e, "safety")}
            >
              <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
                <path d="M7.467.133a1.748 1.748 0 0 1 1.066 0l5.25 1.68A1.75 1.75 0 0 1 15 3.48V7c0 1.566-.32 3.182-1.303 4.682-.983 1.498-2.585 2.813-5.032 3.855a1.697 1.697 0 0 1-1.33 0c-2.447-1.042-4.049-2.357-5.032-3.855C1.32 10.182 1 8.566 1 7V3.48a1.75 1.75 0 0 1 1.217-1.667Zm.61 1.429a.25.25 0 0 0-.153 0l-5.25 1.68a.25.25 0 0 0-.174.238V7c0 1.358.275 2.666 1.057 3.86.784 1.194 2.121 2.34 4.366 3.297a.196.196 0 0 0 .154 0c2.245-.957 3.582-2.103 4.366-3.297C13.225 9.666 13.5 8.358 13.5 7V3.48a.25.25 0 0 0-.174-.237Z" />
              </svg>
              <span>{t("wsModal.settings.tabSafety")}</span>
            </button>
            <button
              id="settings-tab-updates"
              type="button"
              role="tab"
              aria-selected={settingsTab === "updates"}
              aria-controls="settings-panel-updates"
              tabindex={settingsTab === "updates" ? 0 : -1}
              class="settings-nav-item {settingsTab === 'updates' ? 'active' : ''}"
              onclick={() => (settingsTab = "updates")}
              onkeydown={(e) => handleSettingsTabKeydown(e, "updates")}
            >
              <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
                <path d="M1.705 8.005a.75.75 0 0 1 .834.656 5.5 5.5 0 0 0 9.592 2.97l-1.204-1.204a.25.25 0 0 1 .177-.427h3.646a.25.25 0 0 1 .25.25v3.646a.25.25 0 0 1-.427.177l-1.38-1.38A7.002 7.002 0 0 1 1.05 8.84a.75.75 0 0 1 .656-.834ZM8 2.5a5.487 5.487 0 0 0-4.131 1.869l1.204 1.204A.25.25 0 0 1 4.896 6H1.25A.25.25 0 0 1 1 5.75V2.104a.25.25 0 0 1 .427-.177l1.38 1.38A7.002 7.002 0 0 1 14.95 7.16a.75.75 0 0 1-1.49.178A5.5 5.5 0 0 0 8 2.5Z" />
              </svg>
              <span>{t("wsModal.settings.tabUpdates")}</span>
            </button>
            <button
              id="settings-tab-ai"
              type="button"
              role="tab"
              aria-selected={settingsTab === "ai"}
              aria-controls="settings-panel-ai"
              tabindex={settingsTab === "ai" ? 0 : -1}
              class="settings-nav-item {settingsTab === 'ai' ? 'active' : ''}"
              onclick={() => (settingsTab = "ai")}
              onkeydown={(e) => handleSettingsTabKeydown(e, "ai")}
            >
              <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
                <path d="M7.53 1.282a.5.5 0 0 1 .94 0l1.158 3.128a2 2 0 0 0 1.186 1.186l3.128 1.158a.5.5 0 0 1 0 .94l-3.128 1.158a2 2 0 0 0-1.186 1.186L8.47 13.166a.5.5 0 0 1-.94 0l-1.158-3.128a2 2 0 0 0-1.186-1.186L2.058 7.694a.5.5 0 0 1 0-.94l3.128-1.158a2 2 0 0 0 1.186-1.186L7.53 1.282Z" />
              </svg>
              <span>{t("wsModal.settings.tabAi")}</span>
            </button>
          </div>

          <div class="settings-panel" role="tabpanel" id="settings-panel-{settingsTab}" aria-labelledby="settings-tab-{settingsTab}">
            {#if settingsTab === "git"}
              <div class="settings-panel-header">
                <h4 class="settings-panel-title">{t("wsModal.settings.tabGit")}</h4>
                <p class="settings-panel-desc">{t("wsModal.settings.tabGitDesc")}</p>
              </div>
              <div class="settings-group">
                <div class="settings-group-head">
                  <span class="settings-group-title">
                    {t("wsModal.settings.localRepoSection")}{isLoadingSettings ? t("wsModal.settings.loadingSuffix") : ""}
                  </span>
                  <span class="settings-hint">{t("wsModal.settings.localRepoHint")}</span>
                </div>
                <div class="form-grid-2">
                  <div class="form-field">
                    <label class="form-label" for="repo-user-name">{t("wsModal.settings.repoNameLabel")}</label>
                    <input id="repo-user-name" type="text" class="form-input" bind:value={draftRepoConfig.user_name} oninput={() => (settingsAuthorsDirty = true)} placeholder={t("wsModal.settings.repoNamePlaceholder")} />
                  </div>
                  <div class="form-field">
                    <label class="form-label" for="repo-user-email">{t("wsModal.settings.repoEmailLabel")}</label>
                    <input id="repo-user-email" type="text" class="form-input" bind:value={draftRepoConfig.user_email} oninput={() => (settingsAuthorsDirty = true)} placeholder={t("wsModal.settings.repoEmailPlaceholder")} />
                  </div>
                </div>
              </div>
              <div class="settings-group">
                <div class="settings-group-head">
                  <span class="settings-group-title">{t("wsModal.settings.globalGitSection")}</span>
                  <span class="settings-hint">{t("wsModal.settings.globalGitHint")}</span>
                </div>
                <div class="form-grid-2">
                  <div class="form-field">
                    <label class="form-label" for="global-user-name">{t("wsModal.settings.globalNameLabel")}</label>
                    <input id="global-user-name" type="text" class="form-input" bind:value={draftGlobalConfig.user_name} oninput={() => (settingsAuthorsDirty = true)} placeholder={t("wsModal.settings.globalNamePlaceholder")} />
                  </div>
                  <div class="form-field">
                    <label class="form-label" for="global-user-email">{t("wsModal.settings.globalEmailLabel")}</label>
                    <input id="global-user-email" type="text" class="form-input" bind:value={draftGlobalConfig.user_email} oninput={() => (settingsAuthorsDirty = true)} placeholder={t("wsModal.settings.globalEmailPlaceholder")} />
                  </div>
                </div>
              </div>
            {:else if settingsTab === "appearance"}
              <div class="settings-panel-header">
                <h4 class="settings-panel-title">{t("wsModal.settings.tabAppearance")}</h4>
                <p class="settings-panel-desc">{t("wsModal.settings.tabAppearanceDesc")}</p>
              </div>
              <div class="settings-group">
                <div class="settings-group-head">
                  <span class="settings-group-title">{t("wsModal.settings.appearanceSyncSection")}</span>
                </div>
                <div class="settings-row">
                  <label class="form-label" for="language-select">{t("wsModal.settings.languageLabel")}</label>
                  <select id="language-select" class="form-select" bind:value={draftLocale}>
                    {#each SUPPORTED_LOCALES as loc (loc.value)}
                      <option value={loc.value}>{loc.label}</option>
                    {/each}
                  </select>
                </div>
                <div class="settings-row">
                  <label class="form-label" for="theme-select">{t("wsModal.settings.themeLabel")}</label>
                  <select id="theme-select" class="form-select" bind:value={draftTheme}>
                    <option value="dark">{t("wsModal.settings.themeDark")}</option>
                    <option value="light">{t("wsModal.settings.themeLight")}</option>
                    <option value="system">{t("wsModal.settings.themeSystem")}</option>
                  </select>
                </div>
              </div>
              <div class="settings-group">
                <div class="settings-group-head">
                  <span class="settings-group-title">{t("wsModal.settings.externalEditorSection")}</span>
                </div>
                <div class="settings-row">
                  <label class="form-label" for="external-editor-select">{t("wsModal.settings.textEditorLabel")}</label>
                  <select id="external-editor-select" class="form-select" bind:value={draftEditorPreset}>
                    {#each EXTERNAL_EDITOR_PRESETS as preset (preset.value)}
                      <option value={preset.value}>{preset.label}</option>
                    {/each}
                  </select>
                </div>
                {#if draftEditorPreset === "custom"}
                  <div class="form-field">
                    <label class="form-label" for="custom-editor-input">{t("wsModal.settings.customEditorLabel")}</label>
                    <div style="display: flex; gap: 6px;">
                      <input
                        id="custom-editor-input"
                        type="text"
                        class="form-input"
                        bind:value={draftCustomEditor}
                        placeholder={t("wsModal.settings.customEditorPlaceholder")}
                        style="flex: 1;"
                      />
                      <button type="button" class="btn-secondary btn-xs" onclick={handlePickCustomEditor}>
                        {t("wsModal.settings.pickBtn")}
                      </button>
                    </div>
                  </div>
                {/if}
                <p class="settings-hint">
                  {draftEditorPreset === "system" ? t("wsModal.settings.editorSystemDesc") : t("wsModal.settings.editorCustomDesc")}
                </p>
              </div>
            {:else if settingsTab === "safety"}
              <div class="settings-panel-header">
                <h4 class="settings-panel-title">{t("wsModal.settings.tabSafety")}</h4>
                <p class="settings-panel-desc">{t("wsModal.settings.tabSafetyDesc")}</p>
              </div>
              <div class="settings-group">
                <div class="settings-group-head">
                  <span class="settings-group-title">{t("wsModal.settings.autofetchSection")}</span>
                </div>
                <div class="settings-row">
                  <label class="form-label" for="autofetch-seconds">{t("wsModal.settings.autofetchLabel")}</label>
                  <div class="autofetch-control">
                    <div class="autofetch-input-group">
                      <input
                        id="autofetch-seconds"
                        type="number"
                        class="form-input autofetch-input tabular-nums"
                        min="0"
                        max="86400"
                        step="1"
                        placeholder={t("wsModal.settings.autofetchPlaceholder")}
                        bind:value={draftAutofetchSeconds}
                      />
                      <span class="autofetch-unit">{t("wsModal.settings.autofetchUnitSeconds")}</span>
                    </div>
                    <div class="autofetch-presets">
                      {#each [
                        { value: 0, label: t("wsModal.settings.autofetchOff") },
                        { value: 30, label: t("wsModal.settings.autofetch30s") },
                        { value: 60, label: t("wsModal.settings.autofetch60s") },
                        { value: 300, label: t("wsModal.settings.autofetch300s") },
                        { value: 600, label: t("wsModal.settings.autofetch600s") },
                      ] as preset (preset.value)}
                        <button
                          type="button"
                          class="autofetch-preset-btn {draftAutofetchSeconds === preset.value ? 'active' : ''}"
                          onclick={() => (draftAutofetchSeconds = preset.value)}
                        >
                          {preset.label}
                        </button>
                      {/each}
                    </div>
                  </div>
                </div>
                <p class="settings-hint">{t("wsModal.settings.autofetchHint")}</p>
              </div>
              <div class="settings-group">
                <div class="settings-group-head">
                  <span class="settings-group-title">{t("wsModal.settings.safetySection")}</span>
                </div>
                <label class="settings-check-row">
                  <input type="checkbox" bind:checked={draftConfirmDiscard} />
                  <span class="settings-check-copy">
                    <span class="settings-check-title">{t("wsModal.settings.confirmDiscardLabel")}</span>
                    <span class="settings-hint">{t("wsModal.settings.confirmDiscardHint")}</span>
                  </span>
                </label>
                <label class="settings-check-row">
                  <input type="checkbox" bind:checked={draftConfirmForcePush} />
                  <span class="settings-check-copy">
                    <span class="settings-check-title">{t("wsModal.settings.confirmForcePushLabel")}</span>
                    <span class="settings-hint">{t("wsModal.settings.confirmForcePushHint")}</span>
                  </span>
                </label>
                <label class="settings-check-row">
                  <input type="checkbox" bind:checked={draftDesktopNotif} />
                  <span class="settings-check-copy">
                    <span class="settings-check-title">{t("wsModal.settings.desktopNotifLabel")}</span>
                    <span class="settings-hint">{t("wsModal.settings.desktopNotifHint")}</span>
                  </span>
                </label>
              </div>
            {:else if settingsTab === "updates"}
              <div class="settings-panel-header">
                <h4 class="settings-panel-title">{t("wsModal.settings.tabUpdates")}</h4>
                <p class="settings-panel-desc">{t("wsModal.settings.tabUpdatesDesc")}</p>
              </div>
              <div class="settings-group">
                <div class="settings-group-head">
                  <span class="settings-group-title">{t("wsModal.settings.updatesSection")}</span>
                </div>
                <div class="settings-version-row">
                  <div class="settings-version-meta">
                    <span class="form-label">{t("wsModal.settings.currentVersion")}</span>
                    <div style="display: flex; align-items: center; gap: 6px;">
                      <strong class="tabular-nums" style="font-size: 13px;">v{appUpdateInfo?.current_version || appRuntimeInfo?.current_version || "1.1.0"}</strong>
                      {#if appUpdateInfo?.install_mode || appRuntimeInfo?.install_mode}
                        <span class="badge-neutral">
                          {(appUpdateInfo?.install_mode || appRuntimeInfo?.install_mode) === "portable" ? "Portable" : "Installed"}
                          ({appUpdateInfo?.package_kind || appRuntimeInfo?.package_kind})
                        </span>
                      {/if}
                    </div>
                  </div>
                  <button
                    type="button"
                    class="btn-secondary btn-xs"
                    disabled={isCheckingUpdate || isInstallingUpdate}
                    onclick={() => onCheckUpdates(draftUpdateChannel)}
                  >
                    {isCheckingUpdate ? t("wsModal.settings.checkingUpdatesBtn") : t("wsModal.settings.checkUpdatesBtn")}
                  </button>
                </div>
                <div class="settings-row">
                  <label class="form-label" for="update-channel-select">{t("wsModal.settings.updateChannelLabel")}</label>
                  <select id="update-channel-select" class="form-select" bind:value={draftUpdateChannel}>
                    <option value="auto">{t("wsModal.settings.channelAuto")}</option>
                    <option value="stable">{t("wsModal.settings.channelStable")}</option>
                    <option value="beta">{t("wsModal.settings.channelBeta")}</option>
                  </select>
                </div>
                {#if updateError}
                  <div class="settings-status-box is-error">{updateError}</div>
                {/if}
                {#if appUpdateInfo}
                  <div class="settings-status-box {appUpdateInfo.has_update ? 'is-accent' : 'is-success'}">
                    {#if appUpdateInfo.has_update}
                      <div>
                        {t("wsModal.settings.newVersionAvailablePrefix")}<strong>v{appUpdateInfo.latest_version}</strong>
                        {t("wsModal.settings.newVersionAvailableSuffix", { tier: appUpdateInfo.is_prerelease ? "Beta" : "Stable" })}
                        {#if appUpdateInfo.asset_name}
                          <div class="settings-hint" style="margin-top: 2px;">
                            {t("wsModal.settings.packageLabel", { name: appUpdateInfo.asset_name })}
                            {#if appUpdateInfo.asset_size}
                              ({(appUpdateInfo.asset_size / (1024 * 1024)).toFixed(1)} MB)
                            {/if}
                          </div>
                        {/if}
                      </div>
                      {#if appUpdateInfo.release_notes?.trim()}
                        <div class="settings-release-notes">
                          <div style="font-weight: 600; color: var(--text-primary); margin-bottom: 4px;">
                            {t("wsModal.settings.releaseNotesHeader", { tier: appUpdateInfo.is_prerelease ? "Beta" : "Stable" })}
                          </div>
                          <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
                          <div class="settings-release-scroll settings-release-md" onclick={handleReleaseNoteClick}>
                            {@html renderMarkdownHtml(appUpdateInfo.release_notes.trim())}
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
                            {isInstallingUpdate ? t("wsModal.settings.installingUpdateBtn") : t("wsModal.settings.installUpdateBtn")}
                          </button>
                        {:else}
                          <span class="settings-hint">{t("wsModal.settings.manualUpdateNote")}</span>
                        {/if}
                        {#if appUpdateInfo.html_url}
                          <button type="button" class="link-btn" onclick={() => openExternalUrl(appUpdateInfo!.html_url!)}>
                            {t("wsModal.settings.openReleasesBtn")}
                          </button>
                        {/if}
                      </div>
                    {:else}
                      <div>
                        {t("wsModal.settings.alreadyLatest", { version: appUpdateInfo.current_version, channel: appUpdateInfo.channel })}
                      </div>
                      {#if appUpdateInfo.release_notes?.trim()}
                        <details style="margin-top: 2px;">
                          <summary style="cursor: pointer; color: var(--text-secondary);">
                            {t("wsModal.settings.viewReleaseNotes", { tier: appUpdateInfo.is_prerelease ? "Beta" : "Stable", version: appUpdateInfo.latest_version })}
                          </summary>
                          <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
                          <div class="settings-release-notes settings-release-scroll settings-release-md" style="margin-top: 6px;" onclick={handleReleaseNoteClick}>
                            {@html renderMarkdownHtml(appUpdateInfo.release_notes.trim())}
                          </div>
                        </details>
                      {/if}
                    {/if}
                  </div>
                {/if}
              </div>
            {:else if settingsTab === "ai"}
              <div class="settings-panel-header">
                <h4 class="settings-panel-title">
                  {t("ai.settingsSection")}{isLoadingAi ? t("wsModal.settings.loadingSuffix") : ""}
                </h4>
                <p class="settings-panel-desc">{t("wsModal.settings.tabAiDesc")}</p>
              </div>
              <div class="settings-group">
                <div class="settings-group-head">
                  <span class="settings-group-title">{t("ai.connectionGroup")}</span>
                </div>
                <div class="settings-row">
                  <label class="form-label" for="ai-provider-select">{t("ai.providerKindLabel")}</label>
                  <select id="ai-provider-select" class="form-select" bind:value={aiConfig.provider_kind}>
                    <option value="openai_compatible">OpenAI-Compatible (/chat/completions)</option>
                    <option value="anthropic">Anthropic (/messages)</option>
                  </select>
                </div>
                <div class="form-field">
                  <label class="form-label" for="ai-endpoint-input">{t("ai.endpointLabel")}</label>
                  <input
                    id="ai-endpoint-input"
                    type="text"
                    class="form-input"
                    style="font-family: var(--font-mono);"
                    bind:value={aiConfig.endpoint}
                    placeholder={t("ai.endpointPlaceholder")}
                  />
                  <span class="settings-hint">{t("ai.endpointHint")}</span>
                </div>
                <div class="form-field">
                  <label class="form-label" for="ai-model-input">{t("ai.modelLabel")}</label>
                  <input
                    id="ai-model-input"
                    type="text"
                    class="form-input"
                    style="font-family: var(--font-mono);"
                    bind:value={aiConfig.model}
                    placeholder={t("ai.modelPlaceholder")}
                  />
                </div>
              </div>
              <div class="settings-group">
                <div class="settings-group-head">
                  <span class="settings-group-title">{t("ai.authGroup")}</span>
                </div>
                <div class="form-field">
                  <div style="display: flex; justify-content: space-between; align-items: center;">
                    <label class="form-label" for="ai-apikey-input">{t("ai.apiKeyLabel")}</label>
                    {#if aiConfig.has_api_key}
                      <span class="badge-neutral">{aiConfig.memory_only ? t("ai.keyStoredMemory") : t("ai.keyStoredKeyring")}</span>
                    {/if}
                  </div>
                  <div style="display: flex; gap: 6px; align-items: center;">
                    <input
                      id="ai-apikey-input"
                      type={showAiApiKey ? "text" : "password"}
                      class="form-input"
                      style="flex: 1; font-family: var(--font-mono);"
                      bind:value={aiApiKeyInput}
                      placeholder={aiConfig.has_api_key ? t("ai.apiKeyKeepExisting") : "sk-..."}
                      autocomplete="off"
                    />
                    <button type="button" class="btn-secondary btn-xs" onclick={() => (showAiApiKey = !showAiApiKey)}>
                      {showAiApiKey ? t("wsModal.auth.hidePat") : t("wsModal.auth.showPat")}
                    </button>
                  </div>
                </div>
                <div class="form-field">
                  <label class="form-label" for="ai-system-prompt">{t("ai.systemPromptLabel")}</label>
                  <textarea
                    id="ai-system-prompt"
                    class="form-input"
                    rows="2"
                    bind:value={aiConfig.system_prompt}
                    placeholder={t("ai.systemPromptPlaceholder")}
                  ></textarea>
                </div>
                <div class="settings-ai-test-row">
                  <button
                    type="button"
                    class="btn-secondary btn-xs"
                    disabled={isTestingAi || !aiConfig.endpoint.trim() || !aiConfig.model.trim()}
                    onclick={() => void handleTestAiConnection(false)}
                  >
                    {isTestingAi ? t("ai.testingBtn") : t("ai.testConnectionBtn")}
                  </button>
                  {#if aiKeyringFallbackPending}
                    <button type="button" class="btn-primary btn-xs" onclick={() => void handleTestAiConnection(true)}>
                      {t("wsModal.auth.saveMemoryOnlyBtn")}
                    </button>
                  {/if}
                  {#if aiTestResult}
                    <span class="settings-test-result {aiTestResult.ok ? 'is-ok' : 'is-err'}">
                      {aiTestResult.msg}
                    </span>
                  {/if}
                </div>
              </div>
            {/if}
          </div>
        </div>
      {/if}
    </div>

    <div class="modal-actions">
      {#if modal === "open"}
        <button type="button" class="btn-secondary" onclick={onClose}>{t("common.cancel")}</button>
        <button
          type="button"
          class="btn-primary"
          disabled={!openRepoPath.trim()}
          onclick={() => {
            onClose();
            onLoadRepo(openRepoPath.trim());
          }}
        >
          {t("common.open")}
        </button>
      {:else if modal === "init"}
        <button type="button" class="btn-secondary" onclick={onClose}>{t("common.cancel")}</button>
        <button type="button" class="btn-primary" onclick={handleInitRepo}>{t("wsModal.init.createBtn")}</button>
      {:else if modal === "clone"}
        <button type="button" class="btn-secondary" onclick={onClose}>{t("common.cancel")}</button>
        <button type="button" class="btn-primary" onclick={handleClone}>{t("wsModal.clone.cloneBtn")}</button>
      {:else if modal === "publish"}
        <button type="button" class="btn-secondary" onclick={onClose}>{t("common.cancel")}</button>
        <button type="button" class="btn-primary" disabled={!publishName.trim()} onclick={handlePublishSubmit}>
          {t("wsModal.publish.publishBtn")}
        </button>
      {:else if modal === "diag"}
        <button
          type="button"
          class="btn-secondary"
          disabled={!diagReport}
          onclick={() => {
            if (diagReport) {
              const text = `App: Forge Desktop v${diagReport.app_version}\nOS: ${diagReport.os_info}\nGit: ${diagReport.git_version}\nConfig:\n${diagReport.sanitized_config.join("\n")}`;
              navigator.clipboard.writeText(text);
              onStatus(t("wsModal.diag.copiedStatus"));
            }
          }}
        >
          {t("wsModal.diag.copyBtn")}
        </button>
        <button type="button" class="btn-primary" onclick={onClose}>{t("common.close")}</button>
      {:else if modal === "logs"}
        <button
          type="button"
          class="btn-secondary"
          disabled={appErrorLogs.length === 0}
          onclick={() => {
            clearAppErrorLogs();
            onStatus(t("wsModal.logs.clearedStatus"));
          }}
        >
          {t("wsModal.logs.clearBtn")}
        </button>
        <button
          type="button"
          class="btn-secondary"
          disabled={appErrorLogs.length === 0}
          onclick={async () => {
            try {
              await navigator.clipboard.writeText(formatAppErrorLogsText(appRuntimeInfo?.current_version));
              onStatus(t("wsModal.logs.copiedStatus"));
            } catch (err) {
              onStatus(String(err), true);
            }
          }}
        >
          {t("wsModal.logs.copyBtn")}
        </button>
        <button type="button" class="btn-primary" onclick={onClose}>{t("common.close")}</button>
      {:else if modal === "settings"}
        <button type="button" class="btn-secondary" onclick={onClose}>{t("common.cancel")}</button>
        <button
          type="button"
          class="btn-primary"
          onclick={async () => {
            if (draftEditorPreset === "custom" && !draftCustomEditor.trim()) {
              settingsTab = "appearance";
              inlineError = t("wsModal.settings.customEditorRequired");
              return;
            }
            const parsedAutofetch = Number(draftAutofetchSeconds);
            if (
              draftAutofetchSeconds === null ||
              !Number.isFinite(parsedAutofetch) ||
              !Number.isInteger(parsedAutofetch) ||
              parsedAutofetch < 0 ||
              parsedAutofetch > 86400
            ) {
              settingsTab = "safety";
              inlineError = t("wsModal.settings.autofetchInvalid");
              return;
            }
            inlineError = "";
            const aiOk = await persistAiConfig(aiConfig.memory_only);
            if (!aiOk && aiKeyringFallbackPending) {
              settingsTab = "ai";
              return;
            }
            onSaveSettings({
              repoConfig: draftRepoConfig,
              globalConfig: draftGlobalConfig,
              activeTheme: draftTheme,
              locale: draftLocale,
              autofetchSeconds: parsedAutofetch,
              confirmDiscard: draftConfirmDiscard,
              confirmForcePush: draftConfirmForcePush,
              desktopNotifEnabled: draftDesktopNotif,
              externalEditor:
                draftEditorPreset === "custom" ? draftCustomEditor.trim() : draftEditorPreset,
              updateChannel: draftUpdateChannel,
            });
          }}
        >
          {t("wsModal.settings.saveAllBtn")}
        </button>
      {:else}
        <button type="button" class="btn-secondary" onclick={onClose}>{t("common.close")}</button>
      {/if}
    </div>
  </div>
</div>
