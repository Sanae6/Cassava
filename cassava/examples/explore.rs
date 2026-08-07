use std::{env::args, fs, path::Path};

use cassava::{byaml::{self, Container, Element}, sarc, util::BinaryReader, yaz0};

fn main() {
	let location = args().nth(1).unwrap();
	let data = fs::read(&location).unwrap();
	let mut reader = BinaryReader::new(&data[0x10..]);
	let b = byaml::Byml::read(&mut reader).unwrap();
	let Container::Dictionary(dict) = b.root_node.unwrap() else { panic!()};
	let dict = dict.borrow();
	let Element::Array(shine_data) = dict.get("ShineData").unwrap() else { unreachable!()};
	let shine_data = shine_data.borrow();
	let mut found = 0;
	for shine in &*shine_data {
		let Element::Dictionary(shine) = shine else {panic!()};
		let shine = shine.borrow();
		let Element::Boolean(is_get) = shine.get("IsGet").unwrap() else { panic!()};
		let Element::I32(unique_id) = shine.get("UniqueId").unwrap() else { panic!()};
		if *is_get {
			println!("found unique shine: {unique_id} {shine:?}");
			found += 1;
		}
	}
	println!("total found moons: {found}");

	// let dir = fs::read_dir(location).unwrap();
	// dir.for_each(|x| {
	// 	let y = x.unwrap();
	// 	let p = y.path();
	// 	if !p
	// 		.file_name()
	// 		.unwrap()
	// 		.to_str()
	// 		.unwrap()
	// 		.ends_with("Design.szs")
	// 	{
	// 		return;
	// 	}
	// 	let f = fs::read(&p).unwrap();
	// 	let d = yaz0::decompress(&f).unwrap();
	// 	let data = sarc::Reader::new(d.0).unwrap();
	// 	// let sound_count = data.get_filenames().count();
	// 	// println!("{}: {sound_count}", p.file_name().unwrap().to_string_lossy())
	// 	println!("{}", p.file_name().unwrap().to_string_lossy());
	// 	let stem = p.file_stem().unwrap();
	// 	for name in data.filenames() {
	// 		let path = Path::new(name);
	// 		if path
	// 			.file_stem()
	// 			.unwrap()
	// 			.to_str()
	// 			.unwrap()
	// 			.eq(stem.to_str().unwrap())
	// 		{
	// 			continue;
	// 		}
	// 		if name == "CameraParam.byml" {
	// 			continue;
	// 		}
	// 		println!("\t{}", name)
	// 	}
	// });
}
