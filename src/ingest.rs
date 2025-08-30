use std::{
    fs::{remove_dir_all, remove_file},
    path::Path,
};

use anyhow::Result;
use git2::Repository;
use ignore::Walk;
use qdrant_client::{Qdrant, qdrant::PointStruct};
use rig::{embeddings::EmbeddingModel, providers::gemini};
use serde_json::json;

use crate::ALLOWED_EXTS;

pub async fn ingest_repo(
    repo_url: &str,
    qdrant_client: &Qdrant,
    embedding_model: &gemini::embedding::EmbeddingModel,
) -> Result<()> {
    let repo_temp_path = "./tmp";
    let content_file_path = "./tmp/content.txt";

    let _repository = fetch_repo(repo_url, repo_temp_path).await?;

    println!("Cloned repository to {}", repo_temp_path);

    // Delete the content file if it exists
    if Path::new(content_file_path).exists() {
        remove_file(content_file_path)?;
    }

    let mut all_content = String::new();

    // Walk repo files
    for entry_result in Walk::new(repo_temp_path) {
        // Process each entry
        let entry = entry_result?;

        if entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
            // Process the file
            let path = entry.path();

            if path
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| ALLOWED_EXTS.contains(&s))
                .unwrap_or(false)
            {
                dbg!("Processing file", path);
                let path_content = tokio::fs::read_to_string(path).await?;

                all_content.push_str(&path_content);
                all_content.push_str("\n");

                // Embed the text
                let path_embedding = embedding_model.embed_text(&path_content).await?;

                // Convert Vec<f64> to Vec<f32> because qdrant vectors use f32
                let vectors_f32: Vec<f32> =
                    path_embedding.vec.into_iter().map(|v| v as f32).collect();

                // Prepare payload as a HashMap<String, qdrant::Value> so it can be converted into Payload
                let mut payload = std::collections::HashMap::new();
                payload.insert(
                    "path".to_string(),
                    qdrant_client::qdrant::Value::from(path.to_string_lossy().to_string()),
                );
                payload.insert(
                    "text".to_string(),
                    qdrant_client::qdrant::Value::from(path_content.clone()),
                );

                let qdrant_point_struct =
                    PointStruct::new(uuid::Uuid::new_v4().to_string(), vectors_f32, payload);

                // Use the UpsertPoints builder API: pass a single builder instance (it will be converted into UpsertPoints)
                let qdrant_points_resp = qdrant_client
                    .upsert_points(qdrant_client::qdrant::UpsertPointsBuilder::new(
                        "repo",
                        vec![qdrant_point_struct],
                    ))
                    .await?;

                dbg!("Qdrant points response", qdrant_points_resp);
            }
        }
    }

    // Write all content to the content file
    tokio::fs::write(content_file_path, all_content).await?;

    Ok(())
}

pub async fn fetch_repo(repo_url: &str, to_path: &str) -> Result<Repository> {
    // If dir already exists, remove it
    if Path::new(to_path).exists() {
        remove_dir_all(to_path)?;
    }

    // Clone the Repo
    Ok(Repository::clone(repo_url, to_path)?)
}
