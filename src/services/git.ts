import { invoke } from "@tauri-apps/api/core";
import type {
  AheadBehind,
  BranchInfo,
  CheckRunInfo,
  CommitInfo,
  DeviceFlowInit,
  DiagnosticReport,
  IssueInfo,
  PullRequestInfo,
  RemoteInfo,
  RepoConfig,
  RepoStatus,
  TokenPollResult,
} from "../types/git";

export async function inspectRepository(repo: string): Promise<RepoStatus> {
  return invoke<RepoStatus>("inspect_repository", { repo });
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
  limit?: number
): Promise<CommitInfo[]> {
  return invoke<CommitInfo[]>("get_commit_history", { repo, limit });
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
  targetDir: string
): Promise<string> {
  return invoke<string>("clone_repository", { url, targetDir });
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
  clientId: string
): Promise<DeviceFlowInit> {
  return invoke<DeviceFlowInit>("github_start_device_flow", { clientId });
}

export async function githubPollToken(
  clientId: string,
  deviceCode: string
): Promise<TokenPollResult> {
  return invoke<TokenPollResult>("github_poll_token", { clientId, deviceCode });
}

export async function configureGitCredentials(
  username: string,
  token: string
): Promise<void> {
  return invoke("configure_git_credentials", { username, token });
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
): Promise<[string, string] | null> {
  return invoke<[string, string] | null>("parse_github_repository", {
    remoteUrl,
  });
}

export async function listGithubPullRequests(
  owner: string,
  repo: string,
  token?: string
): Promise<PullRequestInfo[]> {
  return invoke<PullRequestInfo[]>("list_github_pull_requests", {
    owner,
    repo,
    token,
  });
}

export async function listGithubIssues(
  owner: string,
  repo: string,
  token?: string
): Promise<IssueInfo[]> {
  return invoke<IssueInfo[]>("list_github_issues", {
    owner,
    repo,
    token,
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
  token?: string
): Promise<CheckRunInfo[]> {
  return invoke<CheckRunInfo[]>("get_commit_check_runs", {
    owner,
    repo,
    commitSha,
    token,
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
