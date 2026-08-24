use std::path::PathBuf;

use cassava_project::ALProject;
use clap::{Parser, Subcommand};
use snafu::Whatever;


#[derive(Parser)]
#[command(about)]
struct Args {
	#[command(subcommand)]
	command: Commands,
  #[arg()]
  romfs_dir: PathBuf,
}

#[derive(Subcommand)]
enum Commands {
	ImportStage { in_path: PathBuf, stage_name: String },
}

#[snafu::report]
fn main() -> Result<(), Whatever> {
  let args = Args::parse();
  match args.command {
    Commands::ImportStage { in_path, stage_name } => {
      let mut project = ALProject::load(&in_path)?;
      project.import_stage(&args.romfs_dir, &stage_name)?;
    }
  }
  Ok(())
}
