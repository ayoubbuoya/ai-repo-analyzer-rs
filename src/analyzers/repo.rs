use std::path::PathBuf;

use anyhow::Result;
use chrono::Utc;
use log::info;

use crate::{
    analyzers::{
        code_metrics::CodeMetricsCalculator, filesystem::FileSystemAnalyzer,
        security::SecurityAnalyzer, type_detector::ProjectTypeDetector,
    },
    git::GitManager,
    github::GitHubClient,
    types::{CodeMetrics, GitAnalysis, ProjectInfo, RepositoryAnalysis, RepositoryMetadata},
    utils::parse_github_url,
};

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
