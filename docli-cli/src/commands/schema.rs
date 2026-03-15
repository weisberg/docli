use clap::{Args, ValueEnum};
use serde::Serialize;

use serde_json::json;

#[derive(Debug, Args)]
pub struct SchemaArgs {
    #[arg(long, default_value = "job", value_enum)]
    pub schema_type: SchemaType,
}

#[derive(Clone, Debug, ValueEnum)]
pub enum SchemaType {
    Job,
    Target,
    Envelope,
}

#[derive(Debug, Serialize)]
pub struct SchemaData {
    pub kind: String,
    pub generated: bool,
    pub schema: serde_json::Value,
}

pub fn run(args: &SchemaArgs) -> Result<SchemaData, docli_core::DocliError> {
    let (kind, schema) = match args.schema_type {
        SchemaType::Job => (
            "job",
            json!({
                "title": "docli core job",
                "description": "Top-level job object with operation list",
                "required": ["operations"],
                "properties": {
                    "operations": {
                        "type": "array",
                        "items": {"type": "object"}
                    }
                }
            }),
        ),
        SchemaType::Target => (
            "target",
            json!({
                "title": "docli selector target",
                "description": "Target description used by edit and review commands",
                "type": "object",
            }),
        ),
        SchemaType::Envelope => (
            "envelope",
            json!({
                "title": "docli envelope",
                "oneOf": [
                    {"type": "object", "required": ["ok"]},
                    {"type": "object", "required": ["error"]}
                ]
            }),
        ),
    };

    Ok(SchemaData {
        kind: kind.to_string(),
        generated: false,
        schema,
    })
}
