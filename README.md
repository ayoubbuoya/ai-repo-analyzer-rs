# ai-repo-analyzer-rs

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
