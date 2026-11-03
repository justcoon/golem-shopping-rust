use crate::ai::schema::{
    AgentConfig, ChatChunk, ChatMessage, ChatMessageType, ChatResponse, ChatRole, ClientMessage,
    ToolCallRecord,
};
use golem_rust::agentic::{AgentStream, AgentStreamWriter, Config, spawn_local};
use golem_rust::http::{self, Request};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::future::Future;
use std::rc::Rc;

pub const MAX_LOOP_TURNS: usize = 10;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct OpenAiChatRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<OpenAiTool>,
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
            cfg.llm.api_url.clone(),
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

pub fn process_chat_session<D: ToolDispatcher + 'static>(
    config: &Config<AgentConfig>,
    messages: Rc<RefCell<Vec<ChatMessage>>>,
    mut input: AgentStream<ClientMessage>,
    system_prompt: &'static str,
    tools: Vec<ToolSpec>,
    missing_key_hint: &'static str,
    dispatcher: D,
) -> AgentStream<ChatChunk> {
    let (mut writer, output) = AgentStream::new();
    let (api_key, model, api_url) = match config.get() {
        Ok(cfg) => (
            cfg.llm.api_key.get().unwrap_or_default(),
            cfg.llm.model.get().unwrap_or_default(),
            cfg.llm.api_url.clone(),
        ),
        Err(_) => (String::new(), String::new(), None),
    };

    spawn_local(async move {
        while let Ok(Some(client_msg)) = input.next().await {
            let user_text = client_msg.message.trim().to_string();
            if user_text.is_empty() {
                continue;
            }

            let mut sink = AgentStreamChunkSink {
                writer: &mut writer,
            };
            let history = messages.borrow().clone();
            let result = execute_agent_chat_turn_internal(
                system_prompt,
                &tools,
                &api_key,
                &model,
                api_url.as_deref(),
                &history,
                &user_text,
                Some(missing_key_hint),
                &dispatcher,
                &mut sink,
            )
            .await;

            let now = chrono::Utc::now().to_rfc3339();
            let user_msg = ChatMessage {
                role: ChatRole::User,
                content: user_text,
                tool_calls: None,
                timestamp: now.clone(),
                r#type: Some(ChatMessageType::Ok),
            };
            let mut updated = messages.borrow().clone();
            updated.push(user_msg);

            let assistant_text = result.reply;
            let used_tools = result.tool_calls_used;
            let is_err = result.is_error;
            updated.push(ChatMessage {
                role: ChatRole::Assistant,
                content: assistant_text,
                tool_calls: if used_tools.is_empty() {
                    None
                } else {
                    Some(used_tools)
                },
                timestamp: now,
                r#type: Some(if is_err {
                    ChatMessageType::Error
                } else {
                    ChatMessageType::Ok
                }),
            });
            *messages.borrow_mut() = updated;
        }
    });

    output
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

pub trait ChunkSink {
    fn emit(&mut self, chunk: ChatChunk) -> impl Future<Output = ()>;
}

pub struct NoopChunkSink;
impl ChunkSink for NoopChunkSink {
    async fn emit(&mut self, _chunk: ChatChunk) {}
}

pub struct AgentStreamChunkSink<'a> {
    pub writer: &'a mut AgentStreamWriter<ChatChunk>,
}

impl ChunkSink for AgentStreamChunkSink<'_> {
    async fn emit(&mut self, chunk: ChatChunk) {
        let _ = self.writer.write_one(chunk).await;
    }
}

pub trait ToolDispatcher {
    fn execute(&self, name: &str, arguments: &str) -> impl Future<Output = (String, String)>;
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
    let mut sink = NoopChunkSink;
    execute_agent_chat_turn_internal(
        system_prompt,
        tools,
        api_key,
        model,
        api_url,
        history,
        user_message,
        missing_api_key_message,
        dispatcher,
        &mut sink,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn execute_agent_chat_turn_internal<D: ToolDispatcher, S: ChunkSink>(
    system_prompt: &str,
    tools: &[ToolSpec],
    api_key: &str,
    model: &str,
    api_url: Option<&str>,
    history: &[ChatMessage],
    user_message: &str,
    missing_api_key_message: Option<&str>,
    dispatcher: &D,
    sink: &mut S,
) -> ChatTurnResult {
    let trimmed_key = api_key.trim();
    if trimmed_key.is_empty() {
        let msg = missing_api_key_message.unwrap_or(
            "AI agent is ready, but no LLM API key is configured. Please provide GEMINI_API_KEY or OPENROUTER_KEY in your environment or secrets.",
        );
        let c1 = ChatChunk::TextDelta {
            content: msg.to_string(),
        };
        let c2 = ChatChunk::Error {
            content: msg.to_string(),
        };
        let c3 = ChatChunk::Done {
            content: String::new(),
        };
        sink.emit(c1.clone()).await;
        sink.emit(c2.clone()).await;
        sink.emit(c3.clone()).await;
        return ChatTurnResult {
            reply: msg.to_string(),
            tool_calls_used: Vec::new(),
            chunks: vec![c1, c2, c3],
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

    macro_rules! emit {
        ($chunk:expr) => {{
            let c = $chunk;
            sink.emit(c.clone()).await;
            chunks.push(c);
        }};
    }

    for turn in 0..MAX_LOOP_TURNS {
        let request_payload = OpenAiChatRequest {
            model: model.to_string(),
            messages: messages.clone(),
            tools: open_ai_tools.clone(),
        };

        let request_bytes = match serde_json::to_vec(&request_payload) {
            Ok(b) => b,
            Err(e) => {
                let err_msg = format!("Failed to serialize chat request: {e}");
                emit!(ChatChunk::TextDelta {
                    content: err_msg.clone(),
                });
                emit!(ChatChunk::Error {
                    content: err_msg.clone(),
                });
                emit!(ChatChunk::Done {
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
                emit!(ChatChunk::TextDelta {
                    content: err_msg.clone(),
                });
                emit!(ChatChunk::Error {
                    content: err_msg.clone(),
                });
                emit!(ChatChunk::Done {
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
                emit!(ChatChunk::TextDelta {
                    content: err_msg.clone(),
                });
                emit!(ChatChunk::Error {
                    content: err_msg.clone(),
                });
                emit!(ChatChunk::Done {
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
                emit!(ChatChunk::TextDelta {
                    content: err_msg.clone(),
                });
                emit!(ChatChunk::Error {
                    content: err_msg.clone(),
                });
                emit!(ChatChunk::Done {
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
            emit!(ChatChunk::TextDelta {
                content: err_msg.clone(),
            });
            emit!(ChatChunk::Error {
                content: err_msg.clone(),
            });
            emit!(ChatChunk::Done {
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
                emit!(ChatChunk::TextDelta {
                    content: err_msg.clone(),
                });
                emit!(ChatChunk::Error {
                    content: err_msg.clone(),
                });
                emit!(ChatChunk::Done {
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
                emit!(ChatChunk::TextDelta {
                    content: err_msg.clone(),
                });
                emit!(ChatChunk::Error {
                    content: err_msg.clone(),
                });
                emit!(ChatChunk::Done {
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
                emit!(ChatChunk::TextDelta {
                    content: err_msg.clone(),
                });
                emit!(ChatChunk::Error {
                    content: err_msg.clone(),
                });
                emit!(ChatChunk::Done {
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

                emit!(ChatChunk::ToolStart {
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

                emit!(ChatChunk::ToolResult {
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
            emit!(ChatChunk::TextDelta {
                content: final_reply.clone(),
            });
            emit!(ChatChunk::Done {
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
    emit!(ChatChunk::TextDelta {
        content: limit_msg.clone(),
    });
    emit!(ChatChunk::Error {
        content: limit_msg.clone(),
    });
    emit!(ChatChunk::Done {
        content: String::new(),
    });

    ChatTurnResult {
        reply: limit_msg,
        tool_calls_used: accumulated_tool_calls,
        chunks,
        is_error: true,
    }
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
}
