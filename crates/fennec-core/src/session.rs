use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Uuid,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub title: Option<String>,
}

impl Session {
    pub fn new() -> Self {
        let now = chrono::Utc::now();
        Self {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            title: None,
        }
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_new() {
        let session = Session::new();
        assert!(!session.id.is_nil());
        assert!(session.title.is_none());
        assert_eq!(session.created_at, session.updated_at);
    }

    #[test]
    fn test_session_default() {
        let session = Session::default();
        assert!(!session.id.is_nil());
        assert!(session.title.is_none());
    }

    #[test]
    fn test_session_unique_ids() {
        let session1 = Session::new();
        let session2 = Session::new();
        assert_ne!(session1.id, session2.id);
    }

    #[test]
    fn test_session_clone() {
        let session = Session::new();
        let cloned = session.clone();
        assert_eq!(session.id, cloned.id);
        assert_eq!(session.created_at, cloned.created_at);
        assert_eq!(session.updated_at, cloned.updated_at);
        assert_eq!(session.title, cloned.title);
    }

    #[test]
    fn test_session_with_title() {
        let mut session = Session::new();
        session.title = Some("Test Session".to_string());
        assert_eq!(session.title, Some("Test Session".to_string()));
    }

    #[test]
    fn test_session_serialization() {
        let session = Session::new();
        let serialized = serde_json::to_string(&session).unwrap();
        assert!(serialized.contains("id"));
        assert!(serialized.contains("created_at"));
        assert!(serialized.contains("updated_at"));
    }

    #[test]
    fn test_session_deserialization() {
        let session = Session::new();
        let serialized = serde_json::to_string(&session).unwrap();
        let deserialized: Session = serde_json::from_str(&serialized).unwrap();
        assert_eq!(session.id, deserialized.id);
        assert_eq!(session.created_at, deserialized.created_at);
        assert_eq!(session.updated_at, deserialized.updated_at);
    }

    #[test]
    fn test_session_debug_format() {
        let session = Session::new();
        let debug = format!("{:?}", session);
        assert!(debug.contains("Session"));
        assert!(debug.contains("id"));
    }
}
