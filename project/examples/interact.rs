#![feature(trim_prefix_suffix)]

use std::{fs, path::PathBuf};

use cassava_project::ALProject;
use clap::{Parser, Subcommand};
use indexmap::IndexSet;
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
	ImportStage {
		in_path: PathBuf,
		stage_name: String,
	},
	ImportAll {
		in_path: PathBuf,
	},
	NewProject {
		in_path: PathBuf,
		name: String,
	},
}

#[snafu::report]
fn main() -> Result<(), Whatever> {
	let args = Args::parse();
	match args.command {
		Commands::ImportStage {
			in_path,
			stage_name,
		} => {
			let mut project = ALProject::load(&in_path)?;
			project.import_stage(&args.romfs_dir, &stage_name)?;
		}
		Commands::ImportAll { in_path } => {
			let mut project = ALProject::load(&in_path)?;
			let dir = fs::read_dir(args.romfs_dir.join("StageData")).unwrap();
			let mut stages = IndexSet::new();
			dir.for_each(|entry| {
				let path = entry.unwrap().path();
				let stem = path.file_stem().unwrap().to_str().unwrap().to_owned();
				let stage_name = stem
					.trim_suffix("Design")
					.trim_suffix("Map")
					.trim_suffix("Sound")
					.to_owned();
				stages.insert(stage_name);
			});

			for stage_name in stages {
				println!("testing {stage_name}");
				project.import_stage(&args.romfs_dir, &stage_name)?;
			}
		}
		Commands::NewProject { in_path, name } => {
			ALProject::create(&in_path, &name).unwrap();
		}
	}
	Ok(())
}
