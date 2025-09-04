use std::collections::HashMap;

use crate::types::CodeMetrics;
use crate::types::DirectoryInfo;
use crate::types::FileInfo;
use crate::types::LanguageStats;

// Code metrics calculator
pub struct CodeMetricsCalculator;

impl CodeMetricsCalculator {
    pub fn calculate_metrics(&self, directory_info: &DirectoryInfo) -> CodeMetrics {
        let mut language_stats: HashMap<String, LanguageStats> = HashMap::new();
        let mut total_files = 0u32;
        let mut total_lines = 0u32;
        let mut total_loc = 0u32;
        let mut total_blank_lines = 0u32;
        let mut total_comment_lines = 0u32;
        let mut total_size = 0u64;
        let mut all_files = Vec::new();

        self.collect_file_stats(directory_info, &mut all_files);

        for file in &all_files {
            if file.is_text {
                total_files += 1;
                total_size += file.size;

                let lines = file.lines_of_code.unwrap_or(0)
                    + file.blank_lines.unwrap_or(0)
                    + file.comment_lines.unwrap_or(0);
                total_lines += lines;
                total_loc += file.lines_of_code.unwrap_or(0);
                total_blank_lines += file.blank_lines.unwrap_or(0);
                total_comment_lines += file.comment_lines.unwrap_or(0);

                if let Some(language) = &file.language {
                    let stats =
                        language_stats
                            .entry(language.clone())
                            .or_insert_with(|| LanguageStats {
                                language: language.clone(),
                                file_count: 0,
                                lines_of_code: 0,
                                blank_lines: 0,
                                comment_lines: 0,
                                total_bytes: 0,
                                percentage: 0.0,
                                complexity_score: None,
                            });

                    stats.file_count += 1;
                    stats.lines_of_code += file.lines_of_code.unwrap_or(0);
                    stats.blank_lines += file.blank_lines.unwrap_or(0);
                    stats.comment_lines += file.comment_lines.unwrap_or(0);
                    stats.total_bytes += file.size;
                }
            }
        }

        // Calculate percentages
        let total_bytes = total_size;
        for stats in language_stats.values_mut() {
            stats.percentage = if total_bytes > 0 {
                (stats.total_bytes as f64 / total_bytes as f64) * 100.0
            } else {
                0.0
            };
        }

        // Find largest files
        let mut largest_files = all_files.clone();
        largest_files.sort_by(|a, b| b.size.cmp(&a.size));
        largest_files.truncate(10);

        // Find most complex files (using LOC as a simple complexity metric)
        let mut most_complex_files = all_files.clone();
        most_complex_files.sort_by(|a, b| {
            let a_complexity = a.lines_of_code.unwrap_or(0);
            let b_complexity = b.lines_of_code.unwrap_or(0);
            b_complexity.cmp(&a_complexity)
        });
        most_complex_files.truncate(10);

        let average_file_size = if total_files > 0 {
            total_size as f64 / total_files as f64
        } else {
            0.0
        };

        CodeMetrics {
            total_files,
            total_lines,
            total_loc,
            total_blank_lines,
            total_comment_lines,
            total_size,
            language_stats,
            average_file_size,
            largest_files,
            most_complex_files,
        }
    }

    fn collect_file_stats(&self, dir: &DirectoryInfo, all_files: &mut Vec<FileInfo>) {
        for file in &dir.files {
            all_files.push(file.clone());
        }

        for subdir in &dir.subdirectories {
            self.collect_file_stats(subdir, all_files);
        }
    }
}
