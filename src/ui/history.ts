import type { CommitInfo } from "../types/git";

export function renderHistory(container: HTMLElement, commits: CommitInfo[]): void {
  if (commits.length === 0) {
    container.innerHTML = `
      <div style="padding: 32px; text-align: center; color: var(--text-muted);">
        Belum ada riwayat commit di branch ini.
      </div>
    `;
    return;
  }

  const rows = commits
    .map(
      (commit) => `
    <div class="commit-row" tabindex="0">
      <span class="commit-sha">${escapeHtml(commit.short_sha)}</span>
      <span class="commit-summary" title="${escapeHtml(commit.summary)}">${escapeHtml(commit.summary)}</span>
      <span class="commit-author">${escapeHtml(commit.author)}</span>
      <span class="commit-date">${escapeHtml(commit.date)}</span>
    </div>
  `
    )
    .join("");

  container.innerHTML = `
    <div class="history-container">
      <div style="padding: 8px 16px; border-bottom: 1px solid var(--border-default); background-color: var(--bg-panel); font-size: 11px; font-weight: 600; color: var(--text-muted); display: grid; grid-template-columns: 80px 1fr 140px 100px; gap: 12px;">
        <span>SHA</span>
        <span>KOMIT</span>
        <span>AUTHOR</span>
        <span style="text-align: right;">TANGGAL</span>
      </div>
      ${rows}
    </div>
  `;
}

function escapeHtml(text: string): string {
  const div = document.createElement("div");
  div.textContent = text;
  return div.innerHTML;
}
