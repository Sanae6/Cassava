#![feature(error_generic_member_access)] // for snafu backtrace
use std::{fs, path::PathBuf};

use cassava::{
	nw::bnsh::{self, ShaderFile, Variation}, util::{BinaryReader, ByteOrder}
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
fn main() -> Result<(), bnsh::Error> {
	let args = Args::parse();
	match args.command {
		Commands::Dump { in_path } => {
			let file = fs::read(in_path).unwrap();
			let shader_file = ShaderFile::read(&mut BinaryReader::new(&file));
			match shader_file {
				Ok(shader_file) => {
					let variation: &Variation = shader_file.shader_variations.get(0).unwrap();
					println!("fuck {}", shader_file.shader_variations.len());
					let program = &variation.binary_program.as_ref().unwrap();
					let shader = program.vertex_shader.as_ref().unwrap();
					fs::write("romfs/vsh.control.bin", &shader.control).unwrap();
					fs::write("romfs/vsh.data.bin", &shader.data).unwrap();
					println!("{program:#?}");
					// println!("{shader_file:#?}")
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
