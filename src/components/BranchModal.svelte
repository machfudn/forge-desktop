<script lang="ts">
  import type { AheadBehind, BranchInfo } from "../types/git";
  import * as gitService from "../services/git";

  interface Props {
    activeRepo: string;
    currentBranch: string;
    branches: BranchInfo[];
    branchDivergenceMap: Record<string, AheadBehind>;
    isLoadingBranches?: boolean;
    onInspectBranch?: (branchName: string) => void;
    hasDirtyFiles: boolean;
    dirtyFileCount: number;
    hasRemote: boolean;
    hasCommits: boolean;
    onClose: () => void;
    onReloadRepo: () => Promise<void>;
    onStatus: (msg: string, isError?: boolean) => void;
    onRequestConfirm: (
      title: string,
      message: string,
      action: () => Promise<void>,
      danger?: boolean
    ) => void;
  }

  let {
    activeRepo,
    currentBranch,
    branches = $bindable(),
    branchDivergenceMap,
    isLoadingBranches = false,
    onInspectBranch,
    hasDirtyFiles,
    dirtyFileCount,
    hasRemote,
    hasCommits,
    onClose,
    onReloadRepo,
    onStatus,
    onRequestConfirm,
  }: Props = $props();

  let pushingBranch = $state<string | null>(null);
  let branchSearch = $state("");
  let newBranchName = $state("");
  let branchToRename = $state<string | null>(null);
  let renameBranchInput = $state("");
  let inlineError = $state("");

  let filteredBranches = $derived(
    branches.filter((b) =>
      b.name.toLowerCase().includes(branchSearch.trim().toLowerCase())
    )
  );

  async function handleSwitchBranch(name: string) {
    if (hasDirtyFiles) {
      onRequestConfirm(
        "Smart Stash & Pindah Branch",
        `Terdapat ${dirtyFileCount} file dengan perubahan belum dikomit. Simpan perubahan ke Stash sebelum berpindah ke "${name}"?`,
        async () => {
          try {
            onStatus("Menyimpan perubahan ke stash...");
            await gitService.gitStashSave(activeRepo, `Auto-stash sebelum checkout ke ${name}`);
            onStatus(`Berpindah ke branch ${name}...`);
            await gitService.switchBranch(activeRepo, name);
            onStatus(`Berhasil pindah ke branch ${name} (perubahan tersimpan di Stash)`);
            await onReloadRepo();
          } catch (err) {
            onStatus(String(err), true);
          }
        },
        false
      );
      return;
    }

    onClose();
    try {
      await gitService.switchBranch(activeRepo, name);
      await onReloadRepo();
    } catch (err) {
      onStatus(String(err), true);
    }
  }

  async function handleCreateBranch() {
    const name = newBranchName.trim();
    if (!name) {
      inlineError = "Nama branch baru tidak boleh kosong.";
      return;
    }
    inlineError = "";
    onClose();
    try {
      await gitService.createBranch(activeRepo, name);
      onStatus(`Branch ${name} dibuat dan diaktifkan`);
      await onReloadRepo();
    } catch (err) {
      onStatus(String(err), true);
    }
  }

  function handleDeleteBranch(name: string, isCurrent: boolean) {
    if (isCurrent) {
      inlineError = "Tidak dapat menghapus branch yang sedang aktif.";
      return;
    }
    onRequestConfirm(
      "Hapus Branch Lokal",
      `Yakin ingin menghapus branch "${name}" secara paksa (-D)?`,
      async () => {
        try {
          await gitService.deleteBranch(activeRepo, name, true);
          branches = await gitService.listBranches(activeRepo);
          onStatus(`Branch ${name} berhasil dihapus`);
        } catch (err) {
          onStatus(String(err), true);
        }
      },
      true
    );
  }

  function handleStartRename(name: string) {
    branchToRename = name;
    renameBranchInput = name;
    inlineError = "";
  }

  async function handleConfirmRename() {
    if (!branchToRename) return;
    const newName = renameBranchInput.trim();
    if (!newName || newName === branchToRename) {
      branchToRename = null;
      return;
    }
    try {
      await gitService.renameBranch(activeRepo, branchToRename, newName);
      branchToRename = null;
      branches = await gitService.listBranches(activeRepo);
      await onReloadRepo();
      onStatus(`Branch berhasil diubah menjadi ${newName}`);
    } catch (err) {
      inlineError = String(err);
    }
  }

  function handleMergeBranch(targetName: string) {
    onRequestConfirm(
      "Gabungkan Branch (Merge)",
      `Gabungkan branch "${targetName}" ke branch aktif "${currentBranch}"?`,
      async () => {
        try {
          onStatus(`Menggabungkan ${targetName}...`);
          const res = await gitService.mergeBranch(activeRepo, targetName);
          onStatus(res || "Merge berhasil");
          await onReloadRepo();
        } catch (err) {
          onStatus(String(err), true);
        }
      },
      false
    );
  }

  function handleRebase(targetName: string) {
    onRequestConfirm(
      "Rebase Branch",
      `Rebase branch aktif "${currentBranch}" ke atas "${targetName}"?`,
      async () => {
        try {
          onStatus(`Me-rebase ke ${targetName}...`);
          const res = await gitService.rebaseBranch(activeRepo, targetName);
          onStatus(res || "Rebase berhasil");
          await onReloadRepo();
        } catch (err) {
          onStatus(String(err), true);
        }
      },
      false
    );
  }

  async function handlePushBranch(name: string) {
    inlineError = "";
    pushingBranch = name;
    try {
      onStatus(`Melakukan push branch ${name}...`);
      const res = await gitService.gitPush(activeRepo, undefined, name);
      branches = await gitService.listBranches(activeRepo);
      await onReloadRepo();
      onStatus(res || `Branch ${name} berhasil di-push ke remote`);
    } catch (err) {
      inlineError = String(err);
      onStatus(`Gagal push ${name}: ${err}`, true);
    } finally {
      pushingBranch = null;
    }
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="modal-overlay"
  role="presentation"
  onclick={(e) => {
    if (e.target === e.currentTarget) onClose();
  }}
  onkeydown={(e) => {
    if (e.key === "Escape") onClose();
  }}
>
  <div
    class="modal-dialog"
    style="--modal-w: 510px;"
    role="dialog"
    aria-modal="true"
    aria-labelledby="branch-modal-title"
  >
    <div class="modal-header">
      <span id="branch-modal-title">Pilih atau Kelola Branch</span>
      <button type="button" class="icon-btn compact" onclick={onClose} aria-label="Tutup dialog">
        <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
          <path d="M3.72 3.72a.75.75 0 0 1 1.06 0L8 6.94l3.22-3.22a.749.749 0 0 1 1.275.326.749.749 0 0 1-.215.734L9.06 8l3.22 3.22a.749.749 0 0 1-.326 1.275.749.749 0 0 1-.734-.215L8 9.06l-3.22 3.22a.751.751 0 0 1-1.042-.018.751.751 0 0 1-.018-1.042L6.94 8 3.72 4.78a.75.75 0 0 1 0-1.06Z" />
        </svg>
      </button>
    </div>

    <div class="modal-body">
      <input
        type="search"
        class="form-input"
        aria-label="Filter nama branch"
        placeholder="Cari / filter branch..."
        bind:value={branchSearch}
      />
      <div style="display: flex; flex-wrap: wrap; gap: 6px;">
        <input
          type="text"
          class="form-input"
          aria-label="Nama branch baru"
          placeholder="Nama branch baru..."
          style="flex: 1 1 180px;"
          bind:value={newBranchName}
          onkeydown={(e) => {
            if (e.key === "Enter") handleCreateBranch();
          }}
        />
        <button type="button" onclick={handleCreateBranch} class="btn-primary">Buat Branch</button>
      </div>

      {#if inlineError}
        <div class="alert-banner" style="border-radius: var(--radius-sm); border: 1px solid var(--danger-border);">
          <span>{inlineError}</span>
        </div>
      {/if}

      {#if branchToRename}
        <div style="display: flex; flex-wrap: wrap; gap: 6px; padding: 8px; background: var(--bg-subtle); border-radius: var(--radius-sm); align-items: center;">
          <span style="font-size: 11px; color: var(--text-secondary); white-space: nowrap;">
            Ubah '{branchToRename}':
          </span>
          <input
            type="text"
            class="form-input"
            aria-label="Nama branch pengganti"
            bind:value={renameBranchInput}
            style="flex: 1 1 140px; font-size: 11px;"
            onkeydown={(e) => {
              if (e.key === "Enter") handleConfirmRename();
            }}
          />
          <button type="button" onclick={handleConfirmRename} class="btn-primary btn-xs">Simpan</button>
          <button type="button" onclick={() => (branchToRename = null)} class="btn-secondary btn-xs">Batal</button>
        </div>
      {/if}

      <div class="list-box">
        {#if filteredBranches.length === 0}
          <div style="padding: 20px; text-align: center; color: var(--text-muted); font-size: 12px;">
            {#if isLoadingBranches}
              Memuat daftar branch...
            {:else if !hasCommits && branches.length === 0}
              Branch '{currentBranch}' belum punya komit. Buat komit pertama agar branch tercatat di daftar.
            {:else}
              Tidak ada branch ditemukan
            {/if}
          </div>
        {:else}
          {#each filteredBranches as b (b.name)}
            <div class="list-row {b.is_current ? 'is-active' : ''}">
              <button
                type="button"
                onclick={() => handleSwitchBranch(b.name)}
                onmouseenter={() => {
                  if (!b.is_current) onInspectBranch?.(b.name);
                }}
                onfocus={() => {
                  if (!b.is_current) onInspectBranch?.(b.name);
                }}
                style="flex: 1; min-width: 0; text-align: left; display: flex; align-items: center; gap: 6px;"
              >
                <span style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap; {b.is_current ? 'font-weight: 600;' : ''}">
                  {b.name}
                </span>
                {#if b.is_current}
                  <span style="color: var(--text-accent); font-size: 10px; font-weight: 600; flex-shrink: 0;">(aktif)</span>
                {/if}
                {#if b.upstream_gone}
                  <span class="badge-warn" style="flex-shrink: 0;" title="Branch remote ({b.upstream}) sudah dihapus">
                    Remote Hilang
                  </span>
                {:else if !hasRemote}
                  <span class="badge-neutral" style="flex-shrink: 0;" title="Belum ada remote terhubung">
                    Lokal
                  </span>
                {:else if b.upstream}
                  {#if b.ahead === 0 && b.behind === 0}
                    <span class="badge-success" style="flex-shrink: 0;" title="Sudah di-push & sinkron dengan {b.upstream}">
                      Sudah Push
                    </span>
                  {:else}
                    <span class="badge-warning tabular-nums" style="flex-shrink: 0;" title="Upstream: {b.upstream}">
                      {b.ahead > 0 ? `Belum Push ↑${b.ahead}` : ""}{b.ahead > 0 && b.behind > 0 ? " " : ""}{b.behind > 0 ? `↓${b.behind}` : ""}
                    </span>
                  {/if}
                {:else if b.is_pushed}
                  <span class="badge-neutral" style="flex-shrink: 0;" title="Ada di remote, belum set upstream tracking">
                    Di Remote
                  </span>
                {:else}
                  <span class="badge-warning" style="flex-shrink: 0;" title="Branch belum pernah di-push ke remote">
                    Belum Push
                  </span>
                {/if}
                {#if !b.is_current && branchDivergenceMap[b.name]}
                  {@const div = branchDivergenceMap[b.name]}
                  {#if div.ahead > 0 || div.behind > 0}
                    <span class="tabular-nums" style="font-size: 10px; color: var(--text-muted); font-family: var(--font-mono); display: inline-flex; gap: 4px; flex-shrink: 0;" title="Perbandingan terhadap {currentBranch}">
                      {#if div.ahead > 0}<span style="color: var(--diff-add-text);">+{div.ahead}</span>{/if}
                      {#if div.behind > 0}<span style="color: var(--diff-del-text);">-{div.behind}</span>{/if}
                    </span>
                  {/if}
                {/if}
              </button>

              <div class="branch-row-actions">
                {#if hasRemote && hasCommits && (!b.is_pushed || b.ahead > 0)}
                  <button
                    type="button"
                    onclick={() => handlePushBranch(b.name)}
                    disabled={pushingBranch === b.name}
                    class="btn-primary btn-xs"
                    title={b.upstream ? `Push ke ${b.upstream}` : `Push & set upstream untuk ${b.name}`}
                  >
                    {pushingBranch === b.name ? "Push..." : "Push"}
                  </button>
                {/if}
                {#if !b.is_current}
                  <button type="button" onclick={() => handleMergeBranch(b.name)} class="btn-secondary btn-xs" title="Merge ke branch aktif">
                    Merge
                  </button>
                  <button type="button" onclick={() => handleRebase(b.name)} class="btn-secondary btn-xs" title="Rebase ke atas branch ini">
                    Rebase
                  </button>
                  <button type="button" onclick={() => handleStartRename(b.name)} class="btn-secondary btn-xs" title="Ganti nama branch">
                    Rename
                  </button>
                  <button type="button" onclick={() => handleDeleteBranch(b.name, b.is_current)} class="btn-secondary btn-xs btn-danger branch-delete-btn" title="Hapus branch lokal">
                    Hapus
                  </button>
                {/if}
              </div>
            </div>
          {/each}
        {/if}
      </div>
    </div>

    <div class="modal-actions">
      <button type="button" class="btn-secondary" onclick={onClose}>Tutup</button>
    </div>
  </div>
</div>
