//! Safety Service — content safety, reports, blocks, and DM review for OneLink.

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    safety_service::run().await
}
