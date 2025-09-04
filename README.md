# AI Repository Analyzer

A comprehensive Rust-based tool for analyzing GitHub repositories to extract detailed information for AI agent analysis.

## Features

This tool provides comprehensive repository analysis including:

### Repository Metadata
- GitHub API data (stars, forks, issues, contributors)
- Repository description, topics, and license information
- Release history and recent activity
- Programming language distribution

### Code Analysis
- File structure analysis with detailed metrics
- Lines of code, comments, and blank lines counting
- Language detection and statistics
- File size analysis and complexity metrics
- Binary vs text file classification

### Project Intelligence
- Framework and technology detection (React, Vue, Express, etc.)
- Build tool identification (Webpack, Vite, Cargo, Maven, etc.)
- Package manager detection (npm, cargo, pip, etc.)
- Testing framework identification
- CI/CD pipeline detection

### Configuration Analysis
- Package.json, Cargo.toml, requirements.txt parsing
- Dependency extraction and version analysis
- Script and build command identification
- Docker and deployment configuration detection

### Documentation Analysis
- README, CHANGELOG, LICENSE file analysis
- Documentation structure and quality metrics
- Badge and table of contents detection
- Word count and section analysis

### Security Assessment
- Security policy detection
- Dependabot and CodeQL integration status
- Dependency vulnerability assessment
- License compatibility analysis

### Git History Analysis
- Commit frequency and contributor activity
- Most active files and modification patterns
- Branch and tag counting
- Repository timeline analysis

## Installation

1. Clone this repository:
```bash
git clone https://github.com/ayoubbuoya/ai-repo-analyzer-rs.git
cd ai-repo-analyzer-rs
```

2. Build the project:
```bash
cargo build --release
```

## Usage

### Basic Usage

```bash
# Analyze a repository
./target/release/ai-repo-analyzer-rs https://github.com/owner/repo

# With GitHub token for higher API limits
./target/release/ai-repo-analyzer-rs https://github.com/owner/repo --token ghp_your_token_here

# Export to JSON file
./target/release/ai-repo-analyzer-rs https://github.com/owner/repo --output json --output-file analysis.json

# Export to YAML file
./target/release/ai-repo-analyzer-rs https://github.com/owner/repo --output yaml --output-file analysis.yaml
```

### Environment Variables

You can set your GitHub token as an environment variable:

```bash
export GITHUB_TOKEN=ghp_your_token_here
./target/release/ai-repo-analyzer-rs https://github.com/owner/repo
```

### Command Line Options

- `--token <token>`: GitHub personal access token for API authentication
- `--output <format>`: Output format (json or yaml, default: json)
- `--output-file <path>`: Save analysis to file instead of stdout

## Output Format

The analyzer produces a comprehensive JSON/YAML output containing:

```json
{
  "url": "https://github.com/owner/repo",
  "analyzed_at": "2025-09-03T22:00:00Z",
  "metadata": {
    "name": "repo",
    "description": "Repository description",
    "stargazers_count": 1234,
    "forks_count": 56,
    "language": "Rust",
    "topics": ["rust", "cli", "analysis"],
    // ... more metadata
  },
  "file_structure": {
    "path": "./",
    "name": "repo",
    "file_count": 42,
    "total_size": 1024000,
    "files": [
      {
        "path": "src/main.rs",
        "size": 2048,
        "lines_of_code": 150,
        "language": "Rust",
        // ... more file details
      }
    ]
  },
  "code_metrics": {
    "total_files": 42,
    "total_loc": 5000,
    "language_stats": {
      "Rust": {
        "file_count": 20,
        "lines_of_code": 4000,
        "percentage": 80.0
      }
    }
  },
  "project_info": {
    "primary_language": "Rust",
    "project_type": ["cli-application", "library"],
    "frameworks": ["tokio", "serde"],
    "build_tools": ["cargo"]
  },
  "git_analysis": {
    "total_commits": 150,
    "contributors": [...],
    "recent_commits": [...],
    "commit_frequency": {...}
  },
  "security_info": {
    "has_security_policy": true,
    "has_dependabot": true,
    "vulnerability_alerts": []
  },
  "documentation": [...],
  "config_files": [...],
  "analysis_summary": "Comprehensive text summary of the analysis"
}
```

## Use Cases for AI Agents

This tool is designed to provide AI agents with comprehensive repository context for various tasks:

### Code Review and Analysis
- Code quality assessment
- Architecture review
- Best practices compliance
- Security vulnerability analysis

### Project Understanding
- Technology stack identification
- Dependency analysis
- Build and deployment process understanding
- Testing strategy assessment

### Documentation Generation
- README improvement suggestions
- API documentation generation
- Architecture documentation creation
- Changelog generation

### Maintenance and Optimization
- Dependency update recommendations
- Performance optimization suggestions
- Code refactoring opportunities
- Technical debt identification

### Compliance and Security
- License compliance checking
- Security policy assessment
- Vulnerability scanning
- Audit trail generation

## GitHub Token Setup

For higher API rate limits and access to private repositories, set up a GitHub Personal Access Token:

1. Go to GitHub Settings → Developer settings → Personal access tokens
2. Generate a new token with `repo` scope for private repos or `public_repo` for public repos
3. Use the token with the `--token` flag or `GITHUB_TOKEN` environment variable

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- Built with Rust 🦀
- Uses the GitHub API for repository metadata
- Leverages git2 for Git history analysis
- Uses various parsing libraries for configuration file analysis

AI Repo Analyzer (ai-repo-analyzer-rs) is a Rust-based AI agent that helps developers
explore and understand GitHub repositories efficiently.

Features:

- Clones and parses source files from a repository
- Splits large files into manageable chunks with semantic context
- Generates embeddings for each chunk and stores them in Qdrant
- Supports natural-language queries over the repository
- Retrieves, re-ranks, and stitches relevant code chunks
- Provides clear, human-readable explanations of code

Tech Stack:

- Rust
- Rig (AI agent orchestration)
- Qdrant (vector database)
