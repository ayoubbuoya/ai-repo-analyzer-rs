use crate::types::RepositoryAnalysis;
use anyhow::Result;
use printpdf::*;
use pulldown_cmark::{Parser, Event, Tag};
use std::fs::File;
use std::io::BufWriter;

const PAGE_WIDTH_MM: f32 = 210.0;
const PAGE_HEIGHT_MM: f32 = 297.0;
const LEFT_MARGIN_MM: f32 = 15.0;
const RIGHT_MARGIN_MM: f32 = 15.0;
const TOP_MARGIN_MM: f32 = 20.0;
const BOTTOM_MARGIN_MM: f32 = 20.0;

pub fn generate_pdf_report(analysis: &RepositoryAnalysis, output_path: &str) -> Result<()> {
    // Create document
    let (mut doc, mut page, mut layer) = PdfDocument::new(
        format!("AI Repository Analysis - {}", analysis.metadata.name),
        Mm(PAGE_WIDTH_MM),
        Mm(PAGE_HEIGHT_MM),
        "Layer 1",
    );

    // Fonts
    let regular = doc.add_builtin_font(BuiltinFont::Helvetica)?;
    let bold = doc.add_builtin_font(BuiltinFont::HelveticaBold)?;

    // Layout state
    let mut cursor_y: f32 = PAGE_HEIGHT_MM - TOP_MARGIN_MM; // mm from bottom in printpdf uses Mm(y)
    let line_height: f32 = 6.0; // mm
    // let max_width_mm: f32 = PAGE_WIDTH_MM - LEFT_MARGIN_MM - RIGHT_MARGIN_MM;

    // Helper to ensure space and create new page if needed
    fn add_page_if_needed(doc: &mut printpdf::PdfDocumentReference, page: &mut printpdf::PdfPageIndex, layer: &mut printpdf::PdfLayerIndex, cursor_y: &mut f32, needed: f32) {
        if *cursor_y - needed < BOTTOM_MARGIN_MM {
            let (new_page, new_layer) = doc.add_page(Mm(PAGE_WIDTH_MM), Mm(PAGE_HEIGHT_MM), "Layer");
            *page = new_page;
            *layer = new_layer;
            *cursor_y = PAGE_HEIGHT_MM - TOP_MARGIN_MM;
        }
    }

    // Title
    add_page_if_needed(&mut doc, &mut page, &mut layer, &mut cursor_y, 10.0);
    doc.get_page(page).get_layer(layer).use_text(
        format!("AI Repository Analysis Report"),
        18.0,
        Mm(LEFT_MARGIN_MM),
        Mm(cursor_y),
        &bold,
    );
    cursor_y -= line_height * 1.5;

    // Repo info
    doc.get_page(page).get_layer(layer).use_text(
        format!("Repository: {}", analysis.metadata.full_name),
        12.0,
        Mm(LEFT_MARGIN_MM),
        Mm(cursor_y),
        &bold,
    );
    cursor_y -= line_height;

    doc.get_page(page).get_layer(layer).use_text(
        format!("Analyzed at: {}", analysis.analyzed_at.format("%Y-%m-%d %H:%M:%S UTC")),
        10.0,
        Mm(LEFT_MARGIN_MM),
        Mm(cursor_y),
        &regular,
    );
    cursor_y -= line_height * 1.5;

    // Analysis summary
    doc.get_page(page).get_layer(layer).use_text("Analysis Summary:", 12.0, Mm(LEFT_MARGIN_MM), Mm(cursor_y), &bold);
    cursor_y -= line_height;
    for line in split_text_into_lines(&analysis.analysis_summary, 90) {
        add_page_if_needed(&mut doc, &mut page, &mut layer, &mut cursor_y, line_height);
        doc.get_page(page).get_layer(layer).use_text(&line, 10.0, Mm(LEFT_MARGIN_MM), Mm(cursor_y), &regular);
        cursor_y -= line_height;
    }

    // AI insights: render markdown
    if let Some(ai) = &analysis.ai_insights {
    cursor_y -= line_height; // spacing
    add_page_if_needed(&mut doc, &mut page, &mut layer, &mut cursor_y, line_height);
    doc.get_page(page).get_layer(layer).use_text("AI-Generated Technical Report:", 12.0, Mm(LEFT_MARGIN_MM), Mm(cursor_y), &bold);
    cursor_y -= line_height;

        // Parse markdown and render basic elements
        let parser = Parser::new(ai);
        let mut list_indent = 0usize;

        for event in parser {
            match event {
                Event::Start(tag) => match tag {
                    Tag::Heading(..) => {
                        // reserve space for heading
                        add_page_if_needed(&mut doc, &mut page, &mut layer, &mut cursor_y, line_height * 1.4);
                        // next text events will be used as heading content; handled in Text event
                    }
                    Tag::List(_) => {
                        list_indent += 1;
                    }
                    Tag::CodeBlock(_) => {
                        add_page_if_needed(&mut doc, &mut page, &mut layer, &mut cursor_y, line_height);
                    }
                    _ => {}
                },
                Event::End(tag) => match tag {
                    Tag::List(_) => {
                        if list_indent > 0 { list_indent -= 1; }
                        cursor_y -= line_height * 0.2;
                    }
                    _ => {}
                },
                Event::Text(text) => {
                    // For simplicity, treat text as paragraphs; wrap and render
                    for line in split_text_into_lines(&text, 90 - list_indent * 4) {
                        add_page_if_needed(&mut doc, &mut page, &mut layer, &mut cursor_y, line_height);
                        // indent for list items
                        let indent_x = LEFT_MARGIN_MM + (list_indent as f32) * 5.0;
                        doc.get_page(page).get_layer(layer).use_text(&line, 10.0, Mm(indent_x), Mm(cursor_y), &regular);
                        cursor_y -= line_height;
                    }
                }
                Event::Code(code) => {
                    // inline code: render with monospace sized box
                    add_page_if_needed(&mut doc, &mut page, &mut layer, &mut cursor_y, line_height);
                    doc.get_page(page).get_layer(layer).use_text(&format!("`{}`", code), 10.0, Mm(LEFT_MARGIN_MM), Mm(cursor_y), &regular);
                    cursor_y -= line_height;
                }
                Event::Html(_) | Event::FootnoteReference(_) | Event::SoftBreak => {
                    // treat as space
                    cursor_y -= line_height * 0.2;
                }
                Event::HardBreak => {
                    cursor_y -= line_height;
                }
                _ => {}
            }
        }
    }

    // Save
    doc.save(&mut BufWriter::new(File::create(output_path)?))?;

    Ok(())
}

fn split_text_into_lines(text: &str, max_chars_per_line: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.len() + word.len() + 1 > max_chars_per_line {
            if !current.is_empty() { lines.push(current.clone()); current.clear(); }
            if word.len() > max_chars_per_line {
                // split long word
                let mut w = word;
                while w.len() > max_chars_per_line {
                    let part = &w[0..max_chars_per_line];
                    lines.push(part.to_string());
                    w = &w[max_chars_per_line..];
                }
                if !w.is_empty() { current.push_str(w); }
            } else {
                current.push_str(word);
            }
        } else {
            if !current.is_empty() { current.push(' '); }
            current.push_str(word);
        }
    }
    if !current.is_empty() { lines.push(current); }
    lines
}
