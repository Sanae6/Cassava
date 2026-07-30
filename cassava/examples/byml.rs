use std::{fs, path::PathBuf};

use cassava::{
	byaml::{self, Byml, Container, Element},
	util::{BinaryReader, ByteOrder},
};
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
	Rewrite { in_path: PathBuf, out_path: PathBuf },
}

#[snafu::report]
fn main() -> Result<(), byaml::Error> {
	let args = Args::parse();
	match args.command {
		Commands::Dump { in_path } => {
			let file = fs::read(in_path).unwrap();
			let byml = Byml::read(&mut BinaryReader::new(&file))?;
			println!("filetype byml");
			println!("version {}", byml.version);
			println!(
				"endian {}",
				match byml.endianness {
					ByteOrder::Big => "big",
					ByteOrder::Little => "little",
				}
			);
			match byml.root_node {
				Some(node) => {
					print!("root ");
					dump_container(node, &mut "".to_owned());
				}
				None => {
					println!("root null")
				}
			}
		}
		Commands::Rewrite { in_path, out_path } => {
			let file = fs::read(in_path).unwrap();
			let byml = Byml::read(&mut BinaryReader::new(&file))?;
			let file = byml.write().unwrap();
			fs::write(out_path, file).unwrap();
		}
	}
	Ok(())
}

fn dump_container(el: Container, indent: &mut String) {
	indent.push_str("  ");
	match el {
		Container::Array(array) => {
			println!("array [");
			for el in array.borrow().iter() {
				print!("{indent}");
				dump_element(el, indent);
			}
			indent.pop().unwrap();
			indent.pop().unwrap();
			println!("{indent}]");
		}
		Container::Dictionary(dict) => {
			println!("hash {{");
			for (name, el) in dict.borrow().iter() {
				print!("{indent}{name}: ");
				dump_element(el, indent);
			}
			indent.pop().unwrap();
			indent.pop().unwrap();
			println!("{indent}}}");
		}
	}
}

fn dump_element(el: &Element, indent: &mut String) {
	match el {
		Element::Array(array) => {
			dump_container(Container::Array(array.clone()), indent);
		}
		Element::Dictionary(dict) => {
			dump_container(Container::Dictionary(dict.clone()), indent);
		}
		Element::String(string) => {
			println!("string {string:?}")
		}
		Element::Boolean(bool) => {
			println!("bool {bool}");
		}
		Element::I32(value) => {
			println!("s32 {value}")
		}
		Element::F32(value) => {
			println!("f32 {value:?}")
		}
		Element::U32(value) => {
			println!("u32 {value}")
		}
		Element::I64(_) => todo!(),
		Element::U64(_) => todo!(),
		Element::F64(_) => todo!(),
		Element::Null => println!("null"),
	}
}
