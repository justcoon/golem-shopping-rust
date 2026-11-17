use crate::ai::agent_loop::{ChatTurnResult, execute_agent_chat_stream, process_chat_turn};
use crate::ai::schema::{AgentConfig, ChatChunk, ChatMessage};
use crate::ai::tools::ToolRegistry;
use golem_rust::agentic::{AgentStream, Config, spawn_local};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex, PoisonError};

#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq)]
pub struct ChatSnapshot {
    #[serde(default)]
    pub messages: Vec<ChatMessage>,
}

#[derive(Clone, Default)]
pub struct ChatAgentState {
    messages: Arc<Mutex<Vec<ChatMessage>>>,
}

impl ChatAgentState {
    pub fn new() -> Self {
        Self {
            messages: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn with_messages(messages: Vec<ChatMessage>) -> Self {
        Self {
            messages: Arc::new(Mutex::new(messages)),
        }
    }

    pub fn get_history(&self) -> Vec<ChatMessage> {
        self.messages
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    pub fn clear_history(&self) {
        self.messages
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
    }

    pub fn messages_arc(&self) -> Arc<Mutex<Vec<ChatMessage>>> {
        self.messages.clone()
    }

    pub async fn execute_turn(
        &self,
        config: &Config<AgentConfig>,
        user_message: String,
        system_prompt: &str,
        no_key_message: &'static str,
        tools: &ToolRegistry,
    ) -> ChatTurnResult {
        let mut messages = self.get_history();
        let result = process_chat_turn(
            config,
            &mut messages,
            user_message,
            system_prompt,
            &tools.specs(),
            no_key_message,
            tools,
        )
        .await;
        *self.messages.lock().unwrap_or_else(PoisonError::into_inner) = messages;
        result
    }

    pub fn execute_stream(
        &self,
        config: &Config<AgentConfig>,
        user_message: String,
        system_prompt: &'static str,
        no_key_message: &'static str,
        tools_factory: impl FnOnce() -> ToolRegistry + Send + 'static,
    ) -> AgentStream<ChatChunk> {
        let (writer, stream) = AgentStream::new();
        let messages = self.messages.clone();
        let (api_key, model, api_url) = match config.get() {
            Ok(cfg) => (
                cfg.llm.api_key.get().unwrap_or_default(),
                cfg.llm.model.get().unwrap_or_default(),
                cfg.llm.api_url.get().ok(),
            ),
            Err(_) => (String::new(), String::new(), None),
        };

        spawn_local(async move {
            let tools = tools_factory();
            execute_agent_chat_stream(
                api_key,
                model,
                api_url,
                messages,
                user_message,
                system_prompt,
                &tools.specs(),
                no_key_message,
                &tools,
                writer,
            )
            .await;
        });

        stream
    }

    pub fn save_snapshot(&self) -> Result<Vec<u8>, String> {
        let snapshot = ChatSnapshot {
            messages: self.get_history(),
        };
        crate::common::snapshot::serialize(&snapshot)
    }

    pub fn restore_from_snapshot(bytes: &[u8]) -> Self {
        if let Ok(snapshot) = crate::common::snapshot::deserialize::<ChatSnapshot>(bytes) {
            Self::with_messages(snapshot.messages)
        } else {
            Self::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::schema::{ChatMessageType, ChatRole};

    #[test]
    fn test_chat_state_lifecycle() {
        let state = ChatAgentState::new();
        assert!(state.get_history().is_empty());

        let msg = ChatMessage {
            role: ChatRole::User,
            content: "Hello".to_string(),
            tool_calls: None,
            timestamp: "2026-11-16T00:00:00Z".to_string(),
            r#type: Some(ChatMessageType::Ok),
        };

        let state_with_msgs = ChatAgentState::with_messages(vec![msg.clone()]);
        assert_eq!(state_with_msgs.get_history().len(), 1);
        assert_eq!(state_with_msgs.get_history()[0].content, "Hello");

        state_with_msgs.clear_history();
        assert!(state_with_msgs.get_history().is_empty());
    }

    #[test]
    fn test_chat_snapshot_serialization_and_legacy_compatibility() {
        let msg = ChatMessage {
            role: ChatRole::User,
            content: "Test".to_string(),
            tool_calls: None,
            timestamp: "2026-11-16T00:00:00Z".to_string(),
            r#type: Some(ChatMessageType::Ok),
        };
        let state = ChatAgentState::with_messages(vec![msg]);

        let bytes = state.save_snapshot().expect("save snapshot");
        let restored = ChatAgentState::restore_from_snapshot(&bytes);
        assert_eq!(restored.get_history().len(), 1);
        assert_eq!(restored.get_history()[0].content, "Test");

        // Verify that legacy snapshots with an extra "recommended_items" field deserialize seamlessly
        let legacy_json = r#"{"recommended_items":{"product_ids":["p1"]},"messages":[{"role":"assistant","content":"Hi","timestamp":"2026-11-16T00:00:00Z"}]}"#;
        let mut legacy_bytes = vec![crate::common::snapshot::SERIALIZATION_VERSION_V1];
        legacy_bytes.extend(legacy_json.as_bytes());

        let restored_legacy = ChatAgentState::restore_from_snapshot(&legacy_bytes);
        assert_eq!(restored_legacy.get_history().len(), 1);
        assert_eq!(restored_legacy.get_history()[0].content, "Hi");
    }
}
