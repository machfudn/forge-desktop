import type { FileEntry } from "../types/git";

export interface ChangesCallbacks {
  onSelectFile: (file: FileEntry) => void;
  onToggleStage: (file: FileEntry) => void;
  onStageAll: () => void;
  onUnstageAll: () => void;
  onCommit: (message: string) => void;
}

export function renderChangesSidebar(
  sidebarContainer: HTMLElement,
  files: FileEntry[],
  selectedFile: FileEntry | null,
  callbacks: ChangesCallbacks
): void {
  const stagedCount = files.filter((f) => f.staged).length;
  const totalCount = files.length;

  sidebarContainer.innerHTML = `
    <div class="sidebar-header">
      <h3>Perubahan (${totalCount})</h3>
      <div style="display: flex; gap: 4px;">
        <button class="btn-secondary" id="btn-stage-all" style="font-size: 11px; padding: 2px 6px;">Stage Semua</button>
        <button class="btn-secondary" id="btn-unstage-all" style="font-size: 11px; padding: 2px 6px;">Unstage</button>
      </div>
    </div>
    <ul class="file-list" id="file-list-container">
      ${
        files.length === 0
          ? `<li style="padding: 24px 12px; text-align: center; color: var(--text-muted);">Working tree bersih</li>`
          : files
              .map((file) => {
                const isSelected = selectedFile?.path === file.path;
                const statusChar = file.status_code.trim().charAt(0) || "M";
                const badgeClass =
                  statusChar === "A"
                    ? "status-A"
                    : statusChar === "D"
                    ? "status-D"
                    : statusChar === "?"
                    ? "status-Q"
                    : "status-M";

                return `
              <li class="file-item ${isSelected ? "selected" : ""}" data-path="${escapeHtml(file.path)}" tabindex="0">
                <input type="checkbox" class="file-stage-cb" ${file.staged ? "checked" : ""} aria-label="Stage file" />
                <span class="file-status-badge ${badgeClass}">${escapeHtml(file.status_code.trim())}</span>
                <span class="file-name" title="${escapeHtml(file.path)}">${escapeHtml(file.path)}</span>
              </li>
            `;
              })
              .join("")
      }
    </ul>
    <div class="commit-box">
      <input type="text" id="commit-summary-input" class="commit-input" placeholder="Ringkasan komit" maxlength="150" />
      <button id="btn-commit-action" class="btn-commit" ${stagedCount === 0 ? "disabled" : ""}>
        Komit ke branch (${stagedCount} file)
      </button>
    </div>
  `;

  // Attach event handlers
  sidebarContainer.querySelector("#btn-stage-all")?.addEventListener("click", () => callbacks.onStageAll());
  sidebarContainer.querySelector("#btn-unstage-all")?.addEventListener("click", () => callbacks.onUnstageAll());

  sidebarContainer.querySelectorAll(".file-item").forEach((el) => {
    const path = el.getAttribute("data-path");
    const file = files.find((f) => f.path === path);
    if (!file) return;

    el.addEventListener("click", (e) => {
      // Don't trigger select when clicking checkbox
      if ((e.target as HTMLElement).tagName.toLowerCase() === "input") return;
      callbacks.onSelectFile(file);
    });

    const cb = el.querySelector(".file-stage-cb");
    cb?.addEventListener("change", () => {
      callbacks.onToggleStage(file);
    });
  });

  const commitBtn = sidebarContainer.querySelector<HTMLButtonElement>("#btn-commit-action");
  const commitInput = sidebarContainer.querySelector<HTMLInputElement>("#commit-summary-input");

  commitBtn?.addEventListener("click", () => {
    const msg = commitInput?.value.trim();
    if (msg) {
      callbacks.onCommit(msg);
    }
  });

  commitInput?.addEventListener("keydown", (e) => {
    if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
      const msg = commitInput.value.trim();
      if (msg && stagedCount > 0) {
        callbacks.onCommit(msg);
      }
    }
  });
}

function escapeHtml(text: string): string {
  const div = document.createElement("div");
  div.textContent = text;
  return div.innerHTML;
}
