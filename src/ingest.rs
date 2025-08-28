use std::{fs::remove_dir_all, path::Path};

use anyhow::Result;
use git2::Repository;

pub async fn fetch_repo(repo_url: &str) -> Result<()> {
    let repo_temp_path = "./tmp";

    // If dir already exists, remove it
    if Path::new(repo_temp_path).exists() {
        remove_dir_all(repo_temp_path)?;
    }

    // Clone the Repo
    let repository = Repository::clone(repo_url, repo_temp_path)?;

    println!("Cloned repository to {}", repo_temp_path);

    Ok(())
}
