use crate::ai::schema::{
    AgentConfig, ChatChunk, ChatMessage, ChatMessageType, ChatResponse, ChatRole, ToolCallRecord,
};
use golem_rust::agentic::{AgentStream, AgentStreamWriter, Config, spawn_local};
use golem_rust::http::{self, Request};
use http_body_util::BodyExt;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::sync::{Arc, Mutex};

pub const MAX_LOOP_TURNS: usize = 10;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

fn is_false(b: &bool) -> bool {
    !*b
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct OpenAiChatRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<OpenAiTool>,
    #[serde(skip_serializing_if = "is_false", default)]
    stream: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct OpenAiTool {
    r#type: String,
    function: OpenAiFunction,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct OpenAiFunction {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct OpenAiMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OpenAiToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    extra_content: Option<serde_json::Value>,
    #[serde(
        default,
        flatten,
        skip_serializing_if = "std::collections::HashMap::is_empty"
    )]
    extra: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct OpenAiToolCall {
    id: String,
    r#type: String,
    function: OpenAiToolCallFunction,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    extra_content: Option<serde_json::Value>,
    #[serde(
        default,
        flatten,
        skip_serializing_if = "std::collections::HashMap::is_empty"
    )]
    extra: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct OpenAiToolCallFunction {
    name: String,
    arguments: String,
    #[serde(
        default,
        flatten,
        skip_serializing_if = "std::collections::HashMap::is_empty"
    )]
    extra: std::collections::HashMap<String, serde_json::Value>,
}

fn ensure_gemini_thought_signature(tool_call: &mut OpenAiToolCall) {
    let has_signature = tool_call.extra_content.as_ref().is_some_and(|ec| {
        ec.get("google")
            .and_then(|g| g.get("thought_signature"))
            .is_some()
            || ec.get("thought_signature").is_some()
    }) || tool_call.extra.contains_key("thought_signature")
        || tool_call.function.extra.contains_key("thought_signature");

    if !has_signature {
        tool_call.extra_content = Some(serde_json::json!({
            "google": {
                "thought_signature": "skip_thought_signature_validator"
            }
        }));
    }
}

#[derive(Deserialize, Debug)]
struct OpenAiChatResponse {
    choices: Option<Vec<OpenAiChoice>>,
    error: Option<OpenAiErrorPayload>,
}

#[derive(Deserialize, Debug)]
struct OpenAiChoice {
    message: Option<OpenAiMessage>,
    #[allow(dead_code)]
    finish_reason: Option<String>,
}

#[derive(Deserialize, Debug)]
struct OpenAiErrorPayload {
    message: Option<String>,
}

#[derive(Deserialize, Debug)]
struct OpenAiStreamChunk {
    choices: Option<Vec<OpenAiStreamChoice>>,
    error: Option<OpenAiErrorPayload>,
}

#[derive(Deserialize, Debug)]
struct OpenAiStreamChoice {
    delta: Option<OpenAiStreamDelta>,
    #[allow(dead_code)]
    finish_reason: Option<String>,
}

#[derive(Deserialize, Debug, Default)]
struct OpenAiStreamDelta {
    content: Option<String>,
    tool_calls: Option<Vec<OpenAiStreamToolCall>>,
}

#[derive(Deserialize, Debug, Clone)]
struct OpenAiStreamToolCall {
    index: Option<usize>,
    id: Option<String>,
    #[allow(dead_code)]
    r#type: Option<String>,
    function: Option<OpenAiStreamFunction>,
    extra_content: Option<serde_json::Value>,
    #[serde(
        default,
        flatten,
        skip_serializing_if = "std::collections::HashMap::is_empty"
    )]
    extra: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Deserialize, Debug, Clone)]
struct OpenAiStreamFunction {
    name: Option<String>,
    arguments: Option<String>,
    #[serde(
        default,
        flatten,
        skip_serializing_if = "std::collections::HashMap::is_empty"
    )]
    extra: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Default, Clone, Debug)]
struct StreamedToolCallBuilder {
    id: String,
    name: String,
    arguments: String,
    extra_content: Option<serde_json::Value>,
    extra: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Default, Debug)]
pub struct SseDecoder {
    buffer: Vec<u8>,
}

impl SseDecoder {
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    pub fn push_bytes(&mut self, bytes: &[u8]) -> Vec<String> {
        self.buffer.extend_from_slice(bytes);
        let mut lines = Vec::new();

        while let Some(pos) = self.buffer.iter().position(|&b| b == b'\n') {
            let mut line = self.buffer.drain(..=pos).collect::<Vec<u8>>();
            if line.ends_with(b"\n") {
                line.pop();
            }
            if line.ends_with(b"\r") {
                line.pop();
            }
            if let Ok(s) = String::from_utf8(line) {
                let trimmed = s.trim();
                if let Some(data) = trimmed.strip_prefix("data:") {
                    let data = data.trim();
                    if !data.is_empty() {
                        lines.push(data.to_string());
                    }
                }
            }
        }

        lines
    }

    pub fn finish(&mut self) -> Vec<String> {
        let mut lines = Vec::new();
        if !self.buffer.is_empty() {
            let mut line = std::mem::take(&mut self.buffer);
            if line.ends_with(b"\n") {
                line.pop();
            }
            if line.ends_with(b"\r") {
                line.pop();
            }
            if let Ok(s) = String::from_utf8(line) {
                let trimmed = s.trim();
                if let Some(data) = trimmed.strip_prefix("data:") {
                    let data = data.trim();
                    if !data.is_empty() {
                        lines.push(data.to_string());
                    }
                }
            }
        }
        lines
    }
}

#[derive(Clone, Debug)]
pub struct ChatTurnResult {
    pub reply: String,
    pub tool_calls_used: Vec<ToolCallRecord>,
    pub chunks: Vec<ChatChunk>,
    pub is_error: bool,
}

impl ChatTurnResult {
    pub fn to_response(&self) -> ChatResponse {
        ChatResponse {
            reply: self.reply.clone(),
            tool_calls_used: self.tool_calls_used.clone(),
            r#type: Some(if self.is_error {
                ChatMessageType::Error
            } else {
                ChatMessageType::Ok
            }),
        }
    }

    pub fn to_stream(self) -> AgentStream<ChatChunk> {
        let (mut writer, stream) = AgentStream::new();
        spawn_local(async move {
            for chunk in self.chunks {
                if writer.write_one(chunk).await.is_err() {
                    break;
                }
            }
        });
        stream
    }
}

pub async fn process_chat_turn<D: ToolDispatcher>(
    config: &Config<AgentConfig>,
    messages: &mut Vec<ChatMessage>,
    message: String,
    system_prompt: &str,
    tools: &[ToolSpec],
    missing_key_hint: &'static str,
    dispatcher: &D,
) -> ChatTurnResult {
    let (api_key, model, api_url) = match config.get() {
        Ok(cfg) => (
            cfg.llm.api_key.get().unwrap_or_default(),
            cfg.llm.model.get().unwrap_or_default(),
            cfg.llm.api_url.get().ok(),
        ),
        Err(_) => (String::new(), String::new(), None),
    };

    let result = execute_agent_chat_turn(
        system_prompt,
        tools,
        &api_key,
        &model,
        api_url.as_deref(),
        messages,
        &message,
        Some(missing_key_hint),
        dispatcher,
    )
    .await;

    let now = chrono::Utc::now().to_rfc3339();
    messages.push(ChatMessage {
        role: ChatRole::User,
        content: message,
        tool_calls: None,
        timestamp: now.clone(),
        r#type: Some(ChatMessageType::Ok),
    });

    let msg_type = if result.is_error {
        ChatMessageType::Error
    } else {
        ChatMessageType::Ok
    };

    messages.push(ChatMessage {
        role: ChatRole::Assistant,
        content: result.reply.clone(),
        tool_calls: if result.tool_calls_used.is_empty() {
            None
        } else {
            Some(result.tool_calls_used.clone())
        },
        timestamp: now,
        r#type: Some(msg_type),
    });

    result
}

pub fn resolve_endpoint_url(model: &str, api_url: Option<&str>) -> String {
    let normalized = api_url
        .map(|u| {
            u.trim_end_matches('/')
                .trim_end_matches("/chat/completions")
        })
        .unwrap_or("");

    if !normalized.is_empty() {
        if normalized.contains("generativelanguage.googleapis.com") {
            if normalized.ends_with("/openai") {
                format!("{normalized}/chat/completions")
            } else {
                format!("{normalized}/openai/chat/completions")
            }
        } else {
            format!("{normalized}/chat/completions")
        }
    } else if model.starts_with("gemini-") {
        "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions".to_string()
    } else {
        "https://openrouter.ai/api/v1/chat/completions".to_string()
    }
}

pub trait ToolDispatcher {
    fn execute(&self, name: &str, arguments: &str) -> impl Future<Output = (String, String)>;
}

#[derive(Default, Clone, Copy, Debug)]
pub struct NoopDispatcher;

impl ToolDispatcher for NoopDispatcher {
    async fn execute(&self, _name: &str, _arguments: &str) -> (String, String) {
        (String::new(), String::new())
    }
}

pub fn extract_json_str(text: &str) -> &str {
    let trimmed = text.trim();
    if let Some(start) = trimmed.find("```json") {
        let after_fence = &trimmed[start + 7..];
        if let Some(end) = after_fence.rfind("```") {
            return after_fence[..end].trim();
        }
    } else if let Some(start) = trimmed.find("```") {
        let after_fence = &trimmed[start + 3..];
        if let Some(end) = after_fence.rfind("```") {
            return after_fence[..end].trim();
        }
    }

    if let (Some(first_brace), Some(last_brace)) = (trimmed.find('{'), trimmed.rfind('}'))
        && first_brace <= last_brace
    {
        return &trimmed[first_brace..=last_brace];
    }

    trimmed
}

#[allow(clippy::too_many_arguments)]
pub async fn execute_agent_chat_turn<D: ToolDispatcher>(
    system_prompt: &str,
    tools: &[ToolSpec],
    api_key: &str,
    model: &str,
    api_url: Option<&str>,
    history: &[ChatMessage],
    user_message: &str,
    missing_api_key_message: Option<&str>,
    dispatcher: &D,
) -> ChatTurnResult {
    let trimmed_key = api_key.trim();
    if trimmed_key.is_empty() {
        let msg = missing_api_key_message.unwrap_or(
            "AI agent is ready, but no LLM API key is configured. Please provide GEMINI_API_KEY or OPENROUTER_KEY in your environment or secrets.",
        );
        return ChatTurnResult {
            reply: msg.to_string(),
            tool_calls_used: Vec::new(),
            chunks: vec![
                ChatChunk::TextDelta {
                    content: msg.to_string(),
                },
                ChatChunk::Error {
                    content: msg.to_string(),
                },
                ChatChunk::Done {
                    content: String::new(),
                },
            ],
            is_error: true,
        };
    }

    let endpoint_url = resolve_endpoint_url(model, api_url);

    let mut messages: Vec<OpenAiMessage> = Vec::new();
    messages.push(OpenAiMessage {
        role: "system".to_string(),
        content: Some(system_prompt.to_string()),
        tool_calls: None,
        tool_call_id: None,
        extra_content: None,
        extra: std::collections::HashMap::new(),
    });

    for h in history {
        match h.role {
            ChatRole::User => messages.push(OpenAiMessage {
                role: "user".to_string(),
                content: Some(h.content.clone()),
                tool_calls: None,
                tool_call_id: None,
                extra_content: None,
                extra: std::collections::HashMap::new(),
            }),
            ChatRole::Assistant => {
                // In conversation history, the assistant's previous turns are already answered
                // and finalized in `h.content`. We do not send unfulfilled/orphan tool_calls
                // without tool responses into the LLM context.
                messages.push(OpenAiMessage {
                    role: "assistant".to_string(),
                    content: Some(h.content.clone()),
                    tool_calls: None,
                    tool_call_id: None,
                    extra_content: None,
                    extra: std::collections::HashMap::new(),
                });
            }
            ChatRole::System => messages.push(OpenAiMessage {
                role: "system".to_string(),
                content: Some(h.content.clone()),
                tool_calls: None,
                tool_call_id: None,
                extra_content: None,
                extra: std::collections::HashMap::new(),
            }),
            ChatRole::Tool => {}
        }
    }

    messages.push(OpenAiMessage {
        role: "user".to_string(),
        content: Some(user_message.to_string()),
        tool_calls: None,
        tool_call_id: None,
        extra_content: None,
        extra: std::collections::HashMap::new(),
    });

    let open_ai_tools: Vec<OpenAiTool> = tools
        .iter()
        .map(|t| OpenAiTool {
            r#type: "function".to_string(),
            function: OpenAiFunction {
                name: t.name.clone(),
                description: t.description.clone(),
                parameters: t.parameters.clone(),
            },
        })
        .collect();

    let mut accumulated_tool_calls: Vec<ToolCallRecord> = Vec::new();
    let mut chunks: Vec<ChatChunk> = Vec::new();

    for turn in 0..MAX_LOOP_TURNS {
        let request_payload = OpenAiChatRequest {
            model: model.to_string(),
            messages: messages.clone(),
            tools: open_ai_tools.clone(),
            stream: false,
        };

        let request_bytes = match serde_json::to_vec(&request_payload) {
            Ok(b) => b,
            Err(e) => {
                let err_msg = format!("Failed to serialize chat request: {e}");
                chunks.push(ChatChunk::TextDelta {
                    content: err_msg.clone(),
                });
                chunks.push(ChatChunk::Error {
                    content: err_msg.clone(),
                });
                chunks.push(ChatChunk::Done {
                    content: String::new(),
                });
                return ChatTurnResult {
                    reply: err_msg,
                    tool_calls_used: accumulated_tool_calls,
                    chunks,
                    is_error: true,
                };
            }
        };

        let request = match Request::post(&endpoint_url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("Authorization", format!("Bearer {trimmed_key}"))
            .header("HTTP-Referer", "https://golem.cloud")
            .header("X-Title", "Golem Shopping Agent")
            .body(request_bytes)
        {
            Ok(req) => req,
            Err(e) => {
                let err_msg = format!("Failed to build HTTP request: {e}");
                chunks.push(ChatChunk::TextDelta {
                    content: err_msg.clone(),
                });
                chunks.push(ChatChunk::Error {
                    content: err_msg.clone(),
                });
                chunks.push(ChatChunk::Done {
                    content: String::new(),
                });
                return ChatTurnResult {
                    reply: err_msg,
                    tool_calls_used: accumulated_tool_calls,
                    chunks,
                    is_error: true,
                };
            }
        };

        log::info!("AI loop turn {turn}: Sending request to {endpoint_url} with model {model}");

        let response = match http::send(request).await {
            Ok(resp) => resp,
            Err(e) => {
                let err_msg = format!("AI service network request failed: {e}");
                chunks.push(ChatChunk::TextDelta {
                    content: err_msg.clone(),
                });
                chunks.push(ChatChunk::Error {
                    content: err_msg.clone(),
                });
                chunks.push(ChatChunk::Done {
                    content: String::new(),
                });
                return ChatTurnResult {
                    reply: err_msg,
                    tool_calls_used: accumulated_tool_calls,
                    chunks,
                    is_error: true,
                };
            }
        };

        let status = response.status();
        let body_bytes = match http::response_bytes(response).await {
            Ok(b) => b,
            Err(e) => {
                let err_msg = format!("Failed to read AI response body: {e}");
                chunks.push(ChatChunk::TextDelta {
                    content: err_msg.clone(),
                });
                chunks.push(ChatChunk::Error {
                    content: err_msg.clone(),
                });
                chunks.push(ChatChunk::Done {
                    content: String::new(),
                });
                return ChatTurnResult {
                    reply: err_msg,
                    tool_calls_used: accumulated_tool_calls,
                    chunks,
                    is_error: true,
                };
            }
        };

        if !status.is_success() {
            let body_str = String::from_utf8_lossy(&body_bytes);
            let parsed_error: Option<OpenAiChatResponse> = serde_json::from_slice(&body_bytes).ok();
            let err_detail = parsed_error
                .and_then(|r| r.error)
                .and_then(|e| e.message)
                .unwrap_or_else(|| body_str.to_string());
            let err_msg = format!("AI service error (HTTP {status}): {err_detail}");
            chunks.push(ChatChunk::TextDelta {
                content: err_msg.clone(),
            });
            chunks.push(ChatChunk::Error {
                content: err_msg.clone(),
            });
            chunks.push(ChatChunk::Done {
                content: String::new(),
            });
            return ChatTurnResult {
                reply: err_msg,
                tool_calls_used: accumulated_tool_calls,
                chunks,
                is_error: true,
            };
        }

        let parsed: OpenAiChatResponse = match serde_json::from_slice(&body_bytes) {
            Ok(p) => p,
            Err(e) => {
                let body_str = String::from_utf8_lossy(&body_bytes);
                let err_msg = format!("Failed to parse AI response JSON: {e} (body: {body_str})");
                chunks.push(ChatChunk::TextDelta {
                    content: err_msg.clone(),
                });
                chunks.push(ChatChunk::Error {
                    content: err_msg.clone(),
                });
                chunks.push(ChatChunk::Done {
                    content: String::new(),
                });
                return ChatTurnResult {
                    reply: err_msg,
                    tool_calls_used: accumulated_tool_calls,
                    chunks,
                    is_error: true,
                };
            }
        };

        let choice = match parsed.choices.and_then(|mut cs| cs.pop()) {
            Some(c) => c,
            None => {
                let err_msg = "AI service returned empty response choices".to_string();
                chunks.push(ChatChunk::TextDelta {
                    content: err_msg.clone(),
                });
                chunks.push(ChatChunk::Error {
                    content: err_msg.clone(),
                });
                chunks.push(ChatChunk::Done {
                    content: String::new(),
                });
                return ChatTurnResult {
                    reply: err_msg,
                    tool_calls_used: accumulated_tool_calls,
                    chunks,
                    is_error: true,
                };
            }
        };

        let message = match choice.message {
            Some(m) => m,
            None => {
                let err_msg = "AI choice did not contain a message".to_string();
                chunks.push(ChatChunk::TextDelta {
                    content: err_msg.clone(),
                });
                chunks.push(ChatChunk::Error {
                    content: err_msg.clone(),
                });
                chunks.push(ChatChunk::Done {
                    content: String::new(),
                });
                return ChatTurnResult {
                    reply: err_msg,
                    tool_calls_used: accumulated_tool_calls,
                    chunks,
                    is_error: true,
                };
            }
        };

        if let Some(mut tool_calls) = message.tool_calls.filter(|calls| !calls.is_empty()) {
            let is_gemini = endpoint_url.contains("generativelanguage.googleapis.com")
                || model.starts_with("gemini-");

            if is_gemini {
                for tc in &mut tool_calls {
                    ensure_gemini_thought_signature(tc);
                }
            }

            messages.push(OpenAiMessage {
                role: "assistant".to_string(),
                content: message.content.clone(),
                tool_calls: Some(tool_calls.clone()),
                tool_call_id: None,
                extra_content: message.extra_content.clone(),
                extra: message.extra.clone(),
            });

            for tc in tool_calls {
                let tool_id = tc.id;
                let tool_name = tc.function.name;
                let tool_args = tc.function.arguments;

                let start_record = ToolCallRecord {
                    id: tool_id.clone(),
                    tool_name: tool_name.clone(),
                    arguments: tool_args.clone(),
                    result: String::new(),
                };

                chunks.push(ChatChunk::ToolStart {
                    content: format!("Executing {tool_name}..."),
                    tool_call: start_record,
                });

                log::info!("AI loop executing tool '{tool_name}' with args: {tool_args}");
                let (result_str, summary) = dispatcher.execute(&tool_name, &tool_args).await;

                let end_record = ToolCallRecord {
                    id: tool_id.clone(),
                    tool_name: tool_name.clone(),
                    arguments: tool_args.clone(),
                    result: result_str.clone(),
                };

                chunks.push(ChatChunk::ToolResult {
                    content: summary,
                    tool_call: end_record.clone(),
                });

                accumulated_tool_calls.push(end_record);

                messages.push(OpenAiMessage {
                    role: "tool".to_string(),
                    content: Some(result_str),
                    tool_calls: None,
                    tool_call_id: Some(tool_id),
                    extra_content: None,
                    extra: std::collections::HashMap::new(),
                });
            }
        } else {
            let final_reply = message.content.unwrap_or_default();
            chunks.push(ChatChunk::TextDelta {
                content: final_reply.clone(),
            });
            chunks.push(ChatChunk::Done {
                content: String::new(),
            });

            return ChatTurnResult {
                reply: final_reply,
                tool_calls_used: accumulated_tool_calls,
                chunks,
                is_error: false,
            };
        }
    }

    let limit_msg = "AI loop reached maximum tool call iterations without finishing.".to_string();
    chunks.push(ChatChunk::TextDelta {
        content: limit_msg.clone(),
    });
    chunks.push(ChatChunk::Error {
        content: limit_msg.clone(),
    });
    chunks.push(ChatChunk::Done {
        content: String::new(),
    });

    ChatTurnResult {
        reply: limit_msg,
        tool_calls_used: accumulated_tool_calls,
        chunks,
        is_error: true,
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn execute_agent_chat_stream<D: ToolDispatcher>(
    api_key: String,
    model: String,
    api_url: Option<String>,
    messages_arc: Arc<Mutex<Vec<ChatMessage>>>,
    user_message: String,
    system_prompt: &str,
    tools: &[ToolSpec],
    missing_key_hint: &'static str,
    dispatcher: &D,
    mut writer: AgentStreamWriter<ChatChunk>,
) {
    let trimmed_key = api_key.trim();
    if trimmed_key.is_empty() {
        let msg = missing_key_hint;
        let _ = writer
            .write_one(ChatChunk::TextDelta {
                content: msg.to_string(),
            })
            .await;
        let _ = writer
            .write_one(ChatChunk::Error {
                content: msg.to_string(),
            })
            .await;
        let _ = writer
            .write_one(ChatChunk::Done {
                content: String::new(),
            })
            .await;
        return;
    }

    let endpoint_url = resolve_endpoint_url(&model, api_url.as_deref());
    let is_gemini =
        endpoint_url.contains("generativelanguage.googleapis.com") || model.starts_with("gemini-");

    let history = {
        let guard = messages_arc.lock().unwrap();
        guard.clone()
    };

    let mut messages: Vec<OpenAiMessage> = Vec::new();
    messages.push(OpenAiMessage {
        role: "system".to_string(),
        content: Some(system_prompt.to_string()),
        tool_calls: None,
        tool_call_id: None,
        extra_content: None,
        extra: std::collections::HashMap::new(),
    });

    for h in &history {
        match h.role {
            ChatRole::User => messages.push(OpenAiMessage {
                role: "user".to_string(),
                content: Some(h.content.clone()),
                tool_calls: None,
                tool_call_id: None,
                extra_content: None,
                extra: std::collections::HashMap::new(),
            }),
            ChatRole::Assistant => messages.push(OpenAiMessage {
                role: "assistant".to_string(),
                content: Some(h.content.clone()),
                tool_calls: None,
                tool_call_id: None,
                extra_content: None,
                extra: std::collections::HashMap::new(),
            }),
            ChatRole::System => messages.push(OpenAiMessage {
                role: "system".to_string(),
                content: Some(h.content.clone()),
                tool_calls: None,
                tool_call_id: None,
                extra_content: None,
                extra: std::collections::HashMap::new(),
            }),
            ChatRole::Tool => {}
        }
    }

    messages.push(OpenAiMessage {
        role: "user".to_string(),
        content: Some(user_message.clone()),
        tool_calls: None,
        tool_call_id: None,
        extra_content: None,
        extra: std::collections::HashMap::new(),
    });

    let open_ai_tools: Vec<OpenAiTool> = tools
        .iter()
        .map(|t| OpenAiTool {
            r#type: "function".to_string(),
            function: OpenAiFunction {
                name: t.name.clone(),
                description: t.description.clone(),
                parameters: t.parameters.clone(),
            },
        })
        .collect();

    let mut accumulated_tool_calls: Vec<ToolCallRecord> = Vec::new();
    let mut final_assistant_reply = String::new();
    let mut is_error = false;

    for turn in 0..MAX_LOOP_TURNS {
        let request_payload = OpenAiChatRequest {
            model: model.clone(),
            messages: messages.clone(),
            tools: open_ai_tools.clone(),
            stream: true,
        };

        let request_bytes = match serde_json::to_vec(&request_payload) {
            Ok(b) => b,
            Err(e) => {
                let err_msg = format!("Failed to serialize chat request: {e}");
                let _ = writer
                    .write_one(ChatChunk::TextDelta {
                        content: err_msg.clone(),
                    })
                    .await;
                let _ = writer
                    .write_one(ChatChunk::Error {
                        content: err_msg.clone(),
                    })
                    .await;
                let _ = writer
                    .write_one(ChatChunk::Done {
                        content: String::new(),
                    })
                    .await;
                final_assistant_reply = err_msg;
                is_error = true;
                break;
            }
        };

        let request = match Request::post(&endpoint_url)
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream")
            .header("Authorization", format!("Bearer {trimmed_key}"))
            .header("HTTP-Referer", "https://golem.cloud")
            .header("X-Title", "Golem Shopping Agent")
            .body(request_bytes)
        {
            Ok(req) => req,
            Err(e) => {
                let err_msg = format!("Failed to build HTTP request: {e}");
                let _ = writer
                    .write_one(ChatChunk::TextDelta {
                        content: err_msg.clone(),
                    })
                    .await;
                let _ = writer
                    .write_one(ChatChunk::Error {
                        content: err_msg.clone(),
                    })
                    .await;
                let _ = writer
                    .write_one(ChatChunk::Done {
                        content: String::new(),
                    })
                    .await;
                final_assistant_reply = err_msg;
                is_error = true;
                break;
            }
        };

        log::info!(
            "AI stream loop turn {turn}: Sending request to {endpoint_url} with model {model}"
        );

        let response = match http::send(request).await {
            Ok(resp) => resp,
            Err(e) => {
                let err_msg = format!("AI service network request failed: {e}");
                let _ = writer
                    .write_one(ChatChunk::TextDelta {
                        content: err_msg.clone(),
                    })
                    .await;
                let _ = writer
                    .write_one(ChatChunk::Error {
                        content: err_msg.clone(),
                    })
                    .await;
                let _ = writer
                    .write_one(ChatChunk::Done {
                        content: String::new(),
                    })
                    .await;
                final_assistant_reply = err_msg;
                is_error = true;
                break;
            }
        };

        let status = response.status();
        if !status.is_success() {
            let body_bytes = http::response_bytes(response).await.unwrap_or_default();
            let body_str = String::from_utf8_lossy(&body_bytes);
            let parsed_error: Option<OpenAiChatResponse> = serde_json::from_slice(&body_bytes).ok();
            let err_detail = parsed_error
                .and_then(|r| r.error)
                .and_then(|e| e.message)
                .unwrap_or_else(|| body_str.to_string());
            let err_msg = format!("AI service error (HTTP {status}): {err_detail}");
            let _ = writer
                .write_one(ChatChunk::TextDelta {
                    content: err_msg.clone(),
                })
                .await;
            let _ = writer
                .write_one(ChatChunk::Error {
                    content: err_msg.clone(),
                })
                .await;
            let _ = writer
                .write_one(ChatChunk::Done {
                    content: String::new(),
                })
                .await;
            final_assistant_reply = err_msg;
            is_error = true;
            break;
        }

        let mut body = response.into_body();
        let mut sse_decoder = SseDecoder::new();
        let mut turn_reply = String::new();
        let mut streamed_tools: Vec<StreamedToolCallBuilder> = Vec::new();
        let mut stream_done = false;

        while let Some(frame_res) = body.frame().await {
            let frame = match frame_res {
                Ok(f) => f,
                Err(e) => {
                    let err_msg = format!("AI response stream read error: {e}");
                    if !turn_reply.is_empty() {
                        if !final_assistant_reply.is_empty() {
                            final_assistant_reply.push(' ');
                        }
                        final_assistant_reply.push_str(&turn_reply);
                    }
                    if !final_assistant_reply.is_empty() {
                        final_assistant_reply.push(' ');
                    }
                    final_assistant_reply.push_str(&err_msg);
                    is_error = true;
                    let _ = writer
                        .write_one(ChatChunk::Error { content: err_msg })
                        .await;
                    let _ = writer
                        .write_one(ChatChunk::Done {
                            content: String::new(),
                        })
                        .await;
                    break;
                }
            };

            if let Ok(data) = frame.into_data() {
                for line in sse_decoder.push_bytes(&data) {
                    if line == "[DONE]" {
                        stream_done = true;
                        break;
                    }
                    let chunk: OpenAiStreamChunk = match serde_json::from_str(&line) {
                        Ok(c) => c,
                        Err(_) => continue,
                    };
                    if let Some(err) = chunk.error {
                        let err_msg = err
                            .message
                            .unwrap_or_else(|| "Unknown AI stream error".to_string());
                        if !final_assistant_reply.is_empty() {
                            final_assistant_reply.push(' ');
                        }
                        final_assistant_reply.push_str(&err_msg);
                        is_error = true;
                        let _ = writer
                            .write_one(ChatChunk::Error { content: err_msg })
                            .await;
                        let _ = writer
                            .write_one(ChatChunk::Done {
                                content: String::new(),
                            })
                            .await;
                        stream_done = true;
                        break;
                    }
                    if let Some(choices) = chunk.choices {
                        for choice in choices {
                            if let Some(delta) = choice.delta {
                                if let Some(content) = delta.content
                                    && !content.is_empty()
                                {
                                    turn_reply.push_str(&content);
                                    if writer
                                        .write_one(ChatChunk::TextDelta { content })
                                        .await
                                        .is_err()
                                    {
                                        return;
                                    }
                                }
                                if let Some(tcs) = delta.tool_calls {
                                    for tc in tcs {
                                        let idx = tc.index.unwrap_or(streamed_tools.len());
                                        while streamed_tools.len() <= idx {
                                            streamed_tools.push(StreamedToolCallBuilder::default());
                                        }
                                        let builder = &mut streamed_tools[idx];
                                        if let Some(id) = tc.id {
                                            builder.id.push_str(&id);
                                        }
                                        if let Some(func) = tc.function {
                                            if let Some(name) = func.name {
                                                builder.name.push_str(&name);
                                            }
                                            if let Some(args) = func.arguments {
                                                builder.arguments.push_str(&args);
                                            }
                                            builder.extra.extend(func.extra);
                                        }
                                        if let Some(ec) = tc.extra_content {
                                            builder.extra_content = Some(ec);
                                        }
                                        builder.extra.extend(tc.extra);
                                    }
                                }
                            }
                        }
                    }
                }
                if stream_done {
                    break;
                }
            }
        }

        for line in sse_decoder.finish() {
            if line == "[DONE]" {
                break;
            }
            if let Ok(chunk) = serde_json::from_str::<OpenAiStreamChunk>(&line)
                && let Some(choices) = chunk.choices
            {
                for choice in choices {
                    if let Some(delta) = choice.delta
                        && let Some(content) = delta.content
                        && !content.is_empty()
                    {
                        turn_reply.push_str(&content);
                        if writer
                            .write_one(ChatChunk::TextDelta { content })
                            .await
                            .is_err()
                        {
                            return;
                        }
                    }
                }
            }
        }

        if !turn_reply.is_empty() {
            if !final_assistant_reply.is_empty() {
                final_assistant_reply.push(' ');
            }
            final_assistant_reply.push_str(&turn_reply);
        }

        let valid_tools: Vec<OpenAiToolCall> = streamed_tools
            .into_iter()
            .filter(|t| !t.name.is_empty())
            .enumerate()
            .map(|(i, t)| {
                let id = if t.id.is_empty() {
                    format!("call_{turn}_{i}")
                } else {
                    t.id
                };
                OpenAiToolCall {
                    id,
                    r#type: "function".to_string(),
                    function: OpenAiToolCallFunction {
                        name: t.name,
                        arguments: t.arguments,
                        extra: t.extra,
                    },
                    extra_content: t.extra_content,
                    extra: std::collections::HashMap::new(),
                }
            })
            .collect();

        if !valid_tools.is_empty() {
            let mut valid_tools = valid_tools;
            if is_gemini {
                for tc in &mut valid_tools {
                    ensure_gemini_thought_signature(tc);
                }
            }

            messages.push(OpenAiMessage {
                role: "assistant".to_string(),
                content: if turn_reply.is_empty() {
                    None
                } else {
                    Some(turn_reply)
                },
                tool_calls: Some(valid_tools.clone()),
                tool_call_id: None,
                extra_content: None,
                extra: std::collections::HashMap::new(),
            });

            for tc in valid_tools {
                let tool_id = tc.id;
                let tool_name = tc.function.name;
                let tool_args = tc.function.arguments;

                let start_record = ToolCallRecord {
                    id: tool_id.clone(),
                    tool_name: tool_name.clone(),
                    arguments: tool_args.clone(),
                    result: String::new(),
                };

                if writer
                    .write_one(ChatChunk::ToolStart {
                        content: format!("Executing {tool_name}..."),
                        tool_call: start_record,
                    })
                    .await
                    .is_err()
                {
                    return;
                }

                log::info!("AI stream loop executing tool '{tool_name}' with args: {tool_args}");
                let (result_str, summary) = dispatcher.execute(&tool_name, &tool_args).await;

                let end_record = ToolCallRecord {
                    id: tool_id.clone(),
                    tool_name: tool_name.clone(),
                    arguments: tool_args.clone(),
                    result: result_str.clone(),
                };

                if writer
                    .write_one(ChatChunk::ToolResult {
                        content: summary,
                        tool_call: end_record.clone(),
                    })
                    .await
                    .is_err()
                {
                    return;
                }

                accumulated_tool_calls.push(end_record);

                messages.push(OpenAiMessage {
                    role: "tool".to_string(),
                    content: Some(result_str),
                    tool_calls: None,
                    tool_call_id: Some(tool_id),
                    extra_content: None,
                    extra: std::collections::HashMap::new(),
                });
            }
        } else {
            let _ = writer
                .write_one(ChatChunk::Done {
                    content: String::new(),
                })
                .await;
            break;
        }
    }

    let now = chrono::Utc::now().to_rfc3339();
    let mut guard = messages_arc.lock().unwrap();
    guard.push(ChatMessage {
        role: ChatRole::User,
        content: user_message,
        tool_calls: None,
        timestamp: now.clone(),
        r#type: Some(ChatMessageType::Ok),
    });

    let msg_type = if is_error {
        ChatMessageType::Error
    } else {
        ChatMessageType::Ok
    };

    guard.push(ChatMessage {
        role: ChatRole::Assistant,
        content: final_assistant_reply,
        tool_calls: if accumulated_tool_calls.is_empty() {
            None
        } else {
            Some(accumulated_tool_calls)
        },
        timestamp: now,
        r#type: Some(msg_type),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gemini_thought_signature_preservation() {
        let json_input = serde_json::json!({
            "id": "call_123",
            "type": "function",
            "function": {
                "name": "default_api:get_cart",
                "arguments": "{}"
            },
            "extra_content": {
                "google": {
                    "thought_signature": "authentic_sig_abc"
                }
            }
        });

        let mut tool_call: OpenAiToolCall =
            serde_json::from_value(json_input).expect("deserialization should succeed");
        ensure_gemini_thought_signature(&mut tool_call);

        let serialized = serde_json::to_value(&tool_call).expect("serialization should succeed");
        assert_eq!(
            serialized["extra_content"]["google"]["thought_signature"],
            "authentic_sig_abc"
        );
    }

    #[test]
    fn test_gemini_thought_signature_fallback_when_missing() {
        let json_input = serde_json::json!({
            "id": "call_456",
            "type": "function",
            "function": {
                "name": "default_api:get_cart",
                "arguments": "{}"
            }
        });

        let mut tool_call: OpenAiToolCall =
            serde_json::from_value(json_input).expect("deserialization should succeed");
        ensure_gemini_thought_signature(&mut tool_call);

        let serialized = serde_json::to_value(&tool_call).expect("serialization should succeed");
        assert_eq!(
            serialized["extra_content"]["google"]["thought_signature"],
            "skip_thought_signature_validator"
        );
    }

    #[test]
    fn test_extract_json_str() {
        let raw_json = r#"{"product_ids":["p1"],"product_brands":["b1"]}"#;
        assert_eq!(extract_json_str(raw_json), raw_json);

        let fenced_json = format!("```json\n{raw_json}\n```");
        assert_eq!(extract_json_str(&fenced_json), raw_json);

        let fenced_no_lang = format!("```\n{raw_json}\n```");
        assert_eq!(extract_json_str(&fenced_no_lang), raw_json);

        let text_around = format!("Here are recommendations:\n{raw_json}\nHope this helps!");
        assert_eq!(extract_json_str(&text_around), raw_json);
    }

    #[wstd::test]
    async fn test_noop_dispatcher() {
        let dispatcher = NoopDispatcher;
        let (res, summary) = dispatcher.execute("any_tool", "{}").await;
        assert_eq!(res, "");
        assert_eq!(summary, "");
    }

    #[test]
    fn test_sse_decoder() {
        let mut decoder = SseDecoder::new();
        let chunk1 = b"data: {\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}\n\ndata: {\"choices\":[{\"delta\":";
        let lines1 = decoder.push_bytes(chunk1);
        assert_eq!(lines1.len(), 1);
        assert_eq!(
            lines1[0],
            "{\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}"
        );

        let chunk2 = b"{\"content\":\" world\"}}]}\n\ndata: [DONE]\n";
        let lines2 = decoder.push_bytes(chunk2);
        assert_eq!(lines2.len(), 2);
        assert_eq!(
            lines2[0],
            "{\"choices\":[{\"delta\":{\"content\":\" world\"}}]}"
        );
        assert_eq!(lines2[1], "[DONE]");
    }

    #[test]
    fn test_streamed_tool_call_builder() {
        let mut streamed_tools: Vec<StreamedToolCallBuilder> = Vec::new();
        let part1 = OpenAiStreamToolCall {
            index: Some(0),
            id: Some("call_abc".to_string()),
            r#type: Some("function".to_string()),
            function: Some(OpenAiStreamFunction {
                name: Some("search_products".to_string()),
                arguments: Some("{\"query\":".to_string()),
                extra: std::collections::HashMap::new(),
            }),
            extra_content: None,
            extra: std::collections::HashMap::new(),
        };
        let part2 = OpenAiStreamToolCall {
            index: Some(0),
            id: None,
            r#type: None,
            function: Some(OpenAiStreamFunction {
                name: None,
                arguments: Some("\"shoes\"}".to_string()),
                extra: std::collections::HashMap::new(),
            }),
            extra_content: None,
            extra: std::collections::HashMap::new(),
        };

        for tc in [part1, part2] {
            let idx = tc.index.unwrap_or(0);
            while streamed_tools.len() <= idx {
                streamed_tools.push(StreamedToolCallBuilder::default());
            }
            let builder = &mut streamed_tools[idx];
            if let Some(id) = tc.id {
                builder.id.push_str(&id);
            }
            if let Some(func) = tc.function {
                if let Some(name) = func.name {
                    builder.name.push_str(&name);
                }
                if let Some(args) = func.arguments {
                    builder.arguments.push_str(&args);
                }
            }
        }

        assert_eq!(streamed_tools.len(), 1);
        assert_eq!(streamed_tools[0].id, "call_abc");
        assert_eq!(streamed_tools[0].name, "search_products");
        assert_eq!(streamed_tools[0].arguments, "{\"query\":\"shoes\"}");
    }
}
