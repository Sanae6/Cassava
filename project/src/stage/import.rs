use std::{borrow::Cow, collections::HashMap, fs, path::Path, sync::Arc};

use cassava::{
	atomic_refcell::AtomicRefCell,
	byaml::{self, Array, Dictionary, Element},
	util::{ArcAtomicRef, ArcAtomicRefCellExt, BinaryReader},
};
use glam::Vec3;
use indexmap::IndexMap;
use slotmap::SlotMap;
use smol_str::SmolStr;
use snafu::{OptionExt, ResultExt, Whatever, ensure_whatever, whatever};
use tracing::warn;

use crate::{
	DiskRef,
	stage::{
		Camera, CameraId, Entity, EntityKey, Folder, Node, Parameter, Scenario, Scenarios, Stage,
		SuffixCamera, Transform,
	},
	util::load_szs,
};

impl Stage {
	pub fn import(romfs_root: &Path, stage_name: &str) -> Result<Self, Whatever> {
		let stage_data = romfs_root.join("StageData");
		let map_path = stage_data.join(format!("{stage_name}Map.szs"));
		let sound_path = stage_data.join(format!("{stage_name}Sound.szs"));
		let design_path = stage_data.join(format!("{stage_name}Design.szs"));
		ensure_whatever!(
			map_path.exists(),
			"StageData/{stage_name}Map.szs is expected to exist"
		);

		let sound_byml = sound_path
			.exists()
			.then(|| -> Result<_, Whatever> {
				let szs = fs::read(sound_path).whatever_context("failed to read stage's sound archive")?;
				let sarc = load_szs(&szs)?;
				let byml = sarc
					.get_file(&format!("{stage_name}Sound.byml"))
					.whatever_context("unable to find stage byml in stage's sound archive")?;
				if sarc.filenames().count() > 0 {
					warn!(
						"{stage_name}Sound.szs has more files than expected ({:?})",
						sarc.filenames().collect::<Vec<_>>()
					)
				}
				Ok(
					byaml::Byml::read(&mut BinaryReader::new(&byml))
						.whatever_context("failed to read stage sound byml")?,
				)
			})
			.transpose()?;
		let (design_byml, design_archive) = design_path
			.exists()
			.then(|| -> Result<_, Whatever> {
				let szs =
					fs::read(design_path).whatever_context("failed to read stage's design archive")?;
				let sarc = load_szs(&szs)?;
				let byml = sarc
					.get_file(&format!("{stage_name}Design.byml"))
					.whatever_context("unable to find stage byml in stage's design archive")?;
				Ok((
					byaml::Byml::read(&mut BinaryReader::new(&byml))
						.whatever_context("failed to read stage design byml")?,
					sarc,
				))
			})
			.transpose()?
			.unzip();
		let design_byml_name = format!("{stage_name}Map.byml");
		let (map_byml, map_archive) = {
			let szs = fs::read(map_path).whatever_context("failed to read stage's design archive")?;
			let sarc = load_szs(&szs)?;
			let byml = sarc
				.get_file(&design_byml_name)
				.whatever_context("unable to find stage byml in stage's design archive")?;
			(
				byaml::Byml::read(&mut BinaryReader::new(&byml))
					.whatever_context("failed to read stage design byml")?,
				sarc,
			)
		};

		let (map_byml, sound_byml, design_byml) = (
			map_byml
				.as_array()
				.whatever_context("map byml root node is not an array")?,
			sound_byml
				.filter(|byml| byml.root_node.is_some())
				.map(|byml| {
					byml
						.as_array()
						.whatever_context("sound byml root node is not an array")
				})
				.transpose()?,
			design_byml
				.filter(|byml| byml.root_node.is_some())
				.map(|byml| {
					byml
						.as_array()
						.whatever_context("design byml root node is not an array")
				})
				.transpose()?,
		);

		let design_files = if let Some(archive) = design_archive {
			archive
				.files()
				.filter(|(file_name, _)| *file_name == design_byml_name)
				.map(|(file_name, data)| {
					(
						file_name.into(),
						DiskRef {
							path: file_name.into(),
							value: Some(data.to_vec()),
						},
					)
				})
				.collect()
		} else {
			Default::default()
		};

		let (default_camera, entrance_cameras, mut object_cameras): (
			SuffixCamera,
			Vec<SuffixCamera>,
			Vec<(CameraId, Camera)>,
		) = if let Some(camera_param) = map_archive.get_file("CameraParam.byml") {
			let camera_param = byaml::Byml::read(&mut BinaryReader::new(camera_param))
				.whatever_context("failed to read camera param byml")?;
			let camera_param = camera_param
				.as_dictionary()
				.whatever_context("expected dictionary at camera param root")?
				.into_borrow();

			let default_tickets = camera_param
				.get("DefaultTickets")
				.and_then(Element::as_array)
				.whatever_context("no default camera ticket was found")?
				.into_borrow();
			ensure_whatever!(
				default_tickets.len() == 1,
				"more than one default camera ticket was found"
			);
			let (default_camera_id, default_camera) = Self::import_camera(&default_tickets[0])?;
			ensure_whatever!(
				default_camera_id.object.is_none(),
				"default camera has an object id"
			);
			let default_camera_suffix = default_camera_id
				.suffix
				.whatever_context("default camera has no suffix")?;

			let entrance_tickets = camera_param
				.get("StartTickets")
				.map(|tickets| {
					tickets
						.as_array()
						.whatever_context("expected entrance ticket list to be an array")
				})
				.transpose()?;

			let entrance_cameras = entrance_tickets
				.map(|tickets| {
					tickets
						.into_borrow()
						.iter()
						.map(|camera| {
							Self::import_camera(camera).and_then(|(id, camera)| {
								ensure_whatever!(id.object.is_none(), "entrance camera has an object id");
								let suffix = id
									.suffix
									.whatever_context("entrance camera has no suffix")?;
								Ok(SuffixCamera { suffix, camera })
							})
						})
						.collect::<Result<Vec<_>, _>>()
				})
				.transpose()?
				.unwrap_or_default();

			let object_tickets = camera_param
				.get("Tickets")
				.map(|tickets| {
					tickets
						.as_array()
						.whatever_context("expected camera ticket list to be an array")
				})
				.transpose()?;
			let object_cameras = object_tickets
				.map(|tickets| {
					tickets
						.into_borrow()
						.iter()
						.map(Self::import_camera)
						.collect::<Result<Vec<_>, _>>()
				})
				.transpose()?
				.unwrap_or_default();
			(
				SuffixCamera {
					suffix: default_camera_suffix,
					camera: default_camera,
				},
				entrance_cameras,
				object_cameras,
			)
		} else {
			(
				SuffixCamera {
					suffix: "Default".into(),
					camera: Camera {
						factory_name: "制限付きフォロー".into(),
						parameters: Default::default(),
					},
				},
				Default::default(),
				Default::default(),
			)
		};

		let global_cameras = object_cameras
			.extract_if(.., |(id, _)| id.suffix.is_some() && id.object.is_none())
			.map(|(id, camera)| {
				Ok(SuffixCamera {
					suffix: id.suffix.expect("id is suppose to be Some"),
					camera,
				})
			})
			.collect::<Result<_, _>>()?;

		let mut object_cameras = object_cameras
			.into_iter()
			.map(|(id, camera)| {
				Ok((
					id.object
						.whatever_context("camera has no object id or suffix")?,
					(id.suffix, camera),
				))
			})
			.collect::<Result<_, _>>()?;
		let mut entities = SlotMap::with_key();
		let mut placement_to_key = IndexMap::new();
		let mut layer_folders = IndexMap::new();
		Self::import_placement_file(
			&mut entities,
			&mut placement_to_key,
			&mut layer_folders,
			&mut object_cameras,
			&map_byml.borrow(),
		)?;

		Ok(Self {
			entities,
			root: Folder {
				name: stage_name.into(),
				scenarios: None,
				children: layer_folders.into_values().map(Node::Folder).collect(),
			},
			design_files,
			default_camera,
			entrance_cameras,
			global_cameras,
			unused_cameras: object_cameras,
			koopa_freerunning: Default::default(),
		})
	}

	fn import_placement_file(
		entities: &mut SlotMap<EntityKey, Entity>,
		placement_to_key: &mut IndexMap<(Cow<SmolStr>, Cow<SmolStr>), EntityKey>,
		layer_folders: &mut IndexMap<SmolStr, Folder>,
		object_cameras: &mut IndexMap<SmolStr, (Option<SmolStr>, Camera)>,
		placement_file: &Array,
	) -> Result<(), Whatever> {
		macro_rules! get {
			($on: expr, $key: literal, $as: ident, $what: literal) => {
				($on)
					.get($key)
					.whatever_context(concat!("entity is missing `", $key, "`"))?
					.$as()
					.whatever_context(concat!("`", $key, "` must be ", $what))?
			};
		}

		// keyed by object id, unit config name
		let mut placement_entities: IndexMap<(Cow<SmolStr>, Cow<SmolStr>), IndexMap<Scenario, _>> =
			IndexMap::new();
		let mut layer_scenarios = HashMap::new();
		ensure_whatever!(
			placement_file.len() <= 16,
			"up to 16 scenarios in one file are supported"
		);
		for (scenario_no, scenario) in placement_file.iter().enumerate() {
			let scenario_no = Scenario::new(scenario_no as u8 + 1);
			let scenario = scenario
				.as_dictionary()
				.whatever_context("scenario list map must be a dictionary")?
				.into_borrow();

			for (_list_name, list) in scenario.iter() {
				let list = list
					.as_array()
					.whatever_context("placement list must be an array")?
					.into_borrow();

				for object in list.iter() {
					let object_arc = object
						.as_dictionary()
						.whatever_context("object must be a dictionary")?;
					let object = object_arc.clone().into_borrow();

					let layer = get!(object, "LayerConfigName", as_string, "a string");

					layer_scenarios
						.entry(layer)
						.or_insert(Scenarios::new(0))
						.set(scenario_no);
					drop(object);

					let mut object_queue: Vec<_> = vec![object_arc];
					while let Some(object_arc) = object_queue.pop() {
						{
							let mut object = object_arc.borrow_mut();
							let id = object["Id"].unwrap_string();
							if id.starts_with("rail") {
								object["Id"] = Element::String(format!("_{id}_scen{}", scenario_no.0).into())
							}
						}
						let object = object_arc.borrow();
						let id = get!(object, "Id", as_string, "a string");
						let object_name = get!(object, "UnitConfigName", as_string, "a string");
						placement_entities
							.entry((Cow::Owned(id.clone()), Cow::Owned(object_name.clone())))
							.or_insert(IndexMap::new())
							// entities can appear more than once in a scenario and it will be fine
							.insert(scenario_no, object_arc.clone());

						let links = get!(object, "Links", as_dictionary, "a dictionary").into_borrow();
						for link in links.values() {
							let link = link
								.as_array()
								.whatever_context("linked object list is not an array")?
								.into_borrow();
							for linked in link.iter() {
								let object = linked
									.as_dictionary()
									.whatever_context("linked object must be a dictionary")?;
								object_queue.push(object);
							}
						}
					}
				}
			}
		}

		for (layer, scenarios) in layer_scenarios {
			let folder = layer_folders
				.entry(layer.clone())
				.or_insert_with(|| Folder {
					name: layer,
					scenarios: Some(scenarios),
					children: vec![],
				});
			if let Some(mut scenarios) = folder.scenarios {
				scenarios.set_all_from(scenarios);
			}
		}

		for ((id, object_name), scenario_objects) in &placement_entities {
			if placement_to_key.contains_key(&(Cow::Borrowed(&**id), Cow::Borrowed(&**object_name))) {
				continue;
			}
			// as we aren't looking at links yet, we can read a single object and
			// treat it as an entity.
			let (_scenario, entity_data) = scenario_objects
				.first()
				.expect("placement entity should have at least one scenario entity");
			let entity_data = entity_data.borrow();

			let unit_config =
				get!(entity_data, "UnitConfig", as_dictionary, "a dictionary").into_borrow();
			let factory_name = get!(unit_config, "ParameterConfigName", as_string, "a string");
			let display_name = get!(unit_config, "DisplayName", as_string, "a string");
			let model_name = entity_data
				.get("ModelName")
				.filter(|name| !name.is_null())
				.map(|el| {
					el.as_string()
						.whatever_context("model name must be a string")
				})
				.transpose()?;
			let comment = entity_data
				.get("comment")
				.and_then(|el| el.as_string())
				.unwrap_or_default();
			let transform = Self::read_transform(&entity_data, false)?;
			let display_transform = Self::read_transform(&unit_config, false)?;
			let cameras = object_cameras
				.extract_if(.., |key, _| *key == **id)
				.map(|(_, camera)| camera)
				.collect();
			let layer = get!(entity_data, "LayerConfigName", as_string, "a string");

			const NON_PARMETERS: &[&'static str] = &[
				"Id",
				"IsLinkDest",
				"LayerConfigName",
				"Links",
				"ModelName",
				"PlacementFileName",
				"Rotate",
				"Scale",
				"Translate",
				"UnitConfigName",
				"UnitConfig",
				"comment",
				// fully unimportant information
				"SrcUnitLayerList",
			];

			let mut params = IndexMap::new();
			for (param_name, param) in entity_data.iter() {
				if NON_PARMETERS.contains(&param_name.as_str()) {
					continue;
				}

				params.insert(param_name.clone().into(), Self::read_param(param)?);
			}
			let entity_key = entities.insert(Entity {
				id: id.clone().into_owned(),
				factory_name,
				object_name: object_name.clone().into_owned(),
				model_name,
				display_name,
				comment,
				cameras,
				transform,
				display_transform,
				links: Default::default(),
				params,
			});
			layer_folders
				.get_mut(&layer)
				.unwrap()
				.children
				.push(Node::Entity(entity_key));

			//
			placement_to_key.insert((id.clone(), object_name.clone()), entity_key);
		}

		for (entity_key, entity) in entities {
			let objects = placement_entities
				.get(&(
					Cow::Borrowed(&entity.id),
					Cow::Borrowed(&entity.object_name),
				))
				.expect("should have that entity already");
			for (_, object) in objects {
				let object = object.borrow();
				// get!(object, "Links", as_dictionary,);
				todo!("finish adding links")
			}
			// println!("object: {:?}");
		}
/*
A -> B
A, A/B
A: Common
B: Scenario_1
 */
		Ok(())
	}

	fn read_transform(transform: &Dictionary, display: bool) -> Result<Transform, Whatever> {
		const REGULAR: [&'static str; 3] = ["Translate", "Rotate", "Scale"];
		const DISPLAY: [&'static str; 3] = ["DisplayTranslate", "DisplayRotate", "DisplayScale"];
		let mut vecs = (0..3).map(|index| {
			transform
				.get(if display {
					DISPLAY[index]
				} else {
					REGULAR[index]
				})
				.map(|vec| {
					Vec3::new(
						vec.get_key_f32("X").unwrap_or_default(),
						vec.get_key_f32("Y").unwrap_or_default(),
						vec.get_key_f32("Z").unwrap_or_default(),
					)
				})
				.unwrap_or_default()
		});

		Ok(Transform {
			translation: vecs.next().unwrap(),
			rotation: vecs.next().unwrap(),
			scale: vecs.next().unwrap(),
		})
	}

	fn read_param(param: &Element) -> Result<Parameter, Whatever> {
		Ok(match param {
			Element::String(value) => Parameter::String(value.clone()),
			Element::Boolean(value) => Parameter::Boolean(*value),
			Element::I32(value) => Parameter::I32(*value),
			Element::F32(value) => Parameter::F32(*value),
			Element::Dictionary(value) => Parameter::Dictionary(
				value
					.borrow()
					.iter()
					.map(|(name, param)| Ok((name.clone(), Self::read_param(param)?)))
					.collect::<Result<_, _>>()?,
			),
			Element::Array(value) => Parameter::Array(
				value
					.borrow()
					.iter()
					.map(Self::read_param)
					.collect::<Result<_, _>>()?,
			),
			Element::Null => Parameter::Null,
			_ => whatever!("invalid camera param type {:?}", param.data_type()),
		})
	}

	fn import_camera(camera: &Element) -> Result<(CameraId, Camera), Whatever> {
		let camera = camera
			.as_dictionary()
			.whatever_context("expected camera ticket to be a dictionary")?
			.into_borrow();

		let factory_name = camera
			.get("Class")
			.whatever_context("failed to find camera class")?
			.as_dictionary()
			.whatever_context("expected camera class to be a dictionary")?
			.into_borrow()
			.get("Name")
			.whatever_context("camera factory name was not found")?
			.as_string()
			.whatever_context("camera factory name was not a string")?
			.into();

		let id = camera
			.get("Id")
			.whatever_context("failed to find camera id")?
			.as_dictionary()
			.whatever_context("expected camera id to be a dictionary")?
			.into_borrow();
		let object = id
			.get("ObjId")
			.map(|name| {
				name
					.as_string()
					.whatever_context("camera object id was not a string")
			})
			.transpose()?
			.into();
		let suffix = id
			.get("Suffix")
			.map(|name| {
				name
					.as_string()
					.whatever_context("camera suffix name was not a string")
			})
			.transpose()?
			.into();

		let param = camera
			.get("Param")
			.whatever_context("failed to find camera params")?;

		let parameters = match param {
			Element::Dictionary(params) => params
				.borrow()
				.iter()
				.map(|(name, param)| Ok((name.clone(), Self::read_param(param)?)))
				.collect::<Result<_, _>>()?,
			Element::Null => Default::default(),
			_ => whatever!("unexpected type for camera param element"),
		};

		Ok((
			CameraId { object, suffix },
			Camera {
				factory_name,
				parameters,
			},
		))
	}
}
