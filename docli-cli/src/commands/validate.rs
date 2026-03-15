use std::path::PathBuf;

use clap::Args;
use docli_core::{
    run_shadow_pipeline, Durability, PipelineHooks, PipelineRequest,
};
use docli_core::{Package, DocliError};
use docli_schema::{
    check_invariants, repair_durable_id_overflow, ensure_xml_space_preserve, validate_redlines,
    validate_structure,
};
use serde::Serialize;

#[derive(Clone, Debug, Args)]
pub struct ValidateArgs {
    pub source: PathBuf,
    #[arg(long, default_value_t = false)]
    pub repair: bool,
    #[arg(long)]
    pub output: Option<PathBuf>,
}

#[derive(Debug, Serialize)]
pub struct ValidateData {
    pub source: String,
    pub output: Option<String>,
    pub valid: bool,
    pub issue_count: usize,
    pub repair: bool,
    pub repaired: bool,
    pub issues: Vec<docli_schema::ValidationIssue>,
    pub warnings: Vec<String>,
}

pub fn run(
    args: &ValidateArgs,
    durability: Durability,
) -> Result<ValidateData, DocliError> {
    let package = Package::open(&args.source)?;
    let mut issues = Vec::new();
    issues.extend(validate_structure(&package));
    issues.extend(check_invariants(&package));
    issues.extend(validate_redlines(&package));

    let mut repaired = false;
    let mut output = None;

    if args.repair {
        let request = PipelineRequest {
            command: "validate".to_string(),
            source: args.source.clone(),
            output: args
                .output
                .clone()
                .unwrap_or_else(|| default_repair_output(&args.source)),
            durability,
            revalidate_after_write: true,
        };
        output = Some(request.output.clone().display().to_string());

        let _ = run_shadow_pipeline(&request, PipelineHooks {
            apply_ops: Some(&|context| {
                for (path, bytes) in context.package.xml_parts.clone() {
                    let reparsed = ensure_xml_space_preserve(&bytes)?;
                    let repaired_bytes = repair_durable_id_overflow(&reparsed)?;
                    if repaired_bytes != bytes {
                        context.xml_parts.insert(path.clone(), repaired_bytes);
                        context.touched_parts.insert(path);
                    }
                }
                Ok(())
            }),
            validate: None,
            serialize_touched_parts: None,
            render_check: None,
        })?;
        repaired = true;
    }

    Ok(ValidateData {
        source: args.source.display().to_string(),
        output,
        valid: issues.is_empty(),
        issue_count: issues.len(),
        repair: args.repair,
        repaired,
        issues,
        warnings: Vec::new(),
    })
}

fn default_repair_output(source: &PathBuf) -> PathBuf {
    let stem = source
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("document");
    let parent = source.parent().unwrap_or_else(|| std::path::Path::new("."));
    parent.join(format!("{stem}.repaired.docx"))
}
