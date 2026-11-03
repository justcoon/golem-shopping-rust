pub mod agent_loop;
pub mod schema;

pub use agent_loop::*;
pub use schema::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_endpoint_url() {
        assert_eq!(
            resolve_endpoint_url("openrouter/owl-alpha", None),
            "https://openrouter.ai/api/v1/chat/completions"
        );
        assert_eq!(
            resolve_endpoint_url("gemini-3.1-flash-lite", None),
            "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions"
        );
        assert_eq!(
            resolve_endpoint_url(
                "gemini-3.1-flash-lite",
                Some("https://generativelanguage.googleapis.com/v1beta")
            ),
            "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions"
        );
        assert_eq!(
            resolve_endpoint_url("custom-model", Some("https://my-proxy.example.com/v1")),
            "https://my-proxy.example.com/v1/chat/completions"
        );
    }

    #[test]
    fn test_chat_schema_serialization() {
        let msg = ChatMessage {
            role: ChatRole::User,
            content: "Hello assistant".to_string(),
            tool_calls: None,
            timestamp: "2026-11-03T09:00:00Z".to_string(),
            r#type: Some(ChatMessageType::Ok),
        };
        let serialized = serde_json::to_string(&msg).expect("serialize message");
        assert!(serialized.contains(r#""role":"user""#));
        assert!(serialized.contains(r#""content":"Hello assistant""#));

        let deserialized: ChatMessage =
            serde_json::from_str(&serialized).expect("deserialize message");
        assert_eq!(deserialized.role, ChatRole::User);
        assert_eq!(deserialized.content, "Hello assistant");
    }

    #[test]
    fn test_tool_call_record_and_chunk_serialization() {
        let record = ToolCallRecord {
            id: "call_1".to_string(),
            tool_name: "search_products".to_string(),
            arguments: r#"{"query":"shoes"}"#.to_string(),
            result: r#"{"count":3}"#.to_string(),
        };

        let chunk = ChatChunk::ToolStart {
            content: "Executing search_products...".to_string(),
            tool_call: record.clone(),
        };

        let json_val = serde_json::to_value(&chunk).expect("serialize chunk");
        assert!(json_val.get("ToolStart").is_some());
    }

    struct DummyDispatcher;
    impl ToolDispatcher for DummyDispatcher {
        async fn execute(&self, _name: &str, _arguments: &str) -> (String, String) {
            (String::new(), String::new())
        }
    }

    #[wstd::test]
    async fn test_missing_api_key_returns_graceful_error() {
        let result = execute_agent_chat_turn(
            "system prompt",
            &[],
            "",
            "gemini-flash",
            None,
            &[],
            "Hello",
            Some("No API key available"),
            &DummyDispatcher,
        )
        .await;

        assert!(result.is_error);
        assert_eq!(result.reply, "No API key available");
        assert_eq!(result.chunks.len(), 3);
    }
}
