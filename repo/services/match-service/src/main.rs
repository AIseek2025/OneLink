//! Match Service — candidate recommendation engine for OneLink.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match_service::run().await
}
