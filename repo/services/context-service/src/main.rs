//! Context Service — Memory Compute Layer (skeleton only).

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    context_service::run().await
}
