use crate::git::accounts::{KeyringSecretStore, SecretStore};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub const AI_KEYRING_ENTRY: &str = "custom_ai_api_key";
const MAX_CONTEXT_CHARS: usize = 28_000;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum AiProviderKind {
    #[default]
    OpenaiCompatible,
    Anthropic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    #[serde(default)]
    pub provider_kind: AiProviderKind,
    #[serde(default)]
    pub endpoint: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub system_prompt: String,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default)]
    pub memory_only: bool,
}

fn default_max_tokens() -> u32 {
    1200
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            provider_kind: AiProviderKind::OpenaiCompatible,
            endpoint: String::new(),
            model: String::new(),
            system_prompt: String::new(),
            max_tokens: default_max_tokens(),
            memory_only: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfigPublic {
    pub provider_kind: AiProviderKind,
    pub endpoint: String,
    pub model: String,
    pub system_prompt: String,
    pub max_tokens: u32,
    pub has_api_key: bool,
    pub api_key_preview: String,
    pub memory_only: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SaveAiConfigInput {
    pub provider_kind: AiProviderKind,
    pub endpoint: String,
    pub model: String,
    #[serde(default)]
    pub system_prompt: String,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    pub api_key: Option<String>,
    #[serde(default)]
    pub clear_api_key: bool,
    #[serde(default)]
    pub allow_memory_fallback: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiTestResult {
    pub ok: bool,
    pub resolved_url: String,
    pub model: String,
    pub reply_preview: String,
    pub latency_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiTaskResult {
    pub content: String,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub model: String,
    pub latency_ms: u64,
}

struct AiRuntimeState {
    config: AiConfig,
    memory_api_key: Option<String>,
}

pub struct AiManager {
    config_path: PathBuf,
    secret_store: Arc<dyn SecretStore>,
    state: Mutex<AiRuntimeState>,
}

impl AiManager {
    pub fn new(app_data_dir: PathBuf) -> Self {
        Self::with_secret_store(app_data_dir, Arc::new(KeyringSecretStore))
    }

    pub fn with_secret_store(app_data_dir: PathBuf, secret_store: Arc<dyn SecretStore>) -> Self {
        let config_path = app_data_dir.join("ai-config.json");
        let config = fs::read_to_string(&config_path)
            .ok()
            .and_then(|raw| serde_json::from_str::<AiConfig>(&raw).ok())
            .unwrap_or_default();
        Self {
            config_path,
            secret_store,
            state: Mutex::new(AiRuntimeState {
                config,
                memory_api_key: None,
            }),
        }
    }

    fn persist_config(&self, cfg: &AiConfig) -> Result<(), String> {
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Gagal membuat folder konfigurasi AI: {e}"))?;
        }
        let mut to_save = cfg.clone();
        to_save.memory_only = false;
        let json = serde_json::to_string_pretty(&to_save)
            .map_err(|e| format!("Gagal serialize konfigurasi AI: {e}"))?;
        fs::write(&self.config_path, json)
            .map_err(|e| format!("Gagal menulis konfigurasi AI: {e}"))
    }

    fn current_api_key_locked(&self, state: &AiRuntimeState) -> Option<String> {
        if let Some(mem) = state.memory_api_key.as_ref().filter(|s| !s.trim().is_empty()) {
            return Some(mem.clone());
        }
        self.secret_store
            .get_token(AI_KEYRING_ENTRY)
            .ok()
            .flatten()
            .filter(|s| !s.trim().is_empty())
    }

    fn make_public(&self, state: &AiRuntimeState) -> AiConfigPublic {
        let key = self.current_api_key_locked(state);
        let (has_api_key, api_key_preview) = match key {
            Some(ref k) if !k.trim().is_empty() => (true, mask_api_key(k)),
            _ => (false, String::new()),
        };
        AiConfigPublic {
            provider_kind: state.config.provider_kind,
            endpoint: state.config.endpoint.clone(),
            model: state.config.model.clone(),
            system_prompt: state.config.system_prompt.clone(),
            max_tokens: state.config.max_tokens,
            has_api_key,
            api_key_preview,
            memory_only: state.config.memory_only && state.memory_api_key.is_some(),
        }
    }

    pub fn get_config(&self) -> Result<AiConfigPublic, String> {
        let state = self
            .state
            .lock()
            .map_err(|_| "State AI sedang terkunci".to_string())?;
        Ok(self.make_public(&state))
    }

    pub fn save_config(&self, input: SaveAiConfigInput) -> Result<AiConfigPublic, String> {
        let endpoint = input.endpoint.trim().to_string();
        if !endpoint.is_empty() {
            validate_ai_endpoint(&endpoint)?;
        }
        let model = input.model.trim().to_string();
        let max_tokens = input.max_tokens.clamp(64, 32768);

        let mut state = self
            .state
            .lock()
            .map_err(|_| "State AI sedang terkunci".to_string())?;

        if input.clear_api_key {
            let _ = self.secret_store.delete_token(AI_KEYRING_ENTRY);
            state.memory_api_key = None;
            state.config.memory_only = false;
        } else if let Some(raw_key) = input.api_key {
            let trimmed = raw_key.trim().to_string();
            if !trimmed.is_empty() {
                if trimmed.contains(['\r', '\n', '"']) {
                    return Err("API key mengandung karakter terlarang".into());
                }
                match self.secret_store.set_token(AI_KEYRING_ENTRY, &trimmed) {
                    Ok(()) => {
                        state.memory_api_key = None;
                        state.config.memory_only = false;
                    }
                    Err(err) => {
                        if input.allow_memory_fallback {
                            state.memory_api_key = Some(trimmed);
                            state.config.memory_only = true;
                        } else {
                            return Err(format!("[KEYRING_UNAVAILABLE] {}", err.message()));
                        }
                    }
                }
            }
        }

        state.config.provider_kind = input.provider_kind;
        state.config.endpoint = endpoint;
        state.config.model = model;
        state.config.system_prompt = input.system_prompt.trim().to_string();
        state.config.max_tokens = max_tokens;

        self.persist_config(&state.config)?;
        Ok(self.make_public(&state))
    }

    pub fn test_connection(&self) -> Result<AiTestResult, String> {
        let (cfg, api_key) = self.snapshot_credentials()?;
        let url = resolve_completion_url(cfg.provider_kind, &cfg.endpoint)?;
        let model = require_model(&cfg)?;
        let start = Instant::now();
        let reply = call_ai_completion(
            cfg.provider_kind,
            &url,
            &model,
            api_key.as_deref(),
            "Reply with exactly OK.",
            "Ping",
            32,
        )?;
        Ok(AiTestResult {
            ok: true,
            resolved_url: url,
            model,
            reply_preview: reply.trim().chars().take(120).collect(),
            latency_ms: start.elapsed().as_millis() as u64,
        })
    }

    pub fn run_task(
        &self,
        task_kind: &str,
        prompt_input: &str,
        repo_path: Option<&str>,
        extra_context: Option<&str>,
    ) -> Result<AiTaskResult, String> {
        let (cfg, api_key) = self.snapshot_credentials()?;
        let url = resolve_completion_url(cfg.provider_kind, &cfg.endpoint)?;
        let model = require_model(&cfg)?;

        let (sys_prompt, user_prompt) = build_task_prompts(
            task_kind,
            prompt_input,
            repo_path,
            extra_context,
            &cfg.system_prompt,
        )?;

        let start = Instant::now();
        let raw = call_ai_completion(
            cfg.provider_kind,
            &url,
            &model,
            api_key.as_deref(),
            &sys_prompt,
            &user_prompt,
            cfg.max_tokens,
        )?;
        let latency_ms = start.elapsed().as_millis() as u64;

        let cleaned = match task_kind {
            "resolve_conflict" => strip_outer_code_fence(&raw),
            _ => raw.trim().to_string(),
        };

        let (summary, description) = if task_kind == "commit_message" {
            parse_commit_suggestion(&cleaned)
        } else {
            (None, None)
        };

        Ok(AiTaskResult {
            content: cleaned,
            summary,
            description,
            model,
            latency_ms,
        })
    }

    fn snapshot_credentials(&self) -> Result<(AiConfig, Option<String>), String> {
        let state = self
            .state
            .lock()
            .map_err(|_| "State AI sedang terkunci".to_string())?;
        let key = self.current_api_key_locked(&state);
        Ok((state.config.clone(), key))
    }
}

fn require_model(cfg: &AiConfig) -> Result<String, String> {
    let m = cfg.model.trim();
    if m.is_empty() {
        return Err("Model AI belum diisi di Settings -> Custom AI".into());
    }
    Ok(m.to_string())
}

pub fn mask_api_key(key: &str) -> String {
    let trimmed = key.trim();
    let chars: Vec<char> = trimmed.chars().collect();
    if chars.len() <= 8 {
        return "••••••••".into();
    }
    let prefix: String = chars[..4].iter().collect();
    let suffix: String = chars[chars.len() - 4..].iter().collect();
    format!("{prefix}••••{suffix}")
}

pub fn validate_ai_endpoint(endpoint: &str) -> Result<(), String> {
    let trimmed = endpoint.trim();
    if trimmed.is_empty() {
        return Err("Endpoint AI belum dikonfigurasi".into());
    }
    if !(trimmed.starts_with("https://") || trimmed.starts_with("http://")) {
        return Err("Endpoint AI harus diawali http:// atau https://".into());
    }
    if trimmed.contains(['\r', '\n', ' ']) {
        return Err("Endpoint AI tidak valid".into());
    }
    Ok(())
}

pub fn resolve_completion_url(provider: AiProviderKind, raw_endpoint: &str) -> Result<String, String> {
    validate_ai_endpoint(raw_endpoint)?;
    let trimmed = raw_endpoint.trim().trim_end_matches('/');
    match provider {
        AiProviderKind::OpenaiCompatible => {
            if trimmed.ends_with("/chat/completions")
                || trimmed.ends_with("/completions")
                || trimmed.ends_with("/responses")
            {
                Ok(trimmed.to_string())
            } else if trimmed.ends_with("/v1")
                || trimmed.ends_with("/openai")
                || trimmed.contains("/v1/")
            {
                Ok(format!("{trimmed}/chat/completions"))
            } else {
                Ok(format!("{trimmed}/v1/chat/completions"))
            }
        }
        AiProviderKind::Anthropic => {
            if trimmed.ends_with("/messages") {
                Ok(trimmed.to_string())
            } else if trimmed.ends_with("/v1") {
                Ok(format!("{trimmed}/messages"))
            } else {
                Ok(format!("{trimmed}/v1/messages"))
            }
        }
    }
}

struct TempPayloadFile {
    path: PathBuf,
}

impl TempPayloadFile {
    fn write_json(bytes: &[u8]) -> Result<Self, String> {
        let mut dir = std::env::temp_dir();
        let pid = std::process::id();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        dir.push(format!("forge-ai-{pid}-{nanos}.json"));
        let mut f = fs::File::create(&dir)
            .map_err(|e| format!("Gagal membuat file payload sementara AI: {e}"))?;
        f.write_all(bytes)
            .map_err(|e| format!("Gagal menulis payload AI: {e}"))?;
        Ok(Self { path: dir })
    }
}

impl Drop for TempPayloadFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn call_ai_completion(
    provider: AiProviderKind,
    url: &str,
    model: &str,
    api_key: Option<&str>,
    system_prompt: &str,
    user_prompt: &str,
    max_tokens: u32,
) -> Result<String, String> {
    let payload_value = match provider {
        AiProviderKind::OpenaiCompatible => {
            let mut msgs = Vec::new();
            if !system_prompt.trim().is_empty() {
                msgs.push(serde_json::json!({ "role": "system", "content": system_prompt }));
            }
            msgs.push(serde_json::json!({ "role": "user", "content": user_prompt }));
            serde_json::json!({
                "model": model,
                "messages": msgs,
                "max_tokens": max_tokens,
                "stream": false
            })
        }
        AiProviderKind::Anthropic => serde_json::json!({
            "model": model,
            "system": system_prompt,
            "messages": [{ "role": "user", "content": user_prompt }],
            "max_tokens": max_tokens
        }),
    };

    let payload_bytes = serde_json::to_vec(&payload_value)
        .map_err(|e| format!("Gagal menyusun payload AI: {e}"))?;
    let tmp_file = TempPayloadFile::write_json(&payload_bytes)?;
    let data_arg = format!("@{}", tmp_file.path.to_string_lossy());

    let mut cmd = crate::system::command_no_window("curl");
    cmd.args([
        "-s",
        "-i",
        "--max-time",
        "90",
        "-X",
        "POST",
        "-K",
        "-",
        "--data-binary",
        &data_arg,
        url,
    ])
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Gagal menjalankan curl ke endpoint AI: {e}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        let mut cfg = String::from("header = \"Content-Type: application/json\"\n");
        cfg.push_str(&format!(
            "header = \"User-Agent: ForgeDesktop/{}\"\n",
            env!("CARGO_PKG_VERSION")
        ));
        if let Some(key) = api_key.map(str::trim).filter(|k| !k.is_empty()) {
            match provider {
                AiProviderKind::OpenaiCompatible => {
                    cfg.push_str(&format!("header = \"Authorization: Bearer {key}\"\n"));
                }
                AiProviderKind::Anthropic => {
                    cfg.push_str(&format!("header = \"x-api-key: {key}\"\n"));
                    cfg.push_str("header = \"anthropic-version: 2023-06-01\"\n");
                }
            }
        } else if provider == AiProviderKind::Anthropic {
            cfg.push_str("header = \"anthropic-version: 2023-06-01\"\n");
        }
        let _ = stdin.write_all(cfg.as_bytes());
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("Gagal membaca respon AI: {e}"))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if err.is_empty() {
            "Gagal terhubung ke endpoint AI".into()
        } else {
            err
        });
    }

    let raw_str = String::from_utf8_lossy(&output.stdout);
    let parsed_http = crate::git::github::parse_curl_http_response(&raw_str);
    if parsed_http.status_code >= 400 || parsed_http.status_code == 0 {
        let msg = extract_api_error(&parsed_http.body).unwrap_or_else(|| {
            if parsed_http.body.trim().is_empty() {
                format!("HTTP {}", parsed_http.status_code)
            } else {
                parsed_http.body.chars().take(400).collect()
            }
        });
        return Err(format!(
            "Endpoint AI error (HTTP {}): {msg}",
            parsed_http.status_code
        ));
    }

    extract_ai_text(provider, &parsed_http.body)
}

fn extract_api_error(body: &str) -> Option<String> {
    let val: serde_json::Value = serde_json::from_str(body).ok()?;
    val.get("error")
        .and_then(|e| e.get("message").or(Some(e)))
        .and_then(|m| m.as_str())
        .or_else(|| val.get("message").and_then(|m| m.as_str()))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub fn extract_ai_text(provider: AiProviderKind, body: &str) -> Result<String, String> {
    let val: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| format!("Respon AI bukan JSON valid: {e}"))?;

    if provider == AiProviderKind::Anthropic {
        if let Some(blocks) = val.get("content").and_then(|a| a.as_array()) {
            let joined = blocks
                .iter()
                .filter_map(|b| b.get("text").and_then(|t| t.as_str()))
                .collect::<Vec<_>>()
                .join("\n");
            if !joined.trim().is_empty() {
                return Ok(joined);
            }
        }
        return Err("Respon AI Anthropic tidak memiliki teks keluaran".into());
    }

    let text_opt = val
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|first| {
            first
                .get("message")
                .and_then(|m| m.get("content"))
                .and_then(|c| c.as_str())
                .or_else(|| first.get("text").and_then(|t| t.as_str()))
        })
        .or_else(|| val.get("output_text").and_then(|t| t.as_str()))
        .or_else(|| {
            val.get("message")
                .and_then(|m| m.get("content"))
                .and_then(|c| c.as_str())
        });

    text_opt
        .map(|s| s.to_string())
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| "Respon AI tidak memiliki teks keluaran".into())
}

fn truncate_context(s: &str) -> String {
    if s.chars().count() <= MAX_CONTEXT_CHARS {
        return s.to_string();
    }
    let mut out: String = s.chars().take(MAX_CONTEXT_CHARS).collect();
    out.push_str("\n... [diff truncated]");
    out
}

fn read_repo_diff_context(repo: &str) -> String {
    let status = crate::git::cli::run(repo, &["status", "--short"]).unwrap_or_default();
    let staged = crate::git::cli::run(repo, &["diff", "--cached", "--no-color"]).unwrap_or_default();
    let unstaged = crate::git::cli::run(repo, &["diff", "--no-color"]).unwrap_or_default();
    let mut parts = Vec::new();
    if !status.trim().is_empty() {
        parts.push(format!("Status:\n{}", status.trim()));
    }
    if !staged.trim().is_empty() {
        parts.push(format!("Staged Diff:\n{}", staged.trim()));
    } else if !unstaged.trim().is_empty() {
        parts.push(format!("Working Tree Diff:\n{}", unstaged.trim()));
    }
    truncate_context(&parts.join("\n\n"))
}

fn build_task_prompts(
    task_kind: &str,
    prompt_input: &str,
    repo_path: Option<&str>,
    extra_context: Option<&str>,
    custom_system: &str,
) -> Result<(String, String), String> {
    let base_sys = if custom_system.trim().is_empty() {
        "You are an expert senior software engineer integrated into Forge Desktop (a Git client). Be concise, accurate, and directly actionable."
    } else {
        custom_system.trim()
    };

    match task_kind {
        "commit_message" => {
            let diff_ctx = if let Some(ctx) = extra_context.filter(|s| !s.trim().is_empty()) {
                truncate_context(ctx)
            } else if let Some(repo) = repo_path.filter(|s| !s.trim().is_empty()) {
                read_repo_diff_context(repo)
            } else {
                truncate_context(prompt_input)
            };
            if diff_ctx.trim().is_empty() {
                return Err("Tidak ada perubahan diff untuk dibuatkan pesan commit".into());
            }
            let sys = format!(
                "{base_sys}\nGenerate a conventional Git commit message for the diff. Output plain text ONLY in exact format:\n<summary line <= 72 chars>\n\n<optional concise bullet points explaining why/what>"
            );
            let user = if prompt_input.trim().is_empty() {
                diff_ctx
            } else {
                format!("Hint: {}\n\n{}", prompt_input.trim(), diff_ctx)
            };
            Ok((sys, user))
        }
        "resolve_conflict" => {
            let sys = format!(
                "{base_sys}\nResolve the Git merge conflict cleanly preserving both sides' intent where compatible. Return ONLY the final resolved source code with NO conflict markers and NO markdown code fences."
            );
            let user = match extra_context.filter(|s| !s.trim().is_empty()) {
                Some(ctx) => format!("{}\n\n{}", prompt_input.trim(), truncate_context(ctx)),
                None => truncate_context(prompt_input),
            };
            Ok((sys, user))
        }
        "explain_diff" | "explain_commit" | "review_changes" => {
            let ctx = if let Some(c) = extra_context.filter(|s| !s.trim().is_empty()) {
                truncate_context(c)
            } else if let Some(repo) = repo_path.filter(|s| !s.trim().is_empty()) {
                read_repo_diff_context(repo)
            } else {
                String::new()
            };
            let focus = match task_kind {
                "explain_commit" => "Explain this Git commit clearly: intent, key changes, and potential risks.",
                "review_changes" => "Review these code changes for bugs, edge cases, regressions, or security risks. Keep findings concrete.",
                _ => "Explain this Git diff concisely: what changed and why.",
            };
            let sys = format!("{base_sys}\n{focus}");
            let user = if ctx.is_empty() {
                truncate_context(prompt_input)
            } else if prompt_input.trim().is_empty() {
                ctx
            } else {
                format!("{}\n\n{}", prompt_input.trim(), ctx)
            };
            Ok((sys, user))
        }
        _ => {
            let mut user = prompt_input.trim().to_string();
            if user.is_empty() {
                return Err("Prompt AI tidak boleh kosong".into());
            }
            if let Some(ctx) = extra_context.filter(|s| !s.trim().is_empty()) {
                user.push_str("\n\nContext:\n");
                user.push_str(&truncate_context(ctx));
            } else if let Some(repo) = repo_path.filter(|s| !s.trim().is_empty()) {
                let repo_ctx = read_repo_diff_context(repo);
                if !repo_ctx.is_empty() {
                    user.push_str("\n\nActive Repository Context:\n");
                    user.push_str(&repo_ctx);
                }
            }
            Ok((base_sys.to_string(), user))
        }
    }
}

pub fn strip_outer_code_fence(raw: &str) -> String {
    let trimmed = raw.trim();
    if let Some(rest) = trimmed.strip_prefix("```") {
        let after_lang = match rest.find('\n') {
            Some(idx) => &rest[idx + 1..],
            None => rest,
        };
        if let Some(body) = after_lang.strip_suffix("```") {
            return body.trim_matches('\n').to_string();
        }
    }
    trimmed.to_string()
}

pub fn parse_commit_suggestion(raw: &str) -> (Option<String>, Option<String>) {
    let cleaned = strip_outer_code_fence(raw);
    let mut lines = cleaned.lines();
    let first = lines
        .find(|l| !l.trim().is_empty())
        .map(|l| l.trim().trim_matches('`').to_string());
    let rest: Vec<&str> = lines.collect();
    let desc = rest.join("\n").trim().to_string();
    (
        first,
        if desc.is_empty() { None } else { Some(desc) },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_urls_correctly() {
        assert_eq!(
            resolve_completion_url(AiProviderKind::OpenaiCompatible, "https://api.openai.com/v1").unwrap(),
            "https://api.openai.com/v1/chat/completions"
        );
        assert_eq!(
            resolve_completion_url(
                AiProviderKind::OpenaiCompatible,
                "https://openrouter.ai/api/v1/chat/completions"
            )
            .unwrap(),
            "https://openrouter.ai/api/v1/chat/completions"
        );
        assert_eq!(
            resolve_completion_url(AiProviderKind::OpenaiCompatible, "http://localhost:11434").unwrap(),
            "http://localhost:11434/v1/chat/completions"
        );
        assert_eq!(
            resolve_completion_url(AiProviderKind::Anthropic, "https://api.anthropic.com").unwrap(),
            "https://api.anthropic.com/v1/messages"
        );
    }

    #[test]
    fn extract_openai_and_anthropic_responses() {
        let oa = r#"{"choices":[{"message":{"content":"feat: add graph"}}]}"#;
        assert_eq!(
            extract_ai_text(AiProviderKind::OpenaiCompatible, oa).unwrap(),
            "feat: add graph"
        );
        let ant = r#"{"content":[{"type":"text","text":"Resolved output"}]}"#;
        assert_eq!(
            extract_ai_text(AiProviderKind::Anthropic, ant).unwrap(),
            "Resolved output"
        );
    }

    #[test]
    fn parse_commit_suggestion_splits_title_and_body() {
        let (title, desc) = parse_commit_suggestion("feat(ai): wire custom endpoint\n\n- add keyring support\n- add test");
        assert_eq!(title.as_deref(), Some("feat(ai): wire custom endpoint"));
        assert_eq!(desc.as_deref(), Some("- add keyring support\n- add test"));
    }

    #[test]
    fn strip_fence_works() {
        assert_eq!(
            strip_outer_code_fence("```rust\nfn main() {}\n```"),
            "fn main() {}"
        );
    }
}
