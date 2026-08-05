use cassava::yaz0;
use clap::Parser;
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

	let (decompressed, _alignment) = yaz0::decompress(&in_data).unwrap();

	fs::write(args.out_path, decompressed).unwrap();

	// std::thread::scope(|scope| {
	// 	use std::{fs::File, io::Write, time::Instant};
	// 	let in_data: &[u8] = &in_data;
	// 	let dec = {
	// 		// let mut out_file = File::create(format!("target/Test.dec")).unwrap();
	// 		// barrier_dec.wait();
	// 		let start = Instant::now();
	// 		let (decompressed, _alignment) = yaz0::decompress(&in_data).unwrap();

	// 		let mut bytes: Vec<u8> = Vec::with_capacity(decompressed.len());
	// 		bytes.write_all(&decompressed).unwrap();
	// 		let end = Instant::now();
	// 		// barrier_dec.wait();
	// 		end - start
	// 	};
	// 	println!("dec speed: {}", dec.as_secs_f32());

	// 	let io = {
	// 		let start = Instant::now();
	// 		let mut data = yaz0::Yaz0Stream::new(&in_data).unwrap();
	// 		let mut bytes: Vec<u8> = Vec::with_capacity(data.decompressed_size());
	// 		std::io::copy(&mut data, &mut bytes).unwrap();
	// 		let end = Instant::now();

	// 		end - start
	// 	};

	// 	println!("io speed: {}", io.as_secs_f32());
	// });

	Ok(())
}
