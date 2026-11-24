import { t } from "../i18n/index.svelte";

export function localizeBackendError(error: unknown): string {
  if (!error) return "";
  const raw =
    typeof error === "string"
      ? error
      : error instanceof Error
        ? error.message
        : String(error);
  const trimmed = raw.trim();

  // 1. AI Errors
  if (
    trimmed.includes("Endpoint AI belum dikonfigurasi") ||
    trimmed.includes("[AI_ERR_NO_ENDPOINT]")
  ) {
    return t("ai.errors.endpointEmpty");
  }
  if (
    trimmed.includes("Model AI belum diisi") ||
    trimmed.includes("[AI_ERR_NO_MODEL]")
  ) {
    return t("ai.errors.modelEmpty");
  }
  if (
    trimmed.includes("Endpoint AI harus diawali") ||
    trimmed.includes("[AI_ERR_INVALID_PROTOCOL]")
  ) {
    return t("ai.errors.endpointProtocol");
  }
  if (
    trimmed.includes("Endpoint AI tidak valid") ||
    trimmed.includes("[AI_ERR_INVALID_ENDPOINT]")
  ) {
    return t("ai.errors.endpointInvalid");
  }
  if (
    trimmed.includes("Gagal terhubung ke endpoint AI") ||
    trimmed.includes("Gagal menjalankan curl ke endpoint AI") ||
    trimmed.includes("[AI_ERR_CONNECT_FAILED]")
  ) {
    return t("ai.errors.connectionFailed");
  }
  if (
    trimmed.includes("Tidak ada perubahan diff untuk dibuatkan pesan commit") ||
    trimmed.includes("[AI_ERR_NO_DIFF]")
  ) {
    return t("ai.errors.noDiff");
  }
  if (
    trimmed.includes("Prompt AI tidak boleh kosong") ||
    trimmed.includes("[AI_ERR_PROMPT_EMPTY]")
  ) {
    return t("ai.promptRequired");
  }
  if (
    trimmed.includes("Respon AI tidak memiliki teks keluaran") ||
    trimmed.includes("Respon AI Anthropic tidak memiliki teks keluaran") ||
    trimmed.includes("[AI_ERR_EMPTY_RESPONSE]")
  ) {
    return t("ai.errors.emptyResponse");
  }

  // 2. Keyring & OS Credentials
  if (
    trimmed.includes("Penyimpanan kredensial OS tidak tersedia") ||
    trimmed.includes("[KEYRING_UNAVAILABLE]")
  ) {
    return t("wsModal.auth.keyringUnavailable");
  }

  // 3. Git Operations & Status
  if (trimmed.includes("File .git/index.lock terdeteksi")) {
    return t("app.status.indexLockDetected");
  }
  if (trimmed.includes("Working tree masih memiliki perubahan belum di-commit")) {
    return t("app.status.dirtyWorkingTree");
  }
  if (trimmed.includes("Belum ada komit untuk di-push")) {
    return t("app.status.noCommitsToPush");
  }
  if (trimmed.includes("Tidak dapat push dari posisi detached HEAD")) {
    return t("app.status.detachedHeadPush");
  }
  if (trimmed.includes("Pesan commit wajib diisi")) {
    return t("app.status.commitSummaryRequired");
  }
  if (trimmed.includes("Belum ada komit untuk dibatalkan")) {
    return t("app.status.noCommitToUndo");
  }
  if (
    trimmed.includes("git tidak ditemukan") ||
    trimmed.includes("git tidak tersedia")
  ) {
    return t("app.status.gitNotFound");
  }

  // 4. Git CLI Common English Errors (Translate to localized form)
  const lower = trimmed.toLowerCase();
  if (
    lower.includes("could not resolve host") ||
    lower.includes("could not resolve hostname")
  ) {
    return t("gitErrors.dnsFailed");
  }
  if (lower.includes("permission denied (publickey)")) {
    return t("gitErrors.permissionDeniedSsh");
  }
  if (
    lower.includes("authentication failed for") ||
    lower.includes("could not read username")
  ) {
    return t("gitErrors.authFailed");
  }
  if (
    (lower.includes("fatal: repository") && lower.includes("not found")) ||
    lower.includes("repository not found")
  ) {
    return t("gitErrors.repoNotFound");
  }

  // Strip internal tags if left
  return trimmed.replace(/^\[[A-Z0-9_]+\]\s*/, "");
}
