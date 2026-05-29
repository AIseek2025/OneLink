use crate::config::AppConfig;
use reqwest::Client;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub http: Client,
}

impl AppState {
    pub fn new(config: AppConfig) -> Arc<Self> {
        Arc::new(Self {
            config,
            http: Client::new(),
        })
    }
}
