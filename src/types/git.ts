export interface FileEntry {
  path: string;
  status_code: string;
  staged: boolean;
  is_conflict: boolean;
  is_lfs?: boolean;
}

export interface RepoStatus {
  branch: string;
  files: FileEntry[];
  has_conflicts: boolean;
  in_merge: boolean;
  in_rebase: boolean;
  has_commits: boolean;
}

export type RepoPathState = "git" | "non_git" | "missing";

export interface CommitInfo {
  sha: string;
  short_sha: string;
  author: string;
  date: string;
  summary: string;
  is_pushed: boolean;
}

export interface CommitChangedFile {
  path: string;
  old_path?: string | null;
  status_code: string;
}

export interface CommitDetail {
  sha: string;
  short_sha: string;
  author: string;
  author_email: string;
  date: string;
  summary: string;
  description: string;
  files: CommitChangedFile[];
}

export interface BranchInfo {
  name: string;
  is_current: boolean;
  upstream: string | null;
  ahead: number;
  behind: number;
  is_pushed: boolean;
  upstream_gone: boolean;
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
  has_remote: boolean;
  has_upstream: boolean;
  upstream: string | null;
  remote_branch_exists: boolean;
  is_unborn: boolean;
  unpushed_commits: number;
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

export interface CheckRunInfo {
  id: number;
  name: string;
  status: string;
  conclusion?: string;
  html_url?: string;
}

export interface PublishRepoParams {
  name: string;
  description?: string;
  private: boolean;
}

export interface ImageDiffData {
  old_image_url?: string | null;
  new_image_url?: string | null;
  extension: string;
}

export type UpdateChannel = "auto" | "stable" | "beta";

export interface AppRuntimeInfo {
  current_version: string;
  os: string;
  arch: string;
  install_mode: string;
  package_kind: string;
  default_channel: string;
}

export interface AppUpdateInfo {
  current_version: string;
  latest_version: string;
  has_update: boolean;
  channel: string;
  is_prerelease: boolean;
  install_mode: string;
  package_kind: string;
  can_self_update: boolean;
  asset_name?: string | null;
  asset_url?: string | null;
  asset_size?: number | null;
  sha256?: string | null;
  release_notes?: string | null;
  html_url?: string | null;
}

export interface UpdateApplyResult {
  launched: boolean;
  restart_pending: boolean;
  message: string;
  downloaded_path?: string | null;
}

export interface RebaseStep {
  sha: string;
  action: "pick" | "drop";
}

export type AuthMethod =
  | "pat"
  | "device_flow"
  | "gh_cli_import"
  | "git_credentials_import";

export interface AccountProfile {
  id: string;
  host: string;
  login: string;
  name?: string | null;
  email?: string | null;
  avatar_url?: string | null;
  html_url?: string | null;
  label?: string | null;
  auth_method: AuthMethod;
  added_at: number;
  memory_only?: boolean;
}

export interface AccountOverview {
  accounts: AccountProfile[];
  default_by_host: Record<string, string>;
  active_repo_account_id?: string | null;
  primary_account_id?: string | null;
}

export type LocalSessionSource = "gh_cli" | "git_credentials";

export interface DetectedSession {
  source: LocalSessionSource;
  host: string;
  username_hint?: string | null;
  display: string;
}

export interface DeviceFlowPollOutcome {
  status: "pending" | "slow_down" | "authorized" | "error";
  profile?: AccountProfile | null;
  error?: string | null;
  sso_url?: string | null;
}

export type AccountHealth =
  | { status: "valid"; profile: AccountProfile }
  | { status: "expired"; message: string }
  | { status: "sso_required"; message: string; sso_url?: string | null }
  | { status: "offline"; message: string };

export interface ParsedGitHubRepo {
  host: string;
  owner: string;
  repo: string;
}
