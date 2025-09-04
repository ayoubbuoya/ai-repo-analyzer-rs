use crate::types::{ConfigFile, DirectoryInfo, FileInfo, SecurityInfo};

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
