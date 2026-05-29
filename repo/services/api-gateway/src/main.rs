//! API Gateway — binary entry (skeleton only).

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    api_gateway::run().await
}
