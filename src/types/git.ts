export interface FileEntry {
  path: string;
  status_code: string;
  staged: boolean;
  is_conflict: boolean;
}

export interface RepoStatus {
  branch: string;
  files: FileEntry[];
  has_conflicts: boolean;
  in_merge: boolean;
  in_rebase: boolean;
}

export interface CommitInfo {
  sha: string;
  short_sha: string;
  author: string;
  date: string;
  summary: string;
}

export interface BranchInfo {
  name: string;
  is_current: boolean;
}

export interface RemoteInfo {
  name: string;
  url: string;
}

export interface DeviceFlowInit {
  device_code: string;
  user_code: string;
  verification_uri: string;
  expires_in: number;
  interval: number;
}

export interface TokenPollResult {
  access_token?: string;
  error?: string;
  error_description?: string;
}

export interface DiagnosticReport {
  app_version: string;
  os_info: string;
  git_version: string;
  sanitized_config: string[];
}

export interface AheadBehind {
  ahead: number;
  behind: number;
}

export interface RepoConfig {
  user_name: string;
  user_email: string;
}

export interface PullRequestInfo {
  id: number;
  number: number;
  title: string;
  author: string;
  head_branch: string;
  base_branch: string;
  html_url: string;
  created_at: string;
}

export interface IssueInfo {
  id: number;
  number: number;
  title: string;
  author: string;
  html_url: string;
  comments_count: number;
}
