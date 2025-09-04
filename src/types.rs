use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

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
