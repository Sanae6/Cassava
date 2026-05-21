use clap::{Parser, Subcommand, ValueEnum};
// use eyre::Result;
use mizuna::sarc::{self, Reader, Writer};
use mizuna::util::{self, ByteOrder};
use std::fs;
use std::path::PathBuf;

#[derive(ValueEnum, Clone, Copy, Debug)]
enum Mode {
	Read,
	Write,
}

#[derive(Parser)]
#[command(about)]
struct Args {
	#[command(subcommand)]
	command: Commands,
}

#[derive(Subcommand)]
enum Commands {
	Read {
		in_path: PathBuf,
		out_path: PathBuf,
	},
	Write {
		in_path: PathBuf,
		out_path: PathBuf,
		#[arg(default_value_t = 0x80)]
		alignment: u32,
	},
}

#[snafu::report]
fn main() -> Result<(), sarc::Error> {
	let args = Args::parse();

	match &args.command {
		Commands::Read { in_path, out_path } => {
			let in_data = fs::read(in_path).expect("could not read file");

			let reader = Reader::new(in_data)?;

			fs::create_dir_all(out_path)?;

			for filename in reader.get_filenames() {
				let data = reader.get_file_data_by_name(filename)?;
				let path = out_path.join(filename);
				println!("saving {path:?}...");
				fs::write(path, data)?;
			}
		}
		Commands::Write {
			in_path,
			out_path,
			alignment,
		} => {
			if !in_path.is_dir() {
				return Err(sarc::Error::FileNotFound);
			}

			if alignment.count_ones() != 1 {
				return Err(mizuna::sarc::Error::ReadError {
					source: util::Error::InvalidAlignment,
				});
			}

			let mut writer = Writer::new(*alignment);

			for entry in fs::read_dir(in_path)? {
				let path = entry?.path();
				let file_data = fs::read(&path)?;
				let relative_path = pathdiff::diff_paths(path, in_path).unwrap();
				writer.add_file(
					&relative_path.to_string_lossy().replace("\\", "/"),
					file_data,
				);
			}

			writer.save(out_path, ByteOrder::Little)?;
		}
	};

	Ok(())
}
