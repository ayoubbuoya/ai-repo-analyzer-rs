use std::{
    fs::{remove_dir_all, remove_file},
    path::Path,
};

use anyhow::Result;
use git2::Repository;
use ignore::Walk;
use qdrant_client::{Qdrant, qdrant::PointStruct};

use crate::ALLOWED_EXTS;

pub async fn ingest_repo(repo_url: &str, qdrant_client: &Qdrant) -> Result<()> {
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

                // let qdrant_point_struct = PointStruct::try_from(path_content)?;
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
