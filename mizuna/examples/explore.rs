use std::{env::args, fs, path::Path};

use mizuna::{sarc, yaz0};

fn main() {
	let location = args().nth(1).unwrap();
	let dir = fs::read_dir(location).unwrap();
	dir.for_each(|x| {
		let y = x.unwrap();
		let p = y.path();
		if !p
			.file_name()
			.unwrap()
			.to_str()
			.unwrap()
			.ends_with("Design.szs")
		{
			return;
		}
		let f = fs::read(&p).unwrap();
		let d = yaz0::decompress(&f).unwrap();
		let data = sarc::Reader::new(d.0).unwrap();
		// let sound_count = data.get_filenames().count();
		// println!("{}: {sound_count}", p.file_name().unwrap().to_string_lossy())
		println!("{}", p.file_name().unwrap().to_string_lossy());
		let stem = p.file_stem().unwrap();
		for name in data.get_filenames() {
			let path = Path::new(name.as_str());
			if path
				.file_stem()
				.unwrap()
				.to_str()
				.unwrap()
				.eq(stem.to_str().unwrap())
			{
				continue;
			}
			if name == "CameraParam.byml" {
				continue;
			}
			println!("\t{}", name)
		}
	});
}
