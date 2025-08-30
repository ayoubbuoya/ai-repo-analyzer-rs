use anyhow::Result;
use qdrant_client::Qdrant;
use rig::{
    client::{EmbeddingsClient, ProviderClient},
    providers::gemini,
};

mod agent;
mod ingest;

pub const TEST_REPO_URL: &str = "https://github.com/ayoubbuoya/orchestra-rs.git";
pub const ALLOWED_EXTS: &[&str] = &["rs", "ts", "md", "sol", "js", "txt", "json", "toml"];

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

    let ai_client = gemini::Client::from_env();

    // let embedding_model = ai_client
    //     .embeddings(gemini::embedding::EMBEDDING_001)
    //     .build()
    //     .await?;

    let embedding_model =
        gemini::embedding::EmbeddingModel::new(ai_client, gemini::embedding::EMBEDDING_001, None);

    ingest::ingest_repo(TEST_REPO_URL, &qdrant_client, &embedding_model).await?;

    Ok(())
}
