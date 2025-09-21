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