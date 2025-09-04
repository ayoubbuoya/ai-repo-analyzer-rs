use std::{collections::HashMap, fs, io::Read, path::{Path, PathBuf}};

use anyhow::Result;
use ignore::WalkBuilder;
use log::{info, warn};
use regex::Regex;
use walkdir::WalkDir;

use crate::types::{ConfigFile, DirectoryInfo, DocumentationFile, FileInfo};

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
