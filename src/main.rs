mod analyzers;
mod git;
mod github;
mod types;
mod utils;

use anyhow::Result;
use log::{error, info, warn};

use crate::{analyzers::repo::RepositoryAnalyzer, types::RepositoryMetadata};

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
