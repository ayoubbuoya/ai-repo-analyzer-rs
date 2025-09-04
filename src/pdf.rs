use crate::types::RepositoryAnalysis;
use anyhow::Result;
use printpdf::*;
use std::fs::File;
use std::io::BufWriter;

pub fn generate_pdf_report(analysis: &RepositoryAnalysis, output_path: &str) -> Result<()> {
    // Create a new PDF document
    let (doc, page1, layer1) = PdfDocument::new(
        format!("AI Repository Analysis Report - {}", analysis.metadata.name),
        Mm(210.0), // A4 width
        Mm(297.0), // A4 height
        "Layer 1"
    );

    let current_layer = doc.get_page(page1).get_layer(layer1);

    // Load a font (we'll use a built-in font for simplicity)
    let font = doc.add_builtin_font(BuiltinFont::Helvetica)?;
    let bold_font = doc.add_builtin_font(BuiltinFont::HelveticaBold)?;

    // Set up text scale
    let normal_scale = 12.0;
    let title_scale = 18.0;
    let header_scale = 14.0;

    // Starting position
    let mut y_position = 280.0; // Start from top
    let left_margin = 20.0;
    let line_height = 5.0;

    // Title
    current_layer.use_text(
        format!("AI Repository Analysis Report"),
        title_scale,
        Mm(left_margin),
        Mm(y_position),
        &bold_font
    );
    y_position -= line_height * 2.0;

    // Repository info
    current_layer.use_text(
        format!("Repository: {}", analysis.metadata.full_name),
        header_scale,
        Mm(left_margin),
        Mm(y_position),
        &bold_font
    );
    y_position -= line_height * 1.5;

    current_layer.use_text(
        format!("Analyzed at: {}", analysis.analyzed_at.format("%Y-%m-%d %H:%M:%S UTC")),
        normal_scale,
        Mm(left_margin),
        Mm(y_position),
        &font
    );
    y_position -= line_height * 1.5;

    // Analysis Summary
    current_layer.use_text(
        "Analysis Summary:",
        header_scale,
        Mm(left_margin),
        Mm(y_position),
        &bold_font
    );
    y_position -= line_height * 1.5;

    // Split summary into lines and add to PDF
    let summary_lines = split_text_into_lines(&analysis.analysis_summary, 80);
    for line in summary_lines {
        if y_position < 20.0 {
            // If we're running out of space, we'd need to add a new page
            // For simplicity, we'll truncate for now
            current_layer.use_text(
                "... (content truncated)",
                normal_scale,
                Mm(left_margin),
                Mm(y_position),
                &font
            );
            break;
        }
        current_layer.use_text(
            line,
            normal_scale,
            Mm(left_margin),
            Mm(y_position),
            &font
        );
        y_position -= line_height;
    }

    // AI Insights (if present)
    if let Some(ai_insights) = &analysis.ai_insights {
        y_position -= line_height * 2.0; // Add some space

        current_layer.use_text(
            "AI-Generated Technical Report:",
            header_scale,
            Mm(left_margin),
            Mm(y_position),
            &bold_font
        );
        y_position -= line_height * 1.5;

        // Split AI insights into lines
        let insights_lines = split_text_into_lines(ai_insights, 80);
        for line in insights_lines {
            if y_position < 20.0 {
                current_layer.use_text(
                    "... (content truncated)",
                    normal_scale,
                    Mm(left_margin),
                    Mm(y_position),
                    &font
                );
                break;
            }
            current_layer.use_text(
                line,
                normal_scale,
                Mm(left_margin),
                Mm(y_position),
                &font
            );
            y_position -= line_height;
        }
    }

    // Save the PDF
    doc.save(&mut BufWriter::new(File::create(output_path)?))?;

    Ok(())
}

fn split_text_into_lines(text: &str, max_chars_per_line: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.len() + word.len() + 1 > max_chars_per_line {
            if !current_line.is_empty() {
                lines.push(current_line);
                current_line = String::new();
            }
            // If a single word is too long, split it
            if word.len() > max_chars_per_line {
                let mut word_chars = word.chars().collect::<Vec<_>>();
                while !word_chars.is_empty() {
                    let take = std::cmp::min(max_chars_per_line, word_chars.len());
                    let chunk: String = word_chars.drain(0..take).collect();
                    lines.push(chunk);
                }
            } else {
                current_line = word.to_string();
            }
        } else {
            if !current_line.is_empty() {
                current_line.push(' ');
            }
            current_line.push_str(word);
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    lines
}
