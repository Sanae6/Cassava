#![feature(trim_prefix_suffix)]

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
use smol_str::{SmolStr, ToSmolStr};

fn main() {
	let location = args().nth(1).unwrap();
	let mut stages = IndexMap::new();
	let dir = fs::read_dir(location).unwrap();
	let mut count = 0;
	dir.for_each(|x| {
		count += 1;
		let y = x.unwrap();
		let p = y.path();
		let f = fs::read(&p).unwrap();
		let d = yaz0::decompress(&f).unwrap();
		let data = sarc::Reader::new(d.0).unwrap();
		let stem = p.file_stem().unwrap().to_str().unwrap().to_owned();
		let stage_name = stem
			.trim_suffix("Design")
			.trim_suffix("Map")
			.trim_suffix("Sound")
			.to_owned();
		let placement_suffix = stem.trim_prefix(&stage_name).to_owned();
		for (name, data) in data.files() {
			let path = Path::new(name);
			if !path
				.file_stem()
				.unwrap()
				.to_str()
				.unwrap()
				.starts_with(&stage_name)
			{
				continue;
			}
			if path.extension().unwrap().to_str().unwrap() != "byml" {
				continue;
			}
			// println!("{stage_name}");
			let byml = Byml::read(&mut BinaryReader::new(data)).unwrap();
			let stage_id_map = stages.entry(stage_name.clone()).or_insert(IndexMap::new());
			let stage = match byml.as_array() {
				Some(array) => array.into_borrow(),
				None => continue,
			};
			for (_scenario_no, scenario) in stage.iter().enumerate() {
				let lists = scenario.unwrap_dictionary().into_borrow();
				for (_list_name, list) in lists.iter() {
					let list = list.unwrap_array().into_borrow();
					for object in list.iter() {
						let mut object_queue = vec![object.unwrap_dictionary()];
						while let Some(object_arc) = object_queue.pop() {
							object_arc.borrow_mut().shift_remove("PlacementFileName");
							object_arc.borrow_mut().shift_remove("SrcUnitLayerList");
							object_arc.borrow_mut().shift_remove("comment");
							// object_arc.borrow_mut().shift_remove("RailPoints");
							{
								let mut object = object_arc.borrow_mut();
								let id = object["Id"].unwrap_string();
								if id.starts_with("rail") {
									object["Id"] = Element::String(format!("_{id}{placement_suffix}").into())
								}
							}
							let object = object_arc.borrow();
							let factory_name = object["UnitConfig"]
								.get_key_string("ParameterConfigName")
								.unwrap();
							let object_name = object["UnitConfigName"].unwrap_string();
							// let layer = object["LayerConfigName"].unwrap_string();

							let id = object["Id"].as_string().unwrap();
							let mut links = object["Links"].unwrap_dictionary().into_borrow_mut();
							for (_link_name, linked_object) in links.drain(..) {
								let linked_objects = linked_object.unwrap_array().into_borrow();
								for linked_object in linked_objects.iter() {
									object_queue.push(linked_object.unwrap_dictionary());
								}
							}
							drop(links);

							if let Some((other_stem, o_scen, other_object)) = stage_id_map.insert(
								(id.clone(), object_name.clone()),
								(stem.clone(), _scenario_no, object_arc.clone()),
							) && Element::Dictionary(object_arc.clone())
								!= Element::Dictionary(other_object.clone())
							{
								// println!();
								let object = object_arc.borrow();
								println!(
									"  {id} {stem}:{_scenario_no}:{factory_name} {other_stem}:{o_scen}:{factory_name}"
								);
								// println!("{object:?}");
								// println!();
								// println!("{other_object:?}");
								// println!();
							}

							// object_to_placement
							// 	.entry(object_name)
							// 	.or_insert(HashSet::new())
							// 	.insert((list_name.clone(), target_file));
						}
					}
				}
			}
		}
	});
	println!("file checked count {count}")
}
