use mizuna::{
	nw::bfres::{self, ModelArchive},
	util::{BinaryReader, ByteOrder},
};
use std::{fs, path::PathBuf};

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(about)]
struct Args {
	#[command(subcommand)]
	command: Commands,
}

#[derive(Subcommand)]
enum Commands {
	Dump { in_path: PathBuf },
}

#[snafu::report]
fn main() -> Result<(), bfres::Error> {
	let args = Args::parse();
	match args.command {
		Commands::Dump { in_path } => {
			let file = fs::read(in_path).unwrap();
			let model_archive =
				ModelArchive::read(&mut BinaryReader::new(&file, ByteOrder::Little))?;
			todo!("{model_archive:#?}")
		}
	}
	Ok(())
}
