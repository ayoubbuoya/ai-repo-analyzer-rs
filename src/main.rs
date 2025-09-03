use anyhow::Result;
use qdrant_client::Qdrant;
use rig::{
    client::{CompletionClient, EmbeddingsClient, ProviderClient},
    completion::Prompt,
    providers::gemini::{self, completion::CompletionModel, completion::GEMINI_2_0_FLASH_LITE},
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

    // let embedding_model =
    //     gemini::embedding::EmbeddingModel::new(ai_client, gemini::embedding::EMBEDDING_001, None);

    // ingest::ingest_repo(TEST_REPO_URL, &qdrant_client, &embedding_model).await?;

    let ai_agent = ai_client.agent(
        GEMINI_2_0_FLASH_LITE
    ).preamble("You are AN AI agent that specialize in alayzing public github repo.
     Your primary job is take the github repo link from the user and read its full codebase then generate a full explainer about this repo. Include mermaid diagrams in your explanation for better explainning the architecture. Return only thye response in markdown that i will later write to md file")
     .temperature(0.0)
     .build();

    println!("AI agent built successfully");

    let repo_explain_prompt = format!(
        "Analyze the following GitHub repository and provide a detailed explanation of its structure and functionality: {TEST_REPO_URL}."
    );

    let ai_resp = ai_agent.prompt(repo_explain_prompt).await?;

    println!("{:?}", ai_resp);

    // store response in a markdown file
    std::fs::write("repo_explanation.md", ai_resp)?;

    Ok(())
}
