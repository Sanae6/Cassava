use clap::Parser;
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

	let (decompressed, _alignment) = yaz0::decompress(&in_data).unwrap();
	// std::thread::scope(|scope| {
	// 	let barrier_dec = Arc::new(Barrier::new(2));
	// 	let barrier_io = barrier_dec.clone();
	// 	let in_data: &[u8] = &in_data;
	// 	let dec = scope.spawn(move || {
	// 		let mut out_file = File::create(format!("target/Test.dec")).unwrap();
	// 		barrier_dec.wait();
	// 		let start = Instant::now();
	// 		let (uncompressed, _alignment) = yaz0::decompress(&in_data).unwrap();

	// 		out_file.write_all(&uncompressed).unwrap();
	// 		let end = Instant::now();
	// 		barrier_dec.wait();
	// 		end - start
	// 	});
	// 	let io = scope.spawn(move || {
	// 		let mut out_file = File::create(format!("target/Test.io")).unwrap();
	// 		barrier_io.wait();
	// 		let start = Instant::now();
	// 		let mut data = yaz0::Yaz0Stream::new(&in_data).unwrap();
	// 		std::io::copy(&mut data, &mut out_file).unwrap();
	// 		let end = Instant::now();
	// 		barrier_io.wait();

	// 		end - start
	// 	});

	// 	let dec = dec.join().unwrap();
	// 	let io = io.join().unwrap();
	// 	println!("dec speed: {}", dec.as_secs_f32());
	// 	println!("io speed: {}", io.as_secs_f32());
	// });

	fs::write(args.out_path, decompressed).unwrap();

	Ok(())
}
