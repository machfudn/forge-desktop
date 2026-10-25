export interface FileEntry {
  path: string;
  status_code: string;
  staged: boolean;
}

export interface RepoStatus {
  branch: string;
  files: FileEntry[];
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
