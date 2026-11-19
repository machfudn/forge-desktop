export function isSshRemoteUrl(url: string): boolean {
  const u = url.trim();
  if (u.startsWith("git@") || u.startsWith("ssh://")) {
    return true;
  }
  const colonIdx = u.indexOf(":");
  if (colonIdx > 1) {
    const host = u.slice(0, colonIdx);
    const path = u.slice(colonIdx + 1);
    if (
      !host.includes("/") &&
      !host.includes("\\") &&
      host.includes(".") &&
      !path.startsWith("\\") &&
      !u.startsWith("http://") &&
      !u.startsWith("https://")
    ) {
      return true;
    }
  }
  return false;
}

export function isHttpRemoteUrl(url: string): boolean {
  const lower = url.trim().toLowerCase();
  return lower.startsWith("https://") || lower.startsWith("http://");
}

export function convertSshToHttps(url: string): string | null {
  const trimmed = url.trim();
  if (!trimmed) return null;
  if (isHttpRemoteUrl(trimmed)) return trimmed;

  // Format: ssh://[user@]host[:port]/path
  const sshMatch = trimmed.match(/^ssh:\/\/(?:[^@]+@)?([^/:]+)(?::\d+)?\/(.+)$/);
  if (sshMatch) {
    let host = sshMatch[1];
    const path = sshMatch[2].replace(/^\/+/, "");
    if (host.toLowerCase() === "ssh.github.com") {
      host = "github.com";
    }
    return `https://${host}/${path}`;
  }

  // Format: [user@]host:path
  const scpMatch = trimmed.match(/^(?:[^@]+@)?([^:]+):(.+)$/);
  if (scpMatch && !scpMatch[1].includes("/") && !scpMatch[1].includes("\\")) {
    let host = scpMatch[1];
    const path = scpMatch[2].replace(/^\/+/, "");
    if (host.toLowerCase() === "ssh.github.com") {
      host = "github.com";
    }
    return `https://${host}/${path}`;
  }

  return null;
}

export function convertHttpsToSsh(url: string): string | null {
  const trimmed = url.trim();
  if (!trimmed) return null;
  if (isSshRemoteUrl(trimmed)) return trimmed;

  const httpMatch = trimmed.match(/^https?:\/\/(?:[^@]+@)?([^/:]+)(?::\d+)?\/(.+)$/);
  if (httpMatch) {
    const host = httpMatch[1];
    const path = httpMatch[2].replace(/^\/+/, "");
    return `git@${host}:${path}`;
  }
  return null;
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

export type SshDiagnosisType = "timeout" | "reset" | "dns" | "auth" | "general";

export function getSshDiagnosisType(error: string): SshDiagnosisType {
  const lower = error.toLowerCase();
  if (lower.includes("could not resolve hostname") || lower.includes("name or service not known")) {
    return "dns";
  }
  if (lower.includes("connection reset")) {
    return "reset";
  }
  if (lower.includes("timed out") || lower.includes("timeout")) {
    return "timeout";
  }
  if (lower.includes("permission denied (publickey)") || lower.includes("host key verification failed")) {
    return "auth";
  }
  return "general";
}
