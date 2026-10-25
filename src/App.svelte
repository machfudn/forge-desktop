<script lang="ts">
  import { onMount } from "svelte";
  import type {
    BranchInfo,
    CommitInfo,
    DeviceFlowInit,
    DiagnosticReport,
    FileEntry,
    RepoStatus,
  } from "./types/git";
  import * as gitService from "./services/git";
  import Changes from "./components/Changes.svelte";
  import Diff from "./components/Diff.svelte";
  import History from "./components/History.svelte";

  let activeRepo = $state(
    localStorage.getItem("forge:active-repo") ||
      "/home/machfudin/Development/github-desktop"
  );
  let activeTab: "changes" | "history" = $state("changes");
  let currentStatus: RepoStatus | null = $state(null);
  let selectedFile: FileEntry | null = $state(null);
  let currentDiff = $state("");
  let commitHistory: CommitInfo[] = $state([]);
  let branches: BranchInfo[] = $state([]);
  let hasRemotes = $state(false);

  let footerStatus = $state("Siap");
  let isStatusError = $state(false);

  // Modal states
  type ModalType = "branch" | "open" | "clone" | "stash" | "diag" | "auth" | null;
  let activeModal: ModalType = $state(null);

  // Modal sub-states
  let newBranchName = $state("");
  let openRepoPath = $state("");
  let recentRepos: string[] = $state([]);
  let cloneUrl = $state("");
  let cloneTarget = $state("");
  let stashMsg = $state("");
  let stashList: string[] = $state([]);
  let diagReport: DiagnosticReport | null = $state(null);

  // Auth flow states
  let oauthClientId = $state("");
  let deviceFlow: DeviceFlowInit | null = $state(null);
  let pollStatus = $state("");
  let pollStatusClass = $state("");
  let pollTimer: number | null = null;

  function setStatus(msg: string, isError = false) {
    footerStatus = msg;
    isStatusError = isError;
  }

  function getStoredRecents(): string[] {
    try {
      return JSON.parse(localStorage.getItem("forge:recent-repos") || "[]");
    } catch {
      return [];
    }
  }

  function saveRecent(path: string) {
    const list = getStoredRecents().filter((p) => p !== path);
    list.unshift(path);
    const sliced = list.slice(0, 10);
    localStorage.setItem("forge:recent-repos", JSON.stringify(sliced));
    recentRepos = sliced;
  }

  async function loadRepo(path: string) {
    try {
      setStatus("Memuat repositori...");
      const status = await gitService.inspectRepository(path);
      activeRepo = path;
      localStorage.setItem("forge:active-repo", path);
      saveRecent(path);
      currentStatus = status;

      try {
        const remotes = await gitService.listRemotes(path);
        hasRemotes = remotes.length > 0;
      } catch {
        hasRemotes = false;
      }

      if (status.files.length > 0) {
        if (!selectedFile || !status.files.some((f) => f.path === selectedFile?.path)) {
          selectedFile = status.files[0];
        }
        await loadDiff(selectedFile);
      } else {
        selectedFile = null;
        currentDiff = "";
      }

      if (activeTab === "history") {
        await loadHistory();
      }

      setStatus("Siap");
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  async function loadDiff(file: FileEntry | null) {
    if (!file) {
      currentDiff = "";
      return;
    }
    try {
      currentDiff = await gitService.getFileDiff(activeRepo, file.path, file.staged);
    } catch (err) {
      currentDiff = `Error loading diff: ${String(err)}`;
    }
  }

  async function loadHistory() {
    try {
      setStatus("Memuat riwayat komit...");
      commitHistory = await gitService.getCommitHistory(activeRepo, 50);
      setStatus("Siap");
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  async function handleToggleStage(file: FileEntry) {
    try {
      if (file.staged) {
        await gitService.unstageFile(activeRepo, file.path);
      } else {
        await gitService.stageFile(activeRepo, file.path);
      }
      await loadRepo(activeRepo);
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  async function handleStageAll() {
    try {
      await gitService.stageAll(activeRepo);
      await loadRepo(activeRepo);
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  async function handleUnstageAll() {
    try {
      await gitService.unstageAll(activeRepo);
      await loadRepo(activeRepo);
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  async function handleCommit(msg: string) {
    try {
      setStatus("Melakukan komit...");
      await gitService.commitChanges(activeRepo, msg);
      setStatus("Komit berhasil dibuat");
      await loadRepo(activeRepo);
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  async function handleAbortMerge() {
    try {
      setStatus("Membatalkan merge...");
      await gitService.abortMerge(activeRepo);
      setStatus("Merge dibatalkan");
      await loadRepo(activeRepo);
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  async function handleAbortRebase() {
    try {
      setStatus("Membatalkan rebase...");
      await gitService.abortRebase(activeRepo);
      setStatus("Rebase dibatalkan");
      await loadRepo(activeRepo);
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  async function handleDiscardFile(file: FileEntry) {
    if (confirm(`Batalkan perubahan pada ${file.path}?`)) {
      try {
        await gitService.discardFileChanges(activeRepo, file.path);
        await loadRepo(activeRepo);
      } catch (err) {
        setStatus(String(err), true);
      }
    }
  }

  // Remote operations
  async function handleFetch() {
    try {
      setStatus("Melakukan fetch dari remote...");
      const res = await gitService.gitFetch(activeRepo);
      setStatus(res || "Fetch selesai");
      await loadRepo(activeRepo);
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  async function handlePull() {
    try {
      setStatus("Melakukan pull dari remote...");
      const res = await gitService.gitPull(activeRepo);
      setStatus(res || "Pull selesai");
      await loadRepo(activeRepo);
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  async function handlePush() {
    try {
      setStatus("Melakukan push ke remote...");
      const res = await gitService.gitPush(activeRepo);
      setStatus(res || "Push selesai");
      await loadRepo(activeRepo);
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  // Modals opening
  async function openBranchModal() {
    try {
      branches = await gitService.listBranches(activeRepo);
      newBranchName = "";
      activeModal = "branch";
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  function openOpenRepoModal() {
    recentRepos = getStoredRecents();
    openRepoPath = activeRepo;
    activeModal = "open";
  }

  function openCloneModal() {
    cloneUrl = "";
    cloneTarget = "";
    activeModal = "clone";
  }

  async function openStashModal() {
    try {
      stashList = await gitService.gitListStashes(activeRepo);
      stashMsg = "";
      activeModal = "stash";
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  async function openDiagModal() {
    try {
      diagReport = await gitService.getDiagnostics(activeRepo);
      activeModal = "diag";
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  function openAuthModal() {
    deviceFlow = null;
    pollStatus = "";
    oauthClientId = "";
    if (pollTimer) clearInterval(pollTimer);
    activeModal = "auth";
  }

  function closeModal() {
    activeModal = null;
    if (pollTimer) clearInterval(pollTimer);
  }

  // Branch modal actions
  async function handleSwitchBranch(name: string) {
    closeModal();
    try {
      await gitService.switchBranch(activeRepo, name);
      await loadRepo(activeRepo);
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  async function handleCreateBranch() {
    const name = newBranchName.trim();
    if (!name) return;
    closeModal();
    try {
      await gitService.createBranch(activeRepo, name);
      await loadRepo(activeRepo);
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  // Stash modal actions
  async function handleStashSave() {
    closeModal();
    try {
      await gitService.gitStashSave(activeRepo, stashMsg || undefined);
      await loadRepo(activeRepo);
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  async function handleStashPop() {
    closeModal();
    try {
      await gitService.gitStashPop(activeRepo);
      await loadRepo(activeRepo);
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  // Clone actions
  async function handleClone() {
    const url = cloneUrl.trim();
    const target = cloneTarget.trim();
    if (!url || !target) {
      alert("URL dan folder tujuan wajib diisi");
      return;
    }
    closeModal();
    try {
      setStatus(`Cloning ${url}...`);
      await gitService.cloneRepository(url, target);
      setStatus("Clone sukses");
      await loadRepo(target);
    } catch (err) {
      setStatus(String(err), true);
    }
  }

  // Auth Device Flow
  async function handleStartDeviceFlow() {
    const cid = oauthClientId.trim();
    if (!cid) {
      alert("Client ID wajib diisi");
      return;
    }
    try {
      pollStatus = "Meminta kode...";
      pollStatusClass = "";
      deviceFlow = await gitService.githubStartDeviceFlow(cid);
      pollStatus = "Menunggu konfirmasi di browser...";

      const sec = Math.max(deviceFlow.interval || 5, 5);
      pollTimer = window.setInterval(async () => {
        try {
          const res = await gitService.githubPollToken(cid, deviceFlow!.device_code);
          if (res.access_token) {
            if (pollTimer) clearInterval(pollTimer);
            await gitService.configureGitCredentials("oauth", res.access_token);
            pollStatus = "Berhasil terhubung ke akun GitHub!";
            pollStatusClass = "success";
          } else if (res.error && res.error !== "authorization_pending") {
            if (pollTimer) clearInterval(pollTimer);
            pollStatus = `Gagal: ${res.error_description || res.error}`;
            pollStatusClass = "error";
          }
        } catch {
          // continue polling
        }
      }, sec * 1000);
    } catch (err) {
      pollStatus = String(err);
      pollStatusClass = "error";
    }
  }

  onMount(() => {
    loadRepo(activeRepo);
  });
</script>

<div class="app-container">
  <header class="app-header">
    <div class="header-left">
      <button onclick={openOpenRepoModal} class="btn-secondary" style="font-weight: 600;">
        Buka Repo
      </button>
      <button onclick={openCloneModal} class="btn-secondary">
        Clone
      </button>
      <div class="repo-badge">
        <span>{activeRepo.split("/").pop() || activeRepo}</span>
      </div>
      <button
        onclick={openBranchModal}
        class="branch-select-btn"
        disabled={!currentStatus}
      >
        Branch: <span>{currentStatus?.branch || "-"}</span>
      </button>
    </div>

    <nav class="nav-tabs" aria-label="Navigasi">
      <button
        class="nav-tab {activeTab === 'changes' ? 'active' : ''}"
        onclick={() => {
          activeTab = "changes";
          if (selectedFile) loadDiff(selectedFile);
        }}
      >
        Changes
      </button>
      <button
        class="nav-tab {activeTab === 'history' ? 'active' : ''}"
        onclick={() => {
          activeTab = "history";
          loadHistory();
        }}
      >
        History
      </button>
    </nav>

    <div class="header-right">
      <button onclick={handleFetch} class="btn-secondary" title="Fetch dari Remote" disabled={!hasRemotes}>Fetch</button>
      <button onclick={handlePull} class="btn-secondary" title="Pull dari Remote" disabled={!hasRemotes}>Pull</button>
      <button onclick={handlePush} class="btn-secondary" title="Push ke Remote" disabled={!hasRemotes}>Push</button>
      <button onclick={openAuthModal} class="btn-secondary" title="Akun GitHub">GitHub</button>
      <button onclick={openStashModal} class="btn-secondary" title="Simpan perubahan ke Stash">Stash</button>
      <button onclick={openDiagModal} class="btn-secondary" title="Laporan Diagnostik Sistem">Diagnostik</button>
      <button onclick={() => loadRepo(activeRepo)} class="btn-secondary" title="Muat Ulang">Refresh</button>
    </div>
  </header>

  <main class="app-body">
    <aside class="sidebar-panel">
      {#if activeTab === "changes"}
        {#if currentStatus}
          <Changes
            status={currentStatus}
            {selectedFile}
            onSelectFile={(f) => {
              selectedFile = f;
              loadDiff(f);
            }}
            onToggleStage={handleToggleStage}
            onStageAll={handleStageAll}
            onUnstageAll={handleUnstageAll}
            onCommit={handleCommit}
            onAbortMerge={handleAbortMerge}
            onAbortRebase={handleAbortRebase}
            onDiscardFile={handleDiscardFile}
          />
        {/if}
      {:else}
        <div class="sidebar-header">
          <h3>Riwayat Komit</h3>
        </div>
        <div style="padding: 12px; font-size: 11px; color: var(--text-secondary);">
          Branch saat ini: <strong>{currentStatus?.branch || "-"}</strong>
        </div>
      {/if}
    </aside>

    <section class="content-panel">
      {#if activeTab === "changes"}
        {#if selectedFile}
          <Diff filePath={selectedFile.path} diffText={currentDiff} />
        {:else}
          <div style="display: grid; place-items: center; height: 100%; color: var(--text-muted);">
            {currentStatus?.files.length === 0 ? "Working tree bersih. Tidak ada perubahan." : "Pilih file untuk melihat diff."}
          </div>
        {/if}
      {:else}
        <History commits={commitHistory} />
      {/if}
    </section>
  </main>

  <footer class="app-footer">
    <span style="color: {isStatusError ? 'var(--diff-del-text)' : 'var(--text-muted)'}">{footerStatus}</span>
    <span>{activeRepo}</span>
  </footer>
</div>

<!-- Modals -->
{#if activeModal === "branch"}
  <div class="modal-overlay">
    <div class="modal-dialog">
      <div class="modal-header">Pilih atau Buat Branch</div>
      <div style="display: flex; gap: 6px;">
        <input type="text" class="commit-input" placeholder="Nama branch baru" style="flex: 1;" bind:value={newBranchName} />
        <button onclick={handleCreateBranch} class="btn-primary" style="font-size: 11px;">Buat</button>
      </div>
      <div style="max-height: 200px; overflow-y: auto; border: 1px solid var(--border-default); border-radius: var(--radius-sm);">
        {#each branches as b (b.name)}
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            class="branch-item"
            onclick={() => handleSwitchBranch(b.name)}
            style="padding: 6px 10px; cursor: pointer; display: flex; justify-content: space-between; font-size: 12px; background: {b.is_current ? 'var(--bg-subtle)' : 'transparent'};"
          >
            <span>{b.name}</span>
            {#if b.is_current}
              <span style="color: var(--accent-blue); font-size: 11px;">aktif</span>
            {/if}
          </div>
        {/each}
      </div>
      <div class="modal-actions">
        <button class="btn-secondary" onclick={closeModal}>Tutup</button>
      </div>
    </div>
  </div>
{:else if activeModal === "open"}
  <div class="modal-overlay">
    <div class="modal-dialog" style="width: 440px;">
      <div class="modal-header">Buka Repositori Git</div>
      <input type="text" class="commit-input" bind:value={openRepoPath} placeholder="/path/ke/repo" />
      {#if recentRepos.length > 0}
        <span style="font-size: 11px; color: var(--text-secondary); margin-top: 8px;">Repositori Terakhir:</span>
        <div style="max-height: 120px; overflow-y: auto; border: 1px solid var(--border-default); border-radius: var(--radius-sm); margin-top: 4px;">
          {#each recentRepos as r (r)}
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
              onclick={() => { openRepoPath = r; }}
              style="padding: 6px 10px; cursor: pointer; font-size: 11px; font-family: var(--font-mono); border-bottom: 1px solid var(--border-muted);"
            >
              {r}
            </div>
          {/each}
        </div>
      {/if}
      <div class="modal-actions" style="margin-top: 12px;">
        <button class="btn-secondary" onclick={closeModal}>Batal</button>
        <button class="btn-primary" onclick={() => { closeModal(); loadRepo(openRepoPath); }}>Buka</button>
      </div>
    </div>
  </div>
{:else if activeModal === "clone"}
  <div class="modal-overlay">
    <div class="modal-dialog">
      <div class="modal-header">Clone Repositori</div>
      <div style="display: flex; flex-direction: column; gap: 6px;">
        <span style="font-size: 11px; color: var(--text-secondary);">URL Repositori (HTTPS / SSH)</span>
        <input type="text" class="commit-input" bind:value={cloneUrl} placeholder="https://github.com/user/repo.git" />
        <span style="font-size: 11px; color: var(--text-secondary); margin-top: 6px;">Folder Tujuan</span>
        <input type="text" class="commit-input" bind:value={cloneTarget} placeholder="/home/user/Development/my-repo" />
      </div>
      <div class="modal-actions">
        <button class="btn-secondary" onclick={closeModal}>Batal</button>
        <button class="btn-primary" onclick={handleClone}>Clone</button>
      </div>
    </div>
  </div>
{:else if activeModal === "stash"}
  <div class="modal-overlay">
    <div class="modal-dialog" style="width: 440px;">
      <div class="modal-header">Git Stash</div>
      <div style="display: flex; flex-direction: column; gap: 8px;">
        <div style="display: flex; gap: 6px;">
          <input type="text" class="commit-input" bind:value={stashMsg} placeholder="Pesan stash (opsional)" style="flex: 1;" />
          <button onclick={handleStashSave} class="btn-primary" style="font-size: 11px;">Simpan Stash</button>
        </div>
        <button onclick={handleStashPop} class="btn-secondary" style="align-self: flex-start; font-size: 11px;" disabled={stashList.length === 0}>
          Terapkan Stash Teratas (Pop)
        </button>
        <span style="font-size: 11px; color: var(--text-secondary); margin-top: 4px;">Daftar Stash ({stashList.length})</span>
        <div style="max-height: 160px; overflow-y: auto; border: 1px solid var(--border-default); border-radius: var(--radius-sm); font-size: 11px; padding: 6px;">
          {#if stashList.length === 0}
            <div style="color: var(--text-muted); text-align: center; padding: 12px;">Tidak ada stash tersimpan</div>
          {:else}
            {#each stashList as s}
              <div style="padding: 4px 0; border-bottom: 1px solid var(--border-muted); font-family: var(--font-mono);">{s}</div>
            {/each}
          {/if}
        </div>
      </div>
      <div class="modal-actions" style="margin-top: 10px;">
        <button class="btn-secondary" onclick={closeModal}>Tutup</button>
      </div>
    </div>
  </div>
{:else if activeModal === "diag"}
  <div class="modal-overlay">
    <div class="modal-dialog" style="width: 500px; max-height: 80vh; display: flex; flex-direction: column;">
      <div class="modal-header">Laporan Diagnostik Sistem</div>
      <div style="overflow-y: auto; flex: 1; font-size: 11px; display: flex; flex-direction: column; gap: 8px;">
        <div><strong>Aplikasi:</strong> Forge Desktop v{diagReport?.app_version}</div>
        <div><strong>OS / Arch:</strong> {diagReport?.os_info}</div>
        <div><strong>Versi Git:</strong> {diagReport?.git_version}</div>
        <div><strong>Konfigurasi Git (Aman / Redacted):</strong></div>
        <pre style="background: var(--bg-subtle); padding: 8px; border-radius: var(--radius-sm); max-height: 180px; overflow-y: auto; font-family: var(--font-mono); font-size: 10px;">{diagReport?.sanitized_config.join("\n") || "Tidak ada config"}</pre>
      </div>
      <div class="modal-actions" style="margin-top: 12px;">
        <button
          class="btn-secondary"
          onclick={() => {
            if (diagReport) {
              const text = `App: Forge Desktop v${diagReport.app_version}\nOS: ${diagReport.os_info}\nGit: ${diagReport.git_version}\nConfig:\n${diagReport.sanitized_config.join("\n")}`;
              navigator.clipboard.writeText(text);
              setStatus("Diagnostik disalin ke clipboard");
            }
          }}
        >
          Salin Info
        </button>
        <button class="btn-primary" onclick={closeModal}>Tutup</button>
      </div>
    </div>
  </div>
{:else if activeModal === "auth"}
  <div class="modal-overlay">
    <div class="modal-dialog" style="width: 420px;">
      <div class="modal-header">Masuk dengan GitHub</div>
      {#if !deviceFlow}
        <p style="font-size: 12px; color: var(--text-secondary); margin-bottom: 8px;">
          Gunakan Device Code Authorization untuk menghubungkan repositori ke akun GitHub Anda.
        </p>
        <span style="font-size: 11px; color: var(--text-secondary);">GitHub OAuth Client ID</span>
        <input type="text" class="commit-input" bind:value={oauthClientId} placeholder="Client ID dari GitHub Developer" />
        <div style="margin-top: 12px; display: flex; justify-content: flex-end;">
          <button onclick={handleStartDeviceFlow} class="btn-primary" style="font-size: 12px;">Minta Kode Perangkat</button>
        </div>
      {:else}
        <div style="display: flex; flex-direction: column; gap: 10px; font-size: 12px;">
          <div>Buka link berikut di browser:</div>
          <a href={deviceFlow.verification_uri} target="_blank" rel="noreferrer" style="color: var(--accent-blue); font-family: var(--font-mono);">{deviceFlow.verification_uri}</a>
          <div>Dan masukkan kode perangkat ini:</div>
          <div style="font-size: 20px; font-weight: 700; font-family: var(--font-mono); text-align: center; padding: 10px; background: var(--bg-subtle); border-radius: var(--radius-sm); letter-spacing: 2px; color: var(--accent-blue);">
            {deviceFlow.user_code}
          </div>
          <div
            style="text-align: center; font-size: 11px; color: {pollStatusClass === 'success' ? 'var(--diff-add-text)' : pollStatusClass === 'error' ? 'var(--diff-del-text)' : 'var(--text-muted)'}; font-weight: {pollStatusClass ? '600' : 'normal'};"
          >
            {pollStatus}
          </div>
        </div>
      {/if}
      <div class="modal-actions" style="margin-top: 8px;">
        <button class="btn-secondary" onclick={closeModal}>Tutup</button>
      </div>
    </div>
  </div>
{/if}
