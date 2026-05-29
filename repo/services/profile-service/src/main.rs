//! Profile Service — binary entry (skeleton only).

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    profile_service::run().await
}
