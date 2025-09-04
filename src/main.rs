use anyhow::Result;
use chrono::{DateTime, Utc};
use git2::Repository;
use ignore::WalkBuilder;
use log::{error, info, warn};
use mime_guess;
use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use url::Url;
use walkdir::WalkDir;

// GitHub API response structures
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GitHubUser {
    pub login: String,
    pub id: u64,
    pub avatar_url: String,
    pub html_url: String,
    pub contributions: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GitHubLicense {
    pub key: String,
    pub name: String,
    pub spdx_id: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GitHubTopic {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GitHubLanguage {
    pub name: String,
    pub bytes: u64,
    pub percentage: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub name: Option<String>,
    pub body: Option<String>,
    pub draft: bool,
    pub prerelease: bool,
    pub created_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
    pub author: GitHubUser,
    pub assets_count: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GitHubIssue {
    pub number: u32,
    pub title: String,
    pub state: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub author: GitHubUser,
    pub labels: Vec<String>,
    pub comments: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GitHubCommit {
    pub sha: String,
    pub message: String,
    pub author: GitHubUser,
    pub date: DateTime<Utc>,
    pub additions: u32,
    pub deletions: u32,
    pub files_changed: u32,
}

// Repository metadata from GitHub API
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RepositoryMetadata {
    pub id: u64,
    pub name: String,
    pub full_name: String,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub html_url: String,
    pub clone_url: String,
    pub ssh_url: String,
    pub git_url: String,
    pub owner: GitHubUser,
    pub private: bool,
    pub fork: bool,
    pub archived: bool,
    pub disabled: bool,
    pub has_issues: bool,
    pub has_projects: bool,
    pub has_wiki: bool,
    pub has_pages: bool,
    pub has_downloads: bool,
    pub has_discussions: bool,
    pub stargazers_count: u32,
    pub watchers_count: u32,
    pub forks_count: u32,
    pub subscribers_count: Option<u32>,
    pub network_count: Option<u32>,
    pub open_issues_count: u32,
    pub license: Option<GitHubLicense>,
    pub topics: Vec<String>,
    pub default_branch: String,
    pub size: u32, // KB
    pub language: Option<String>,
    pub languages: HashMap<String, u64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub pushed_at: DateTime<Utc>,
}

// File analysis structures
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileInfo {
    pub path: PathBuf,
    pub name: String,
    pub extension: Option<String>,
    pub size: u64,
    pub lines_of_code: Option<u32>,
    pub blank_lines: Option<u32>,
    pub comment_lines: Option<u32>,
    pub language: Option<String>,
    pub mime_type: Option<String>,
    pub is_binary: bool,
    pub is_text: bool,
    pub encoding: Option<String>,
    pub hash: String,
    pub content_preview: Option<String>, // First few lines for analysis
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DirectoryInfo {
    pub path: PathBuf,
    pub name: String,
    pub file_count: u32,
    pub subdirectory_count: u32,
    pub total_size: u64,
    pub files: Vec<FileInfo>,
    pub subdirectories: Vec<DirectoryInfo>,
}

// Code analysis structures
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LanguageStats {
    pub language: String,
    pub file_count: u32,
    pub lines_of_code: u32,
    pub blank_lines: u32,
    pub comment_lines: u32,
    pub total_bytes: u64,
    pub percentage: f64,
    pub complexity_score: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CodeMetrics {
    pub total_files: u32,
    pub total_lines: u32,
    pub total_loc: u32,
    pub total_blank_lines: u32,
    pub total_comment_lines: u32,
    pub total_size: u64,
    pub language_stats: HashMap<String, LanguageStats>,
    pub average_file_size: f64,
    pub largest_files: Vec<FileInfo>,
    pub most_complex_files: Vec<FileInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConfigFile {
    pub path: PathBuf,
    pub file_type: String, // package.json, Cargo.toml, requirements.txt, etc.
    pub content: String,
    pub parsed_dependencies: Option<HashMap<String, String>>,
    pub scripts: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DocumentationFile {
    pub path: PathBuf,
    pub file_type: String, // README, CHANGELOG, LICENSE, etc.
    pub content: String,
    pub word_count: u32,
    pub has_badges: bool,
    pub has_toc: bool,
    pub sections: Vec<String>,
}

// Git analysis structures
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GitAnalysis {
    pub total_commits: u32,
    pub contributors: Vec<GitHubUser>,
    pub recent_commits: Vec<GitHubCommit>,
    pub commit_frequency: HashMap<String, u32>, // month -> commit count
    pub most_active_files: Vec<(String, u32)>,  // file path -> modification count
    pub branch_count: u32,
    pub tag_count: u32,
    pub first_commit_date: Option<DateTime<Utc>>,
    pub last_commit_date: Option<DateTime<Utc>>,
}

// Project type detection
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProjectInfo {
    pub primary_language: Option<String>,
    pub project_type: Vec<String>, // web, cli, library, framework, etc.
    pub frameworks: Vec<String>,
    pub build_tools: Vec<String>,
    pub package_managers: Vec<String>,
    pub testing_frameworks: Vec<String>,
    pub ci_cd_tools: Vec<String>,
    pub deployment_configs: Vec<String>,
    pub database_technologies: Vec<String>,
}

// Security and quality analysis
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SecurityInfo {
    pub has_security_policy: bool,
    pub has_dependabot: bool,
    pub has_codeql: bool,
    pub vulnerability_alerts: Vec<String>,
    pub outdated_dependencies: Vec<String>,
    pub license_compatibility: Vec<String>,
}

// Comprehensive repository analysis result
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RepositoryAnalysis {
    pub url: String,
    pub analyzed_at: DateTime<Utc>,
    pub metadata: RepositoryMetadata,
    pub file_structure: DirectoryInfo,
    pub code_metrics: CodeMetrics,
    pub git_analysis: GitAnalysis,
    pub project_info: ProjectInfo,
    pub config_files: Vec<ConfigFile>,
    pub documentation: Vec<DocumentationFile>,
    pub security_info: SecurityInfo,
    pub releases: Vec<GitHubRelease>,
    pub recent_issues: Vec<GitHubIssue>,
    pub analysis_summary: String,
    pub ai_insights: Option<String>,
}

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

// Git repository manager
pub struct GitManager {
    work_dir: PathBuf,
}

impl GitManager {
    pub fn new(work_dir: Option<PathBuf>) -> Self {
        let work_dir = work_dir.unwrap_or_else(|| std::env::temp_dir().join("ai-repo-analyzer"));

        // Create work directory if it doesn't exist
        if !work_dir.exists() {
            std::fs::create_dir_all(&work_dir).unwrap_or_else(|e| {
                warn!("Failed to create work directory: {}", e);
            });
        }

        Self { work_dir }
    }

    pub async fn clone_or_update_repository(
        &self,
        clone_url: &str,
        repo_name: &str,
    ) -> Result<PathBuf> {
        let repo_path = self.work_dir.join(repo_name);

        // Remove existing directory if it exists
        if repo_path.exists() {
            info!("Removing existing repository directory: {:?}", repo_path);
            fs::remove_dir_all(&repo_path)?;
        }

        info!("Cloning repository from {} to {:?}", clone_url, repo_path);

        // Clone the repository
        let _repo = Repository::clone(clone_url, &repo_path)
            .map_err(|e| anyhow::anyhow!("Failed to clone repository: {}", e))?;

        info!("Successfully cloned repository to {:?}", repo_path);
        Ok(repo_path)
    }

    pub fn analyze_git_history(&self, repo_path: &Path) -> Result<GitAnalysis> {
        let repo = Repository::open(repo_path)?;

        // Get all commits
        let mut revwalk = repo.revwalk()?;
        revwalk.push_head()?;
        revwalk.set_sorting(git2::Sort::TIME)?;

        let mut total_commits = 0;
        let mut contributors: HashMap<String, GitHubUser> = HashMap::new();
        let mut recent_commits = Vec::new();
        let mut commit_frequency: HashMap<String, u32> = HashMap::new();
        let mut file_modifications: HashMap<String, u32> = HashMap::new();
        let mut first_commit_date: Option<DateTime<Utc>> = None;
        let mut last_commit_date: Option<DateTime<Utc>> = None;

        for (index, oid) in revwalk.enumerate() {
            if index >= 1000 {
                // Limit to first 1000 commits for performance
                break;
            }

            let oid = oid?;
            let commit = repo.find_commit(oid)?;
            total_commits += 1;

            let commit_time = DateTime::from_timestamp(commit.time().seconds(), 0)
                .unwrap_or_else(|| Utc::now())
                .with_timezone(&Utc);

            if first_commit_date.is_none() {
                first_commit_date = Some(commit_time);
            }
            last_commit_date = Some(commit_time);

            // Track commit frequency by month
            let month_key = commit_time.format("%Y-%m").to_string();
            *commit_frequency.entry(month_key).or_insert(0) += 1;

            // Track contributors
            let author = commit.author();
            if let (Some(name), Some(email)) = (author.name(), author.email()) {
                let key = format!("{}:{}", name, email);
                contributors
                    .entry(key.clone())
                    .or_insert_with(|| GitHubUser {
                        login: name.to_string(),
                        id: 0, // We don't have GitHub ID from git history
                        avatar_url: String::new(),
                        html_url: String::new(),
                        contributions: Some(0),
                    });
                if let Some(user) = contributors.get_mut(&key) {
                    user.contributions = Some(user.contributions.unwrap_or(0) + 1);
                }
            }

            // Store recent commits (first 50)
            if recent_commits.len() < 50 {
                let git_commit = GitHubCommit {
                    sha: format!("{}", oid),
                    message: commit.message().unwrap_or("").to_string(),
                    author: GitHubUser {
                        login: author.name().unwrap_or("Unknown").to_string(),
                        id: 0,
                        avatar_url: String::new(),
                        html_url: String::new(),
                        contributions: None,
                    },
                    date: commit_time,
                    additions: 0, // Git2 doesn't provide easy access to diff stats
                    deletions: 0,
                    files_changed: 0,
                };
                recent_commits.push(git_commit);
            }

            // Track file modifications (simplified)
            if let Ok(tree) = commit.tree() {
                let mut file_count = 0;
                tree.walk(git2::TreeWalkMode::PreOrder, |_root, entry| {
                    if let Some(name) = entry.name() {
                        *file_modifications.entry(name.to_string()).or_insert(0) += 1;
                        file_count += 1;
                    }
                    if file_count > 100 {
                        // Limit file tracking for performance
                        git2::TreeWalkResult::Abort
                    } else {
                        git2::TreeWalkResult::Ok
                    }
                })?;
            }
        }

        // Get most active files
        let mut most_active_files: Vec<_> = file_modifications.into_iter().collect();
        most_active_files.sort_by(|a, b| b.1.cmp(&a.1));
        most_active_files.truncate(20);

        // Count branches and tags
        let branches = repo.branches(Some(git2::BranchType::Local))?;
        let branch_count = branches.count() as u32;

        let tag_count = repo.tag_names(None)?.len() as u32;

        let git_analysis = GitAnalysis {
            total_commits,
            contributors: contributors.into_values().collect(),
            recent_commits,
            commit_frequency,
            most_active_files,
            branch_count,
            tag_count,
            first_commit_date,
            last_commit_date,
        };

        Ok(git_analysis)
    }
}

// File system analyzer
pub struct FileSystemAnalyzer {
    ignore_patterns: Vec<String>,
    max_file_size: u64,
    max_preview_lines: usize,
}

impl FileSystemAnalyzer {
    pub fn new() -> Self {
        Self {
            ignore_patterns: vec![
                ".git".to_string(),
                "node_modules".to_string(),
                "target".to_string(),
                "build".to_string(),
                "dist".to_string(),
                "__pycache__".to_string(),
                ".pytest_cache".to_string(),
                ".venv".to_string(),
                "venv".to_string(),
                ".env".to_string(),
                "*.log".to_string(),
                "*.tmp".to_string(),
                "*.cache".to_string(),
            ],
            max_file_size: 1_000_000, // 1MB
            max_preview_lines: 50,
        }
    }

    pub fn analyze_directory(&self, repo_path: &Path) -> Result<DirectoryInfo> {
        info!("Analyzing directory structure: {:?}", repo_path);
        self.analyze_directory_recursive(repo_path, repo_path)
    }

    fn analyze_directory_recursive(
        &self,
        root_path: &Path,
        current_path: &Path,
    ) -> Result<DirectoryInfo> {
        let mut files = Vec::new();
        let mut subdirectories = Vec::new();
        let mut total_size = 0u64;
        let mut file_count = 0u32;
        let mut subdirectory_count = 0u32;

        let walker = WalkBuilder::new(current_path)
            .max_depth(Some(1))
            .hidden(false)
            .git_ignore(true)
            .build();

        for entry in walker {
            let entry = entry?;
            let path = entry.path();

            if path == current_path {
                continue;
            }

            // Skip ignored patterns
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if self.ignore_patterns.iter().any(|pattern| {
                    pattern.trim_end_matches('*') == file_name
                        || file_name.starts_with(pattern.trim_end_matches('*'))
                }) {
                    continue;
                }
            }

            let relative_path = path.strip_prefix(root_path).unwrap_or(path).to_path_buf();

            if path.is_file() {
                match self.analyze_file(path, relative_path) {
                    Ok(file_info) => {
                        total_size += file_info.size;
                        file_count += 1;
                        files.push(file_info);
                    }
                    Err(e) => {
                        warn!("Failed to analyze file {:?}: {}", path, e);
                    }
                }
            } else if path.is_dir() {
                match self.analyze_directory_recursive(root_path, path) {
                    Ok(dir_info) => {
                        total_size += dir_info.total_size;
                        subdirectory_count += 1;
                        subdirectories.push(dir_info);
                    }
                    Err(e) => {
                        warn!("Failed to analyze directory {:?}: {}", path, e);
                    }
                }
            }
        }

        let dir_name = current_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        Ok(DirectoryInfo {
            path: current_path.to_path_buf(),
            name: dir_name,
            file_count,
            subdirectory_count,
            total_size,
            files,
            subdirectories,
        })
    }

    fn analyze_file(&self, file_path: &Path, relative_path: PathBuf) -> Result<FileInfo> {
        let metadata = fs::metadata(file_path)?;
        let size = metadata.len();

        if size > self.max_file_size {
            return Ok(FileInfo {
                path: relative_path.clone(),
                name: file_path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
                extension: file_path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|s| s.to_string()),
                size,
                lines_of_code: None,
                blank_lines: None,
                comment_lines: None,
                language: None,
                mime_type: Some("application/octet-stream".to_string()),
                is_binary: true,
                is_text: false,
                encoding: None,
                hash: self.calculate_file_hash(file_path)?,
                content_preview: None,
            });
        }

        let mime_type = mime_guess::from_path(file_path)
            .first()
            .map(|m| m.to_string());

        let is_binary = self.is_binary_file(file_path)?;

        let (content_preview, encoding, lines_info) = if !is_binary {
            self.read_text_file_info(file_path)?
        } else {
            (None, None, (None, None, None))
        };

        let language = self.detect_language(file_path);

        Ok(FileInfo {
            path: relative_path,
            name: file_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            extension: file_path
                .extension()
                .and_then(|e| e.to_str())
                .map(|s| s.to_string()),
            size,
            lines_of_code: lines_info.0,
            blank_lines: lines_info.1,
            comment_lines: lines_info.2,
            language,
            mime_type,
            is_binary,
            is_text: !is_binary,
            encoding,
            hash: self.calculate_file_hash(file_path)?,
            content_preview,
        })
    }

    fn is_binary_file(&self, file_path: &Path) -> Result<bool> {
        let mut file = fs::File::open(file_path)?;
        let mut buffer = [0; 512];
        let bytes_read = file.read(&mut buffer)?;

        // Check for null bytes (common in binary files)
        let has_null_bytes = buffer[..bytes_read].contains(&0);

        // Check if it's a known binary extension
        let is_binary_ext = if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
            matches!(
                ext.to_lowercase().as_str(),
                "exe"
                    | "dll"
                    | "so"
                    | "dylib"
                    | "bin"
                    | "obj"
                    | "o"
                    | "a"
                    | "lib"
                    | "jpg"
                    | "jpeg"
                    | "png"
                    | "gif"
                    | "bmp"
                    | "ico"
                    | "svg"
                    | "mp3"
                    | "mp4"
                    | "avi"
                    | "mov"
                    | "wmv"
                    | "flv"
                    | "zip"
                    | "tar"
                    | "gz"
                    | "rar"
                    | "7z"
                    | "bz2"
                    | "pdf"
                    | "doc"
                    | "docx"
                    | "xls"
                    | "xlsx"
                    | "ppt"
                    | "pptx"
            )
        } else {
            false
        };

        Ok(has_null_bytes || is_binary_ext)
    }

    fn read_text_file_info(
        &self,
        file_path: &Path,
    ) -> Result<(
        Option<String>,
        Option<String>,
        (Option<u32>, Option<u32>, Option<u32>),
    )> {
        let content = fs::read(file_path)?;

        // Detect encoding
        let (decoded, encoding_used, _) = encoding_rs::UTF_8.decode(&content);
        let encoding_name = encoding_used.name().to_string();

        let text = decoded.to_string();
        let lines: Vec<&str> = text.lines().collect();

        // Calculate line statistics
        let total_lines = lines.len() as u32;
        let blank_lines = lines.iter().filter(|line| line.trim().is_empty()).count() as u32;

        // Simple comment detection (can be improved with language-specific parsing)
        let comment_lines = self.count_comment_lines(&lines, file_path);
        let lines_of_code = total_lines - blank_lines - comment_lines;

        // Create preview (first N lines)
        let preview_lines: Vec<&str> = lines.iter().take(self.max_preview_lines).cloned().collect();
        let content_preview = if !preview_lines.is_empty() {
            Some(preview_lines.join("\n"))
        } else {
            None
        };

        Ok((
            content_preview,
            Some(encoding_name),
            (Some(lines_of_code), Some(blank_lines), Some(comment_lines)),
        ))
    }

    fn count_comment_lines(&self, lines: &[&str], file_path: &Path) -> u32 {
        let ext = file_path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_default();

        let (single_comment, multi_start, multi_end) = match ext.as_str() {
            "rs" | "js" | "ts" | "jsx" | "tsx" | "c" | "cpp" | "cc" | "cxx" | "h" | "hpp"
            | "java" | "scala" | "kt" | "cs" | "go" | "php" | "swift" => ("//", "/*", "*/"),
            "py" | "sh" | "bash" | "zsh" | "fish" | "rb" | "pl" | "r" => ("#", "\"\"\"", "\"\"\""),
            "html" | "xml" | "svg" => ("", "<!--", "-->"),
            "css" | "scss" | "sass" | "less" => ("", "/*", "*/"),
            "sql" => ("--", "/*", "*/"),
            "hs" => ("--", "{-", "-}"),
            "ml" | "mli" => ("", "(*", "*)"),
            _ => ("", "", ""),
        };

        let mut comment_count = 0;
        let mut in_multi_comment = false;

        for line in lines {
            let trimmed = line.trim();

            if !multi_start.is_empty() && !multi_end.is_empty() {
                if in_multi_comment {
                    comment_count += 1;
                    if trimmed.contains(multi_end) {
                        in_multi_comment = false;
                    }
                    continue;
                }

                if trimmed.contains(multi_start) {
                    comment_count += 1;
                    if !trimmed.contains(multi_end) {
                        in_multi_comment = true;
                    }
                    continue;
                }
            }

            if !single_comment.is_empty() && trimmed.starts_with(single_comment) {
                comment_count += 1;
            }
        }

        comment_count
    }

    fn detect_language(&self, file_path: &Path) -> Option<String> {
        let ext = file_path.extension()?.to_str()?.to_lowercase();

        let language = match ext.as_str() {
            "rs" => "Rust",
            "py" => "Python",
            "js" => "JavaScript",
            "ts" => "TypeScript",
            "jsx" => "JavaScript",
            "tsx" => "TypeScript",
            "java" => "Java",
            "c" => "C",
            "cpp" | "cc" | "cxx" => "C++",
            "h" => "C/C++ Header",
            "hpp" => "C++ Header",
            "cs" => "C#",
            "go" => "Go",
            "php" => "PHP",
            "rb" => "Ruby",
            "pl" => "Perl",
            "swift" => "Swift",
            "kt" => "Kotlin",
            "scala" => "Scala",
            "hs" => "Haskell",
            "ml" | "mli" => "OCaml",
            "r" => "R",
            "m" => "Objective-C",
            "mm" => "Objective-C++",
            "sh" | "bash" | "zsh" | "fish" => "Shell",
            "ps1" => "PowerShell",
            "html" | "htm" => "HTML",
            "css" => "CSS",
            "scss" => "SCSS",
            "sass" => "Sass",
            "less" => "Less",
            "xml" => "XML",
            "json" => "JSON",
            "yaml" | "yml" => "YAML",
            "toml" => "TOML",
            "ini" => "INI",
            "md" => "Markdown",
            "sql" => "SQL",
            "dockerfile" => "Dockerfile",
            "makefile" => "Makefile",
            "cmake" => "CMake",
            "proto" => "Protocol Buffers",
            "graphql" => "GraphQL",
            "vue" => "Vue",
            "svelte" => "Svelte",
            "tex" => "LaTeX",
            _ => return None,
        };

        Some(language.to_string())
    }

    fn calculate_file_hash(&self, file_path: &Path) -> Result<String> {
        let content = fs::read(file_path)?;
        let digest = md5::compute(&content);
        Ok(format!("{:x}", digest))
    }

    pub fn find_config_files(&self, repo_path: &Path) -> Result<Vec<ConfigFile>> {
        let mut config_files = Vec::new();

        let config_patterns = vec![
            ("package.json", "npm"),
            ("Cargo.toml", "cargo"),
            ("requirements.txt", "pip"),
            ("Pipfile", "pipenv"),
            ("pyproject.toml", "python"),
            ("pom.xml", "maven"),
            ("build.gradle", "gradle"),
            ("composer.json", "composer"),
            ("Gemfile", "bundler"),
            ("go.mod", "go"),
            ("pubspec.yaml", "dart"),
            ("project.clj", "leiningen"),
            ("mix.exs", "mix"),
            ("rebar.config", "rebar"),
            ("stack.yaml", "stack"),
            ("cabal.project", "cabal"),
            ("dune-project", "dune"),
            (".travis.yml", "travis"),
            (".github/workflows", "github-actions"),
            ("Dockerfile", "docker"),
            ("docker-compose.yml", "docker-compose"),
            ("kubernetes.yaml", "kubernetes"),
            ("terraform.tf", "terraform"),
            ("ansible.yml", "ansible"),
            (".eslintrc", "eslint"),
            (".prettierrc", "prettier"),
            ("tsconfig.json", "typescript"),
            ("webpack.config.js", "webpack"),
            ("vite.config.js", "vite"),
            ("rollup.config.js", "rollup"),
            ("jest.config.js", "jest"),
            ("cypress.json", "cypress"),
            (".env", "environment"),
            (".gitignore", "git"),
            (".gitattributes", "git"),
        ];

        for (pattern, file_type) in config_patterns {
            if let Ok(found_files) = self.find_files_by_pattern(repo_path, pattern) {
                for file_path in found_files {
                    if let Ok(content) = fs::read_to_string(&file_path) {
                        let relative_path = file_path
                            .strip_prefix(repo_path)
                            .unwrap_or(&file_path)
                            .to_path_buf();

                        let (parsed_deps, scripts) = self.parse_config_file(&content, file_type);

                        config_files.push(ConfigFile {
                            path: relative_path,
                            file_type: file_type.to_string(),
                            content: content.clone(),
                            parsed_dependencies: parsed_deps,
                            scripts,
                        });
                    }
                }
            }
        }

        Ok(config_files)
    }

    fn find_files_by_pattern(&self, repo_path: &Path, pattern: &str) -> Result<Vec<PathBuf>> {
        let mut found_files = Vec::new();

        for entry in WalkDir::new(repo_path)
            .max_depth(3)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                if file_name == pattern || file_name.starts_with(pattern) {
                    found_files.push(path.to_path_buf());
                }
            }
        }

        Ok(found_files)
    }

    fn parse_config_file(
        &self,
        content: &str,
        file_type: &str,
    ) -> (
        Option<HashMap<String, String>>,
        Option<HashMap<String, String>>,
    ) {
        match file_type {
            "npm" => self.parse_package_json(content),
            "cargo" => self.parse_cargo_toml(content),
            "pip" => self.parse_requirements_txt(content),
            "python" => self.parse_pyproject_toml(content),
            _ => (None, None),
        }
    }

    fn parse_package_json(
        &self,
        content: &str,
    ) -> (
        Option<HashMap<String, String>>,
        Option<HashMap<String, String>>,
    ) {
        let json: serde_json::Value = match serde_json::from_str(content) {
            Ok(json) => json,
            Err(_) => return (None, None),
        };

        let mut dependencies = HashMap::new();
        if let Some(deps) = json["dependencies"].as_object() {
            for (name, version) in deps {
                if let Some(ver_str) = version.as_str() {
                    dependencies.insert(name.clone(), ver_str.to_string());
                }
            }
        }
        if let Some(dev_deps) = json["devDependencies"].as_object() {
            for (name, version) in dev_deps {
                if let Some(ver_str) = version.as_str() {
                    dependencies.insert(format!("{} (dev)", name), ver_str.to_string());
                }
            }
        }

        let mut scripts = HashMap::new();
        if let Some(script_obj) = json["scripts"].as_object() {
            for (name, script) in script_obj {
                if let Some(script_str) = script.as_str() {
                    scripts.insert(name.clone(), script_str.to_string());
                }
            }
        }

        (
            if dependencies.is_empty() {
                None
            } else {
                Some(dependencies)
            },
            if scripts.is_empty() {
                None
            } else {
                Some(scripts)
            },
        )
    }

    fn parse_cargo_toml(
        &self,
        content: &str,
    ) -> (
        Option<HashMap<String, String>>,
        Option<HashMap<String, String>>,
    ) {
        let toml: toml::Value = match content.parse() {
            Ok(toml) => toml,
            Err(_) => return (None, None),
        };

        let mut dependencies = HashMap::new();
        if let Some(deps) = toml["dependencies"].as_table() {
            for (name, dep) in deps {
                let version = if let Some(ver_str) = dep.as_str() {
                    ver_str.to_string()
                } else if let Some(dep_table) = dep.as_table() {
                    dep_table
                        .get("version")
                        .and_then(|v| v.as_str())
                        .unwrap_or("*")
                        .to_string()
                } else {
                    "*".to_string()
                };
                dependencies.insert(name.clone(), version);
            }
        }

        (
            if dependencies.is_empty() {
                None
            } else {
                Some(dependencies)
            },
            None,
        )
    }

    fn parse_requirements_txt(
        &self,
        content: &str,
    ) -> (
        Option<HashMap<String, String>>,
        Option<HashMap<String, String>>,
    ) {
        let mut dependencies = HashMap::new();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some((name, version)) = line.split_once("==") {
                dependencies.insert(name.trim().to_string(), version.trim().to_string());
            } else if let Some((name, version)) = line.split_once(">=") {
                dependencies.insert(name.trim().to_string(), format!(">={}", version.trim()));
            } else {
                dependencies.insert(line.to_string(), "*".to_string());
            }
        }

        (
            if dependencies.is_empty() {
                None
            } else {
                Some(dependencies)
            },
            None,
        )
    }

    fn parse_pyproject_toml(
        &self,
        content: &str,
    ) -> (
        Option<HashMap<String, String>>,
        Option<HashMap<String, String>>,
    ) {
        let toml: toml::Value = match content.parse() {
            Ok(toml) => toml,
            Err(_) => return (None, None),
        };

        let mut dependencies = HashMap::new();
        if let Some(project) = toml["project"].as_table() {
            if let Some(deps) = project["dependencies"].as_array() {
                for dep in deps {
                    if let Some(dep_str) = dep.as_str() {
                        if let Some((name, version)) = dep_str.split_once("==") {
                            dependencies
                                .insert(name.trim().to_string(), version.trim().to_string());
                        } else if let Some((name, version)) = dep_str.split_once(">=") {
                            dependencies
                                .insert(name.trim().to_string(), format!(">={}", version.trim()));
                        } else {
                            dependencies.insert(dep_str.to_string(), "*".to_string());
                        }
                    }
                }
            }
        }

        (
            if dependencies.is_empty() {
                None
            } else {
                Some(dependencies)
            },
            None,
        )
    }

    pub fn find_documentation_files(&self, repo_path: &Path) -> Result<Vec<DocumentationFile>> {
        let mut doc_files = Vec::new();

        let doc_patterns = vec![
            ("README", "readme"),
            ("CHANGELOG", "changelog"),
            ("CONTRIBUTING", "contributing"),
            ("LICENSE", "license"),
            ("CODE_OF_CONDUCT", "code_of_conduct"),
            ("SECURITY", "security"),
            ("INSTALL", "install"),
            ("USAGE", "usage"),
            ("API", "api"),
            ("docs/", "documentation"),
        ];

        for (pattern, doc_type) in doc_patterns {
            if let Ok(found_files) = self.find_documentation_by_pattern(repo_path, pattern) {
                for file_path in found_files {
                    if let Ok(content) = fs::read_to_string(&file_path) {
                        let relative_path = file_path
                            .strip_prefix(repo_path)
                            .unwrap_or(&file_path)
                            .to_path_buf();

                        let word_count = content.split_whitespace().count() as u32;
                        let has_badges = content.contains("[![") || content.contains("![");
                        let has_toc = content.to_lowercase().contains("table of contents")
                            || content.contains("## Contents")
                            || content.contains("# Contents");

                        let sections = self.extract_markdown_sections(&content);

                        doc_files.push(DocumentationFile {
                            path: relative_path,
                            file_type: doc_type.to_string(),
                            content,
                            word_count,
                            has_badges,
                            has_toc,
                            sections,
                        });
                    }
                }
            }
        }

        Ok(doc_files)
    }

    fn find_documentation_by_pattern(
        &self,
        repo_path: &Path,
        pattern: &str,
    ) -> Result<Vec<PathBuf>> {
        let mut found_files = Vec::new();

        for entry in WalkDir::new(repo_path)
            .max_depth(3)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                let file_name_upper = file_name.to_uppercase();
                let pattern_upper = pattern.to_uppercase();

                if file_name_upper.starts_with(&pattern_upper)
                    || (pattern.ends_with('/')
                        && path.is_dir()
                        && file_name_upper == pattern_upper.trim_end_matches('/'))
                {
                    found_files.push(path.to_path_buf());
                }
            }
        }

        Ok(found_files)
    }

    fn extract_markdown_sections(&self, content: &str) -> Vec<String> {
        let mut sections = Vec::new();
        let header_regex = Regex::new(r"^#+\s+(.+)$").unwrap();

        for line in content.lines() {
            if let Some(captures) = header_regex.captures(line.trim()) {
                if let Some(section_name) = captures.get(1) {
                    sections.push(section_name.as_str().to_string());
                }
            }
        }

        sections
    }
}

// Code metrics calculator
pub struct CodeMetricsCalculator;

impl CodeMetricsCalculator {
    pub fn calculate_metrics(&self, directory_info: &DirectoryInfo) -> CodeMetrics {
        let mut language_stats: HashMap<String, LanguageStats> = HashMap::new();
        let mut total_files = 0u32;
        let mut total_lines = 0u32;
        let mut total_loc = 0u32;
        let mut total_blank_lines = 0u32;
        let mut total_comment_lines = 0u32;
        let mut total_size = 0u64;
        let mut all_files = Vec::new();

        self.collect_file_stats(directory_info, &mut all_files);

        for file in &all_files {
            if file.is_text {
                total_files += 1;
                total_size += file.size;

                let lines = file.lines_of_code.unwrap_or(0)
                    + file.blank_lines.unwrap_or(0)
                    + file.comment_lines.unwrap_or(0);
                total_lines += lines;
                total_loc += file.lines_of_code.unwrap_or(0);
                total_blank_lines += file.blank_lines.unwrap_or(0);
                total_comment_lines += file.comment_lines.unwrap_or(0);

                if let Some(language) = &file.language {
                    let stats =
                        language_stats
                            .entry(language.clone())
                            .or_insert_with(|| LanguageStats {
                                language: language.clone(),
                                file_count: 0,
                                lines_of_code: 0,
                                blank_lines: 0,
                                comment_lines: 0,
                                total_bytes: 0,
                                percentage: 0.0,
                                complexity_score: None,
                            });

                    stats.file_count += 1;
                    stats.lines_of_code += file.lines_of_code.unwrap_or(0);
                    stats.blank_lines += file.blank_lines.unwrap_or(0);
                    stats.comment_lines += file.comment_lines.unwrap_or(0);
                    stats.total_bytes += file.size;
                }
            }
        }

        // Calculate percentages
        let total_bytes = total_size;
        for stats in language_stats.values_mut() {
            stats.percentage = if total_bytes > 0 {
                (stats.total_bytes as f64 / total_bytes as f64) * 100.0
            } else {
                0.0
            };
        }

        // Find largest files
        let mut largest_files = all_files.clone();
        largest_files.sort_by(|a, b| b.size.cmp(&a.size));
        largest_files.truncate(10);

        // Find most complex files (using LOC as a simple complexity metric)
        let mut most_complex_files = all_files.clone();
        most_complex_files.sort_by(|a, b| {
            let a_complexity = a.lines_of_code.unwrap_or(0);
            let b_complexity = b.lines_of_code.unwrap_or(0);
            b_complexity.cmp(&a_complexity)
        });
        most_complex_files.truncate(10);

        let average_file_size = if total_files > 0 {
            total_size as f64 / total_files as f64
        } else {
            0.0
        };

        CodeMetrics {
            total_files,
            total_lines,
            total_loc,
            total_blank_lines,
            total_comment_lines,
            total_size,
            language_stats,
            average_file_size,
            largest_files,
            most_complex_files,
        }
    }

    fn collect_file_stats(&self, dir: &DirectoryInfo, all_files: &mut Vec<FileInfo>) {
        for file in &dir.files {
            all_files.push(file.clone());
        }

        for subdir in &dir.subdirectories {
            self.collect_file_stats(subdir, all_files);
        }
    }
}

// Project type detector
pub struct ProjectTypeDetector;

impl ProjectTypeDetector {
    pub fn detect_project_info(
        &self,
        config_files: &[ConfigFile],
        file_structure: &DirectoryInfo,
    ) -> ProjectInfo {
        let mut project_types = Vec::new();
        let mut frameworks = Vec::new();
        let mut build_tools = Vec::new();
        let mut package_managers = Vec::new();
        let mut testing_frameworks = Vec::new();
        let mut ci_cd_tools = Vec::new();
        let mut deployment_configs = Vec::new();
        let database_technologies = Vec::new();

        // Analyze config files
        for config in config_files {
            match config.file_type.as_str() {
                "npm" => {
                    package_managers.push("npm".to_string());
                    self.detect_js_frameworks(&config.content, &mut frameworks);
                    self.detect_js_tools(
                        &config.content,
                        &mut build_tools,
                        &mut testing_frameworks,
                    );
                }
                "cargo" => {
                    package_managers.push("cargo".to_string());
                    build_tools.push("cargo".to_string());
                    project_types.push("rust".to_string());
                }
                "pip" => {
                    package_managers.push("pip".to_string());
                    project_types.push("python".to_string());
                }
                "maven" => {
                    package_managers.push("maven".to_string());
                    build_tools.push("maven".to_string());
                    project_types.push("java".to_string());
                }
                "gradle" => {
                    package_managers.push("gradle".to_string());
                    build_tools.push("gradle".to_string());
                    project_types.push("java".to_string());
                }
                "docker" => {
                    deployment_configs.push("docker".to_string());
                }
                "docker-compose" => {
                    deployment_configs.push("docker-compose".to_string());
                }
                "kubernetes" => {
                    deployment_configs.push("kubernetes".to_string());
                }
                "terraform" => {
                    deployment_configs.push("terraform".to_string());
                }
                "github-actions" => {
                    ci_cd_tools.push("github-actions".to_string());
                }
                "travis" => {
                    ci_cd_tools.push("travis-ci".to_string());
                }
                _ => {}
            }
        }

        // Detect primary language from file extensions
        let primary_language = self.detect_primary_language(file_structure);

        // Detect project types based on file structure
        self.detect_project_types_from_structure(file_structure, &mut project_types);

        ProjectInfo {
            primary_language,
            project_type: project_types,
            frameworks,
            build_tools,
            package_managers,
            testing_frameworks,
            ci_cd_tools,
            deployment_configs,
            database_technologies,
        }
    }

    fn detect_js_frameworks(&self, content: &str, frameworks: &mut Vec<String>) {
        let frameworks_to_check = vec![
            ("react", "React"),
            ("vue", "Vue.js"),
            ("angular", "Angular"),
            ("svelte", "Svelte"),
            ("express", "Express.js"),
            ("nestjs", "NestJS"),
            ("next", "Next.js"),
            ("nuxt", "Nuxt.js"),
            ("gatsby", "Gatsby"),
            ("electron", "Electron"),
        ];

        for (dep_name, framework_name) in frameworks_to_check {
            if content.contains(dep_name) {
                frameworks.push(framework_name.to_string());
            }
        }
    }

    fn detect_js_tools(
        &self,
        content: &str,
        build_tools: &mut Vec<String>,
        testing_frameworks: &mut Vec<String>,
    ) {
        let build_tools_to_check = vec![
            ("webpack", "Webpack"),
            ("vite", "Vite"),
            ("rollup", "Rollup"),
            ("parcel", "Parcel"),
            ("esbuild", "ESBuild"),
            ("snowpack", "Snowpack"),
        ];

        let testing_tools_to_check = vec![
            ("jest", "Jest"),
            ("mocha", "Mocha"),
            ("chai", "Chai"),
            ("cypress", "Cypress"),
            ("playwright", "Playwright"),
            ("puppeteer", "Puppeteer"),
            ("jasmine", "Jasmine"),
        ];

        for (tool_name, tool_display) in build_tools_to_check {
            if content.contains(tool_name) {
                build_tools.push(tool_display.to_string());
            }
        }

        for (tool_name, tool_display) in testing_tools_to_check {
            if content.contains(tool_name) {
                testing_frameworks.push(tool_display.to_string());
            }
        }
    }

    fn detect_primary_language(&self, file_structure: &DirectoryInfo) -> Option<String> {
        let mut language_counts: HashMap<String, u32> = HashMap::new();
        self.count_languages(file_structure, &mut language_counts);

        language_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(lang, _)| lang)
    }

    fn count_languages(&self, dir: &DirectoryInfo, language_counts: &mut HashMap<String, u32>) {
        for file in &dir.files {
            if let Some(language) = &file.language {
                *language_counts.entry(language.clone()).or_insert(0) += 1;
            }
        }

        for subdir in &dir.subdirectories {
            self.count_languages(subdir, language_counts);
        }
    }

    fn detect_project_types_from_structure(
        &self,
        file_structure: &DirectoryInfo,
        project_types: &mut Vec<String>,
    ) {
        let mut all_files = Vec::new();
        self.collect_all_files(file_structure, &mut all_files);

        // Check for common project patterns
        let _has_src_dir = self.has_directory(file_structure, "src");
        let _has_lib_dir = self.has_directory(file_structure, "lib");
        let _has_bin_dir = self.has_directory(file_structure, "bin");
        let has_tests_dir = self.has_directory(file_structure, "tests")
            || self.has_directory(file_structure, "test");
        let has_docs_dir = self.has_directory(file_structure, "docs")
            || self.has_directory(file_structure, "documentation");
        let has_examples_dir = self.has_directory(file_structure, "examples");

        // Check for specific file patterns
        let has_main_rs = all_files.iter().any(|f| f.name == "main.rs");
        let has_lib_rs = all_files.iter().any(|f| f.name == "lib.rs");
        let has_index_html = all_files.iter().any(|f| f.name == "index.html");
        let has_server_files = all_files
            .iter()
            .any(|f| f.name.contains("server") || f.name.contains("app"));

        // Determine project types
        if has_main_rs {
            project_types.push("cli-application".to_string());
        }
        if has_lib_rs {
            project_types.push("library".to_string());
        }
        if has_index_html {
            project_types.push("web-application".to_string());
        }
        if has_server_files {
            project_types.push("backend-service".to_string());
        }
        if has_tests_dir {
            project_types.push("tested-project".to_string());
        }
        if has_docs_dir {
            project_types.push("documented-project".to_string());
        }
        if has_examples_dir {
            project_types.push("example-driven".to_string());
        }
    }

    fn has_directory(&self, dir: &DirectoryInfo, name: &str) -> bool {
        dir.subdirectories.iter().any(|d| d.name == name)
    }

    fn collect_all_files(&self, dir: &DirectoryInfo, all_files: &mut Vec<FileInfo>) {
        for file in &dir.files {
            all_files.push(file.clone());
        }

        for subdir in &dir.subdirectories {
            self.collect_all_files(subdir, all_files);
        }
    }
}

// Security analyzer
pub struct SecurityAnalyzer;

impl SecurityAnalyzer {
    pub fn analyze_security(
        &self,
        file_structure: &DirectoryInfo,
        config_files: &[ConfigFile],
    ) -> SecurityInfo {
        let mut has_security_policy = false;
        let mut has_dependabot = false;
        let mut has_codeql = false;
        let vulnerability_alerts = Vec::new(); // Would need external service integration
        let mut outdated_dependencies = Vec::new();
        let license_compatibility = Vec::new();

        // Check for security-related files
        let mut all_files = Vec::new();
        self.collect_all_files(file_structure, &mut all_files);

        for file in &all_files {
            match file.name.to_lowercase().as_str() {
                "security.md" | "security.txt" | ".security" => {
                    has_security_policy = true;
                }
                _ => {}
            }
        }

        // Check for GitHub security features
        if self.has_github_workflow_file(file_structure, "dependabot") {
            has_dependabot = true;
        }

        if self.has_github_workflow_file(file_structure, "codeql") {
            has_codeql = true;
        }

        // Analyze dependencies for potential issues
        for config in config_files {
            if let Some(deps) = &config.parsed_dependencies {
                for (name, version) in deps {
                    // Simple version check (in real implementation, would check against vulnerability databases)
                    if version.contains("*") || version.contains("latest") {
                        outdated_dependencies.push(format!("{}: {}", name, version));
                    }
                }
            }
        }

        SecurityInfo {
            has_security_policy,
            has_dependabot,
            has_codeql,
            vulnerability_alerts,
            outdated_dependencies,
            license_compatibility,
        }
    }

    fn collect_all_files(&self, dir: &DirectoryInfo, all_files: &mut Vec<FileInfo>) {
        for file in &dir.files {
            all_files.push(file.clone());
        }

        for subdir in &dir.subdirectories {
            self.collect_all_files(subdir, all_files);
        }
    }

    fn has_github_workflow_file(&self, file_structure: &DirectoryInfo, keyword: &str) -> bool {
        let github_dir = file_structure
            .subdirectories
            .iter()
            .find(|d| d.name == ".github");

        if let Some(github_dir) = github_dir {
            let workflows_dir = github_dir
                .subdirectories
                .iter()
                .find(|d| d.name == "workflows");

            if let Some(workflows_dir) = workflows_dir {
                return workflows_dir
                    .files
                    .iter()
                    .any(|f| f.name.to_lowercase().contains(keyword));
            }
        }

        false
    }
}

// Main repository analyzer
pub struct RepositoryAnalyzer {
    github_client: GitHubClient,
    git_manager: GitManager,
    fs_analyzer: FileSystemAnalyzer,
    metrics_calculator: CodeMetricsCalculator,
    project_detector: ProjectTypeDetector,
    security_analyzer: SecurityAnalyzer,
}

impl RepositoryAnalyzer {
    pub fn new(github_token: Option<String>, work_dir: Option<PathBuf>) -> Self {
        Self {
            github_client: GitHubClient::new(github_token),
            git_manager: GitManager::new(work_dir),
            fs_analyzer: FileSystemAnalyzer::new(),
            metrics_calculator: CodeMetricsCalculator,
            project_detector: ProjectTypeDetector,
            security_analyzer: SecurityAnalyzer,
        }
    }

    pub async fn analyze_repository(&self, repo_url: &str) -> Result<RepositoryAnalysis> {
        info!("Starting analysis of repository: {}", repo_url);

        // Parse GitHub URL
        let (owner, repo) = parse_github_url(repo_url)?;
        info!("Parsed repository: {}/{}", owner, repo);

        // Fetch repository metadata from GitHub API
        info!("Fetching repository metadata...");
        let metadata = self
            .github_client
            .get_repository_metadata(&owner, &repo)
            .await?;

        // Fetch additional GitHub data
        info!("Fetching contributors...");
        let contributors = self
            .github_client
            .get_contributors(&owner, &repo)
            .await
            .unwrap_or_default();

        info!("Fetching releases...");
        let releases = self
            .github_client
            .get_releases(&owner, &repo, 10)
            .await
            .unwrap_or_default();

        info!("Fetching recent issues...");
        let recent_issues = self
            .github_client
            .get_recent_issues(&owner, &repo, 20)
            .await
            .unwrap_or_default();

        // Clone repository for local analysis
        info!("Cloning repository...");
        let repo_path = self
            .git_manager
            .clone_or_update_repository(&metadata.clone_url, &repo)
            .await?;

        // Analyze Git history
        info!("Analyzing Git history...");
        let mut git_analysis = self.git_manager.analyze_git_history(&repo_path)?;

        // Merge contributors from API with Git analysis
        git_analysis.contributors = contributors;

        // Analyze file structure
        info!("Analyzing file structure...");
        let file_structure = self.fs_analyzer.analyze_directory(&repo_path)?;

        // Calculate code metrics
        info!("Calculating code metrics...");
        let code_metrics = self.metrics_calculator.calculate_metrics(&file_structure);

        // Find and analyze config files
        info!("Analyzing configuration files...");
        let config_files = self.fs_analyzer.find_config_files(&repo_path)?;

        // Find and analyze documentation
        info!("Analyzing documentation...");
        let documentation = self.fs_analyzer.find_documentation_files(&repo_path)?;

        // Detect project information
        info!("Detecting project type and technologies...");
        let project_info = self
            .project_detector
            .detect_project_info(&config_files, &file_structure);

        // Analyze security
        info!("Analyzing security aspects...");
        let security_info = self
            .security_analyzer
            .analyze_security(&file_structure, &config_files);

        // Generate analysis summary
        let analysis_summary =
            self.generate_analysis_summary(&metadata, &code_metrics, &project_info, &git_analysis);

        let analysis = RepositoryAnalysis {
            url: repo_url.to_string(),
            analyzed_at: Utc::now(),
            metadata,
            file_structure,
            code_metrics,
            git_analysis,
            project_info,
            config_files,
            documentation,
            security_info,
            releases,
            recent_issues,
            analysis_summary,
            ai_insights: None, // Can be populated by AI analysis later
        };

        info!("Repository analysis completed successfully!");
        Ok(analysis)
    }

    fn generate_analysis_summary(
        &self,
        metadata: &RepositoryMetadata,
        code_metrics: &CodeMetrics,
        project_info: &ProjectInfo,
        git_analysis: &GitAnalysis,
    ) -> String {
        let mut summary = Vec::new();

        summary.push(format!("Repository: {}", metadata.full_name));
        if let Some(description) = &metadata.description {
            summary.push(format!("Description: {}", description));
        }

        summary.push(format!(
            "Stars: {}, Forks: {}, Open Issues: {}",
            metadata.stargazers_count, metadata.forks_count, metadata.open_issues_count
        ));

        if let Some(primary_lang) = &project_info.primary_language {
            summary.push(format!("Primary Language: {}", primary_lang));
        }

        summary.push(format!(
            "Total Files: {}, Lines of Code: {}, Size: {} KB",
            code_metrics.total_files,
            code_metrics.total_loc,
            code_metrics.total_size / 1024
        ));

        summary.push(format!(
            "Contributors: {}, Total Commits: {}",
            git_analysis.contributors.len(),
            git_analysis.total_commits
        ));

        if !project_info.frameworks.is_empty() {
            summary.push(format!(
                "Frameworks: {}",
                project_info.frameworks.join(", ")
            ));
        }

        if !project_info.project_type.is_empty() {
            summary.push(format!(
                "Project Types: {}",
                project_info.project_type.join(", ")
            ));
        }

        let top_languages: Vec<String> = code_metrics
            .language_stats
            .values()
            .filter(|stats| stats.percentage > 5.0)
            .map(|stats| format!("{} ({:.1}%)", stats.language, stats.percentage))
            .collect();

        if !top_languages.is_empty() {
            summary.push(format!("Languages: {}", top_languages.join(", ")));
        }

        summary.join("\n")
    }

    pub fn export_analysis_json(&self, analysis: &RepositoryAnalysis) -> Result<String> {
        Ok(serde_json::to_string_pretty(analysis)?)
    }

    pub fn export_analysis_yaml(&self, analysis: &RepositoryAnalysis) -> Result<String> {
        Ok(serde_yaml::to_string(analysis)?)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    info!("AI Repository Analyzer starting...");

    // Get command line arguments
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!(
            "Usage: {} <github-repo-url> [--token <github-token>] [--output <json|yaml>] [--output-file <path>]",
            args[0]
        );
        eprintln!("Example: {} https://github.com/owner/repo", args[0]);
        eprintln!(
            "Example: {} https://github.com/owner/repo --token ghp_xxxx --output json --output-file analysis.json",
            args[0]
        );
        std::process::exit(1);
    }

    let repo_url = &args[1];

    // Parse command line options
    let mut github_token = std::env::var("GITHUB_TOKEN").ok();
    let mut output_format = "json".to_string();
    let mut output_file: Option<String> = None;

    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--token" => {
                if i + 1 < args.len() {
                    github_token = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("Error: --token requires a value");
                    std::process::exit(1);
                }
            }
            "--output" => {
                if i + 1 < args.len() {
                    output_format = args[i + 1].clone();
                    i += 2;
                } else {
                    eprintln!("Error: --output requires a value (json or yaml)");
                    std::process::exit(1);
                }
            }
            "--output-file" => {
                if i + 1 < args.len() {
                    output_file = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("Error: --output-file requires a path");
                    std::process::exit(1);
                }
            }
            _ => {
                eprintln!("Unknown option: {}", args[i]);
                std::process::exit(1);
            }
        }
    }

    if github_token.is_none() {
        warn!(
            "No GitHub token provided. API rate limits may apply. Set GITHUB_TOKEN environment variable or use --token option."
        );
    }

    // Create analyzer
    let analyzer = RepositoryAnalyzer::new(github_token, None);

    // Perform analysis
    match analyzer.analyze_repository(repo_url).await {
        Ok(analysis) => {
            info!("Analysis completed successfully!");

            // Export analysis
            let output = match output_format.as_str() {
                "yaml" => analyzer.export_analysis_yaml(&analysis)?,
                "json" | _ => analyzer.export_analysis_json(&analysis)?,
            };

            // Write to file or stdout
            if let Some(file_path) = output_file {
                std::fs::write(&file_path, &output)?;
                info!("Analysis saved to: {}", file_path);
            } else {
                println!("{}", output);
            }

            // Print summary to stderr so it doesn't interfere with output
            eprintln!("\n=== Analysis Summary ===");
            eprintln!("{}", analysis.analysis_summary);
            eprintln!("========================");
        }
        Err(e) => {
            error!("Analysis failed: {}", e);
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}
