use std::path::PathBuf;

use clap::{Args, ValueEnum};
use docli_core::Package;
use docli_query::DocumentIndex;
use serde::Serialize;

#[derive(Debug, Args)]
pub struct ReadArgs {
    pub source: PathBuf,
    #[arg(long, default_value = "markdown", value_enum)]
    pub as_format: ReadFormat,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum ReadFormat {
    Json,
    Text,
    Markdown,
}

#[derive(Debug, Serialize)]
pub struct ReadData {
    pub source: String,
    pub format: String,
    pub content: String,
}

pub fn run(args: &ReadArgs) -> Result<ReadData, docli_core::DocliError> {
    let package = Package::open(&args.source)?;
    let document_xml = package.xml_parts.get("word/document.xml").ok_or_else(|| {
        docli_core::DocliError::InvalidDocx {
            message: "missing word/document.xml".to_string(),
        }
    })?;
    let index = DocumentIndex::build(document_xml)?;

    let content = match args.as_format {
        ReadFormat::Json => serde_json::to_string_pretty(&index)
            .map_err(|error| docli_core::DocliError::InvalidDocx {
                message: error.to_string(),
            })?,
        ReadFormat::Text => format_text(&index),
        ReadFormat::Markdown => format_markdown(&index),
    };

    Ok(ReadData {
        source: args.source.display().to_string(),
        format: format_name(args.as_format),
        content,
    })
}

fn format_name(format: ReadFormat) -> &'static str {
    match format {
        ReadFormat::Json => "json",
        ReadFormat::Text => "text",
        ReadFormat::Markdown => "markdown",
    }
}

fn format_text(index: &DocumentIndex) -> String {
    let mut output = String::new();
    output.push_str(&format!("Paragraphs: {}\n", index.paragraphs.len()));
    output.push_str(&format!("Tables: {}\n", index.tables.len()));
    output.push_str(&format!("Images: {}\n", index.images.len()));
    output.push_str("Headings:\n");
    for heading in &index.headings {
        output.push_str(&format!(
            "  - [{}] {}\n",
            heading.level, heading.text
        ));
    }
    output
}

fn format_markdown(index: &DocumentIndex) -> String {
    let mut output = String::new();
    output.push_str("# Document Read View\n\n");
    for paragraph in &index.paragraphs {
        if let Some(level) = heading_level(paragraph.text.as_str()) {
            output.push_str(&format!("{} {}\n\n", "#".repeat(level as usize), paragraph.text));
        } else {
            output.push_str(&format!("{}\n\n", paragraph.text));
        }
    }
    output
}

fn heading_level(text: &str) -> Option<u8> {
    if text.to_ascii_lowercase().contains("heading") {
        Some(2)
    } else {
        None
    }
}
