use std::collections::HashMap;

use crate::types::ConfigFile;
use crate::types::DirectoryInfo;
use crate::types::FileInfo;
use crate::types::ProjectInfo;

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
