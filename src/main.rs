use anyhow::Result;
use qdrant_client::Qdrant;

mod ingest;

pub const TEST_REPO_URL: &str = "https://github.com/ayoubbuoya/orchestra-rs.git";

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    // Read the qdrant configuration from environment variables
    let qdrant_url = std::env::var("QDRANT_URL")?;
    let qdrant_api_key = std::env::var("QDRANT_API_KEY")?;

    let qdrant_client = Qdrant::from_url(&qdrant_url)
        .api_key(qdrant_api_key)
        .build()?;

    println!("Qdrant client built successfully");

    let collections_list = qdrant_client.list_collections().await?;

    dbg!(collections_list);

    ingest::fetch_repo(TEST_REPO_URL).await?;

    Ok(())
}
