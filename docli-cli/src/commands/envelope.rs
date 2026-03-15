use std::io::{self, Write};

use clap::ValueEnum;
use docli_core::Envelope;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Json,
    Yaml,
    Text,
}

impl OutputFormat {
    fn render<T: Serialize>(&self, pretty: bool, envelope: &Envelope<T>) -> Result<String, String> {
        match self {
            Self::Json => {
                if pretty {
                    serde_json::to_string_pretty(envelope).map_err(|error| error.to_string())
                } else {
                    serde_json::to_string(envelope).map_err(|error| error.to_string())
                }
            }
            Self::Yaml => serde_yaml::to_string(envelope).map_err(|error| error.to_string()),
            Self::Text => {
                if pretty {
                    serde_json::to_string_pretty(envelope).map_err(|error| error.to_string())
                } else {
                    serde_json::to_string(envelope).map_err(|error| error.to_string())
                }
            }
        }
    }
}

pub fn emit_output<T: Serialize>(
    format: &OutputFormat,
    pretty: bool,
    quiet: bool,
    envelope: &Envelope<T>,
) -> io::Result<()> {
    let output = format
        .render(pretty, envelope)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;

    if quiet {
        let mut stderr = io::stderr();
        let _ = writeln!(stderr, "{output}");
        return Ok(());
    }

    let mut stdout = io::stdout();
    writeln!(stdout, "{output}")
        .map_err(|error| io::Error::new(io::ErrorKind::BrokenPipe, error))
}
