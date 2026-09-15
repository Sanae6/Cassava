use std::{
	fs,
	path::{Path, PathBuf},
};

use cassava::{
	nw::{
		bfsha::ShaderArchive,
		bnsh::{self, ShaderContainer},
	},
	util::BinaryReader,
};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(about)]
struct Args {
	#[command(subcommand)]
	format: Format,
}

#[derive(Subcommand)]
enum Format {
	Bfsha {
		#[command(subcommand)]
		command: Commands,
	},
	Bnsh {
		#[command(subcommand)]
		command: Commands,
	},
}

#[derive(Subcommand)]
enum Commands {
	Dump {
		in_path: PathBuf,
		out_path: Option<PathBuf>,
	},
}

fn main() {
	let args = Args::parse();

	match args.format {
		Format::Bfsha { command } => match command {
			Commands::Dump { in_path, out_path } => {
				let data = fs::read(in_path).unwrap();
				let shader = ShaderArchive::read(&mut BinaryReader::new(&data)).unwrap();
				println!("{shader:#?}");
				let Some(out_path) = out_path else {
					return;
				};
				for (model_name, model) in shader.models {
					let path = out_path.join(format!("{model_name}.bnsh"));
					println!("writing {model_name} to {path:?}");
					fs::write(path, model.container).unwrap();
				}
			}
		},
		Format::Bnsh { command } => match command {
			Commands::Dump { in_path, out_path } => {
				let file_name = in_path.file_stem().unwrap().to_str().unwrap().to_owned();
				let file_name = &file_name;
				let data = fs::read(in_path).unwrap();
				let shader = ShaderContainer::read(&mut BinaryReader::new(&data)).unwrap();
				println!("{}", shader.shader_variations.len());
				let Some(out_path) = out_path else {
					return;
				};
				for (index, variation) in shader.shader_variations.iter().enumerate() {
					fn write_shader(
						shader: &bnsh::Shader,
						file_name: &str,
						variation: usize,
						shader_type: &str,
						program_type: &str,
						path: &Path,
					) {
						let control_path = path.join(format!("{file_name}.{variation}.{shader_type}.{program_type}.ctl.bin"));
						let data_path = path.join(format!("{file_name}.{variation}.{shader_type}.{program_type}.data.bin"));
						println!("writing control {variation} of {shader_type} in {program_type} to {control_path:?}");
						fs::write(control_path, &shader.control).unwrap();
						println!("writing data {variation} of {shader_type} in {program_type} to {data_path:?}");
						fs::write(data_path, &shader.data).unwrap();
					}
					fn write_program(
						program: &bnsh::Program,
						file_name: &str,
						variation: usize,
						program_type: &str,
						path: &Path,
					) {
						program.vertex_shader.as_ref().map(|shader| write_shader(&shader, file_name, variation, "vertex", program_type, path));
						program.hull_shader.as_ref().map(|shader| write_shader(&shader, file_name, variation, "hull", program_type, path));
						program.fragment_shader.as_ref().map(|shader| write_shader(&shader, file_name, variation, "fragment", program_type, path));
						program.geometry_shader.as_ref().map(|shader| write_shader(&shader, file_name, variation, "geometry", program_type, path));
						program.domain_shader.as_ref().map(|shader| write_shader(&shader, file_name, variation, "domain", program_type, path));
						program.compute_shader.as_ref().map(|shader| write_shader(&shader, file_name, variation, "compute", program_type, path));
					}
					variation
						.binary_program
						.as_ref()
						.map(|program| write_program(&program, file_name, index, "binary", &out_path));
					variation
						.ir_program
						.as_ref()
						.map(|program| write_program(&program, file_name, index, "ir", &out_path));
					variation
						.source_program
						.as_ref()
						.map(|program| write_program(&program, file_name, index, "source", &out_path));
				}
			}
		},
	}
}
