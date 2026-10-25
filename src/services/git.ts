import { invoke } from "@tauri-apps/api/core";
import type {
  BranchInfo,
  CommitInfo,
  DeviceFlowInit,
  RemoteInfo,
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

export async function commitChanges(repo: string, message: string): Promise<string> {
  return invoke<string>("commit_changes", { repo, message });
}

export async function getFileDiff(
  repo: string,
  filePath: string,
  staged: boolean
): Promise<string> {
  return invoke<string>("get_file_diff", { repo, filePath, staged });
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

export async function abortMerge(repo: string): Promise<void> {
  return invoke("abort_merge", { repo });
}

export async function abortRebase(repo: string): Promise<void> {
  return invoke("abort_rebase", { repo });
}

export async function listRemotes(repo: string): Promise<RemoteInfo[]> {
  return invoke<RemoteInfo[]>("list_remotes", { repo });
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
