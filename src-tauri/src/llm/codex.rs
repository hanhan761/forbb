//! Local Codex app-server provider.
//!
//! The Codex CLI is already authenticated on the user's machine, so NexQ talks
//! to `codex app-server --stdio` instead of asking for another API key. The
//! app-server protocol is newline-delimited JSON-RPC. A single process and
//! thread are kept for the lifetime of this provider so one interview can use
//! one persistent conversation.

use serde_json::{json, Value};
use std::env;
use std::path::Path;
use std::process::Stdio;
use std::sync::Mutex as StdMutex;
use std::time::Instant;
use tauri::Emitter;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex as TokioMutex;

use super::provider::{
    CompletionStats, GenerationParams, LLMError, LLMMessage, LLMProvider, ModelInfo,
    StreamEndPayload, StreamSourcesPayload, StreamTokenPayload, StreamSource,
};
use super::web_cache::WebAnswerCache;

const DEFAULT_MODEL_ID: &str = "codex-default";
const CODEX_BINARY_ENV: &str = "NEXQ_CODEX_BIN";

/// Check whether the Codex CLI can be resolved before selecting it as the
/// default provider. This is intentionally synchronous because it is called
/// during Tauri setup before the async runtime is available.
pub fn is_available() -> bool {
    if let Ok(path) = env::var(CODEX_BINARY_ENV) {
        return Path::new(&path).exists();
    }

    #[cfg(windows)]
    {
        std::process::Command::new("cmd.exe")
            .args(["/D", "/S", "/C", "where codex.cmd"])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    #[cfg(not(windows))]
    {
        std::process::Command::new("sh")
            .args(["-lc", "command -v codex"])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
}

struct CodexSession {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
    initialized: bool,
    thread_id: Option<String>,
}

impl Drop for CodexSession {
    fn drop(&mut self) {
        // Drop is synchronous, but start_kill only schedules the process
        // termination and is safe to use here.
        let _ = self.child.start_kill();
    }
}

impl CodexSession {
    async fn spawn() -> Result<Self, LLMError> {
        let mut command = build_app_server_command();
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            // Codex diagnostics are useful in the terminal during development
            // but should not pollute the JSON-RPC stream or open a console
            // window in the desktop build.
            .stderr(Stdio::null());

        let mut child = command.spawn().map_err(|e| {
            LLMError::ConnectionFailed(format!(
                "Unable to start Codex app-server. Install Codex CLI or set {}: {}",
                CODEX_BINARY_ENV, e
            ))
        })?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| LLMError::ConnectionFailed("Codex stdin unavailable".to_string()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| LLMError::ConnectionFailed("Codex stdout unavailable".to_string()))?;

        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
            next_id: 1,
            initialized: false,
            thread_id: None,
        })
    }

    fn is_alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    fn allocate_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        id
    }

    async fn write_message(&mut self, message: &Value) -> Result<(), LLMError> {
        let encoded = serde_json::to_string(message)
            .map_err(|e| LLMError::InvalidResponse(format!("Codex request encoding failed: {}", e)))?;
        self.stdin
            .write_all(format!("{}\n", encoded).as_bytes())
            .await
            .map_err(|e| LLMError::ConnectionFailed(format!("Codex stdin write failed: {}", e)))?;
        self.stdin
            .flush()
            .await
            .map_err(|e| LLMError::ConnectionFailed(format!("Codex stdin flush failed: {}", e)))?;
        Ok(())
    }

    async fn read_message(&mut self) -> Result<Value, LLMError> {
        let mut line = String::new();
        loop {
            line.clear();
            let bytes = self
                .stdout
                .read_line(&mut line)
                .await
                .map_err(|e| LLMError::ConnectionFailed(format!("Codex stdout read failed: {}", e)))?;
            if bytes == 0 {
                return Err(LLMError::ConnectionFailed(
                    "Codex app-server closed its stdout".to_string(),
                ));
            }

            match serde_json::from_str::<Value>(line.trim()) {
                Ok(message) => return Ok(message),
                Err(error) => {
                    log::warn!("Ignoring malformed Codex app-server line: {} ({})", line.trim(), error);
                }
            }
        }
    }

    async fn request(&mut self, method: &str, params: Value) -> Result<Value, LLMError> {
        let id = self.allocate_id();
        self.write_message(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        }))
        .await?;

        loop {
            let message = self.read_message().await?;
            if message.get("id").and_then(Value::as_u64) != Some(id) {
                // Notifications can arrive during startup. They are handled by
                // the turn loop when they matter; handshake notifications are
                // intentionally ignored here.
                continue;
            }

            if let Some(error) = message.get("error") {
                return Err(LLMError::ProviderError(format!(
                    "Codex {} failed: {}",
                    method, error
                )));
            }

            return Ok(message.get("result").cloned().unwrap_or(Value::Null));
        }
    }

    async fn notify(&mut self, method: &str, params: Value) -> Result<(), LLMError> {
        self.write_message(&json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        }))
        .await
    }

    async fn ensure_initialized(&mut self) -> Result<(), LLMError> {
        if self.initialized && self.is_alive() {
            return Ok(());
        }

        let result = self
            .request(
                "initialize",
                json!({
                    "clientInfo": {
                        "name": "NexQ",
                        "version": env!("CARGO_PKG_VERSION")
                    },
                    "capabilities": {}
                }),
            )
            .await?;

        if result.get("codexHome").and_then(Value::as_str).is_none() {
            return Err(LLMError::InvalidResponse(
                "Codex initialize response did not include codexHome".to_string(),
            ));
        }

        self.notify("initialized", json!({})).await?;
        self.initialized = true;
        Ok(())
    }

    async fn ensure_thread(
        &mut self,
        base_instructions: &str,
        model: Option<&str>,
    ) -> Result<String, LLMError> {
        self.ensure_initialized().await?;
        if let Some(thread_id) = self.thread_id.clone() {
            return Ok(thread_id);
        }

        let cwd = env::current_dir()
            .ok()
            .map(|path| path.to_string_lossy().into_owned());
        let result = self
            .request(
                "thread/start",
                json!({
                    "cwd": cwd,
                    "approvalPolicy": "never",
                    "sandbox": "read-only",
                    "ephemeral": false,
                    "threadSource": "nexq",
                    "model": model.map(Value::from).unwrap_or(Value::Null),
                    "baseInstructions": if base_instructions.is_empty() { Value::Null } else { json!(base_instructions) }
                }),
            )
            .await?;

        let thread_id = result
            .get("thread")
            .and_then(|thread| thread.get("id"))
            .and_then(Value::as_str)
            .ok_or_else(|| {
                LLMError::InvalidResponse("Codex thread/start response did not include thread.id".to_string())
            })?
            .to_string();

        self.thread_id = Some(thread_id.clone());
        Ok(thread_id)
    }

    async fn start_turn(
        &mut self,
        thread_id: &str,
        prompt: &str,
        model: Option<&str>,
        reasoning_effort: Option<&str>,
    ) -> Result<u64, LLMError> {
        let id = self.allocate_id();
        let mut params = json!({
            "threadId": thread_id,
            "input": [{ "type": "text", "text": prompt }]
        });
        if let Some(model) = model {
            params["model"] = json!(model);
        }
        if let Some(effort) = reasoning_effort {
            params["effort"] = json!(effort);
        }

        self.write_message(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "turn/start",
            "params": params
        }))
        .await?;
        Ok(id)
    }

    async fn stream_turn(
        &mut self,
        request_id: u64,
        thread_id: &str,
        app_handle: &tauri::AppHandle,
    ) -> Result<(u64, u64, String), LLMError> {
        let started = Instant::now();
        let mut request_seen = false;
        let mut token_count = 0u64;
        let mut response_text = String::new();

        loop {
            let message = self.read_message().await?;

            if message.get("id").and_then(Value::as_u64) == Some(request_id) {
                request_seen = true;
                if let Some(error) = message.get("error") {
                    return Err(LLMError::ProviderError(format!("Codex turn/start failed: {}", error)));
                }
                continue;
            }

            let Some(method) = message.get("method").and_then(Value::as_str) else {
                continue;
            };
            let params = message.get("params").cloned().unwrap_or(Value::Null);
            let same_thread = params
                .get("threadId")
                .and_then(Value::as_str)
                .map(|id| id == thread_id)
                .unwrap_or(false);

            if method == "item/agentMessage/delta" && same_thread {
                if let Some(delta) = params.get("delta").and_then(Value::as_str) {
                    if !delta.is_empty() {
                        token_count = token_count.saturating_add(1);
                        response_text.push_str(delta);
                        let _ = app_handle.emit(
                            "llm_stream_token",
                            StreamTokenPayload {
                                token: delta.to_string(),
                            },
                        );
                    }
                }
                continue;
            }

            if method == "turn/completed" && same_thread {
                let status = params
                    .get("turn")
                    .and_then(|turn| turn.get("status"))
                    .and_then(Value::as_str)
                    .unwrap_or("completed");

                if status == "failed" {
                    let error = params
                        .get("turn")
                        .and_then(|turn| turn.get("error"))
                        .and_then(|error| error.get("message"))
                        .and_then(Value::as_str)
                        .unwrap_or("Codex turn failed");
                    return Err(LLMError::ProviderError(error.to_string()));
                }

                if !request_seen {
                    return Err(LLMError::InvalidResponse(
                        "Codex completed a turn without acknowledging turn/start".to_string(),
                    ));
                }
                break;
            }
        }

        Ok((token_count, started.elapsed().as_millis() as u64, response_text))
    }

    async fn stop(&mut self) {
        let _ = self.child.kill().await;
        let _ = self.child.wait().await;
    }
}

#[cfg(windows)]
fn build_app_server_command() -> Command {
    let command_line = env::var(CODEX_BINARY_ENV)
        .map(|path| format!("\"{}\" app-server --stdio", path.replace('"', "\\\"")))
        .unwrap_or_else(|_| "codex.cmd app-server --stdio".to_string());

    let mut command = Command::new("cmd.exe");
    command.args(["/D", "/S", "/C", &command_line]);
    // CREATE_NO_WINDOW: keep the helper invisible while the desktop app runs.
    command.creation_flags(0x08000000);
    command
}

#[cfg(not(windows))]
fn build_app_server_command() -> Command {
    let binary = env::var(CODEX_BINARY_ENV).unwrap_or_else(|_| "codex".to_string());
    let mut command = Command::new(binary);
    command.args(["app-server", "--stdio"]);
    command
}

/// A local authenticated Codex app-server client.
pub struct CodexClient {
    session: TokioMutex<Option<CodexSession>>,
    web_cache: StdMutex<WebAnswerCache>,
}

impl CodexClient {
    pub fn new() -> Self {
        Self {
            session: TokioMutex::new(None),
            web_cache: StdMutex::new(WebAnswerCache::default()),
        }
    }

    async fn ensure_session<'a>(
        session: &'a mut Option<CodexSession>,
    ) -> Result<&'a mut CodexSession, LLMError> {
        let needs_restart = session
            .as_mut()
            .map(|current| !current.is_alive())
            .unwrap_or(true);
        if needs_restart {
            if let Some(mut old) = session.take() {
                old.stop().await;
            }
            *session = Some(CodexSession::spawn().await?);
        }
        Ok(session.as_mut().expect("Codex session created above"))
    }
}

#[async_trait::async_trait]
impl LLMProvider for CodexClient {
    fn provider_name(&self) -> &str {
        "codex"
    }

    async fn list_models(&self) -> Result<Vec<ModelInfo>, LLMError> {
        let mut session = self.session.lock().await;
        let current = Self::ensure_session(&mut session).await?;
        current.ensure_initialized().await?;

        let response = current
            .request("model/list", json!({ "includeHidden": false, "limit": 100 }))
            .await;

        let models = response.ok().and_then(|value| {
            value.get("data")?.as_array().map(|items| {
                items
                    .iter()
                    .filter_map(|item| {
                        let id = item
                            .get("id")
                            .and_then(Value::as_str)
                            .or_else(|| item.get("model").and_then(Value::as_str))?;
                        Some(ModelInfo {
                            id: id.to_string(),
                            name: item
                                .get("displayName")
                                .and_then(Value::as_str)
                                .unwrap_or(id)
                                .to_string(),
                            provider: "codex".to_string(),
                            context_window: None,
                        })
                    })
                    .collect::<Vec<_>>()
            })
        });

        if let Some(models) = models.filter(|models| !models.is_empty()) {
            return Ok(models);
        }

        // Older Codex app-server builds may not expose model/list yet. Keep a
        // usable default rather than making the settings screen unusable.
        Ok(vec![ModelInfo {
            id: DEFAULT_MODEL_ID.to_string(),
            name: "Codex local app-server default".to_string(),
            provider: "codex".to_string(),
            context_window: None,
        }])
    }

    async fn test_connection(&self) -> Result<bool, LLMError> {
        let mut session = self.session.lock().await;
        let current = Self::ensure_session(&mut session).await?;
        current.ensure_thread("", None).await.map(|_| true)
    }

    async fn reset_session(&self) -> Result<(), LLMError> {
        let mut session = self.session.lock().await;
        if let Some(mut current) = session.take() {
            current.stop().await;
        }
        Ok(())
    }

    async fn stream_completion(
        &self,
        messages: Vec<LLMMessage>,
        model: &str,
        params: GenerationParams,
        app_handle: tauri::AppHandle,
    ) -> Result<CompletionStats, LLMError> {
        let system_prompt = messages
            .iter()
            .find(|message| message.role == "system")
            .map(|message| message.content.as_str())
            .unwrap_or("");
        let user_prompt = messages
            .iter()
            .filter(|message| message.role != "system")
            .map(|message| message.content.as_str())
            .collect::<Vec<_>>()
            .join("\n\n");

        let prompt = format!(
            "For this interview-assistant turn, follow the instructions below. Never control, join, speak in, or type into a meeting application. Do not modify files.\n\n<instructions>\n{}\n</instructions>\n\n<meeting_context>\n{}\n</meeting_context>\n\nWeb search is {} for this turn. If it is enabled and the question needs current information, you may use web search and make the returned sources clear. If it is disabled, answer only from the supplied context and stable knowledge. If the supplied context does not support a personal claim, say that you do not have enough information instead of inventing it.",
            system_prompt,
            user_prompt,
            if params.enable_web_search { "enabled when needed" } else { "disabled" },
        );

        let requested_model = normalized_model(model);
        let requested_effort = params
            .reasoning_effort
            .as_deref()
            .map(str::trim)
            .filter(|effort| !effort.is_empty());
        let cache_key = params.web_cache_key.as_deref().map(|key| {
            format!(
                "model={};effort={};query={}",
                requested_model.unwrap_or("default"),
                requested_effort.unwrap_or("default"),
                key
            )
        });

        if params.enable_web_search {
            if let Some(key) = cache_key.as_deref() {
                let cached = self
                    .web_cache
                    .lock()
                    .ok()
                    .and_then(|mut cache| cache.get(key));
                if let Some((answer, sources)) = cached {
                    log::info!("Codex web cache hit for query key");
                    let chunks = answer.chars().collect::<Vec<_>>();
                    let chunk_count = chunks.chunks(64).count() as u64;
                    for chunk in chunks.chunks(64) {
                        let token = chunk.iter().collect::<String>();
                        if !token.is_empty() {
                            let _ = app_handle.emit(
                                "llm_stream_token",
                                StreamTokenPayload { token },
                            );
                        }
                    }
                    if !sources.is_empty() {
                        let _ = app_handle.emit(
                            "llm_stream_sources",
                            StreamSourcesPayload { sources },
                        );
                    }
                    let _ = app_handle.emit(
                        "llm_stream_end",
                        StreamEndPayload {
                            total_tokens: chunk_count,
                            latency_ms: 0,
                        },
                    );
                    return Ok(CompletionStats {
                        prompt_tokens: 0,
                        completion_tokens: chunk_count,
                        total_tokens: chunk_count,
                        latency_ms: 0,
                    });
                }
            }
        }

        let mut session = self.session.lock().await;
        let current = Self::ensure_session(&mut session).await?;
        let thread_id = current.ensure_thread(system_prompt, requested_model).await?;
        let request_id = current
            .start_turn(&thread_id, &prompt, requested_model, requested_effort)
            .await?;
        let (token_count, latency_ms, response_text) = current
            .stream_turn(request_id, &thread_id, &app_handle)
            .await?;

        if params.enable_web_search {
            let sources = extract_sources(&response_text);
            if let Some(key) = cache_key.as_deref() {
                if let Ok(mut cache) = self.web_cache.lock() {
                    cache.insert(key, response_text.clone(), sources.clone());
                }
            }
            if !sources.is_empty() {
                let _ = app_handle.emit("llm_stream_sources", StreamSourcesPayload { sources });
            }
        }

        let _ = app_handle.emit(
            "llm_stream_end",
            StreamEndPayload {
                total_tokens: token_count,
                latency_ms,
            },
        );

        Ok(CompletionStats {
            prompt_tokens: 0,
            completion_tokens: token_count,
            total_tokens: token_count,
            latency_ms,
        })
    }
}

fn normalized_model(model: &str) -> Option<&str> {
    let model = model.trim();
    if model.is_empty() || model == DEFAULT_MODEL_ID {
        None
    } else {
        Some(model)
    }
}

/// Extract explicit URLs from a Codex answer so the UI can show a minimal,
/// user-auditable source list. Provider-native citation metadata is not always
/// present in the app-server text stream, so this intentionally handles both
/// Markdown links and bare URLs without introducing a regex dependency.
fn extract_sources(text: &str) -> Vec<StreamSource> {
    let mut sources = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for token in text.split_whitespace() {
        let (title, candidate) = if let Some(stripped) = token.strip_prefix('[') {
            if let Some(close_bracket) = stripped.find("](") {
                let title = &stripped[..close_bracket];
                let url = stripped
                    .get(close_bracket + 2..)
                    .unwrap_or("")
                    .trim_end_matches(|character: char| ")].,;!?\"'".contains(character));
                (title, url)
            } else {
                ("Web source", "")
            }
        } else {
            let url = token
                .trim_start_matches(|character: char| "(<\"'".contains(character))
                .trim_end_matches(|character: char| ")]>.,;!?\"'".contains(character));
            ("Web source", url)
        };

        if !(candidate.starts_with("https://") || candidate.starts_with("http://")) {
            continue;
        }
        if !seen.insert(candidate.to_string()) {
            continue;
        }

        sources.push(StreamSource {
            title: title.to_string(),
            url: candidate.to_string(),
        });
        if sources.len() >= 8 {
            break;
        }
    }

    sources
}

#[cfg(test)]
mod tests {
    use super::extract_sources;

    #[test]
    fn extracts_markdown_and_bare_urls_without_duplicates() {
        let sources = extract_sources(
            "See [Rust](https://www.rust-lang.org/) and https://www.rust-lang.org/ plus https://example.com.",
        );
        assert_eq!(sources.len(), 2);
        assert_eq!(sources[0].title, "Rust");
        assert_eq!(sources[0].url, "https://www.rust-lang.org/");
        assert_eq!(sources[1].url, "https://example.com");
    }
}
