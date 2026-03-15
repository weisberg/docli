use std::collections::HashSet;
use std::path::PathBuf;

use clap::Args;
use docli_core::Package;
use docli_query::DocumentIndex;
use serde::Serialize;

#[derive(Clone, Debug, Args)]
pub struct InspectArgs {
    pub source: PathBuf,
    #[arg(long = "sections", value_delimiter = ',')]
    pub sections: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct InspectData {
    pub source: String,
    pub sections: Vec<String>,
    pub index: DocumentIndex,
}

pub fn run(args: &InspectArgs) -> Result<InspectData, docli_core::DocliError> {
    let package = Package::open(&args.source)?;
    let document_xml = package.xml_parts.get("word/document.xml").ok_or_else(|| {
        docli_core::DocliError::InvalidDocx {
            message: "missing word/document.xml".to_string(),
        }
    })?;
    let index = DocumentIndex::build(document_xml)?;

    let index = apply_section_filter(index, &args.sections);
    let sections = normalize_sections(&args.sections);

    Ok(InspectData {
        source: args.source.display().to_string(),
        sections,
        index,
    })
}

fn apply_section_filter(mut index: DocumentIndex, sections: &[String]) -> DocumentIndex {
    if sections.is_empty() {
        return index;
    }

    let selected: HashSet<String> = sections.iter().map(|section| section.to_lowercase()).collect();

    if !selected.contains("paragraphs") {
        index.paragraphs.clear();
    }
    if !selected.contains("tables") {
        index.tables.clear();
    }
    if !selected.contains("images") {
        index.images.clear();
    }
    if !selected.contains("headings") {
        index.headings.clear();
    }
    if !selected.contains("bookmarks") {
        index.bookmarks.clear();
    }
    if !selected.contains("comments") {
        index.comments = Default::default();
    }
    if !selected.contains("tracked-changes") {
        index.tracked_changes = Default::default();
    }

    index
}

fn normalize_sections(sections: &[String]) -> Vec<String> {
    let mut normalized = sections
        .iter()
        .map(|section| section.to_lowercase())
        .collect::<Vec<_>>();
    normalized.sort();
    normalized
}
