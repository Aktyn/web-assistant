use serde::{Deserialize, Serialize};

pub trait LlmBase {
    #[allow(async_fn_in_trait)]
    async fn invoke(&self, messages: &[ChatMessage]) -> anyhow::Result<String>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".into(),
            content: content.into(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".into(),
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".into(),
            content: content.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_message_system() {
        let msg = ChatMessage::system("You are helpful");
        assert_eq!(msg.role, "system");
        assert_eq!(msg.content, "You are helpful");
    }

    #[test]
    fn test_chat_message_user() {
        let msg = ChatMessage::user("Hello world");
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "Hello world");
    }

    #[test]
    fn test_chat_message_assistant() {
        let msg = ChatMessage::assistant("I can help with that");
        assert_eq!(msg.role, "assistant");
        assert_eq!(msg.content, "I can help with that");
    }

    #[test]
    fn test_chat_message_clone() {
        let msg = ChatMessage {
            role: "user".to_string(),
            content: "test".to_string(),
        };
        let cloned = msg.clone();
        assert_eq!(cloned.role, msg.role);
        assert_eq!(cloned.content, msg.content);
    }

    #[test]
    fn test_chat_message_debug() {
        let msg = ChatMessage::user("debug test");
        let debug_str = format!("{msg:?}");
        assert!(debug_str.contains("ChatMessage"));
        assert!(debug_str.contains("user"));
    }

    #[test]
    fn test_chat_message_serde_roundtrip() {
        let msg = ChatMessage::system("system prompt");
        let json = serde_json::to_string(&msg).unwrap();
        let restored: ChatMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.role, "system");
        assert_eq!(restored.content, "system prompt");
    }

    #[test]
    fn test_llm_trait_exists() {
        // Just verify the trait is defined and has the right signature
        fn _check<T: LlmBase>(_t: &T) {}
        let msg = ChatMessage::user("hi");
        // We can't call invoke without an implementation, but we verify the type exists
        assert_eq!(msg.role, "user");
    }
}
