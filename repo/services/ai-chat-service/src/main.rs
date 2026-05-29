//! Ai Chat Service — binary entry (skeleton only).

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    ai_chat_service::run().await
}
