use std::collections::HashMap;

use super::{ConversationRow, MessageRow};

#[derive(Debug)]
pub struct InMemoryStore {
    pub conversations: HashMap<String, ConversationRow>,
    pub messages: HashMap<String, Vec<MessageRow>>,
}

impl Default for InMemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self {
            conversations: HashMap::new(),
            messages: HashMap::new(),
        }
    }
}
