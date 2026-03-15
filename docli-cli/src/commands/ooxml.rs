use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use clap::{Args, Subcommand};
use roxmltree::Document;
use serde::Serialize;
use zip::{read::ZipArchive, write::SimpleFileOptions, ZipWriter};

#[derive(Debug, Args)]
pub struct OoxmlArgs {
    #[command(subcommand)]
    pub command: OoxmlCommand,
}

#[derive(Debug, Subcommand)]
pub enum OoxmlCommand {
    Unpack(UnpackArgs),
    Pack(PackArgs),
    Query(QueryArgs),
}

#[derive(Debug, Args)]
pub struct UnpackArgs {
    pub source: PathBuf,
    pub output_dir: PathBuf,
}

#[derive(Debug, Args)]
pub struct PackArgs {
    pub source_dir: PathBuf,
    pub output: PathBuf,
}

#[derive(Debug, Args)]
pub struct QueryArgs {
    pub source: PathBuf,
    pub pattern: String,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum OoxmlData {
    Unpack {
        source: String,
        output_dir: String,
        files_written: usize,
    },
    Pack {
        source_dir: String,
        output: String,
        files_written: usize,
    },
    Query {
        source: String,
        pattern: String,
        matches: usize,
    },
}

pub fn run(args: &OoxmlArgs) -> Result<OoxmlData, docli_core::DocliError> {
    match &args.command {
        OoxmlCommand::Unpack(inner) => {
            let files_written = unpack_source(&inner.source, &inner.output_dir)?;
            Ok(OoxmlData::Unpack {
                source: inner.source.display().to_string(),
                output_dir: inner.output_dir.display().to_string(),
                files_written,
            })
        }
        OoxmlCommand::Pack(inner) => {
            let files_written = pack_output(&inner.source_dir, &inner.output)?;
            Ok(OoxmlData::Pack {
                source_dir: inner.source_dir.display().to_string(),
                output: inner.output.display().to_string(),
                files_written,
            })
        }
        OoxmlCommand::Query(inner) => {
            let matches = query_ooxml(&inner.source, &inner.pattern)?;
            Ok(OoxmlData::Query {
                source: inner.source.display().to_string(),
                pattern: inner.pattern.clone(),
                matches,
            })
        }
    }
}

fn unpack_source(source: &Path, output_dir: &Path) -> Result<usize, docli_core::DocliError> {
    let source_file = File::open(source)?;
    let mut archive = ZipArchive::new(source_file).map_err(|error| {
        docli_core::DocliError::InvalidDocx {
            message: error.to_string(),
        }
    })?;
    fs::create_dir_all(output_dir)?;

    let mut count = 0;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let out_path = output_dir.join(entry.name());
        if entry.is_dir() {
            fs::create_dir_all(&out_path)?;
            continue;
        }
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut output_file = File::create(&out_path)?;
        io::copy(&mut entry, &mut output_file)?;
        count += 1;
    }
    Ok(count)
}

fn pack_output(source_dir: &Path, output: &Path) -> Result<usize, docli_core::DocliError> {
    let output_file = File::create(output)?;
    let mut writer = ZipWriter::new(output_file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    let mut count = 0;

    let mut stack = vec![source_dir.to_path_buf()];
    while let Some(directory) = stack.pop() {
        for entry in fs::read_dir(&directory).map_err(|error| docli_core::DocliError::InvalidSpec {
            message: error.to_string(),
        })? {
            let entry = entry.map_err(|error| docli_core::DocliError::InvalidSpec {
                message: error.to_string(),
            })?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let relative = path
                .strip_prefix(source_dir)
                .map_err(|error| docli_core::DocliError::InvalidSpec {
                    message: error.to_string(),
                })?
                .to_string_lossy()
                .replace('\\', "/");
            let mut entry_file = File::open(&path)?;
            let mut bytes = Vec::new();
            entry_file.read_to_end(&mut bytes)?;
            writer
                .start_file(relative, options)
                .map_err(|error| docli_core::DocliError::CommitFailed {
                    message: error.to_string(),
                })?;
            writer.write_all(&bytes)?;
            count += 1;
        }
    }
    writer.finish()?;
    Ok(count)
}

fn query_ooxml(source: &Path, pattern: &str) -> Result<usize, docli_core::DocliError> {
    let mut input = String::new();
    File::open(source)
        .map_err(|error| docli_core::DocliError::InvalidSpec {
            message: error.to_string(),
        })?
        .read_to_string(&mut input)
        .map_err(|error| docli_core::DocliError::InvalidSpec {
            message: error.to_string(),
        })?;

    let document = Document::parse(&input).map_err(|error| docli_core::DocliError::InvalidDocx {
        message: error.to_string(),
    })?;
    let matches = document
        .descendants()
        .filter(|node| node.is_element() && node.tag_name().name() == pattern)
        .count();
    Ok(matches)
}
