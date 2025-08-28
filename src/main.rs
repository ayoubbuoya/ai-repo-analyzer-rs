use anyhow::Result;

mod ingest;

pub const TEST_REPO_URL: &str = "https://github.com/ayoubbuoya/orchestra-rs.git";

#[tokio::main]
async fn main() -> Result<()> {
    println!("Hello, world!");

    ingest::fetch_repo(TEST_REPO_URL).await?;

    Ok(())
}
