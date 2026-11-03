pub mod tools;

use crate::ai::schema::{ChatThreadSummary, CreateChatRequest};
use crate::ai::{
    AgentConfig, ChatChunk, ChatMessage, ChatResponse, ChatTurnResult, ClientMessage,
    ToolDispatcher, process_chat_session, process_chat_turn,
};
use golem_rust::agentic::{AgentStream, Config};
use golem_rust::{agent_definition, agent_implementation, endpoint, read_only};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;
use tools::*;

#[derive(Serialize, Deserialize)]
struct ManagerSnapshot {
    #[serde(default)]
    threads: Vec<ChatThreadSummary>,
}

struct ManagerDispatcher {
    manager_id: String,
}

impl ToolDispatcher for ManagerDispatcher {
    async fn execute(&self, name: &str, arguments: &str) -> (String, String) {
        execute_manager_tool(&self.manager_id, name, arguments).await
    }
}

// -----------------------------------------------------------------------------
// ManagerChatAgent (Individual conversation thread)
// -----------------------------------------------------------------------------

#[agent_definition(mount = "/v1/manager/{manager_id}/chats/{chat_id}")]
pub trait ManagerChatAgent {
    fn new(
        manager_id: String,
        chat_id: String,
        #[agent_config] config: Config<AgentConfig>,
    ) -> Self;

    #[endpoint(post = "/chat")]
    async fn chat(&mut self, message: String) -> ChatResponse;

    #[endpoint(
        post = "/chat-session",
        durable_streams(
            input("input", name = "inbox"),
            output("$result", name = "outbox"),
            allow_external_writes = true,
            allow_stream_delete = true,
            allow_invocation_delete = true,
            max_concurrent_readers_per_stream = 8,
            max_append_requests_per_second_per_stream = 25,
        )
    )]
    fn chat_session(&mut self, input: AgentStream<ClientMessage>) -> AgentStream<ChatChunk>;

    #[read_only]
    #[endpoint(get = "/history")]
    fn get_history(&self) -> Vec<ChatMessage>;

    #[endpoint(post = "/clear-history")]
    fn clear_history(&mut self);
}

pub struct ManagerChatAgentImpl {
    manager_id: String,
    _chat_id: String,
    config: Config<AgentConfig>,
    messages: Rc<RefCell<Vec<ChatMessage>>>,
}

impl ManagerChatAgentImpl {
    async fn execute_turn(&mut self, message: String) -> ChatTurnResult {
        let dispatcher = ManagerDispatcher {
            manager_id: self.manager_id.clone(),
        };
        let tools = get_manager_tools();
        let mut msgs = self.messages.borrow().clone();
        let result = process_chat_turn(
            &self.config,
            &mut msgs,
            message,
            MANAGER_SYSTEM_PROMPT,
            &tools,
            "Manager assistant is ready, but no LLM API key is configured. Please provide GEMINI_API_KEY or OPENROUTER_KEY in your environment or secrets.",
            &dispatcher,
        )
        .await;
        *self.messages.borrow_mut() = msgs;
        result
    }
}

#[agent_implementation]
impl ManagerChatAgent for ManagerChatAgentImpl {
    fn new(
        manager_id: String,
        chat_id: String,
        #[agent_config] config: Config<AgentConfig>,
    ) -> Self {
        ManagerChatAgentImpl {
            manager_id,
            _chat_id: chat_id,
            config,
            messages: Rc::new(RefCell::new(Vec::new())),
        }
    }

    async fn chat(&mut self, message: String) -> ChatResponse {
        self.execute_turn(message).await.to_response()
    }

    fn chat_session(&mut self, input: AgentStream<ClientMessage>) -> AgentStream<ChatChunk> {
        let dispatcher = ManagerDispatcher {
            manager_id: self.manager_id.clone(),
        };
        process_chat_session(
            &self.config,
            self.messages.clone(),
            input,
            MANAGER_SYSTEM_PROMPT,
            get_manager_tools(),
            "Manager assistant is ready, but no LLM API key is configured. Please provide GEMINI_API_KEY or OPENROUTER_KEY in your environment or secrets.",
            dispatcher,
        )
    }

    fn get_history(&self) -> Vec<ChatMessage> {
        self.messages.borrow().clone()
    }

    fn clear_history(&mut self) {
        self.messages.borrow_mut().clear();
    }
}

// -----------------------------------------------------------------------------
// ManagerAgent (Workspace & chat thread index)
// -----------------------------------------------------------------------------

#[agent_definition(mount = "/v1/manager/{id}")]
pub trait ManagerAgent {
    fn new(id: String, #[agent_config] config: Config<AgentConfig>) -> Self;

    #[endpoint(get = "/chats")]
    fn list_chats(&self) -> Vec<ChatThreadSummary>;

    #[endpoint(post = "/chats")]
    fn create_chat(&mut self, req: CreateChatRequest) -> ChatThreadSummary;

    #[endpoint(delete = "/chats/{chat_id}")]
    fn delete_chat(&mut self, chat_id: String) -> bool;

    // Backwards-compatibility proxies to "default" chat
    #[endpoint(get = "/history")]
    async fn get_history(&self) -> Vec<ChatMessage>;

    #[endpoint(post = "/chat")]
    async fn chat(&mut self, message: String) -> ChatResponse;

    #[endpoint(post = "/clear-history")]
    async fn clear_history(&mut self);
}

pub struct ManagerAgentImpl {
    _id: String,
    _config: Config<AgentConfig>,
    threads: Vec<ChatThreadSummary>,
}

#[agent_implementation]
impl ManagerAgent for ManagerAgentImpl {
    fn new(id: String, #[agent_config] config: Config<AgentConfig>) -> Self {
        let default_thread = ChatThreadSummary {
            chat_id: "default".to_string(),
            title: "Store Management Chat".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        ManagerAgentImpl {
            _id: id,
            _config: config,
            threads: vec![default_thread],
        }
    }

    fn list_chats(&self) -> Vec<ChatThreadSummary> {
        self.threads.clone()
    }

    fn create_chat(&mut self, req: CreateChatRequest) -> ChatThreadSummary {
        let chat_id = format!("chat-{}", uuid::Uuid::new_v4().simple());
        let title = req
            .title
            .filter(|t| !t.trim().is_empty())
            .unwrap_or_else(|| "New Management Chat".to_string());
        let summary = ChatThreadSummary {
            chat_id,
            title,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        self.threads.push(summary.clone());
        summary
    }

    fn delete_chat(&mut self, chat_id: String) -> bool {
        let initial_len = self.threads.len();
        self.threads.retain(|t| t.chat_id != chat_id);
        self.threads.len() < initial_len
    }

    async fn get_history(&self) -> Vec<ChatMessage> {
        ManagerChatAgentClient::get(self._id.clone(), "default".to_string())
            .get_history()
            .await
    }

    async fn chat(&mut self, message: String) -> ChatResponse {
        let mut client = ManagerChatAgentClient::get(self._id.clone(), "default".to_string());
        client.chat(message).await
    }

    async fn clear_history(&mut self) {
        let mut client = ManagerChatAgentClient::get(self._id.clone(), "default".to_string());
        client.clear_history().await;
    }

    async fn load_snapshot(
        bytes: Vec<u8>,
        context: golem_rust::agentic::SnapshotRestoreContext,
    ) -> Result<Self, String> {
        let snapshot: ManagerSnapshot = crate::common::snapshot::deserialize(&bytes)?;
        let mut parameters = golem_rust::agentic::DirectAgentInput::new(context.parameters)
            .map_err(|error| error.to_string())?;
        let id: String = parameters.take().map_err(|error| error.to_string())?;
        parameters.finish().map_err(|error| error.to_string())?;

        Ok(Self {
            _id: id,
            _config: Config::new(),
            threads: snapshot.threads,
        })
    }

    async fn save_snapshot(&self) -> Result<Vec<u8>, String> {
        let snapshot = ManagerSnapshot {
            threads: self.threads.clone(),
        };
        crate::common::snapshot::serialize(&snapshot)
    }
}
