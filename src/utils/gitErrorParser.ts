export type GitErrorKind =
  | "workflow_scope"
  | "secret_detected"
  | "protected_branch"
  | "large_file"
  | "pull_needed"
  | "local_overwritten"
  | "ssh_issue"
  | "divergent_branches"
  | "auth_saml"
  | "unknown";

export interface ParsedGitError {
  kind: GitErrorKind;
  raw: string;
  titleKey: string;
  descriptionKey: string;
  detectedFile?: string | null;
  bypassUrl?: string | null;
  branchName?: string | null;
  fileSize?: string | null;
  filesOverwritten?: string[];
  suggestedAction?:
    | "pull"
    | "create_pr"
    | "open_tokens"
    | "open_bypass_url"
    | "stash_and_retry"
    | "switch_ssh"
    | "authorize_saml"
    | null;
}

export function isSshConnectionError(error: string): boolean {
  const lower = error.toLowerCase();
  return (
    lower.includes("ssh:") ||
    lower.includes("ssh.github.com") ||
    lower.includes("connection timed out") ||
    lower.includes("connection reset") ||
    lower.includes("could not resolve hostname") ||
    lower.includes("host key verification failed") ||
    lower.includes("permission denied (publickey)") ||
    lower.includes("port 22: connection timed out") ||
    lower.includes("port 443: connection timed out") ||
    lower.includes("kex_exchange_identification")
  );
}

export function isWorkflowScopeError(error: string): boolean {
  const lower = error.toLowerCase();
  return (
    lower.includes("refusing to allow a personal access token to create or update workflow") ||
    (lower.includes("workflow") &&
      lower.includes("scope") &&
      (lower.includes("token") || lower.includes("remote rejected") || lower.includes("personal access")))
  );
}

export function extractWorkflowFileName(error: string): string | null {
  const match = error.match(
    /(?:\.github\/workflows\/[a-zA-Z0-9_.\-]+)|(?:workflow\s+[`'"]([^`'"]+)[`'"])/i
  );
  if (match) {
    return match[1] || match[0];
  }
  return null;
}

export function isSecretScanningError(error: string): boolean {
  const lower = error.toLowerCase();
  return (
    lower.includes("gh008") ||
    (lower.includes("push rejected") && lower.includes("blocked because it contains secrets")) ||
    (lower.includes("secret") && lower.includes("push protection"))
  );
}

export function extractSecretBypassUrl(error: string): string | null {
  const match = error.match(
    /https?:\/\/[^\s`'"]*(?:secret-scanning\/unblock-secret|unblock-secret|secret-scanning)[^\s`'"]*/i
  );
  return match ? match[0] : null;
}

export function isProtectedBranchError(error: string): boolean {
  const lower = error.toLowerCase();
  return (
    lower.includes("gh006") ||
    lower.includes("protected branch hook declined") ||
    lower.includes("protected branch update failed") ||
    (lower.includes("protected branch") && lower.includes("rejected")) ||
    lower.includes("changes must be made through a pull request")
  );
}

export function extractProtectedBranchName(error: string): string | null {
  const match = error.match(
    /(?:refs\/heads\/([a-zA-Z0-9_.\-\/]+))|(?:update failed for\s+[`'"]?(?:refs\/heads\/)?([a-zA-Z0-9_.\-\/]+)[`'"]?)|(?:remote rejected\]\s+([a-zA-Z0-9_.\-\/]+)\s*->)/i
  );
  if (match) {
    const raw = match[1] || match[2] || match[3] || null;
    if (raw) {
      return raw.replace(/^refs\/heads\//i, "").replace(/[.:;,]+$/, "").trim();
    }
  }
  return null;
}

export function isLargeFileError(error: string): boolean {
  const lower = error.toLowerCase();
  return (
    lower.includes("gh001") ||
    (lower.includes("large files detected") && lower.includes("git-lfs")) ||
    lower.includes("exceeds github's file size limit") ||
    (lower.includes("this exceeds") && lower.includes("file size limit"))
  );
}

export function extractLargeFileInfo(error: string): { file: string | null; size: string | null } {
  const fileMatch = error.match(/File\s+([^\s]+)\s+is\s+([\d.]+\s*[kmgt]?b)/i);
  if (fileMatch) {
    return { file: fileMatch[1], size: fileMatch[2] };
  }
  const sizeMatch = error.match(/([\d.]+\s*MB)/i);
  return { file: null, size: sizeMatch ? sizeMatch[1] : null };
}

export function isPullNeededError(error: string): boolean {
  const lower = error.toLowerCase();
  return (
    lower.includes("updates were rejected because the tip of your current branch is behind") ||
    lower.includes("(fetch first)") ||
    lower.includes("(non-fast-forward)") ||
    (lower.includes("rejected") && lower.includes("remote contains work that you do not have locally"))
  );
}

export function isLocalOverwrittenError(error: string): boolean {
  const lower = error.toLowerCase();
  return (
    lower.includes("would be overwritten by merge") ||
    lower.includes("would be overwritten by checkout") ||
    lower.includes("would be overwritten by rebase") ||
    lower.includes("untracked working tree files would be overwritten")
  );
}

export function extractOverwrittenFiles(error: string): string[] {
  const lines = error.split(/\r?\n/);
  const files: string[] = [];
  let capturing = false;

  for (const line of lines) {
    const trimmed = line.trim();
    if (trimmed.toLowerCase().includes("would be overwritten by")) {
      capturing = true;
      continue;
    }
    if (capturing) {
      if (
        trimmed.toLowerCase().startsWith("please commit your changes") ||
        trimmed.toLowerCase().startsWith("please, commit your changes") ||
        trimmed.toLowerCase().startsWith("please move or remove them") ||
        trimmed.toLowerCase().startsWith("aborting") ||
        trimmed.startsWith("error:") ||
        trimmed.startsWith("fatal:")
      ) {
        break;
      }
      if (trimmed.length > 0 && !trimmed.startsWith("!")) {
        files.push(trimmed);
      }
    }
  }
  return files;
}

export function isDivergentBranchesError(error: string): boolean {
  const lower = error.toLowerCase();
  return (
    lower.includes("need to specify how to reconcile divergent branches") ||
    (lower.includes("divergent branches") && lower.includes("reconcile"))
  );
}

export function isAuthSamlError(error: string): boolean {
  const lower = error.toLowerCase();
  return (
    lower.includes("resource protected by organization saml enforcement") ||
    (lower.includes("saml") && lower.includes("single sign-on") && lower.includes("enforcement"))
  );
}

export function classifyGitError(error: string): ParsedGitError {
  const trimmed = error.trim();

  if (isWorkflowScopeError(trimmed)) {
    return {
      kind: "workflow_scope",
      raw: trimmed,
      titleKey: "gitErrors.workflowScopeTitle",
      descriptionKey: "gitErrors.workflowScopeDesc",
      detectedFile: extractWorkflowFileName(trimmed),
      suggestedAction: "open_tokens",
    };
  }

  if (isSecretScanningError(trimmed)) {
    return {
      kind: "secret_detected",
      raw: trimmed,
      titleKey: "gitErrors.secretDetectedTitle",
      descriptionKey: "gitErrors.secretDetectedDesc",
      bypassUrl: extractSecretBypassUrl(trimmed),
      suggestedAction: "open_bypass_url",
    };
  }

  if (isProtectedBranchError(trimmed)) {
    return {
      kind: "protected_branch",
      raw: trimmed,
      titleKey: "gitErrors.protectedBranchTitle",
      descriptionKey: "gitErrors.protectedBranchDesc",
      branchName: extractProtectedBranchName(trimmed),
      suggestedAction: "create_pr",
    };
  }

  if (isLargeFileError(trimmed)) {
    const largeInfo = extractLargeFileInfo(trimmed);
    return {
      kind: "large_file",
      raw: trimmed,
      titleKey: "gitErrors.largeFileTitle",
      descriptionKey: "gitErrors.largeFileDesc",
      detectedFile: largeInfo.file,
      fileSize: largeInfo.size,
      suggestedAction: null,
    };
  }

  if (isPullNeededError(trimmed)) {
    return {
      kind: "pull_needed",
      raw: trimmed,
      titleKey: "gitErrors.pullNeededTitle",
      descriptionKey: "gitErrors.pullNeededDesc",
      suggestedAction: "pull",
    };
  }

  if (isLocalOverwrittenError(trimmed)) {
    return {
      kind: "local_overwritten",
      raw: trimmed,
      titleKey: "gitErrors.localOverwrittenTitle",
      descriptionKey: "gitErrors.localOverwrittenDesc",
      filesOverwritten: extractOverwrittenFiles(trimmed),
      suggestedAction: "stash_and_retry",
    };
  }

  if (isAuthSamlError(trimmed)) {
    return {
      kind: "auth_saml",
      raw: trimmed,
      titleKey: "gitErrors.authSamlTitle",
      descriptionKey: "gitErrors.authSamlDesc",
      suggestedAction: "authorize_saml",
    };
  }

  if (isDivergentBranchesError(trimmed)) {
    return {
      kind: "divergent_branches",
      raw: trimmed,
      titleKey: "gitErrors.divergentBranchesTitle",
      descriptionKey: "gitErrors.divergentBranchesDesc",
      suggestedAction: null,
    };
  }

  if (isSshConnectionError(trimmed)) {
    return {
      kind: "ssh_issue",
      raw: trimmed,
      titleKey: "sshModal.title",
      descriptionKey: "sshModal.subtitle",
      suggestedAction: "switch_ssh",
    };
  }

  return {
    kind: "unknown",
    raw: trimmed,
    titleKey: "common.error",
    descriptionKey: "common.unknownError",
    suggestedAction: null,
  };
}
