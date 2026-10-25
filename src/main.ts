import type { FileEntry, RepoStatus } from "./types/git";
import * as gitService from "./services/git";
import { renderChangesSidebar } from "./ui/changes";
import { renderDiff } from "./ui/diff";
import { renderHistory } from "./ui/history";

let activeRepo = "/home/machfudin/Development/github-desktop";
let activeTab: "changes" | "history" = "changes";
let currentStatus: RepoStatus | null = null;
let selectedFile: FileEntry | null = null;

const headerRepoName = document.querySelector<HTMLElement>("#header-repo-name")!;
const headerBranchName = document.querySelector<HTMLElement>("#header-branch-name")!;
const btnBranchSelector = document.querySelector<HTMLButtonElement>("#btn-branch-selector")!;
const btnOpenRepo = document.querySelector<HTMLButtonElement>("#btn-open-repo")!;
const btnRefresh = document.querySelector<HTMLButtonElement>("#btn-refresh")!;
const sidebarPanel = document.querySelector<HTMLElement>("#sidebar-panel")!;
const contentPanel = document.querySelector<HTMLElement>("#content-panel")!;
const footerStatus = document.querySelector<HTMLElement>("#footer-status")!;
const footerPath = document.querySelector<HTMLElement>("#footer-path")!;
const modalContainer = document.querySelector<HTMLElement>("#modal-container")!;
const navTabs = document.querySelectorAll<HTMLButtonElement>(".nav-tab");

function setStatusMessage(msg: string, isError = false): void {
  footerStatus.textContent = msg;
  footerStatus.style.color = isError ? "var(--diff-del-text)" : "var(--text-muted)";
}

async function loadRepo(path: string): Promise<void> {
  try {
    setStatusMessage("Memuat repositori...");
    const status = await gitService.inspectRepository(path);
    activeRepo = path;
    currentStatus = status;

    headerRepoName.textContent = path.split("/").pop() || path;
    headerBranchName.textContent = status.branch;
    btnBranchSelector.disabled = false;
    footerPath.textContent = path;

    // Default select first file if available
    if (status.files.length > 0) {
      if (!selectedFile || !status.files.some((f) => f.path === selectedFile?.path)) {
        selectedFile = status.files[0];
      }
    } else {
      selectedFile = null;
    }

    renderCurrentView();
    setStatusMessage("Siap");
  } catch (err) {
    setStatusMessage(String(err), true);
  }
}

async function renderCurrentView(): Promise<void> {
  if (!currentStatus) return;

  if (activeTab === "changes") {
    renderChangesSidebar(sidebarPanel, currentStatus.files, selectedFile, {
      onSelectFile: async (file) => {
        selectedFile = file;
        renderChangesSidebar(sidebarPanel, currentStatus!.files, selectedFile, {
          onSelectFile: () => {},
          onToggleStage: () => {},
          onStageAll: () => {},
          onUnstageAll: () => {},
          onCommit: () => {},
        });
        await loadAndRenderDiff(file);
      },
      onToggleStage: async (file) => {
        try {
          if (file.staged) {
            await gitService.unstageFile(activeRepo, file.path);
          } else {
            await gitService.stageFile(activeRepo, file.path);
          }
          await loadRepo(activeRepo);
        } catch (err) {
          setStatusMessage(String(err), true);
        }
      },
      onStageAll: async () => {
        try {
          await gitService.stageAll(activeRepo);
          await loadRepo(activeRepo);
        } catch (err) {
          setStatusMessage(String(err), true);
        }
      },
      onUnstageAll: async () => {
        try {
          await gitService.unstageAll(activeRepo);
          await loadRepo(activeRepo);
        } catch (err) {
          setStatusMessage(String(err), true);
        }
      },
      onCommit: async (msg) => {
        try {
          setStatusMessage("Melakukan komit...");
          await gitService.commitChanges(activeRepo, msg);
          setStatusMessage("Komit berhasil dibuat");
          await loadRepo(activeRepo);
        } catch (err) {
          setStatusMessage(String(err), true);
        }
      },
    });

    if (selectedFile) {
      await loadAndRenderDiff(selectedFile);
    } else {
      contentPanel.innerHTML = `
        <div style="display: grid; place-items: center; height: 100%; color: var(--text-muted);">
          ${currentStatus.files.length === 0 ? "Working tree bersih. Tidak ada perubahan." : "Pilih file untuk melihat diff."}
        </div>
      `;
    }
  } else if (activeTab === "history") {
    sidebarPanel.innerHTML = `
      <div class="sidebar-header">
        <h3>Riwayat Komit</h3>
      </div>
      <div style="padding: 12px; font-size: 11px; color: var(--text-secondary);">
        Branch saat ini: <strong>${headerBranchName.textContent}</strong>
      </div>
    `;

    try {
      setStatusMessage("Memuat riwayat komit...");
      const history = await gitService.getCommitHistory(activeRepo, 50);
      renderHistory(contentPanel, history);
      setStatusMessage("Siap");
    } catch (err) {
      setStatusMessage(String(err), true);
    }
  }
}

async function loadAndRenderDiff(file: FileEntry): Promise<void> {
  try {
    const diff = await gitService.getFileDiff(activeRepo, file.path, file.staged);
    renderDiff(contentPanel, file.path, diff);
  } catch (err) {
    contentPanel.innerHTML = `<div style="padding: 24px; color: var(--diff-del-text);">${String(err)}</div>`;
  }
}

// Branch modal
btnBranchSelector.addEventListener("click", async () => {
  try {
    const branches = await gitService.listBranches(activeRepo);
    modalContainer.innerHTML = `
      <div class="modal-overlay" id="modal-bg">
        <div class="modal-dialog">
          <div class="modal-header">Pilih atau Buat Branch</div>
          <div style="display: flex; gap: 6px;">
            <input type="text" id="new-branch-input" class="commit-input" placeholder="Nama branch baru" style="flex: 1;" />
            <button id="btn-create-branch" class="btn-primary" style="font-size: 11px;">Buat</button>
          </div>
          <div style="max-height: 200px; overflow-y: auto; border: 1px solid var(--border-default); border-radius: var(--radius-sm);">
            ${branches
              .map(
                (b) => `
              <div class="branch-item" data-name="${b.name}" style="padding: 6px 10px; cursor: pointer; display: flex; justify-content: space-between; font-size: 12px; background: ${b.is_current ? "var(--bg-subtle)" : "transparent"};">
                <span>${b.name}</span>
                ${b.is_current ? '<span style="color: var(--accent-blue); font-size: 11px;">aktif</span>' : ""}
              </div>
            `
              )
              .join("")}
          </div>
          <div class="modal-actions">
            <button class="btn-secondary" id="btn-close-modal">Tutup</button>
          </div>
        </div>
      </div>
    `;

    document.querySelector("#btn-close-modal")?.addEventListener("click", () => {
      modalContainer.innerHTML = "";
    });

    document.querySelectorAll(".branch-item").forEach((el) => {
      el.addEventListener("click", async () => {
        const name = el.getAttribute("data-name");
        if (name) {
          modalContainer.innerHTML = "";
          try {
            await gitService.switchBranch(activeRepo, name);
            await loadRepo(activeRepo);
          } catch (err) {
            setStatusMessage(String(err), true);
          }
        }
      });
    });

    document.querySelector("#btn-create-branch")?.addEventListener("click", async () => {
      const input = document.querySelector<HTMLInputElement>("#new-branch-input");
      const name = input?.value.trim();
      if (name) {
        modalContainer.innerHTML = "";
        try {
          await gitService.createBranch(activeRepo, name);
          await loadRepo(activeRepo);
        } catch (err) {
          setStatusMessage(String(err), true);
        }
      }
    });
  } catch (err) {
    setStatusMessage(String(err), true);
  }
});

// Open repo modal
btnOpenRepo.addEventListener("click", () => {
  modalContainer.innerHTML = `
    <div class="modal-overlay" id="modal-bg">
      <div class="modal-dialog">
        <div class="modal-header">Buka Repositori Git</div>
        <input type="text" id="repo-path-input" class="commit-input" value="${activeRepo}" placeholder="/path/ke/repo" />
        <div class="modal-actions">
          <button class="btn-secondary" id="btn-cancel-open">Batal</button>
          <button class="btn-primary" id="btn-confirm-open">Buka</button>
        </div>
      </div>
    </div>
  `;

  document.querySelector("#btn-cancel-open")?.addEventListener("click", () => {
    modalContainer.innerHTML = "";
  });

  document.querySelector("#btn-confirm-open")?.addEventListener("click", async () => {
    const input = document.querySelector<HTMLInputElement>("#repo-path-input");
    const path = input?.value.trim();
    if (path) {
      modalContainer.innerHTML = "";
      await loadRepo(path);
    }
  });
});

// Refresh button
btnRefresh.addEventListener("click", () => {
  loadRepo(activeRepo);
});

// Tabs switching
navTabs.forEach((tab) => {
  tab.addEventListener("click", () => {
    navTabs.forEach((t) => t.classList.remove("active"));
    tab.classList.add("active");
    activeTab = tab.getAttribute("data-tab") as "changes" | "history";
    renderCurrentView();
  });
});

// Initial boot
loadRepo(activeRepo);
