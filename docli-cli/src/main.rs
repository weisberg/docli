use std::path::PathBuf;
use std::process;

use clap::{Args, Parser, Subcommand};
use docli_core::{DocliError, Durability, EnvelopeBuilder};
use serde::Serialize;

mod commands;

#[derive(Debug, Parser)]
#[command(name = "docli", version, about = "DOCX tooling CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[command(flatten)]
    global: GlobalArgs,
}

#[derive(Debug, Args)]
struct GlobalArgs {
    #[arg(long, default_value = "json", global = true, value_enum)]
    format: commands::OutputFormat,
    #[arg(long, default_value_t = false, global = true)]
    pretty: bool,
    #[arg(long, default_value_t = false, global = true)]
    quiet: bool,
    #[arg(long = "kb-path", default_value = "./kb", global = true)]
    kb_path: PathBuf,
    #[arg(long, default_value = "Claude", global = true)]
    author: String,
    #[arg(long, default_value_t = false, global = true)]
    verbose: bool,
    #[arg(long, default_value = "durable", global = true, value_parser = ["fast", "durable", "paranoid"])]
    durability: String,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Inspect(commands::inspect::InspectArgs),
    Validate(commands::validate::ValidateArgs),
    Ooxml(commands::ooxml::OoxmlArgs),
    Kb(commands::kb::KbArgs),
    Schema(commands::schema::SchemaArgs),
    Doctor(commands::doctor::DoctorArgs),
    Read(commands::read::ReadArgs),
}

fn main() {
    let cli = Cli::parse();
    let exit_code = match cli.command {
        Commands::Inspect(args) => run_command(&cli.global, "inspect", || commands::inspect::run(&args)),
        Commands::Validate(args) => {
            run_command(&cli.global, "validate", || commands::validate::run(&args, cli.global.durability()))
        }
        Commands::Ooxml(args) => run_command(&cli.global, "ooxml", || commands::ooxml::run(&args)),
        Commands::Kb(args) => {
            run_command(&cli.global, "kb", || commands::kb::run(&args, &cli.global.kb_path))
        }
        Commands::Schema(args) => run_command(&cli.global, "schema", || commands::schema::run(&args)),
        Commands::Doctor(args) => {
            run_command(&cli.global, "doctor", || commands::doctor::run(&args, &cli.global.kb_path))
        }
        Commands::Read(args) => run_command(&cli.global, "read", || commands::read::run(&args)),
    };
    process::exit(exit_code);
}

fn run_command<T, F>(global: &GlobalArgs, command: &str, action: F) -> i32
where
    T: Serialize,
    F: FnOnce() -> Result<T, DocliError>,
{
    let builder = EnvelopeBuilder::new(command);
    let envelope = match action() {
        Ok(data) => builder.ok(data),
        Err(error) => {
            if !global.quiet && global.verbose {
                eprintln!("error: {}", error);
            }
            builder.err::<T>(&error)
        }
    };

    if let Err(io_error) = commands::envelope::emit_output(
        &global.format,
        global.pretty,
        global.quiet,
        &envelope,
    ) {
        eprintln!("failed to write output: {io_error}");
        return 1;
    }

    match envelope {
        docli_core::Envelope::Ok(_) => 0,
        docli_core::Envelope::Err(_) => 1,
    }
}

impl GlobalArgs {
    fn durability(&self) -> Durability {
        match self.durability.as_str() {
            "fast" => Durability::Fast,
            "paranoid" => Durability::Paranoid,
            _ => Durability::Durable,
        }
    }
}
