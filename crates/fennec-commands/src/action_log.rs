use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Maximum number of actions to keep in history
const MAX_HISTORY_SIZE: usize = 100;

/// Represents the state of a file system entity before or after an action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionState {
    /// File was created
    FileCreated { path: PathBuf },
    /// File was modified
    FileModified {
        path: PathBuf,
        content: Vec<u8>,
        content_hash: String,
    },
    /// File was deleted
    FileDeleted { path: PathBuf, content: Vec<u8> },
    /// File was moved/renamed
    FileMoved { from: PathBuf, to: PathBuf },
    /// Directory was created
    DirectoryCreated { path: PathBuf },
    /// Directory was deleted (with contents)
    DirectoryDeleted {
        path: PathBuf,
        contents: Vec<(PathBuf, Vec<u8>)>,
    },
}

impl ActionState {
    /// Get the primary path affected by this state
    pub fn path(&self) -> &PathBuf {
        match self {
            ActionState::FileCreated { path } => path,
            ActionState::FileModified { path, .. } => path,
            ActionState::FileDeleted { path, .. } => path,
            ActionState::FileMoved { to, .. } => to,
            ActionState::DirectoryCreated { path } => path,
            ActionState::DirectoryDeleted { path, .. } => path,
        }
    }
}

/// Represents a reversible action performed on the file system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub id: Uuid,
    pub command: String,
    pub timestamp: DateTime<Utc>,
    pub state_before: ActionState,
    pub state_after: ActionState,
    pub reversible: bool,
    pub description: String,
}

impl Action {
    /// Create a new action
    pub fn new(
        command: String,
        state_before: ActionState,
        state_after: ActionState,
        description: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            command,
            timestamp: Utc::now(),
            state_before,
            state_after,
            reversible: true,
            description,
        }
    }

    /// Create an action for file creation
    pub fn file_created(command: String, path: PathBuf, description: String) -> Self {
        Self::new(
            command,
            ActionState::FileDeleted {
                path: path.clone(),
                content: Vec::new(),
            },
            ActionState::FileCreated { path },
            description,
        )
    }

    /// Create an action for file modification
    pub fn file_modified(
        command: String,
        path: PathBuf,
        old_content: Vec<u8>,
        new_content: Vec<u8>,
        description: String,
    ) -> Self {
        let old_hash = format!("{:x}", md5::compute(&old_content));
        let new_hash = format!("{:x}", md5::compute(&new_content));

        Self::new(
            command,
            ActionState::FileModified {
                path: path.clone(),
                content: old_content,
                content_hash: old_hash,
            },
            ActionState::FileModified {
                path,
                content: new_content,
                content_hash: new_hash,
            },
            description,
        )
    }

    /// Create an action for file deletion
    pub fn file_deleted(
        command: String,
        path: PathBuf,
        content: Vec<u8>,
        description: String,
    ) -> Self {
        Self::new(
            command,
            ActionState::FileCreated { path: path.clone() },
            ActionState::FileDeleted { path, content },
            description,
        )
    }

    /// Create an action for file move/rename
    pub fn file_moved(command: String, from: PathBuf, to: PathBuf, description: String) -> Self {
        Self::new(
            command,
            ActionState::FileMoved {
                from: to.clone(),
                to: from.clone(),
            },
            ActionState::FileMoved { from, to },
            description,
        )
    }
}

/// Manages the action log with undo/redo capabilities
#[derive(Debug, Clone)]
pub struct ActionLog {
    actions: Arc<RwLock<VecDeque<Action>>>,
    current_index: Arc<RwLock<usize>>,
    max_size: usize,
}

impl ActionLog {
    /// Create a new action log
    pub fn new() -> Self {
        Self {
            actions: Arc::new(RwLock::new(VecDeque::new())),
            current_index: Arc::new(RwLock::new(0)),
            max_size: MAX_HISTORY_SIZE,
        }
    }

    /// Create a new action log with custom max size
    pub fn with_max_size(max_size: usize) -> Self {
        Self {
            actions: Arc::new(RwLock::new(VecDeque::new())),
            current_index: Arc::new(RwLock::new(0)),
            max_size,
        }
    }

    /// Record a new action
    pub async fn record(&self, action: Action) {
        let mut actions = self.actions.write().await;
        let mut index = self.current_index.write().await;

        // Remove any actions after current index (they've been undone)
        actions.truncate(*index);

        // Add the new action
        actions.push_back(action);

        // Maintain max size
        if actions.len() > self.max_size {
            actions.pop_front();
        } else {
            *index += 1;
        }
    }

    /// Undo the last action
    pub async fn undo(&self) -> Result<Option<Action>> {
        let mut index = self.current_index.write().await;

        if *index == 0 {
            return Ok(None);
        }

        *index -= 1;
        let actions = self.actions.read().await;

        if let Some(action) = actions.get(*index) {
            Ok(Some(action.clone()))
        } else {
            Ok(None)
        }
    }

    /// Redo the last undone action
    pub async fn redo(&self) -> Result<Option<Action>> {
        let mut index = self.current_index.write().await;
        let actions = self.actions.read().await;

        if *index >= actions.len() {
            return Ok(None);
        }

        if let Some(action) = actions.get(*index) {
            *index += 1;
            Ok(Some(action.clone()))
        } else {
            Ok(None)
        }
    }

    /// Get all actions in chronological order
    pub async fn get_history(&self) -> Vec<Action> {
        let actions = self.actions.read().await;
        actions.iter().cloned().collect()
    }

    /// Get the current action index
    pub async fn current_index(&self) -> usize {
        *self.current_index.read().await
    }

    /// Get the number of actions that can be undone
    pub async fn can_undo_count(&self) -> usize {
        *self.current_index.read().await
    }

    /// Get the number of actions that can be redone
    pub async fn can_redo_count(&self) -> usize {
        let index = *self.current_index.read().await;
        let actions = self.actions.read().await;
        actions.len() - index
    }

    /// Check if undo is possible
    pub async fn can_undo(&self) -> bool {
        *self.current_index.read().await > 0
    }

    /// Check if redo is possible
    pub async fn can_redo(&self) -> bool {
        let index = *self.current_index.read().await;
        let actions = self.actions.read().await;
        index < actions.len()
    }

    /// Clear all history
    pub async fn clear(&self) {
        let mut actions = self.actions.write().await;
        let mut index = self.current_index.write().await;
        actions.clear();
        *index = 0;
    }

    /// Get a specific action by index
    pub async fn get_action(&self, idx: usize) -> Option<Action> {
        let actions = self.actions.read().await;
        actions.get(idx).cloned()
    }
}

impl Default for ActionLog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_record_action() {
        let log = ActionLog::new();
        let action = Action::file_created(
            "create".to_string(),
            PathBuf::from("test.txt"),
            "Created test.txt".to_string(),
        );

        log.record(action).await;

        let history = log.get_history().await;
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].command, "create");
    }

    #[tokio::test]
    async fn test_undo_redo() {
        let log = ActionLog::new();

        let action1 = Action::file_created(
            "create".to_string(),
            PathBuf::from("test1.txt"),
            "Created test1.txt".to_string(),
        );
        let action2 = Action::file_created(
            "create".to_string(),
            PathBuf::from("test2.txt"),
            "Created test2.txt".to_string(),
        );

        log.record(action1).await;
        log.record(action2).await;

        assert_eq!(log.current_index().await, 2);
        assert!(log.can_undo().await);
        assert!(!log.can_redo().await);

        // Undo last action
        let undone = log.undo().await.unwrap();
        assert!(undone.is_some());
        assert_eq!(log.current_index().await, 1);
        assert!(log.can_redo().await);

        // Redo
        let redone = log.redo().await.unwrap();
        assert!(redone.is_some());
        assert_eq!(log.current_index().await, 2);
        assert!(!log.can_redo().await);
    }

    #[tokio::test]
    async fn test_max_size() {
        let log = ActionLog::with_max_size(3);

        for i in 0..5 {
            let action = Action::file_created(
                "create".to_string(),
                PathBuf::from(format!("test{}.txt", i)),
                format!("Created test{}.txt", i),
            );
            log.record(action).await;
        }

        let history = log.get_history().await;
        assert_eq!(history.len(), 3);
    }

    #[tokio::test]
    async fn test_clear() {
        let log = ActionLog::new();

        let action = Action::file_created(
            "create".to_string(),
            PathBuf::from("test.txt"),
            "Created test.txt".to_string(),
        );
        log.record(action).await;

        assert_eq!(log.get_history().await.len(), 1);

        log.clear().await;

        assert_eq!(log.get_history().await.len(), 0);
        assert_eq!(log.current_index().await, 0);
    }

    #[tokio::test]
    async fn test_undo_redo_counts() {
        let log = ActionLog::new();

        for i in 0..3 {
            let action = Action::file_created(
                "create".to_string(),
                PathBuf::from(format!("test{}.txt", i)),
                format!("Created test{}.txt", i),
            );
            log.record(action).await;
        }

        assert_eq!(log.can_undo_count().await, 3);
        assert_eq!(log.can_redo_count().await, 0);

        log.undo().await.unwrap();
        log.undo().await.unwrap();

        assert_eq!(log.can_undo_count().await, 1);
        assert_eq!(log.can_redo_count().await, 2);
    }

    #[tokio::test]
    async fn test_new_action_clears_redo_stack() {
        let log = ActionLog::new();

        let action1 = Action::file_created(
            "create".to_string(),
            PathBuf::from("test1.txt"),
            "Created test1.txt".to_string(),
        );
        let action2 = Action::file_created(
            "create".to_string(),
            PathBuf::from("test2.txt"),
            "Created test2.txt".to_string(),
        );
        let action3 = Action::file_created(
            "create".to_string(),
            PathBuf::from("test3.txt"),
            "Created test3.txt".to_string(),
        );

        log.record(action1).await;
        log.record(action2).await;
        log.undo().await.unwrap();

        assert_eq!(log.can_redo_count().await, 1);

        // Recording new action should clear redo stack
        log.record(action3).await;

        assert_eq!(log.can_redo_count().await, 0);
        assert_eq!(log.get_history().await.len(), 2);
    }
}
