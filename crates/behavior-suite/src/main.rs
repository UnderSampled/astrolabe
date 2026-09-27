use anyhow::{Result, ensure};
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "behavior-suite",
    about = "Pair independently passing OpenSpace and Godot behavior reports"
)]
struct Cli {
    #[arg(long)]
    openspace: PathBuf,
    #[arg(long)]
    godot: PathBuf,
    #[arg(long)]
    output: PathBuf,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let comparison = behavior_suite::compare_reports(&cli.openspace, &cli.godot, &cli.output)?;
    println!("{}", cli.output.display());
    ensure!(
        comparison.status == "passed",
        "functional behavior comparison failed"
    );
    Ok(())
}
