import { invoke } from "@tauri-apps/api/core";
import type { BranchInfo, CommitInfo, RepoStatus } from "../types/git";

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
