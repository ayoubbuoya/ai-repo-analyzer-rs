use anyhow::Result;
use chrono::{DateTime, Utc};
use git2::Repository;
use log::{info, warn};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::types::{GitAnalysis, GitHubCommit, GitHubUser};

/// Git repository manager for cloning and analyzing repositories
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
