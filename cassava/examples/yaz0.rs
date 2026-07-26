use clap::Parser;
// use eyre::Result;
use cassava::yaz0;
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(about)]
struct Args {
	in_path: PathBuf,
	out_path: PathBuf,
}

#[snafu::report]
fn main() -> Result<(), yaz0::Error> {
	let args = Args::parse();

	let in_data = fs::read(args.in_path).expect("could not read file");

	let (uncompressed, _alignment) = yaz0::decompress(&in_data)?;

	fs::write(args.out_path, uncompressed)?;

	Ok(())
}
