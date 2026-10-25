import type { FileEntry, RepoStatus } from "./types/git";
import * as gitService from "./services/git";
import { renderChangesSidebar } from "./ui/changes";
import { renderDiff } from "./ui/diff";
import { renderHistory } from "./ui/history";

let activeRepo =
  localStorage.getItem("forge:active-repo") ||
  "/home/machfudin/Development/github-desktop";
let activeTab: "changes" | "history" = "changes";
let currentStatus: RepoStatus | null = null;
let selectedFile: FileEntry | null = null;

const headerRepoName = document.querySelector<HTMLElement>("#header-repo-name")!;
const headerBranchName = document.querySelector<HTMLElement>("#header-branch-name")!;
const btnBranchSelector = document.querySelector<HTMLButtonElement>("#btn-branch-selector")!;
const btnOpenRepo = document.querySelector<HTMLButtonElement>("#btn-open-repo")!;
const btnCloneRepo = document.querySelector<HTMLButtonElement>("#btn-clone-repo")!;
const btnFetch = document.querySelector<HTMLButtonElement>("#btn-fetch")!;
const btnPull = document.querySelector<HTMLButtonElement>("#btn-pull")!;
const btnPush = document.querySelector<HTMLButtonElement>("#btn-push")!;
const btnAuth = document.querySelector<HTMLButtonElement>("#btn-auth")!;
const btnStash = document.querySelector<HTMLButtonElement>("#btn-stash")!;
const btnDiagnostics = document.querySelector<HTMLButtonElement>("#btn-diagnostics")!;
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

function getRecentRepos(): string[] {
  try {
    return JSON.parse(localStorage.getItem("forge:recent-repos") || "[]");
  } catch {
    return [];
  }
}

function addRecentRepo(path: string): void {
  const recents = getRecentRepos().filter((p) => p !== path);
  recents.unshift(path);
  localStorage.setItem("forge:recent-repos", JSON.stringify(recents.slice(0, 10)));
}

async function loadRepo(path: string): Promise<void> {
  try {
    setStatusMessage("Memuat repositori...");
    const status = await gitService.inspectRepository(path);
    activeRepo = path;
    localStorage.setItem("forge:active-repo", path);
    addRecentRepo(path);
    currentStatus = status;

    headerRepoName.textContent = path.split("/").pop() || path;
    headerBranchName.textContent = status.branch;
    btnBranchSelector.disabled = false;
    footerPath.textContent = path;

    // Check remotes to enable/disable remote buttons
    try {
      const remotes = await gitService.listRemotes(path);
      const hasRemotes = remotes.length > 0;
      btnFetch.disabled = !hasRemotes;
      btnPull.disabled = !hasRemotes;
      btnPush.disabled = !hasRemotes;
    } catch {
      btnFetch.disabled = true;
      btnPull.disabled = true;
      btnPush.disabled = true;
    }

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
    renderChangesSidebar(sidebarPanel, currentStatus, selectedFile, {
      onSelectFile: async (file) => {
        selectedFile = file;
        renderChangesSidebar(sidebarPanel, currentStatus!, selectedFile, {
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
      onAbortMerge: async () => {
        try {
          setStatusMessage("Membatalkan merge...");
          await gitService.abortMerge(activeRepo);
          setStatusMessage("Merge dibatalkan");
          await loadRepo(activeRepo);
        } catch (err) {
          setStatusMessage(String(err), true);
        }
      },
      onAbortRebase: async () => {
        try {
          setStatusMessage("Membatalkan rebase...");
          await gitService.abortRebase(activeRepo);
          setStatusMessage("Rebase dibatalkan");
          await loadRepo(activeRepo);
        } catch (err) {
          setStatusMessage(String(err), true);
        }
      },
      onDiscardFile: async (file) => {
        if (confirm(`Batalkan perubahan pada ${file.path}?`)) {
          try {
            await gitService.discardFileChanges(activeRepo, file.path);
            await loadRepo(activeRepo);
          } catch (err) {
            setStatusMessage(String(err), true);
          }
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

// Remote operations: Fetch, Pull, Push
btnFetch.addEventListener("click", async () => {
  try {
    setStatusMessage("Melakukan fetch dari remote...");
    const res = await gitService.gitFetch(activeRepo);
    setStatusMessage(res || "Fetch selesai");
    await loadRepo(activeRepo);
  } catch (err) {
    setStatusMessage(String(err), true);
  }
});

btnPull.addEventListener("click", async () => {
  try {
    setStatusMessage("Melakukan pull dari remote...");
    const res = await gitService.gitPull(activeRepo);
    setStatusMessage(res || "Pull selesai");
    await loadRepo(activeRepo);
  } catch (err) {
    setStatusMessage(String(err), true);
  }
});

btnPush.addEventListener("click", async () => {
  try {
    setStatusMessage("Melakukan push ke remote...");
    const res = await gitService.gitPush(activeRepo);
    setStatusMessage(res || "Push selesai");
    await loadRepo(activeRepo);
  } catch (err) {
    setStatusMessage(String(err), true);
  }
});

// Clone modal
btnCloneRepo.addEventListener("click", () => {
  modalContainer.innerHTML = `
    <div class="modal-overlay" id="modal-bg">
      <div class="modal-dialog">
        <div class="modal-header">Clone Repositori</div>
        <div style="display: flex; flex-direction: column; gap: 6px;">
          <label style="font-size: 11px; color: var(--text-secondary);">URL Repositori (HTTPS / SSH)</label>
          <input type="text" id="clone-url-input" class="commit-input" placeholder="https://github.com/user/repo.git" />
          <label style="font-size: 11px; color: var(--text-secondary); margin-top: 6px;">Folder Tujuan</label>
          <input type="text" id="clone-target-input" class="commit-input" placeholder="/home/user/Development/my-repo" />
        </div>
        <div class="modal-actions">
          <button class="btn-secondary" id="btn-cancel-clone">Batal</button>
          <button class="btn-primary" id="btn-confirm-clone">Clone</button>
        </div>
      </div>
    </div>
  `;

  document.querySelector("#btn-cancel-clone")?.addEventListener("click", () => {
    modalContainer.innerHTML = "";
  });

  document.querySelector("#btn-confirm-clone")?.addEventListener("click", async () => {
    const url = document.querySelector<HTMLInputElement>("#clone-url-input")?.value.trim();
    const target = document.querySelector<HTMLInputElement>("#clone-target-input")?.value.trim();
    if (!url || !target) {
      alert("URL dan folder tujuan wajib diisi");
      return;
    }

    modalContainer.innerHTML = "";
    try {
      setStatusMessage(`Cloning ${url}...`);
      await gitService.cloneRepository(url, target);
      setStatusMessage("Clone sukses");
      await loadRepo(target);
    } catch (err) {
      setStatusMessage(String(err), true);
    }
  });
});

// Stash Modal
btnStash?.addEventListener("click", async () => {
  try {
    const stashes = await gitService.gitListStashes(activeRepo);
    modalContainer.innerHTML = `
      <div class="modal-overlay" id="modal-bg">
        <div class="modal-dialog" style="width: 440px;">
          <div class="modal-header">Git Stash</div>
          <div style="display: flex; flex-direction: column; gap: 8px;">
            <div style="display: flex; gap: 6px;">
              <input type="text" id="stash-msg-input" class="commit-input" placeholder="Pesan stash (opsional)" style="flex: 1;" />
              <button id="btn-stash-save" class="btn-primary" style="font-size: 11px;">Simpan Stash</button>
            </div>
            <button id="btn-stash-pop" class="btn-secondary" style="align-self: flex-start; font-size: 11px;" ${stashes.length === 0 ? "disabled" : ""}>
              Terapkan Stash Teratas (Pop)
            </button>
            <label style="font-size: 11px; color: var(--text-secondary); margin-top: 4px;">Daftar Stash (${stashes.length})</label>
            <div style="max-height: 160px; overflow-y: auto; border: 1px solid var(--border-default); border-radius: var(--radius-sm); font-size: 11px; padding: 6px;">
              ${
                stashes.length === 0
                  ? '<div style="color: var(--text-muted); text-align: center; padding: 12px;">Tidak ada stash tersimpan</div>'
                  : stashes.map((s) => `<div style="padding: 4px 0; border-bottom: 1px solid var(--border-muted); font-family: var(--font-mono);">${s}</div>`).join("")
              }
            </div>
          </div>
          <div class="modal-actions" style="margin-top: 10px;">
            <button class="btn-secondary" id="btn-close-stash">Tutup</button>
          </div>
        </div>
      </div>
    `;

    document.querySelector("#btn-close-stash")?.addEventListener("click", () => {
      modalContainer.innerHTML = "";
    });

    document.querySelector("#btn-stash-save")?.addEventListener("click", async () => {
      const msg = document.querySelector<HTMLInputElement>("#stash-msg-input")?.value.trim();
      modalContainer.innerHTML = "";
      try {
        await gitService.gitStashSave(activeRepo, msg || undefined);
        await loadRepo(activeRepo);
      } catch (err) {
        setStatusMessage(String(err), true);
      }
    });

    document.querySelector("#btn-stash-pop")?.addEventListener("click", async () => {
      modalContainer.innerHTML = "";
      try {
        await gitService.gitStashPop(activeRepo);
        await loadRepo(activeRepo);
      } catch (err) {
        setStatusMessage(String(err), true);
      }
    });
  } catch (err) {
    setStatusMessage(String(err), true);
  }
});

// Diagnostics Modal
btnDiagnostics?.addEventListener("click", async () => {
  try {
    const diag = await gitService.getDiagnostics(activeRepo);
    modalContainer.innerHTML = `
      <div class="modal-overlay" id="modal-bg">
        <div class="modal-dialog" style="width: 500px; max-height: 80vh; display: flex; flex-direction: column;">
          <div class="modal-header">Laporan Diagnostik Sistem</div>
          <div style="overflow-y: auto; flex: 1; font-size: 11px; display: flex; flex-direction: column; gap: 8px;">
            <div><strong>Aplikasi:</strong> Forge Desktop v${diag.app_version}</div>
            <div><strong>OS / Arch:</strong> ${diag.os_info}</div>
            <div><strong>Versi Git:</strong> ${diag.git_version}</div>
            <div><strong>Konfigurasi Git (Aman / Redacted):</strong></div>
            <pre style="background: var(--bg-subtle); padding: 8px; border-radius: var(--radius-sm); max-height: 180px; overflow-y: auto; font-family: var(--font-mono); font-size: 10px;">${diag.sanitized_config.join("\n") || "Tidak ada config"}</pre>
          </div>
          <div class="modal-actions" style="margin-top: 12px;">
            <button class="btn-secondary" id="btn-copy-diag">Salin Info</button>
            <button class="btn-primary" id="btn-close-diag">Tutup</button>
          </div>
        </div>
      </div>
    `;

    document.querySelector("#btn-close-diag")?.addEventListener("click", () => {
      modalContainer.innerHTML = "";
    });

    document.querySelector("#btn-copy-diag")?.addEventListener("click", () => {
      const text = `App: Forge Desktop v${diag.app_version}\nOS: ${diag.os_info}\nGit: ${diag.git_version}\nConfig:\n${diag.sanitized_config.join("\n")}`;
      navigator.clipboard.writeText(text);
      setStatusMessage("Diagnostik disalin ke clipboard");
    });
  } catch (err) {
    setStatusMessage(String(err), true);
  }
});

// GitHub OAuth Device Flow Modal
btnAuth.addEventListener("click", () => {
  modalContainer.innerHTML = `
    <div class="modal-overlay" id="modal-bg">
      <div class="modal-dialog" style="width: 420px;">
        <div class="modal-header">Masuk dengan GitHub</div>
        <div id="auth-flow-body">
          <p style="font-size: 12px; color: var(--text-secondary); margin-bottom: 8px;">
            Gunakan Device Code Authorization untuk menghubungkan repositori ke akun GitHub Anda.
          </p>
          <label style="font-size: 11px; color: var(--text-secondary);">GitHub OAuth Client ID</label>
          <input type="text" id="oauth-client-id-input" class="commit-input" placeholder="Contoh: Iv1.b507a08c87ecfe98 atau buat di GitHub Developer" />
          <div style="margin-top: 12px; display: flex; justify-content: flex-end;">
            <button id="btn-start-oauth" class="btn-primary" style="font-size: 12px;">Minta Kode Perangkat</button>
          </div>
        </div>
        <div class="modal-actions" style="margin-top: 8px;">
          <button class="btn-secondary" id="btn-close-auth">Tutup</button>
        </div>
      </div>
    </div>
  `;

  document.querySelector("#btn-close-auth")?.addEventListener("click", () => {
    modalContainer.innerHTML = "";
  });

  document.querySelector("#btn-start-oauth")?.addEventListener("click", async () => {
    const clientId = document.querySelector<HTMLInputElement>("#oauth-client-id-input")?.value.trim();
    if (!clientId) {
      alert("Client ID wajib diisi");
      return;
    }

    const flowBody = document.querySelector<HTMLElement>("#auth-flow-body");
    if (!flowBody) return;

    try {
      flowBody.innerHTML = `<div style="text-align: center; padding: 16px;">Meminta kode...</div>`;
      const flow = await gitService.githubStartDeviceFlow(clientId);

      flowBody.innerHTML = `
        <div style="display: flex; flex-direction: column; gap: 10px; font-size: 12px;">
          <div>Buka link berikut di browser:</div>
          <a href="${flow.verification_uri}" target="_blank" style="color: var(--accent-blue); font-family: var(--font-mono);">${flow.verification_uri}</a>
          <div>Dan masukkan kode perangkat ini:</div>
          <div style="font-size: 20px; font-weight: 700; font-family: var(--font-mono); text-align: center; padding: 10px; background: var(--bg-subtle); border-radius: var(--radius-sm); letter-spacing: 2px; color: var(--accent-blue);">
            ${flow.user_code}
          </div>
          <div id="poll-status" style="color: var(--text-muted); font-size: 11px; text-align: center;">Menunggu konfirmasi di browser...</div>
        </div>
      `;

      const intervalSec = Math.max(flow.interval || 5, 5);
      const pollTimer = setInterval(async () => {
        try {
          const res = await gitService.githubPollToken(clientId, flow.device_code);
          if (res.access_token) {
            clearInterval(pollTimer);
            await gitService.configureGitCredentials("oauth", res.access_token);
            const statusEl = document.querySelector("#poll-status");
            if (statusEl) {
              statusEl.textContent = "Berhasil terhubung ke akun GitHub!";
              statusEl.setAttribute("style", "color: var(--diff-add-text); font-weight: 600; text-align: center;");
            }
          } else if (res.error && res.error !== "authorization_pending") {
            clearInterval(pollTimer);
            const statusEl = document.querySelector("#poll-status");
            if (statusEl) {
              statusEl.textContent = `Gagal: ${res.error_description || res.error}`;
              statusEl.setAttribute("style", "color: var(--diff-del-text); text-align: center;");
            }
          }
        } catch {
          // continue polling
        }
      }, intervalSec * 1000);
    } catch (err) {
      flowBody.innerHTML = `<div style="color: var(--diff-del-text); padding: 10px;">${String(err)}</div>`;
    }
  });
});

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

// Open repo modal with recents
btnOpenRepo.addEventListener("click", () => {
  const recents = getRecentRepos();
  modalContainer.innerHTML = `
    <div class="modal-overlay" id="modal-bg">
      <div class="modal-dialog" style="width: 440px;">
        <div class="modal-header">Buka Repositori Git</div>
        <input type="text" id="repo-path-input" class="commit-input" value="${activeRepo}" placeholder="/path/ke/repo" />
        ${
          recents.length > 0
            ? `
          <label style="font-size: 11px; color: var(--text-secondary); margin-top: 8px;">Repositori Terakhir:</label>
          <div style="max-height: 120px; overflow-y: auto; border: 1px solid var(--border-default); border-radius: var(--radius-sm); margin-top: 4px;">
            ${recents
              .map(
                (r) => `
              <div class="recent-repo-item" data-path="${r}" style="padding: 6px 10px; cursor: pointer; font-size: 11px; font-family: var(--font-mono); border-bottom: 1px solid var(--border-muted);">
                ${r}
              </div>
            `
              )
              .join("")}
          </div>
        `
            : ""
        }
        <div class="modal-actions" style="margin-top: 12px;">
          <button class="btn-secondary" id="btn-cancel-open">Batal</button>
          <button class="btn-primary" id="btn-confirm-open">Buka</button>
        </div>
      </div>
    </div>
  `;

  document.querySelector("#btn-cancel-open")?.addEventListener("click", () => {
    modalContainer.innerHTML = "";
  });

  document.querySelectorAll(".recent-repo-item").forEach((el) => {
    el.addEventListener("click", () => {
      const p = el.getAttribute("data-path");
      if (p) {
        const inp = document.querySelector<HTMLInputElement>("#repo-path-input");
        if (inp) inp.value = p;
      }
    });
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
