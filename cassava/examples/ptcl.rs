#![feature(error_generic_member_access)] // for snafu backtrace
use std::{fs, path::PathBuf};

use cassava::{
	nw::{
		bfres,
		ptcl::{self, EffectArchive},
		res_dict::DictError,
	},
	util::{BinaryReader, ByteOrder},
};

use clap::{Parser, Subcommand};
use snafu::{ErrorCompat, Report};

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
fn main() -> Result<(), ptcl::Error> {
	let args = Args::parse();
	match args.command {
		Commands::Dump { in_path } => {
			let file = fs::read(in_path).unwrap();
			let effect_archive = EffectArchive::read(&mut BinaryReader::new(&file, ByteOrder::Little));
			match effect_archive {
				Ok(effect_archive) => {
					// println!("{effect_archive:#?}")
				}
				Err(error) => {
					println!("{error}");
					let backtrace = error.backtrace().unwrap();
					println!("{backtrace}");
					// let ptcl::Error::ModelArchiveError { source: error } = error else {
					// 	unreachable!()
					// };
					// let backtrace = error.backtrace();
					// println!("{backtrace:?}");
					// let bfres::Error::ModelError { source: error } = error else {
					// 	unreachable!()
					// };
					// let backtrace = error.backtrace();
					// println!("{backtrace:?}");
					// let DictError::ElementError {
					// 	source: bfres::ModelError::MaterialError { source: error },
					// } = error
					// else {
					// 	unreachable!()
					// };
					// let backtrace = error.backtrace();
					// println!("{backtrace:?}");
					// let bfres::MaterialError::ShaderAssignError { source: error } = error else {
					// 	unreachable!()
					// };
					// let backtrace = error.backtrace();
					// println!("{backtrace:?}");
				}
			}
		}
	}
	Ok(())
}
