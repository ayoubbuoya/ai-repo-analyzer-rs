use anyhow::Result;
use url::Url;

// Utility function to parse GitHub URL
pub fn parse_github_url(url: &str) -> Result<(String, String)> {
    let parsed_url = Url::parse(url)?;

    if parsed_url.host_str() != Some("github.com") {
        anyhow::bail!("URL is not a GitHub repository URL");
    }

    let path_segments: Vec<&str> = parsed_url
        .path_segments()
        .ok_or_else(|| anyhow::anyhow!("Invalid URL path"))?
        .collect();

    if path_segments.len() < 2 {
        anyhow::bail!("Invalid GitHub repository URL format");
    }

    let owner = path_segments[0].to_string();
    let repo = path_segments[1].trim_end_matches(".git").to_string();

    Ok((owner, repo))
}
