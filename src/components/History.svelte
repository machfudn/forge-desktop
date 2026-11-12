<script lang="ts">
  import type { CommitChangedFile, CommitDetail, CommitInfo } from "../types/git";
  import * as gitService from "../services/git";
  import Diff from "./Diff.svelte";

  interface Props {
    repoPath?: string;
    commits: CommitInfo[];
    searchQuery?: string;
    checkRunsMap?: Record<string, { status: string; conclusion?: string }>;
    onCherryPick?: (sha: string) => void;
    onRevert?: (sha: string) => void;
    onTag?: (sha: string) => void;
    onReset?: (sha: string) => void;
    onSquash?: (commitIndex: number, sha: string) => void;
    onStatus?: (msg: string, isError?: boolean) => void;
  }

  let {
    repoPath = "",
    commits,
    searchQuery = "",
    checkRunsMap = {},
    onCherryPick,
    onRevert,
    onTag,
    onReset,
    onSquash,
    onStatus,
  }: Props = $props();

  let selectedSha = $state<string | null>(null);
  let selectedDetail = $state<CommitDetail | null>(null);
  let detailLoading = $state(false);
  let detailError = $state("");
  let selectedCommitFile = $state<CommitChangedFile | null>(null);
  let commitFileDiff = $state("");
  let diffLoading = $state(false);
  let commitIgnoreWhitespace = $state(false);
  let detailRequestSeq = 0;
  let diffRequestSeq = 0;
  let lastRepoPath = "";

  const DETAIL_PCT_DEFAULT = 58;
  const DETAIL_PCT_MIN = 24;
  const DETAIL_PCT_MAX = 84;
  const DESC_H_MIN = 56;
  const FILES_W_DEFAULT = 220;
  const FILES_W_MIN = 140;
  const FILES_W_MAX = 480;
  const FILES_H_DEFAULT = 136;
  const FILES_H_MIN = 56;
  const FILES_H_MAX = 440;

  function readSavedNumber(key: string): number | null {
    const raw = localStorage.getItem(key);
    if (!raw) return null;
    const n = Number(raw);
    return Number.isFinite(n) ? n : null;
  }

  function clamp(val: number, min: number, max: number): number {
    return Math.min(max, Math.max(min, Math.round(val)));
  }

  let detailHeightPct = $state(
    clamp(readSavedNumber("forge_history_detail_pct") ?? DETAIL_PCT_DEFAULT, DETAIL_PCT_MIN, DETAIL_PCT_MAX)
  );
  let descHeaderHeight = $state<number | null>(readSavedNumber("forge_history_desc_h"));
  let filesWidthPx = $state(
    clamp(readSavedNumber("forge_history_files_w") ?? FILES_W_DEFAULT, FILES_W_MIN, FILES_W_MAX)
  );
  let filesHeightPx = $state(
    clamp(readSavedNumber("forge_history_files_h") ?? FILES_H_DEFAULT, FILES_H_MIN, FILES_H_MAX)
  );
  let filesLayout = $state<"horizontal" | "vertical">(
    localStorage.getItem("forge_history_files_layout") === "vertical" ? "vertical" : "horizontal"
  );

  let historyShellEl = $state<HTMLDivElement | null>(null);
  let detailPaneEl = $state<HTMLElement | null>(null);
  let detailHeaderEl = $state<HTMLDivElement | null>(null);
  let isCompactWidth = $state(false);
  let activeResizer = $state<"detail" | "desc" | "files" | null>(null);

  let isFilesStacked = $derived(isCompactWidth || filesLayout === "vertical");

  $effect(() => {
    const el = historyShellEl;
    if (!el) return;
    const updateWidth = () => {
      isCompactWidth = el.clientWidth > 0 && el.clientWidth <= 640;
    };
    updateWidth();
    const observer = new ResizeObserver(updateWidth);
    observer.observe(el);
    return () => observer.disconnect();
  });

  function updateDetailPct(next: number) {
    const clamped = clamp(next, DETAIL_PCT_MIN, DETAIL_PCT_MAX);
    detailHeightPct = clamped;
    localStorage.setItem("forge_history_detail_pct", String(clamped));
  }

  function updateDescHeaderHeight(next: number) {
    const maxH = Math.max(140, Math.round((detailPaneEl?.clientHeight ?? 520) * 0.72));
    const clamped = clamp(next, DESC_H_MIN, maxH);
    descHeaderHeight = clamped;
    localStorage.setItem("forge_history_desc_h", String(clamped));
  }

  function resetDescHeaderHeight() {
    descHeaderHeight = null;
    localStorage.removeItem("forge_history_desc_h");
  }

  function updateFilesWidth(next: number) {
    const maxW = Math.min(FILES_W_MAX, Math.max(FILES_W_MIN, Math.round((detailPaneEl?.clientWidth ?? 700) * 0.65)));
    const clamped = clamp(next, FILES_W_MIN, maxW);
    filesWidthPx = clamped;
    localStorage.setItem("forge_history_files_w", String(clamped));
  }

  function updateFilesHeight(next: number) {
    const maxH = Math.min(FILES_H_MAX, Math.max(120, Math.round((detailPaneEl?.clientHeight ?? 520) * 0.65)));
    const clamped = clamp(next, FILES_H_MIN, maxH);
    filesHeightPx = clamped;
    localStorage.setItem("forge_history_files_h", String(clamped));
  }

  function toggleFilesLayout() {
    filesLayout = filesLayout === "horizontal" ? "vertical" : "horizontal";
    localStorage.setItem("forge_history_files_layout", filesLayout);
  }

  function startDrag(
    kind: "detail" | "desc" | "files",
    axis: "x" | "y",
    e: PointerEvent,
    onMoveDelta: (delta: number) => void
  ) {
    e.preventDefault();
    const startPos = axis === "y" ? e.clientY : e.clientX;
    activeResizer = kind;

    const onMove = (ev: PointerEvent) => {
      const pos = axis === "y" ? ev.clientY : ev.clientX;
      onMoveDelta(pos - startPos);
    };
    const onUp = () => {
      activeResizer = null;
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
      window.removeEventListener("pointercancel", onUp);
    };
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
    window.addEventListener("pointercancel", onUp);
  }

  function startDetailResize(e: PointerEvent) {
    const shellH = historyShellEl?.clientHeight || 500;
    const startPct = detailHeightPct;
    startDrag("detail", "y", e, (dy) => {
      updateDetailPct(startPct - (dy / shellH) * 100);
    });
  }

  function handleDetailResizerKeydown(e: KeyboardEvent) {
    const step = e.shiftKey ? 8 : 4;
    if (e.key === "ArrowUp") {
      e.preventDefault();
      updateDetailPct(detailHeightPct + step);
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      updateDetailPct(detailHeightPct - step);
    }
  }

  function startDescResize(e: PointerEvent) {
    const startH = detailHeaderEl?.getBoundingClientRect().height ?? 110;
    startDrag("desc", "y", e, (dy) => {
      updateDescHeaderHeight(startH + dy);
    });
  }

  function handleDescResizerKeydown(e: KeyboardEvent) {
    const current = descHeaderHeight ?? (detailHeaderEl?.getBoundingClientRect().height ?? 110);
    const step = e.shiftKey ? 28 : 14;
    if (e.key === "ArrowDown") {
      e.preventDefault();
      updateDescHeaderHeight(current + step);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      updateDescHeaderHeight(current - step);
    }
  }

  function startFilesResize(e: PointerEvent) {
    if (isFilesStacked) {
      const startH = filesHeightPx;
      startDrag("files", "y", e, (dy) => {
        updateFilesHeight(startH + dy);
      });
    } else {
      const startW = filesWidthPx;
      startDrag("files", "x", e, (dx) => {
        updateFilesWidth(startW + dx);
      });
    }
  }

  function handleFilesResizerKeydown(e: KeyboardEvent) {
    const step = e.shiftKey ? 24 : 12;
    if (isFilesStacked) {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        updateFilesHeight(filesHeightPx + step);
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        updateFilesHeight(filesHeightPx - step);
      }
    } else {
      if (e.key === "ArrowRight") {
        e.preventDefault();
        updateFilesWidth(filesWidthPx + step);
      } else if (e.key === "ArrowLeft") {
        e.preventDefault();
        updateFilesWidth(filesWidthPx - step);
      }
    }
  }
  let indexedCommits = $derived(
    commits
      .map((commit, index) => ({ commit, index }))
      .filter(({ commit }) => {
        const q = searchQuery.trim().toLowerCase();
        if (!q) return true;
        return (
          commit.summary.toLowerCase().includes(q) ||
          commit.author.toLowerCase().includes(q) ||
          commit.short_sha.toLowerCase().includes(q) ||
          commit.sha.toLowerCase().includes(q)
        );
      })
  );

  let selectedCommitSummary = $derived(
    commits.find((c) => c.sha === selectedSha) ?? null
  );

  function clearSelection() {
    detailRequestSeq++;
    diffRequestSeq++;
    selectedSha = null;
    selectedDetail = null;
    detailLoading = false;
    detailError = "";
    selectedCommitFile = null;
    commitFileDiff = "";
    diffLoading = false;
  }

  $effect(() => {
    if (repoPath !== lastRepoPath) {
      lastRepoPath = repoPath;
      if (selectedSha) clearSelection();
      return;
    }
    if (selectedSha && !commits.some((c) => c.sha === selectedSha)) {
      clearSelection();
    }
  });

  async function loadFileDiff(
    sha: string,
    file: CommitChangedFile,
    ignoreWs = commitIgnoreWhitespace
  ) {
    if (!repoPath) return;
    const reqId = ++diffRequestSeq;
    selectedCommitFile = file;
    diffLoading = true;
    try {
      const diff = await gitService.getCommitFileDiff(
        repoPath,
        sha,
        file.path,
        file.old_path,
        ignoreWs
      );
      if (reqId !== diffRequestSeq) return;
      commitFileDiff = diff;
    } catch (err) {
      if (reqId !== diffRequestSeq) return;
      commitFileDiff = "";
      onStatus?.(String(err), true);
    } finally {
      if (reqId === diffRequestSeq) {
        diffLoading = false;
      }
    }
  }

  async function handleSelectCommit(sha: string) {
    if (selectedSha === sha) {
      clearSelection();
      return;
    }
    selectedSha = sha;
    if (!repoPath) return;

    const reqId = ++detailRequestSeq;
    diffRequestSeq++;
    detailLoading = true;
    detailError = "";
    selectedDetail = null;
    selectedCommitFile = null;
    commitFileDiff = "";
    diffLoading = false;

    try {
      const detail = await gitService.getCommitDetail(repoPath, sha);
      if (reqId !== detailRequestSeq) return;
      selectedDetail = detail;
      if (detail.files.length > 0) {
        await loadFileDiff(detail.sha, detail.files[0]);
      }
    } catch (err) {
      if (reqId !== detailRequestSeq) return;
      detailError = String(err);
      onStatus?.(`Gagal memuat detail commit: ${err}`, true);
    } finally {
      if (reqId === detailRequestSeq) {
        detailLoading = false;
      }
    }
  }

  async function handleToggleWhitespace() {
    commitIgnoreWhitespace = !commitIgnoreWhitespace;
    if (selectedSha && selectedCommitFile) {
      await loadFileDiff(selectedSha, selectedCommitFile, commitIgnoreWhitespace);
    }
  }
</script>

{#if commits.length === 0}
  <div class="empty-state">
    <span>Belum ada riwayat commit di branch ini.</span>
  </div>
{:else}
  <div
    bind:this={historyShellEl}
    class="history-shell {activeResizer ? 'is-resizing-any' : ''}"
    style="--detail-pane-pct: {detailHeightPct}%; --commit-files-w: {filesWidthPx}px; --commit-files-h: {filesHeightPx}px;"
  >
    <div class="history-container {selectedSha ? 'has-detail' : ''}">
    <div class="history-header-bar">
      <span>SHA</span>
      <span>KOMIT</span>
      <span>STATUS</span>
      <span class="commit-author-col">PENULIS</span>
      <span style="text-align: right;">TANGGAL</span>
    </div>

    {#if indexedCommits.length === 0}
      <div class="empty-state">
        <span>Tidak ada komit yang cocok dengan "{searchQuery}".</span>
      </div>
    {:else}
      {#each indexedCommits as { commit, index } (commit.sha)}
        {@const isSelected = selectedSha === commit.sha}
        <button
          type="button"
          class="commit-row {isSelected ? 'selected' : ''}"
          aria-expanded={isSelected}
          onclick={() => handleSelectCommit(commit.sha)}
        >
          <span class="commit-sha">
            {#if checkRunsMap[commit.sha]}
              {@const ci = checkRunsMap[commit.sha]}
              <span
                class="ci-dot"
                style="background-color: {ci.conclusion === 'success'
                  ? 'var(--diff-add-text)'
                  : ci.conclusion === 'failure'
                    ? 'var(--diff-del-text)'
                    : 'var(--warn-text)'};"
                title="CI: {ci.conclusion || ci.status}"
              ></span>
            {/if}
            {commit.short_sha}
          </span>
          <span class="commit-summary" title={commit.summary}>{commit.summary}</span>
          <span
            class="commit-push-badge {commit.is_pushed ? 'is-pushed' : 'is-local'}"
            title={commit.is_pushed
              ? "Sudah di-push ke repository remote"
              : "Masih lokal (belum di-push ke remote)"}
          >
            {#if commit.is_pushed}
              <svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                <path d="M4.5 12.75h7.1a2.65 2.65 0 0 0 .45-5.26A3.85 3.85 0 0 0 4.55 6.4 3.1 3.1 0 0 0 4.5 12.75Z" />
                <path d="M6.4 9.55 7.7 10.85l2.6-2.7" />
              </svg>
              <span>Remote</span>
            {:else}
              <svg width="11" height="11" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                <path d="M8 10V2.5" />
                <path d="M4.75 5.5 8 2.25l3.25 3.25" />
                <path d="M2.5 10.5v2a1 1 0 0 0 1 1h9a1 1 0 0 0 1-1v-2" />
              </svg>
              <span>Lokal</span>
            {/if}
          </span>
          <span class="commit-author">{commit.author}</span>
          <span class="commit-date">{commit.date}</span>
        </button>

        {#if isSelected}
          <div class="history-action-bar">
            <span class="badge-neutral">{commit.author}</span>
            {#if onSquash && index > 0 && !searchQuery.trim()}
              <button
                type="button"
                onclick={() => onSquash(index, commit.sha)}
                class="btn-secondary btn-xs"
                title="Squash {index + 1} komit dari HEAD ke komit ini"
              >
                Squash {index + 1} komit
              </button>
            {/if}
            {#if onCherryPick}
              <button
                type="button"
                onclick={() => onCherryPick(commit.sha)}
                class="btn-secondary btn-xs"
                title="Salin komit ini ke branch saat ini"
              >
                Cherry-pick
              </button>
            {/if}
            {#if onRevert}
              <button
                type="button"
                onclick={() => onRevert(commit.sha)}
                class="btn-secondary btn-xs"
                title="Balikkan perubahan komit ini"
              >
                Revert
              </button>
            {/if}
            {#if onTag}
              <button
                type="button"
                onclick={() => onTag(commit.sha)}
                class="btn-secondary btn-xs"
                title="Buat tag di komit ini"
              >
                Buat Tag
              </button>
            {/if}
            {#if onReset}
              <button
                type="button"
                onclick={() => onReset(commit.sha)}
                class="btn-secondary btn-xs btn-danger"
                style="margin-left: auto;"
                title="Reset branch ke titik komit ini (soft reset)"
              >
                Reset ke sini
              </button>
            {/if}
          </div>
        {/if}
      {/each}
    {/if}
    </div>

    {#if selectedSha && repoPath}
      <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
      <div
        class="pane-resizer-h {activeResizer === 'detail' ? 'is-resizing' : ''}"
        role="separator"
        aria-orientation="horizontal"
        aria-label="Ubah tinggi panel detail komit"
        title="Seret ke atas/bawah untuk ubah tinggi detail (klik ganda untuk reset)"
        tabindex="0"
        onpointerdown={startDetailResize}
        ondblclick={() => updateDetailPct(DETAIL_PCT_DEFAULT)}
        onkeydown={handleDetailResizerKeydown}
      ></div>
      <section
        bind:this={detailPaneEl}
        class="commit-detail-pane"
        aria-label="Detail komit terpilih"
      >
        <div
          bind:this={detailHeaderEl}
          class="commit-detail-header"
          style={descHeaderHeight !== null ? `height: ${descHeaderHeight}px; max-height: none;` : undefined}
        >
          <div class="commit-detail-top">
            <div class="commit-detail-title-group">
              <h3 class="commit-detail-summary">
                {selectedDetail?.summary || selectedCommitSummary?.summary || ""}
              </h3>
              <span class="commit-detail-sha" title={selectedSha}>
                {selectedDetail?.short_sha || selectedCommitSummary?.short_sha || selectedSha.slice(0, 7)}
              </span>
              {#if selectedCommitSummary}
                <span
                  class="commit-push-badge {selectedCommitSummary.is_pushed ? 'is-pushed' : 'is-local'}"
                  title={selectedCommitSummary.is_pushed
                    ? "Sudah di-push ke repository remote"
                    : "Masih lokal (belum di-push ke remote)"}
                >
                  {#if selectedCommitSummary.is_pushed}
                    <svg width="12" height="12" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                      <path d="M4.5 12.75h7.1a2.65 2.65 0 0 0 .45-5.26A3.85 3.85 0 0 0 4.55 6.4 3.1 3.1 0 0 0 4.5 12.75Z" />
                      <path d="M6.4 9.55 7.7 10.85l2.6-2.7" />
                    </svg>
                    <span>Sudah Push</span>
                  {:else}
                    <svg width="11" height="11" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
                      <path d="M8 10V2.5" />
                      <path d="M4.75 5.5 8 2.25l3.25 3.25" />
                      <path d="M2.5 10.5v2a1 1 0 0 0 1 1h9a1 1 0 0 0 1-1v-2" />
                    </svg>
                    <span>Belum Push</span>
                  {/if}
                </span>
              {/if}
            </div>
            <button
              type="button"
              class="btn-secondary btn-xs"
              onclick={clearSelection}
              title="Tutup detail komit"
            >
              Tutup
            </button>
          </div>

          <div class="commit-detail-meta">
            <span>
              {selectedDetail?.author || selectedCommitSummary?.author}{#if selectedDetail?.author_email} &lt;{selectedDetail.author_email}&gt;{/if}
            </span>
            <span>·</span>
            <span>{selectedDetail?.date || selectedCommitSummary?.date}</span>
            {#if selectedDetail}
              <span>·</span>
              <span class="tabular-nums">{selectedDetail.files.length} file diubah</span>
            {/if}
          </div>

          {#if detailLoading}
            <div class="commit-detail-empty-desc">Memuat deskripsi dan daftar perubahan...</div>
          {:else if detailError}
            <div class="alert-banner" style="border-radius: var(--radius-sm); border: 1px solid var(--danger-border);">
              <span>{detailError}</span>
            </div>
          {:else if selectedDetail?.description}
            <pre class="commit-detail-desc">{selectedDetail.description}</pre>
          {:else if selectedDetail}
            <div class="commit-detail-empty-desc">Tidak ada deskripsi tambahan pada komit ini.</div>
          {/if}
        </div>
        {#if !detailLoading && !detailError && selectedDetail}
          <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
          <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
          <div
            class="pane-resizer-h subtle {activeResizer === 'desc' ? 'is-resizing' : ''}"
            role="separator"
            aria-orientation="horizontal"
            aria-label="Ubah tinggi deskripsi komit"
            title="Seret ke atas/bawah untuk lebarkan atau kecilkan deskripsi (klik ganda untuk otomatis)"
            tabindex="0"
            onpointerdown={startDescResize}
            ondblclick={resetDescHeaderHeight}
            onkeydown={handleDescResizerKeydown}
          ></div>
          {#if selectedDetail.files.length === 0}
            <div class="empty-state" style="padding: 18px;">
              <span>Tidak ada perubahan file pada komit ini.</span>
            </div>
          {:else}
            <div class="commit-detail-body {isFilesStacked ? 'is-stacked' : ''}">
              <div class="commit-files-pane">
                <div class="commit-files-header tabular-nums">
                  <span>FILE BERUBAH ({selectedDetail.files.length})</span>
                  {#if !isCompactWidth}
                    <button
                      type="button"
                      class="commit-files-layout-btn"
                      onclick={toggleFilesLayout}
                      title={filesLayout === "vertical"
                        ? "Ubah susunan daftar file ke kiri-kanan"
                        : "Ubah susunan daftar file ke atas-bawah"}
                    >
                      {filesLayout === "vertical" ? "Samping" : "Atas-Bawah"}
                    </button>
                  {/if}
                </div>
                <ul class="commit-files-list">
                  {#each selectedDetail.files as file (file.path)}
                    {@const isFileSelected = selectedCommitFile?.path === file.path}
                    {@const slashIdx = Math.max(file.path.lastIndexOf("/"), file.path.lastIndexOf("\\"))}
                    <li>
                      <button
                        type="button"
                        class="commit-file-btn {isFileSelected ? 'selected' : ''}"
                        onclick={() => loadFileDiff(selectedDetail!.sha, file)}
                        title={file.old_path ? `${file.old_path} → ${file.path}` : file.path}
                      >
                        <span class="file-status-badge status-{file.status_code}">
                          {file.status_code}
                        </span>
                        <span class="file-name">
                          {#if slashIdx >= 0}
                            <span class="file-dir">{file.path.slice(0, slashIdx + 1)}</span><span class="file-base">{file.path.slice(slashIdx + 1)}</span>
                          {:else}
                            <span class="file-base">{file.path}</span>
                          {/if}
                        </span>
                      </button>
                    </li>
                  {/each}
                </ul>
              </div>

              <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
              <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
              <div
                class="{isFilesStacked ? 'pane-resizer-h' : 'pane-resizer-v'} {activeResizer === 'files' ? 'is-resizing' : ''}"
                role="separator"
                aria-orientation={isFilesStacked ? "horizontal" : "vertical"}
                aria-label={isFilesStacked
                  ? "Ubah tinggi daftar file perubahan"
                  : "Ubah lebar daftar file perubahan"}
                title={isFilesStacked
                  ? "Seret ke atas/bawah untuk ubah tinggi daftar file (klik ganda untuk reset)"
                  : "Seret ke kiri/kanan untuk ubah lebar daftar file (klik ganda untuk reset)"}
                tabindex="0"
                onpointerdown={startFilesResize}
                ondblclick={() =>
                  isFilesStacked
                    ? updateFilesHeight(FILES_H_DEFAULT)
                    : updateFilesWidth(FILES_W_DEFAULT)}
                onkeydown={handleFilesResizerKeydown}
              ></div>

              <div class="commit-diff-pane">
                {#if diffLoading}
                  <div class="empty-state">
                    <span>Memuat diff perubahan...</span>
                  </div>
                {:else if selectedCommitFile}
                  <Diff
                    filePath={selectedCommitFile.path}
                    diffText={commitFileDiff}
                    ignoreWhitespace={commitIgnoreWhitespace}
                    onToggleWhitespace={handleToggleWhitespace}
                  />
                {/if}
              </div>
            </div>
          {/if}
        {/if}
      </section>
    {/if}
  </div>
{/if}
