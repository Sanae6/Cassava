use std::{
	collections::{HashMap, HashSet},
	env::args,
	fs,
	path::Path,
	ptr,
};

use cassava::{
	byaml::{self, Byml, Container, Element},
	sarc,
	util::{ArcAtomicRefCellExt, BinaryReader},
	yaz0,
};
use indexmap::IndexMap;
use itertools::Itertools;
use smol_str::SmolStr;

fn main() {
	let location = args().nth(1).unwrap();
	// let mut object_to_placement = HashMap::new();
	let dir = fs::read_dir(location).unwrap();
	dir.for_each(|x| {
		let y = x.unwrap();
		let p = y.path();
		let f = fs::read(&p).unwrap();
		let d = yaz0::decompress(&f).unwrap();
		let data = sarc::Reader::new(d.0).unwrap();
		let stem = p.file_stem().unwrap();
		for (name, data) in data.files() {
			let path = Path::new(name);
			if !path
				.file_stem()
				.unwrap()
				.to_str()
				.unwrap()
				.eq(stem.to_str().unwrap())
			{
				continue;
			}
			if path.extension().unwrap().to_str().unwrap() != "byml" {
				continue;
			}
			let byml = Byml::read(&mut BinaryReader::new(data)).unwrap();
			let stage = match byml.as_array() {
				Some(array) => array.into_borrow(),
				None => return,
			};
			for (scenario_no, scenario) in stage.iter().enumerate() {
				let lists = scenario.unwrap_dictionary().into_borrow();
				for (list_name, list) in lists.iter() {
					let list = list.unwrap_array().into_borrow();
					for object in list.iter() {
						let object = object.unwrap_dictionary().into_borrow();
						let mut object_queue = vec![object];
						while let Some(object) = object_queue.pop() {
							let factory_name = object["UnitConfig"]
								.get_key_string("ParameterConfigName")
								.unwrap();

							let target_file = object["UnitConfig"]
								.get_key_string("PlacementTargetFile")
								.unwrap();

							let object_name = object["UnitConfigName"].unwrap_string();

							// object_to_placement
							// 	.entry(object_name)
							// 	.or_insert(HashSet::new())
							// 	.insert((list_name.clone(), target_file));

							let links = object["Links"].unwrap_dictionary().into_borrow();
							for (link_name, linked_object) in links.iter() {
								let linked_objects = linked_object.unwrap_array().into_borrow();
								for linked_object in linked_objects.iter() {
									object_queue.push(linked_object.unwrap_dictionary().into_borrow());
								}
							}
						}
					}
				}
			}
		}
	});

	let mut fac_plac = object_to_placement
		.iter()
		.map(|(key, value)| {
			(key, {
				let mut vec = value.into_iter().collect_vec();
				vec.sort();
				vec
			})
		})
		.collect_vec();
	fac_plac.sort_by_key(|value| value.0);

	for (key, value) in fac_plac {
		if value.len() > 1 {
			println!("{key} {value:?}");
		}
	}
}
