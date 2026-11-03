use golem_rust::agentic::Secret;
use golem_rust::{ConfigSchema, Schema};
use serde::{Deserialize, Serialize};

#[derive(ConfigSchema)]
pub struct LlmConfig {
    #[config_schema(secret)]
    pub api_key: Secret<String>,
    #[config_schema(secret)]
    pub model: Secret<String>,
    #[config_schema(secret)]
    pub api_url: Secret<String>,
}

#[derive(ConfigSchema)]
pub struct AgentConfig {
    #[config_schema(nested)]
    pub llm: LlmConfig,
}

#[derive(Schema, Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum ChatRole {
    #[serde(rename = "system")]
    System,
    #[serde(rename = "user")]
    User,
    #[serde(rename = "assistant")]
    Assistant,
    #[serde(rename = "tool")]
    Tool,
}

#[derive(Schema, Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct ToolCallRecord {
    pub id: String,
    pub tool_name: String,
    pub arguments: String,
    pub result: String,
}

#[derive(Schema, Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum ChatMessageType {
    #[serde(rename = "ok")]
    Ok,
    #[serde(rename = "error")]
    Error,
}

#[derive(Schema, Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
    pub tool_calls: Option<Vec<ToolCallRecord>>,
    pub timestamp: String,
    pub r#type: Option<ChatMessageType>,
}

#[derive(Schema, Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct ChatResponse {
    pub reply: String,
    pub tool_calls_used: Vec<ToolCallRecord>,
    pub r#type: Option<ChatMessageType>,
}

#[derive(Schema, Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum ChatChunk {
    TextDelta {
        content: String,
    },
    ToolStart {
        content: String,
        tool_call: ToolCallRecord,
    },
    ToolResult {
        content: String,
        tool_call: ToolCallRecord,
    },
    Done {
        content: String,
    },
    Error {
        content: String,
    },
}
