use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub role: MessageRole,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transcript {
    pub messages: Vec<Message>,
    pub session_id: Uuid,
}

impl Transcript {
    pub fn new(session_id: Uuid) -> Self {
        Self {
            messages: Vec::new(),
            session_id,
        }
    }

    pub fn add_message(&mut self, role: MessageRole, content: String) {
        let message = Message {
            id: Uuid::new_v4(),
            role,
            content,
            timestamp: chrono::Utc::now(),
        };
        self.messages.push(message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transcript_new() {
        let session_id = Uuid::new_v4();
        let transcript = Transcript::new(session_id);
        assert_eq!(transcript.session_id, session_id);
        assert!(transcript.messages.is_empty());
    }

    #[test]
    fn test_add_message() {
        let session_id = Uuid::new_v4();
        let mut transcript = Transcript::new(session_id);

        transcript.add_message(MessageRole::User, "Hello".to_string());
        assert_eq!(transcript.messages.len(), 1);
        assert_eq!(transcript.messages[0].content, "Hello");
        assert!(matches!(transcript.messages[0].role, MessageRole::User));
    }

    #[test]
    fn test_add_multiple_messages() {
        let session_id = Uuid::new_v4();
        let mut transcript = Transcript::new(session_id);

        transcript.add_message(MessageRole::User, "Hello".to_string());
        transcript.add_message(MessageRole::Assistant, "Hi there".to_string());
        transcript.add_message(MessageRole::System, "Welcome".to_string());

        assert_eq!(transcript.messages.len(), 3);
        assert!(matches!(transcript.messages[0].role, MessageRole::User));
        assert!(matches!(transcript.messages[1].role, MessageRole::Assistant));
        assert!(matches!(transcript.messages[2].role, MessageRole::System));
    }

    #[test]
    fn test_message_unique_ids() {
        let session_id = Uuid::new_v4();
        let mut transcript = Transcript::new(session_id);

        transcript.add_message(MessageRole::User, "Message 1".to_string());
        transcript.add_message(MessageRole::User, "Message 2".to_string());

        assert_ne!(transcript.messages[0].id, transcript.messages[1].id);
    }

    #[test]
    fn test_message_role_user() {
        let session_id = Uuid::new_v4();
        let mut transcript = Transcript::new(session_id);
        transcript.add_message(MessageRole::User, "test".to_string());
        assert!(matches!(transcript.messages[0].role, MessageRole::User));
    }

    #[test]
    fn test_message_role_assistant() {
        let session_id = Uuid::new_v4();
        let mut transcript = Transcript::new(session_id);
        transcript.add_message(MessageRole::Assistant, "test".to_string());
        assert!(matches!(transcript.messages[0].role, MessageRole::Assistant));
    }

    #[test]
    fn test_message_role_system() {
        let session_id = Uuid::new_v4();
        let mut transcript = Transcript::new(session_id);
        transcript.add_message(MessageRole::System, "test".to_string());
        assert!(matches!(transcript.messages[0].role, MessageRole::System));
    }

    #[test]
    fn test_transcript_clone() {
        let session_id = Uuid::new_v4();
        let mut transcript = Transcript::new(session_id);
        transcript.add_message(MessageRole::User, "test".to_string());

        let cloned = transcript.clone();
        assert_eq!(cloned.session_id, transcript.session_id);
        assert_eq!(cloned.messages.len(), transcript.messages.len());
        assert_eq!(cloned.messages[0].content, transcript.messages[0].content);
    }

    #[test]
    fn test_message_clone() {
        let message = Message {
            id: Uuid::new_v4(),
            role: MessageRole::User,
            content: "test".to_string(),
            timestamp: chrono::Utc::now(),
        };

        let cloned = message.clone();
        assert_eq!(cloned.id, message.id);
        assert_eq!(cloned.content, message.content);
        assert_eq!(cloned.timestamp, message.timestamp);
    }

    #[test]
    fn test_message_role_clone() {
        let role = MessageRole::User;
        let cloned = role.clone();
        assert!(matches!(cloned, MessageRole::User));
    }

    #[test]
    fn test_transcript_serialization() {
        let session_id = Uuid::new_v4();
        let mut transcript = Transcript::new(session_id);
        transcript.add_message(MessageRole::User, "test".to_string());

        let serialized = serde_json::to_string(&transcript).unwrap();
        assert!(serialized.contains("messages"));
        assert!(serialized.contains("session_id"));
        assert!(serialized.contains("test"));
    }

    #[test]
    fn test_transcript_deserialization() {
        let session_id = Uuid::new_v4();
        let mut transcript = Transcript::new(session_id);
        transcript.add_message(MessageRole::User, "test".to_string());

        let serialized = serde_json::to_string(&transcript).unwrap();
        let deserialized: Transcript = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.session_id, transcript.session_id);
        assert_eq!(deserialized.messages.len(), transcript.messages.len());
        assert_eq!(deserialized.messages[0].content, transcript.messages[0].content);
    }

    #[test]
    fn test_message_role_serialization() {
        let roles = vec![
            MessageRole::User,
            MessageRole::Assistant,
            MessageRole::System,
        ];

        for role in roles {
            let serialized = serde_json::to_string(&role).unwrap();
            let deserialized: MessageRole = serde_json::from_str(&serialized).unwrap();
            assert!(matches!(
                (&role, &deserialized),
                (MessageRole::User, MessageRole::User)
                | (MessageRole::Assistant, MessageRole::Assistant)
                | (MessageRole::System, MessageRole::System)
            ));
        }
    }

    #[test]
    fn test_message_debug_format() {
        let message = Message {
            id: Uuid::new_v4(),
            role: MessageRole::User,
            content: "test".to_string(),
            timestamp: chrono::Utc::now(),
        };

        let debug = format!("{:?}", message);
        assert!(debug.contains("Message"));
        assert!(debug.contains("test"));
    }

    #[test]
    fn test_transcript_debug_format() {
        let session_id = Uuid::new_v4();
        let transcript = Transcript::new(session_id);
        let debug = format!("{:?}", transcript);
        assert!(debug.contains("Transcript"));
        assert!(debug.contains("messages"));
    }

    #[test]
    fn test_empty_content_message() {
        let session_id = Uuid::new_v4();
        let mut transcript = Transcript::new(session_id);
        transcript.add_message(MessageRole::User, "".to_string());
        assert_eq!(transcript.messages[0].content, "");
    }

    #[test]
    fn test_long_content_message() {
        let session_id = Uuid::new_v4();
        let mut transcript = Transcript::new(session_id);
        let long_content = "a".repeat(10000);
        transcript.add_message(MessageRole::User, long_content.clone());
        assert_eq!(transcript.messages[0].content, long_content);
    }
}
