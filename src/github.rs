use std::collections::HashMap;

use crate::RepositoryMetadata;
use crate::types::GitHubIssue;
use crate::types::GitHubLicense;
use crate::types::GitHubRelease;
use crate::types::GitHubUser;
use anyhow::Result;
use chrono::Utc;
use reqwest::Client;

use log::{error, info, warn};

// GitHub API client
pub struct GitHubClient {
    client: Client,
    token: Option<String>,
    base_url: String,
}

impl GitHubClient {
    pub fn new(token: Option<String>) -> Self {
        Self {
            client: Client::new(),
            token,
            base_url: "https://api.github.com".to_string(),
        }
    }

    fn get_auth_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::USER_AGENT,
            reqwest::header::HeaderValue::from_static("ai-repo-analyzer-rs/1.0"),
        );

        if let Some(token) = &self.token {
            let auth_value = format!("Bearer {}", token);
            headers.insert(
                reqwest::header::AUTHORIZATION,
                reqwest::header::HeaderValue::from_str(&auth_value).unwrap(),
            );
        }

        headers
    }

    pub async fn get_repository_metadata(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<RepositoryMetadata> {
        let url = format!("{}/repos/{}/{}", self.base_url, owner, repo);
        info!("Fetching repository metadata from: {}", url);

        let response = self
            .client
            .get(&url)
            .headers(self.get_auth_headers())
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!(
                "Failed to fetch repository: {} - {}",
                response.status(),
                response.text().await?
            );
        }

        let repo_data: serde_json::Value = response.json().await?;

        // Fetch additional data
        let languages = self.get_languages(owner, repo).await.unwrap_or_default();
        let topics = self.get_topics(owner, repo).await.unwrap_or_default();

        // Convert to our structure
        let metadata = RepositoryMetadata {
            id: repo_data["id"].as_u64().unwrap_or(0),
            name: repo_data["name"].as_str().unwrap_or("").to_string(),
            full_name: repo_data["full_name"].as_str().unwrap_or("").to_string(),
            description: repo_data["description"].as_str().map(|s| s.to_string()),
            homepage: repo_data["homepage"].as_str().map(|s| s.to_string()),
            html_url: repo_data["html_url"].as_str().unwrap_or("").to_string(),
            clone_url: repo_data["clone_url"].as_str().unwrap_or("").to_string(),
            ssh_url: repo_data["ssh_url"].as_str().unwrap_or("").to_string(),
            git_url: repo_data["git_url"].as_str().unwrap_or("").to_string(),
            owner: GitHubUser {
                login: repo_data["owner"]["login"]
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
                id: repo_data["owner"]["id"].as_u64().unwrap_or(0),
                avatar_url: repo_data["owner"]["avatar_url"]
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
                html_url: repo_data["owner"]["html_url"]
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
                contributions: None,
            },
            private: repo_data["private"].as_bool().unwrap_or(false),
            fork: repo_data["fork"].as_bool().unwrap_or(false),
            archived: repo_data["archived"].as_bool().unwrap_or(false),
            disabled: repo_data["disabled"].as_bool().unwrap_or(false),
            has_issues: repo_data["has_issues"].as_bool().unwrap_or(false),
            has_projects: repo_data["has_projects"].as_bool().unwrap_or(false),
            has_wiki: repo_data["has_wiki"].as_bool().unwrap_or(false),
            has_pages: repo_data["has_pages"].as_bool().unwrap_or(false),
            has_downloads: repo_data["has_downloads"].as_bool().unwrap_or(false),
            has_discussions: repo_data["has_discussions"].as_bool().unwrap_or(false),
            stargazers_count: repo_data["stargazers_count"].as_u64().unwrap_or(0) as u32,
            watchers_count: repo_data["watchers_count"].as_u64().unwrap_or(0) as u32,
            forks_count: repo_data["forks_count"].as_u64().unwrap_or(0) as u32,
            subscribers_count: repo_data["subscribers_count"].as_u64().map(|x| x as u32),
            network_count: repo_data["network_count"].as_u64().map(|x| x as u32),
            open_issues_count: repo_data["open_issues_count"].as_u64().unwrap_or(0) as u32,
            license: repo_data["license"]
                .as_object()
                .map(|license| GitHubLicense {
                    key: license["key"].as_str().unwrap_or("").to_string(),
                    name: license["name"].as_str().unwrap_or("").to_string(),
                    spdx_id: license["spdx_id"].as_str().map(|s| s.to_string()),
                    url: license["url"].as_str().map(|s| s.to_string()),
                }),
            topics,
            default_branch: repo_data["default_branch"]
                .as_str()
                .unwrap_or("main")
                .to_string(),
            size: repo_data["size"].as_u64().unwrap_or(0) as u32,
            language: repo_data["language"].as_str().map(|s| s.to_string()),
            languages,
            created_at: chrono::DateTime::parse_from_rfc3339(
                repo_data["created_at"]
                    .as_str()
                    .unwrap_or("1970-01-01T00:00:00Z"),
            )
            .unwrap()
            .with_timezone(&Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339(
                repo_data["updated_at"]
                    .as_str()
                    .unwrap_or("1970-01-01T00:00:00Z"),
            )
            .unwrap()
            .with_timezone(&Utc),
            pushed_at: chrono::DateTime::parse_from_rfc3339(
                repo_data["pushed_at"]
                    .as_str()
                    .unwrap_or("1970-01-01T00:00:00Z"),
            )
            .unwrap()
            .with_timezone(&Utc),
        };

        Ok(metadata)
    }

    pub async fn get_languages(&self, owner: &str, repo: &str) -> Result<HashMap<String, u64>> {
        let url = format!("{}/repos/{}/{}/languages", self.base_url, owner, repo);

        let response = self
            .client
            .get(&url)
            .headers(self.get_auth_headers())
            .send()
            .await?;

        if response.status().is_success() {
            let languages: HashMap<String, u64> = response.json().await?;
            Ok(languages)
        } else {
            Ok(HashMap::new())
        }
    }

    pub async fn get_topics(&self, owner: &str, repo: &str) -> Result<Vec<String>> {
        let url = format!("{}/repos/{}/{}/topics", self.base_url, owner, repo);

        let response = self
            .client
            .get(&url)
            .headers(self.get_auth_headers())
            .send()
            .await?;

        if response.status().is_success() {
            let data: serde_json::Value = response.json().await?;
            let topics = data["names"]
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect();
            Ok(topics)
        } else {
            Ok(Vec::new())
        }
    }

    pub async fn get_contributors(&self, owner: &str, repo: &str) -> Result<Vec<GitHubUser>> {
        let url = format!("{}/repos/{}/{}/contributors", self.base_url, owner, repo);

        let response = self
            .client
            .get(&url)
            .headers(self.get_auth_headers())
            .send()
            .await?;

        if response.status().is_success() {
            let contributors: Vec<serde_json::Value> = response.json().await?;
            let users = contributors
                .into_iter()
                .map(|c| GitHubUser {
                    login: c["login"].as_str().unwrap_or("").to_string(),
                    id: c["id"].as_u64().unwrap_or(0),
                    avatar_url: c["avatar_url"].as_str().unwrap_or("").to_string(),
                    html_url: c["html_url"].as_str().unwrap_or("").to_string(),
                    contributions: c["contributions"].as_u64().map(|x| x as u32),
                })
                .collect();
            Ok(users)
        } else {
            Ok(Vec::new())
        }
    }

    pub async fn get_releases(
        &self,
        owner: &str,
        repo: &str,
        limit: usize,
    ) -> Result<Vec<GitHubRelease>> {
        let url = format!(
            "{}/repos/{}/{}/releases?per_page={}",
            self.base_url, owner, repo, limit
        );

        let response = self
            .client
            .get(&url)
            .headers(self.get_auth_headers())
            .send()
            .await?;

        if response.status().is_success() {
            let releases: Vec<serde_json::Value> = response.json().await?;
            let parsed_releases = releases
                .into_iter()
                .map(|r| GitHubRelease {
                    tag_name: r["tag_name"].as_str().unwrap_or("").to_string(),
                    name: r["name"].as_str().map(|s| s.to_string()),
                    body: r["body"].as_str().map(|s| s.to_string()),
                    draft: r["draft"].as_bool().unwrap_or(false),
                    prerelease: r["prerelease"].as_bool().unwrap_or(false),
                    created_at: chrono::DateTime::parse_from_rfc3339(
                        r["created_at"].as_str().unwrap_or("1970-01-01T00:00:00Z"),
                    )
                    .unwrap()
                    .with_timezone(&Utc),
                    published_at: r["published_at"]
                        .as_str()
                        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                        .map(|dt| dt.with_timezone(&Utc)),
                    author: GitHubUser {
                        login: r["author"]["login"].as_str().unwrap_or("").to_string(),
                        id: r["author"]["id"].as_u64().unwrap_or(0),
                        avatar_url: r["author"]["avatar_url"].as_str().unwrap_or("").to_string(),
                        html_url: r["author"]["html_url"].as_str().unwrap_or("").to_string(),
                        contributions: None,
                    },
                    assets_count: r["assets"].as_array().map(|a| a.len()).unwrap_or(0),
                })
                .collect();
            Ok(parsed_releases)
        } else {
            Ok(Vec::new())
        }
    }

    pub async fn get_recent_issues(
        &self,
        owner: &str,
        repo: &str,
        limit: usize,
    ) -> Result<Vec<GitHubIssue>> {
        let url = format!(
            "{}/repos/{}/{}/issues?state=all&per_page={}&sort=updated",
            self.base_url, owner, repo, limit
        );

        let response = self
            .client
            .get(&url)
            .headers(self.get_auth_headers())
            .send()
            .await?;

        if response.status().is_success() {
            let issues: Vec<serde_json::Value> = response.json().await?;
            let parsed_issues = issues
                .into_iter()
                .filter(|i| i["pull_request"].is_null()) // Filter out pull requests
                .map(|i| GitHubIssue {
                    number: i["number"].as_u64().unwrap_or(0) as u32,
                    title: i["title"].as_str().unwrap_or("").to_string(),
                    state: i["state"].as_str().unwrap_or("").to_string(),
                    created_at: chrono::DateTime::parse_from_rfc3339(
                        i["created_at"].as_str().unwrap_or("1970-01-01T00:00:00Z"),
                    )
                    .unwrap()
                    .with_timezone(&Utc),
                    updated_at: chrono::DateTime::parse_from_rfc3339(
                        i["updated_at"].as_str().unwrap_or("1970-01-01T00:00:00Z"),
                    )
                    .unwrap()
                    .with_timezone(&Utc),
                    closed_at: i["closed_at"]
                        .as_str()
                        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                        .map(|dt| dt.with_timezone(&Utc)),
                    author: GitHubUser {
                        login: i["user"]["login"].as_str().unwrap_or("").to_string(),
                        id: i["user"]["id"].as_u64().unwrap_or(0),
                        avatar_url: i["user"]["avatar_url"].as_str().unwrap_or("").to_string(),
                        html_url: i["user"]["html_url"].as_str().unwrap_or("").to_string(),
                        contributions: None,
                    },
                    labels: i["labels"]
                        .as_array()
                        .unwrap_or(&Vec::new())
                        .iter()
                        .filter_map(|l| l["name"].as_str())
                        .map(|s| s.to_string())
                        .collect(),
                    comments: i["comments"].as_u64().unwrap_or(0) as u32,
                })
                .collect();
            Ok(parsed_issues)
        } else {
            Ok(Vec::new())
        }
    }
}
