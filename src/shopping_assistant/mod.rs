pub mod tools;

use crate::ai::{
    AgentConfig, ChatChunk, ChatMessage, ChatResponse, ChatTurnResult, LlmConfig, NoopDispatcher,
    execute_agent_chat_stream, execute_agent_chat_turn, extract_json_str, process_chat_turn,
};
use golem_rust::agentic::{AgentStream, Config, spawn_local};
use golem_rust::{agent_definition, agent_implementation, endpoint, read_only};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tools::*;

#[derive(Serialize, Deserialize)]
struct ShoppingAssistantSnapshot {
    messages: Vec<ChatMessage>,
}

#[derive(Serialize, Deserialize)]
struct LegacyShoppingAssistantSnapshot {
    #[serde(default)]
    recommended_items: Option<serde_json::Value>,
    #[serde(default)]
    messages: Vec<ChatMessage>,
}

#[agent_definition(mount = "/v1/assistant/{id}")]
pub trait ShoppingAssistantAgent {
    fn new(id: String, #[agent_config] config: Config<AgentConfig>) -> Self;

    #[endpoint(post = "/chat")]
    async fn chat(&mut self, message: String) -> ChatResponse;

    #[endpoint(post = "/chat-stream")]
    async fn chat_stream(&mut self, message: String) -> AgentStream<ChatChunk>;

    #[read_only]
    #[endpoint(get = "/history")]
    fn get_history(&self) -> Vec<ChatMessage>;

    #[endpoint(post = "/clear-history")]
    fn clear_history(&mut self);
}

pub struct ShoppingAssistantAgentImpl {
    _id: String,
    config: Config<AgentConfig>,
    messages: Arc<Mutex<Vec<ChatMessage>>>,
}

impl ShoppingAssistantAgentImpl {
    async fn execute_turn(&mut self, message: String) -> ChatTurnResult {
        let tools = build_shopping_assistant_tools(self._id.clone());
        let mut messages = { self.messages.lock().unwrap().clone() };
        let result = process_chat_turn(
            &self.config,
            &mut messages,
            message,
            SHOPPING_ASSISTANT_SYSTEM_PROMPT,
            &tools.specs(),
            "Shopping assistant is ready, but no LLM API key is configured. Please provide GEMINI_API_KEY or OPENROUTER_KEY in your environment or secrets.",
            &tools,
        )
        .await;
        *self.messages.lock().unwrap() = messages;
        result
    }
}

#[agent_implementation]
impl ShoppingAssistantAgent for ShoppingAssistantAgentImpl {
    fn new(id: String, #[agent_config] config: Config<AgentConfig>) -> Self {
        ShoppingAssistantAgentImpl {
            _id: id,
            config,
            messages: Arc::new(Mutex::new(Vec::new())),
        }
    }

    async fn chat(&mut self, message: String) -> ChatResponse {
        self.execute_turn(message).await.to_response()
    }

    async fn chat_stream(&mut self, message: String) -> AgentStream<ChatChunk> {
        let (writer, stream) = AgentStream::new();
        let messages = self.messages.clone();
        let (api_key, model, api_url) = match self.config.get() {
            Ok(cfg) => (
                cfg.llm.api_key.get().unwrap_or_default(),
                cfg.llm.model.get().unwrap_or_default(),
                cfg.llm.api_url.get().ok(),
            ),
            Err(_) => (String::new(), String::new(), None),
        };
        let user_id = self._id.clone();
        spawn_local(async move {
            let tools = build_shopping_assistant_tools(user_id);
            execute_agent_chat_stream(
                api_key,
                model,
                api_url,
                messages,
                message,
                SHOPPING_ASSISTANT_SYSTEM_PROMPT,
                &tools.specs(),
                "Shopping assistant is ready, but no LLM API key is configured. Please provide GEMINI_API_KEY or OPENROUTER_KEY in your environment or secrets.",
                &tools,
                writer,
            )
            .await;
        });
        stream
    }

    fn get_history(&self) -> Vec<ChatMessage> {
        self.messages.lock().unwrap().clone()
    }

    fn clear_history(&mut self) {
        self.messages.lock().unwrap().clear();
    }

    async fn load_snapshot(
        bytes: Vec<u8>,
        context: golem_rust::agentic::SnapshotRestoreContext,
    ) -> Result<Self, String> {
        let messages = if let Ok(snapshot) =
            crate::common::snapshot::deserialize::<ShoppingAssistantSnapshot>(&bytes)
        {
            snapshot.messages
        } else if let Ok(legacy) =
            crate::common::snapshot::deserialize::<LegacyShoppingAssistantSnapshot>(&bytes)
        {
            legacy.messages
        } else {
            Vec::new()
        };

        let mut parameters = golem_rust::agentic::DirectAgentInput::new(context.parameters)
            .map_err(|error| error.to_string())?;
        let id: String = parameters.take().map_err(|error| error.to_string())?;
        parameters.finish().map_err(|error| error.to_string())?;

        Ok(Self {
            _id: id,
            config: Config::new(),
            messages: Arc::new(Mutex::new(messages)),
        })
    }

    async fn save_snapshot(&self) -> Result<Vec<u8>, String> {
        let snapshot = ShoppingAssistantSnapshot {
            messages: self.messages.lock().unwrap().clone(),
        };
        crate::common::snapshot::serialize(&snapshot)
    }
}
