use std::{fs, path::PathBuf};

use mizuna::{nw::ptcl::{self, EffectArchive}, util::{BinaryReader, ByteOrder}};

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(about)]
struct Args {
	#[command(subcommand)]
	command: Commands,
}

#[derive(Subcommand)]
enum Commands {
	Dump { in_path: PathBuf,  },
}

#[snafu::report]
fn main() -> Result<(), ptcl::Error> {
	let args = Args::parse();
	match args.command {
		Commands::Dump { in_path, } => {
			let file = fs::read(in_path).unwrap();
			let effect_archive = EffectArchive::read(&mut BinaryReader::new(&file, ByteOrder::Little));
      println!("{effect_archive:#?}")
		}
	}
	Ok(())
}
