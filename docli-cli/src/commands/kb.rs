use std::path::PathBuf;

use clap::{Args, Subcommand};
use docli_kb::{KbResolver};
use serde::Serialize;

#[derive(Debug, Args)]
pub struct KbArgs {
    #[command(subcommand)]
    pub command: KbCommand,
}

#[derive(Debug, Subcommand)]
pub enum KbCommand {
    List(KbListArgs),
    Resolve(KbResolveArgs),
    Validate(KbValidateArgs),
}

#[derive(Debug, Args)]
pub struct KbListArgs {
    pub category: String,
}

#[derive(Debug, Args)]
pub struct KbResolveArgs {
    pub uri: String,
}

#[derive(Debug, Args)]
pub struct KbValidateArgs {}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum KbData {
    List {
        category: String,
        entries: Vec<String>,
    },
    Resolve {
        uri: String,
        path: String,
    },
    Validate {
        valid: bool,
        issues: Vec<String>,
    },
}

pub fn run(
    args: &KbArgs,
    kb_path: &PathBuf,
) -> Result<KbData, docli_core::DocliError> {
    let resolver = KbResolver::new(kb_path);
    match &args.command {
        KbCommand::List(list_args) => Ok(KbData::List {
            category: list_args.category.clone(),
            entries: resolver.list_entries(&list_args.category)?,
        }),
        KbCommand::Resolve(resolve_args) => Ok(KbData::Resolve {
            uri: resolve_args.uri.clone(),
            path: resolver.resolve(&resolve_args.uri)?.display().to_string(),
        }),
        KbCommand::Validate(_validate_args) => {
            let issues = resolver
                .validate_all()
                .into_iter()
                .map(|(uri, message)| format!("{uri}: {message}"))
                .collect::<Vec<_>>();
            Ok(KbData::Validate {
                valid: issues.is_empty(),
                issues,
            })
        }
    }
}
