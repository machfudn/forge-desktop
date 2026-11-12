import { invoke } from "@tauri-apps/api/core";
import type {
  AccountHealth,
  AccountOverview,
  AccountProfile,
  AheadBehind,
  AiConfigPublic,
  AiTaskResult,
  AiTestResult,
  AppRuntimeInfo,
  AppUpdateInfo,
  BlameLine,
  BranchInfo,
  CheckRunInfo,
  CommitDetail,
  CommitInfo,
  ConflictFileData,
  DetectedSession,
  DeviceFlowInit,
  DeviceFlowPollOutcome,
  DiagnosticReport,
  HostingProvider,
  ImageDiffData,
  IssueInfo,
  LocalSessionSource,
  ParsedGitHubRepo,
  ParsedHostingRepo,
  PublishRepoParams,
  PullRequestInfo,
  RebaseStep,
  ReflogEntry,
  RemoteInfo,
  RepoConfig,
  RepoPathState,
  RepoStatus,
  SaveAiConfigInput,
  SubmoduleInfo,
  TerminalExecResult,
  UpdateApplyResult,
  UpdateChannel,
  WorktreeInfo,
} from "../types/git";
export async function inspectRepository(repo: string): Promise<RepoStatus> {
  return invoke<RepoStatus>("inspect_repository", { repo });
}

export async function getRepoPathState(path: string): Promise<RepoPathState> {
  return invoke<RepoPathState>("get_repo_path_state", { path });
}

export async function stageFile(repo: string, filePath: string): Promise<void> {
  return invoke("stage_file", { repo, filePath });
}

export async function unstageFile(repo: string, filePath: string): Promise<void> {
  return invoke("unstage_file", { repo, filePath });
}

export async function stageAll(repo: string): Promise<void> {
  return invoke("stage_all", { repo });
}

export async function unstageAll(repo: string): Promise<void> {
  return invoke("unstage_all", { repo });
}

export async function commitChanges(
  repo: string,
  summary: string,
  description?: string,
  amend?: boolean
): Promise<string> {
  return invoke<string>("commit_changes", {
    repo,
    summary,
    description,
    amend,
  });
}

export async function undoLastCommit(repo: string): Promise<string> {
  return invoke<string>("undo_last_commit", { repo });
}

export async function getFileDiff(
  repo: string,
  filePath: string,
  staged: boolean,
  ignoreWhitespace?: boolean
): Promise<string> {
  return invoke<string>("get_file_diff", {
    repo,
    filePath,
    staged,
    ignoreWhitespace,
  });
}

export async function addToGitignore(repo: string, pattern: string): Promise<void> {
  return invoke("add_to_gitignore", { repo, pattern });
}

export async function cherryPick(repo: string, sha: string): Promise<string> {
  return invoke<string>("cherry_pick", { repo, sha });
}

export async function revertCommit(repo: string, sha: string): Promise<string> {
  return invoke<string>("revert_commit", { repo, sha });
}

export async function resetToCommit(
  repo: string,
  sha: string,
  hard?: boolean
): Promise<string> {
  return invoke<string>("reset_to_commit", { repo, sha, hard });
}

export async function listTags(repo: string): Promise<string[]> {
  return invoke<string[]>("list_tags", { repo });
}

export async function createTag(
  repo: string,
  tagName: string,
  targetSha?: string
): Promise<string> {
  return invoke<string>("create_tag", { repo, tagName, targetSha });
}

export async function deleteTag(repo: string, tagName: string): Promise<string> {
  return invoke<string>("delete_tag", { repo, tagName });
}

export async function rebaseBranch(
  repo: string,
  baseBranch: string
): Promise<string> {
  return invoke<string>("rebase_branch", { repo, baseBranch });
}

export async function gitForcePush(
  repo: string,
  remote?: string,
  branch?: string
): Promise<string> {
  return invoke<string>("git_force_push", { repo, remote, branch });
}

export async function getCommitHistory(
  repo: string,
  limit?: number,
  allBranches = true
): Promise<CommitInfo[]> {
  return invoke<CommitInfo[]>("get_commit_history", { repo, limit, allBranches });
}

export async function getCommitDetail(repo: string, sha: string): Promise<CommitDetail> {
  return invoke<CommitDetail>("get_commit_detail", { repo, sha });
}

export async function getCommitFileDiff(
  repo: string,
  sha: string,
  filePath: string,
  oldPath?: string | null,
  ignoreWhitespace?: boolean
): Promise<string> {
  return invoke<string>("get_commit_file_diff", {
    repo,
    sha,
    filePath,
    oldPath: oldPath || null,
    ignoreWhitespace,
  });
}

export async function listBranches(repo: string): Promise<BranchInfo[]> {
  return invoke<BranchInfo[]>("list_branches", { repo });
}

export async function switchBranch(repo: string, branchName: string): Promise<void> {
  return invoke("switch_branch", { repo, branchName });
}

export async function createBranch(repo: string, branchName: string): Promise<void> {
  return invoke("create_branch", { repo, branchName });
}

export async function deleteBranch(
  repo: string,
  branchName: string,
  force?: boolean
): Promise<string> {
  return invoke<string>("delete_branch", { repo, branchName, force });
}

export async function renameBranch(
  repo: string,
  oldName: string,
  newName: string
): Promise<string> {
  return invoke<string>("rename_branch", { repo, oldName, newName });
}

export async function mergeBranch(
  repo: string,
  targetBranch: string
): Promise<string> {
  return invoke<string>("merge_branch", { repo, targetBranch });
}

export async function abortMerge(repo: string): Promise<void> {
  return invoke("abort_merge", { repo });
}

export async function abortRebase(repo: string): Promise<void> {
  return invoke("abort_rebase", { repo });
}

export async function listRemotes(repo: string): Promise<RemoteInfo[]> {
  return invoke<RemoteInfo[]>("list_remotes", { repo });
}

export async function addRemote(
  repo: string,
  name: string,
  url: string
): Promise<string> {
  return invoke<string>("add_remote", { repo, name, url });
}

export async function setRemoteUrl(
  repo: string,
  name: string,
  url: string
): Promise<string> {
  return invoke<string>("set_remote_url", { repo, name, url });
}

export async function removeRemote(repo: string, name: string): Promise<string> {
  return invoke<string>("remove_remote", { repo, name });
}

export async function getBranchAheadBehind(
  repo: string,
  branch: string
): Promise<AheadBehind> {
  return invoke<AheadBehind>("get_branch_ahead_behind", { repo, branch });
}

export async function gitFetch(repo: string, remote?: string): Promise<string> {
  return invoke<string>("git_fetch", { repo, remote });
}

export async function gitPull(
  repo: string,
  remote?: string,
  branch?: string
): Promise<string> {
  return invoke<string>("git_pull", { repo, remote, branch });
}

export async function gitPush(
  repo: string,
  remote?: string,
  branch?: string
): Promise<string> {
  return invoke<string>("git_push", { repo, remote, branch });
}

export async function cloneRepository(
  url: string,
  targetDir: string,
  accountId?: string
): Promise<string> {
  return invoke<string>("clone_repository", { url, targetDir, accountId });
}
export async function initRepository(
  targetPath: string,
  defaultBranch?: string,
  gitignoreContent?: string,
  licenseContent?: string
): Promise<string> {
  return invoke<string>("init_repository", {
    targetPath,
    defaultBranch,
    gitignoreContent,
    licenseContent,
  });
}

export async function getRepoConfig(repo: string): Promise<RepoConfig> {
  return invoke<RepoConfig>("get_repo_config", { repo });
}

export async function setRepoConfig(
  repo: string,
  userName: string,
  userEmail: string
): Promise<void> {
  return invoke("set_repo_config", { repo, userName, userEmail });
}

export async function githubStartDeviceFlow(
  clientId: string,
  host?: string
): Promise<DeviceFlowInit> {
  return invoke<DeviceFlowInit>("github_start_device_flow", { host, clientId });
}

export async function listGithubAccounts(repo?: string): Promise<AccountOverview> {
  return invoke<AccountOverview>("list_github_accounts", { repo });
}

export async function verifyAndConnectPat(
  host: string,
  token: string,
  label?: string,
  allowMemoryFallback = false
): Promise<AccountProfile> {
  return invoke<AccountProfile>("verify_and_connect_pat", {
    host,
    token,
    label,
    allowMemoryFallback,
  });
}

export async function pollAndConnectDeviceFlow(
  host: string,
  clientId: string,
  deviceCode: string,
  label?: string,
  allowMemoryFallback = false
): Promise<DeviceFlowPollOutcome> {
  return invoke<DeviceFlowPollOutcome>("poll_and_connect_device_flow", {
    host,
    clientId,
    deviceCode,
    label,
    allowMemoryFallback,
  });
}

export async function detectLocalGithubSessions(
  host?: string
): Promise<DetectedSession[]> {
  return invoke<DetectedSession[]>("detect_local_github_sessions", { host });
}

export async function importLocalGithubSession(
  host: string,
  source: LocalSessionSource,
  label?: string,
  allowMemoryFallback = false
): Promise<AccountProfile> {
  return invoke<AccountProfile>("import_local_github_session", {
    host,
    source,
    label,
    allowMemoryFallback,
  });
}

export async function bindAccountToRepo(
  repo: string,
  accountId: string
): Promise<AccountOverview> {
  return invoke<AccountOverview>("bind_account_to_repo", { repo, accountId });
}

export async function setDefaultGithubAccount(
  accountId: string,
  repo?: string
): Promise<AccountOverview> {
  return invoke<AccountOverview>("set_default_github_account", { accountId, repo });
}

export async function removeGithubAccount(
  accountId: string,
  repo?: string
): Promise<AccountOverview> {
  return invoke<AccountOverview>("remove_github_account", { accountId, repo });
}

export async function updateGithubAccountLabel(
  accountId: string,
  label?: string,
  repo?: string
): Promise<AccountOverview> {
  return invoke<AccountOverview>("update_github_account_label", {
    accountId,
    label,
    repo,
  });
}

export async function checkGithubAccountStatus(
  accountId: string
): Promise<AccountHealth> {
  return invoke<AccountHealth>("check_github_account_status", { accountId });
}

export async function discardFileChanges(
  repo: string,
  filePath: string
): Promise<void> {
  return invoke("discard_file_changes", { repo, filePath });
}

export async function discardAllChanges(repo: string): Promise<void> {
  return invoke("discard_all_changes", { repo });
}

export async function gitStashSave(
  repo: string,
  message?: string
): Promise<string> {
  return invoke<string>("git_stash_save", { repo, message });
}

export async function gitStashPop(repo: string): Promise<string> {
  return invoke<string>("git_stash_pop", { repo });
}

export async function gitListStashes(repo: string): Promise<string[]> {
  return invoke<string[]>("git_list_stashes", { repo });
}

export async function gitStashShow(
  repo: string,
  stashRef?: string
): Promise<string> {
  return invoke<string>("git_stash_show", { repo, stashRef });
}

export async function gitStashDrop(
  repo: string,
  stashRef?: string
): Promise<string> {
  return invoke<string>("git_stash_drop", { repo, stashRef });
}

export async function checkRepoExists(path: string): Promise<boolean> {
  return invoke<boolean>("check_repo_exists", { path });
}
export async function checkReposExists(
  paths: string[]
): Promise<Record<string, boolean>> {
  return invoke<Record<string, boolean>>("check_repos_exists", { paths });
}


export async function getDiagnostics(repo?: string): Promise<DiagnosticReport> {
  return invoke<DiagnosticReport>("get_diagnostics", { repo });
}

export async function openInFileManager(path: string): Promise<void> {
  return invoke("open_in_file_manager", { path });
}

export async function openInTerminal(path: string): Promise<void> {
  return invoke("open_in_terminal", { path });
}

export async function openInEditor(path: string, editor?: string): Promise<void> {
  return invoke("open_in_editor", { path, editor });
}

export async function parseGithubRepository(
  remoteUrl: string
): Promise<ParsedGitHubRepo | null> {
  return invoke<ParsedGitHubRepo | null>("parse_github_repository", {
    remoteUrl,
  });
}

export async function listGithubPullRequests(
  owner: string,
  repo: string,
  host?: string,
  repoPath?: string
): Promise<PullRequestInfo[]> {
  return invoke<PullRequestInfo[]>("list_github_pull_requests", {
    owner,
    repo,
    host,
    repoPath,
  });
}

export async function listGithubIssues(
  owner: string,
  repo: string,
  host?: string,
  repoPath?: string
): Promise<IssueInfo[]> {
  return invoke<IssueInfo[]>("list_github_issues", {
    owner,
    repo,
    host,
    repoPath,
  });
}

export async function checkoutGithubPullRequest(
  repoPath: string,
  prNumber: number,
  remote?: string
): Promise<string> {
  return invoke<string>("checkout_github_pull_request", {
    repoPath,
    prNumber,
    remote,
  });
}

export async function getCommitCheckRuns(
  owner: string,
  repo: string,
  commitSha: string,
  host?: string,
  repoPath?: string
): Promise<CheckRunInfo[]> {
  return invoke<CheckRunInfo[]>("get_commit_check_runs", {
    owner,
    repo,
    commitSha,
    host,
    repoPath,
  });
}

export async function squashCommits(
  repo: string,
  count: number,
  newSummary: string,
  newDescription?: string
): Promise<string> {
  return invoke<string>("squash_commits", {
    repo,
    count,
    newSummary,
    newDescription,
  });
}

export async function getGlobalConfig(): Promise<RepoConfig> {
  return invoke<RepoConfig>("get_global_config");
}

export async function setGlobalConfig(
  userName: string,
  userEmail: string
): Promise<void> {
  return invoke("set_global_config", { userName, userEmail });
}

export async function getCliRepoArg(): Promise<string | null> {
  return invoke<string | null>("get_cli_repo_arg");
}

export async function compareBranches(
  repo: string,
  base: string,
  target: string
): Promise<AheadBehind> {
  return invoke<AheadBehind>("compare_branches", { repo, base, target });
}

export async function publishRepositoryToGithub(
  repo: string,
  params: PublishRepoParams,
  accountId?: string,
  host?: string
): Promise<string> {
  return invoke<string>("publish_repository_to_github", {
    repo,
    params,
    accountId,
    host,
  });
}

export async function syncFork(
  repo: string,
  upstreamRemote?: string,
  branch?: string
): Promise<string> {
  return invoke<string>("sync_fork", {
    repo,
    upstreamRemote: upstreamRemote || null,
    branch: branch || null,
  });
}

export async function getImageDiff(
  repo: string,
  filePath: string
): Promise<ImageDiffData> {
  return invoke<ImageDiffData>("get_image_diff", { repo, filePath });
}

export async function getAppRuntimeInfo(): Promise<AppRuntimeInfo> {
  return invoke<AppRuntimeInfo>("get_app_runtime_info");
}

export async function checkAppUpdate(
  channel?: UpdateChannel
): Promise<AppUpdateInfo> {
  return invoke<AppUpdateInfo>("check_app_update", {
    channel: channel ?? null,
  });
}

export async function installAppUpdate(
  channel?: UpdateChannel
): Promise<UpdateApplyResult> {
  return invoke<UpdateApplyResult>("install_app_update", {
    channel: channel ?? null,
  });
}

export async function stageHunk(
  repo: string,
  patchText: string,
  reverse: boolean
): Promise<void> {
  return invoke<void>("stage_hunk", { repo, patchText, reverse });
}

export async function interactiveRebase(
  repo: string,
  baseSha: string,
  steps: RebaseStep[]
): Promise<string> {
  return invoke<string>("interactive_rebase", { repo, baseSha, steps });
}

export async function stageLines(
  repo: string,
  hunkPatch: string,
  selectedIndices: number[],
  reverse: boolean
): Promise<void> {
  return invoke<void>("stage_lines", { repo, hunkPatch, selectedIndices, reverse });
}

export async function getFileBlame(
  repo: string,
  filePath: string,
  rev?: string
): Promise<BlameLine[]> {
  return invoke<BlameLine[]>("get_file_blame", { repo, filePath, rev });
}

export async function getFileHistory(
  repo: string,
  filePath: string,
  limit?: number
): Promise<CommitInfo[]> {
  return invoke<CommitInfo[]>("get_file_history", { repo, filePath, limit });
}

export async function getConflictFile(
  repo: string,
  filePath: string
): Promise<ConflictFileData> {
  return invoke<ConflictFileData>("get_conflict_file", { repo, filePath });
}

export async function resolveConflictFile(
  repo: string,
  filePath: string,
  resolvedContent: string,
  stageAfter = true
): Promise<void> {
  return invoke<void>("resolve_conflict_file", {
    repo,
    filePath,
    resolvedContent,
    stageAfter,
  });
}

export async function listReflog(repo: string, limit?: number): Promise<ReflogEntry[]> {
  return invoke<ReflogEntry[]>("list_reflog", { repo, limit });
}

export async function listSubmodules(repo: string): Promise<SubmoduleInfo[]> {
  return invoke<SubmoduleInfo[]>("list_submodules", { repo });
}

export async function addSubmodule(
  repo: string,
  url: string,
  path: string
): Promise<string> {
  return invoke<string>("add_submodule", { repo, url, path });
}

export async function updateSubmodules(
  repo: string,
  init = true,
  recursive = true
): Promise<string> {
  return invoke<string>("update_submodules", { repo, init, recursive });
}

export async function syncSubmodules(repo: string): Promise<string> {
  return invoke<string>("sync_submodules", { repo });
}

export async function listWorktrees(repo: string): Promise<WorktreeInfo[]> {
  return invoke<WorktreeInfo[]>("list_worktrees", { repo });
}

export async function addWorktree(
  repo: string,
  path: string,
  branch: string,
  createNewBranch = false
): Promise<string> {
  return invoke<string>("add_worktree", { repo, path, branch, createNewBranch });
}

export async function removeWorktree(
  repo: string,
  path: string,
  force = false
): Promise<string> {
  return invoke<string>("remove_worktree", { repo, path, force });
}

export async function pruneWorktrees(repo: string): Promise<string> {
  return invoke<string>("prune_worktrees", { repo });
}

export async function parseHostingRepository(
  remoteUrl: string
): Promise<ParsedHostingRepo | null> {
  return invoke<ParsedHostingRepo | null>("parse_hosting_repository", { remoteUrl });
}

export async function connectProviderPat(
  provider: HostingProvider,
  host: string,
  token: string,
  label?: string,
  allowMemoryFallback = false
): Promise<AccountProfile> {
  return invoke<AccountProfile>("connect_provider_pat", {
    provider,
    host,
    token,
    label,
    allowMemoryFallback,
  });
}

export async function listProviderPullRequests(
  remoteUrl: string,
  repo?: string
): Promise<PullRequestInfo[]> {
  return invoke<PullRequestInfo[]>("list_provider_pull_requests", { remoteUrl, repo });
}

export async function listProviderIssues(
  remoteUrl: string,
  repo?: string
): Promise<IssueInfo[]> {
  return invoke<IssueInfo[]>("list_provider_issues", { remoteUrl, repo });
}

export async function checkoutProviderPullRequest(
  repo: string,
  provider: HostingProvider,
  prNumber: number,
  headBranch: string
): Promise<string> {
  return invoke<string>("checkout_provider_pull_request", {
    repo,
    provider,
    prNumber,
    headBranch,
  });
}

export async function getAiConfig(): Promise<AiConfigPublic> {
  return invoke<AiConfigPublic>("get_ai_config");
}

export async function saveAiConfig(input: SaveAiConfigInput): Promise<AiConfigPublic> {
  return invoke<AiConfigPublic>("save_ai_config", { input });
}

export async function testAiConnection(): Promise<AiTestResult> {
  return invoke<AiTestResult>("test_ai_connection");
}

export async function aiRunTask(
  taskKind:
    | "commit_message"
    | "resolve_conflict"
    | "explain_diff"
    | "explain_commit"
    | "review_changes"
    | "custom_ask",
  promptInput: string,
  repo?: string,
  extraContext?: string
): Promise<AiTaskResult> {
  return invoke<AiTaskResult>("ai_run_task", {
    taskKind,
    promptInput,
    repo,
    extraContext,
  });
}

export async function runTerminalCommand(
  sessionId: string,
  repo: string,
  command: string,
  shellKind?: string
): Promise<TerminalExecResult> {
  return invoke<TerminalExecResult>("run_terminal_command", {
    sessionId,
    repo,
    command,
    shellKind,
  });
}

export async function killTerminalCommand(sessionId: string): Promise<boolean> {
  return invoke<boolean>("kill_terminal_command", { sessionId });
}

export async function getTerminalCwd(sessionId: string, repo: string): Promise<string> {
  return invoke<string>("get_terminal_cwd", { sessionId, repo });
}
