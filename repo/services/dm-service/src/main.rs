//! DM Service — direct messaging threads and messages for OneLink.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dm_service::run().await
}
