mod analyzers;
mod git;
mod github;
mod pdf;
mod types;
mod utils;

use anyhow::Result;
use log::{error, info, warn};
use rig::{client::ProviderClient, completion::Prompt, providers::gemini};

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
            "Usage: {} <github-repo-url> [--token <github-token>] [--output <json|yaml>] [--output-file <path>] [--pdf-output <path>]",
            args[0]
        );
        eprintln!("Example: {} https://github.com/owner/repo", args[0]);
        eprintln!(
            "Example: {} https://github.com/owner/repo --token ghp_xxxx --output json --output-file analysis.json --pdf-output report.pdf",
            args[0]
        );
        std::process::exit(1);
    }

    let repo_url = &args[1];

    // Parse command line options
    let mut github_token = std::env::var("GITHUB_TOKEN").ok();
    let mut output_format = "json".to_string();
    let mut output_file: Option<String> = None;
    let mut pdf_output_file: Option<String> = None;

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
            "--pdf-output" => {
                if i + 1 < args.len() {
                    pdf_output_file = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("Error: --pdf-output requires a path");
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

    // Initialize a gemini AI agent using rig core
    let ai_client = gemini::Client::from_env();
    let ai_agent = ai_client
        .agent("gemini-2.5-flash").temperature(0.0)
        .preamble("You are an expert software engineer and technical analyst specializing in code repository analysis. You will be provided with detailed analysis data about a GitHub repository in JSON format.

Your task is to generate a comprehensive technical development report that includes:

## Executive Summary
- Brief overview of the project's purpose and main functionality
- Key technologies and architecture highlights
- Current development status and maturity level

## Technical Architecture
- Primary programming languages and their usage distribution
- Framework and library ecosystem
- Project structure and organization patterns
- Build system and deployment configurations

## Code Quality Assessment
- Code metrics analysis (lines of code, complexity, file organization, code quality, duplication, following best practices)
- Security considerations and potential vulnerabilities
- Documentation completeness and quality
- Testing coverage and framework usage

## Development Activity
- Git history analysis (commit frequency, contributor engagement)
- Recent development trends and focus areas
- Release management and versioning strategy

## Strengths and Opportunities
- Key strengths of the codebase
- Potential areas for improvement
- Technical debt assessment
- Recommendations for future development

## Risk Assessment
- Security vulnerabilities or concerns
- Outdated dependencies or compatibility issues
- Maintenance challenges or scalability concerns

Provide your analysis in a clear, professional format with specific examples from the data when relevant. Be concise but thorough, focusing on actionable insights that would help developers understand and improve the project.")
        .build();

    // Perform analysis
    match analyzer.analyze_repository(repo_url).await {
        Ok(mut analysis) => {
            info!("Analysis completed successfully!");

            // Generate AI-powered technical report
            info!("Generating AI-powered technical report...");
            match serde_json::to_string_pretty(&analysis) {
                Ok(analysis_json) => {
                    match ai_agent.prompt(&format!("Please analyze this repository data and generate a comprehensive technical report:\n\n{}", analysis_json)).await {
                        Ok(response) => {
                            analysis.ai_insights = Some(response);
                            info!("AI report generated successfully!");

                            // Generate PDF report when AI insights are present
                            let pdf_path = pdf_output_file.unwrap_or_else(|| {
                                // Generate default PDF filename based on repository name
                                let repo_name = analysis.metadata.name.replace("/", "_");
                                format!("{}_analysis_report.pdf", repo_name)
                            });

                            match crate::pdf::generate_pdf_report(&analysis, &pdf_path) {
                                Ok(_) => info!("PDF report generated successfully: {}", pdf_path),
                                Err(e) => warn!("Failed to generate PDF report: {}", e),
                            }
                        }
                        Err(e) => {
                            warn!("Failed to generate AI report: {}. Proceeding with standard analysis.", e);
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to serialize analysis for AI: {}. Proceeding with standard analysis.", e);
                }
            }

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
