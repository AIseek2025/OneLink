use std::collections::HashMap;

#[derive(Debug)]
pub struct InMemoryStore {
    pub users: HashMap<String, UserRow>,
    pub sessions: HashMap<String, SessionRow>,
    pub email_index: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct UserRow {
    pub user_id: String,
    pub status: String,
    pub primary_region: String,
    pub primary_language: String,
    pub timezone: String,
    pub created_at: String,
    pub password_hash: String,
}

#[derive(Debug, Clone)]
pub struct SessionRow {
    pub token: String,
    pub user_id: String,
    pub expires_at: String,
}

impl Default for InMemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self {
            users: HashMap::new(),
            sessions: HashMap::new(),
            email_index: HashMap::new(),
        }
    }
}
