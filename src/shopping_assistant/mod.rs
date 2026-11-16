pub mod tools;

use crate::ai::{AgentConfig, ChatAgentState, ChatChunk, ChatMessage, ChatResponse};
use golem_rust::agentic::{AgentStream, Config};
use golem_rust::{agent_definition, agent_implementation, endpoint, read_only};
use tools::*;

pub const SHOPPING_ASSISTANT_NO_KEY_MSG: &str = "Shopping assistant is ready, but no LLM API key is configured. Please provide GEMINI_API_KEY or OPENROUTER_KEY in your environment or secrets.";

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
    state: ChatAgentState,
}

#[agent_implementation]
impl ShoppingAssistantAgent for ShoppingAssistantAgentImpl {
    fn new(id: String, #[agent_config] config: Config<AgentConfig>) -> Self {
        ShoppingAssistantAgentImpl {
            _id: id,
            config,
            state: ChatAgentState::new(),
        }
    }

    async fn chat(&mut self, message: String) -> ChatResponse {
        let tools = build_shopping_assistant_tools(self._id.clone());
        self.state
            .execute_turn(
                &self.config,
                message,
                SHOPPING_ASSISTANT_SYSTEM_PROMPT,
                SHOPPING_ASSISTANT_NO_KEY_MSG,
                &tools,
            )
            .await
            .to_response()
    }

    async fn chat_stream(&mut self, message: String) -> AgentStream<ChatChunk> {
        let user_id = self._id.clone();
        self.state.execute_stream(
            &self.config,
            message,
            SHOPPING_ASSISTANT_SYSTEM_PROMPT,
            SHOPPING_ASSISTANT_NO_KEY_MSG,
            move || build_shopping_assistant_tools(user_id),
        )
    }

    fn get_history(&self) -> Vec<ChatMessage> {
        self.state.get_history()
    }

    fn clear_history(&mut self) {
        self.state.clear_history();
    }

    async fn load_snapshot(
        bytes: Vec<u8>,
        context: golem_rust::agentic::SnapshotRestoreContext,
    ) -> Result<Self, String> {
        let state = ChatAgentState::restore_from_snapshot(&bytes);
        let id: String = crate::common::snapshot::extract_agent_id(context)?;

        Ok(Self {
            _id: id,
            config: Config::new(),
            state,
        })
    }

    async fn save_snapshot(&self) -> Result<Vec<u8>, String> {
        self.state.save_snapshot()
    }
}
