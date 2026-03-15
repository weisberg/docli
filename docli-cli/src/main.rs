use clap::Parser;

#[derive(Parser)]
#[command(name = "docli", version, about = "Phase 0 workspace scaffold")]
struct Cli;

fn main() {
    let _ = Cli::parse();
}
