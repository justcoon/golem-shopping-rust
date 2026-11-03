pub mod tools;

use crate::ai::{
    AgentConfig, ChatChunk, ChatMessage, ChatResponse, ChatTurnResult, ToolDispatcher,
    process_chat_turn,
};
use golem_rust::agentic::{AgentStream, Config};
use golem_rust::{agent_definition, agent_implementation, endpoint, read_only};
use serde::{Deserialize, Serialize};
use tools::*;

#[derive(Serialize, Deserialize)]
struct ManagerSnapshot {
    #[serde(default)]
    messages: Vec<ChatMessage>,
}

struct ManagerDispatcher<'a> {
    manager_id: &'a str,
}

impl ToolDispatcher for ManagerDispatcher<'_> {
    async fn execute(&self, name: &str, arguments: &str) -> (String, String) {
        execute_manager_tool(self.manager_id, name, arguments).await
    }
}

#[agent_definition(mount = "/v1/manager/{id}")]
pub trait ManagerAgent {
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

pub struct ManagerAgentImpl {
    _id: String,
    config: Config<AgentConfig>,
    messages: Vec<ChatMessage>,
}

impl ManagerAgentImpl {
    async fn execute_turn(&mut self, message: String) -> ChatTurnResult {
        let dispatcher = ManagerDispatcher {
            manager_id: &self._id,
        };
        let tools = get_manager_tools();
        process_chat_turn(
            &self.config,
            &mut self.messages,
            message,
            MANAGER_SYSTEM_PROMPT,
            &tools,
            "Manager assistant is ready, but no LLM API key is configured. Please provide GEMINI_API_KEY or OPENROUTER_KEY in your environment or secrets.",
            &dispatcher,
        )
        .await
    }
}

#[agent_implementation]
impl ManagerAgent for ManagerAgentImpl {
    fn new(id: String, #[agent_config] config: Config<AgentConfig>) -> Self {
        ManagerAgentImpl {
            _id: id,
            config,
            messages: Vec::new(),
        }
    }

    async fn chat(&mut self, message: String) -> ChatResponse {
        self.execute_turn(message).await.to_response()
    }

    async fn chat_stream(&mut self, message: String) -> AgentStream<ChatChunk> {
        self.execute_turn(message).await.to_stream()
    }

    fn get_history(&self) -> Vec<ChatMessage> {
        self.messages.clone()
    }

    fn clear_history(&mut self) {
        self.messages.clear();
    }

    async fn load_snapshot(
        bytes: Vec<u8>,
        context: golem_rust::agentic::SnapshotRestoreContext,
    ) -> Result<Self, String> {
        let messages =
            if let Ok(snapshot) = crate::common::snapshot::deserialize::<ManagerSnapshot>(&bytes) {
                snapshot.messages
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
            messages,
        })
    }

    async fn save_snapshot(&self) -> Result<Vec<u8>, String> {
        let snapshot = ManagerSnapshot {
            messages: self.messages.clone(),
        };
        crate::common::snapshot::serialize(&snapshot)
    }
}
