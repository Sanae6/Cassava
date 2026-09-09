use std::{
	borrow::Cow,
	collections::{HashMap, HashSet},
	fs,
	path::Path,
	sync::LazyLock,
};

use cassava::{
	byaml::{self, Array, Dictionary, Element},
	sarc,
	util::{ArcAtomicRefCellExt, BinaryReader},
};
use glam::Vec3;
use indexmap::{IndexMap, IndexSet};
use regex::Regex;
use slotmap::SlotMap;
use smol_str::SmolStr;
use snafu::{OptionExt, ResultExt, Whatever, ensure_whatever, whatever};
use tracing::warn;

use crate::{
	DiskRef,
	stage::{
		Camera, CameraId, Entity, EntityKey, Folder, KoopaFreerunning, LinkedEntity, Node, Parameter,
		Racer, RacerFrame, RacerFrameCap, RacerFrameEffect, RacerFrameFlags, RacerLevel, Scenario,
		Scenarios, Stage, SuffixCamera, Transform,
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
		let design_byml_name = format!("{stage_name}Design.byml");
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
		let (map_byml, map_archive) = {
			let szs = fs::read(map_path).whatever_context("failed to read stage's map archive")?;
			let sarc = load_szs(&szs)?;
			let byml = sarc
				.get_file(&format!("{stage_name}Map.byml"))
				.whatever_context("unable to find stage byml in stage's map archive")?;
			(
				byaml::Byml::read(&mut BinaryReader::new(&byml))
					.whatever_context("failed to read stage map byml")?,
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
				.filter(|(file_name, _)| *file_name != design_byml_name)
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
				.filter(|ticket| !ticket.is_null())
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
				.filter(|ticket| !ticket.is_null())
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
		const PLACEMENT_SUFFIXES: [&'static str; 3] = ["Map", "Sound", "Design"];
		for (index, byml) in [Some(map_byml), sound_byml, design_byml]
			.into_iter()
			.flatten()
			.enumerate()
		{
			tracing::debug!("reading placement file {}", PLACEMENT_SUFFIXES[index]);
			Self::import_placement_file(
				&mut entities,
				&mut placement_to_key,
				&mut layer_folders,
				&mut object_cameras,
				&byml.borrow(),
				PLACEMENT_SUFFIXES[index],
			)?;
		}

		let koopa_freerunning = Self::import_koopas(&map_archive)?;

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
			koopa_freerunning,
		})
	}

	fn import_placement_file(
		entities: &mut SlotMap<EntityKey, Entity>,
		placement_to_key: &mut IndexMap<(Cow<SmolStr>, Cow<SmolStr>), EntityKey>,
		layer_folders: &mut IndexMap<SmolStr, Folder>,
		object_cameras: &mut IndexMap<SmolStr, (Option<SmolStr>, Camera)>,
		placement_file: &Array,
		placement_suffix: &'static str,
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

					let mut object_queue: Vec<_> = vec![object_arc];
					while let Some(object_arc) = object_queue.pop() {
						{
							let mut object = object_arc.borrow_mut();
							let id = get!(object, "Id", as_string, "a string");
							if id.starts_with("rail") {
								// rails have non-unique ids between scenarios and placement files,
								// so we need to update them accordingly.
								object["Id"] =
									Element::String(format!("{}_{}_{id}", placement_suffix, scenario_no.0).into())
							}
							drop(object);
						}
						let object = object_arc.borrow();
						let id = object["Id"].unwrap_string();
						let object_name = get!(object, "UnitConfigName", as_string, "a string");
						let layer = get!(object, "LayerConfigName", as_string, "a string");
						layer_scenarios
							.entry(layer)
							.or_insert(Scenarios::new(0))
							.set(scenario_no);
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

		for (layer, scenarios) in &layer_scenarios {
			let folder = layer_folders
				.entry(layer.clone())
				.or_insert_with(|| Folder {
					name: layer.clone(),
					scenarios: Some(*scenarios),
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

		for (_, entity) in entities {
			let Some(objects) = placement_entities.get(&(
				Cow::Borrowed(&entity.id),
				Cow::Borrowed(&entity.object_name),
			)) else {
				tracing::trace!(
					"{}:{} doesn't appear in this file",
					entity.id,
					entity.object_name
				);
				continue;
			};

			let mut known_linked_objects: IndexMap<SmolStr, HashSet<EntityKey>> = IndexMap::new();
			for (_, object) in objects {
				let object = object.borrow_mut();
				let entity_layer = object
					.get("LayerConfigName")
					.expect("object was already traversed")
					.unwrap_string();
				let mut links = get!(object, "Links", as_dictionary, "a dictionary").into_borrow_mut();
				for (link_name, linked_objects) in links.drain(..) {
					let mut linked_objects = linked_objects
						.as_array()
						.whatever_context("linked objects must be an array")?
						.into_borrow_mut();

					let known_objects = known_linked_objects.entry(link_name.clone()).or_default();

					let linked_entities = entity
						.links
						.entry(link_name.into())
						.or_insert(IndexSet::new());

					for linked_object in linked_objects.drain(..) {
						let linked_object = linked_object
							.as_dictionary()
							.whatever_context("linked object must be a dictionary")?
							.into_borrow();

						let id = linked_object
							.get("Id")
							.expect("linked object was already traversed")
							.unwrap_string();
						let object_name = linked_object
							.get("UnitConfigName")
							.expect("linked object was already traversed")
							.unwrap_string();
						let linked_entity_key = placement_to_key
							.get(&(Cow::Borrowed(&id), Cow::Borrowed(&object_name)))
							.expect("should have that entity already");

						if !known_objects.insert(*linked_entity_key) {
							continue;
						}

						let linked_entity_layer = linked_object
							.get("LayerConfigName")
							.expect("linked object was already traversed")
							.unwrap_string();

						linked_entities.insert(LinkedEntity {
							scenarios: layer_scenarios
								.get(&linked_entity_layer)
								.filter(|_| linked_entity_layer != entity_layer)
								.cloned(),
							entity: *linked_entity_key,
						});
					}
				}
			}
		}
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

	fn import_koopas(map_archive: &sarc::Reader) -> Result<KoopaFreerunning, Whatever> {
		static KFR_REGEX: LazyLock<Regex> =
			LazyLock::new(|| Regex::new(r"(\d+)_(\d|x)_(\d+)").expect("regex should be good"));

		let mut scenarios: IndexMap<Scenario, IndexMap<u8, IndexMap<RacerLevel, Racer>>> =
			IndexMap::new();
		for (file_name, file) in map_archive.files() {
			let Some(captures) = KFR_REGEX.captures(file_name) else {
				continue;
			};
			let scenario_no = captures
				.get(1)
				.expect("regex failed")
				.as_str()
				.parse()
				.map(Scenario)
				.expect("scenario should be a number");
			let level = captures.get(2).expect("regex failed").as_str();
			let level = if level == "x" {
				RacerLevel::Unspecified
			} else {
				RacerLevel::Level(level.parse().expect("scenario should be a number"))
			};
			let racer_index: u8 = captures
				.get(3)
				.expect("regex failed")
				.as_str()
				.parse()
				.expect("scenario should be a number");

			let racer = byaml::Byml::read(&mut BinaryReader::new(file))
				.whatever_context("failed to read koopa racer file")?
				.as_dictionary()
				.whatever_context("expected koopa racer to be a dictionary")?
				.into_borrow();
			let capture_name = racer
				.get("HackName")
				.whatever_context("hack name is missing")?
				.as_string()
				.map(|value| value.to_string());
			fn string_array(
				racer: &Dictionary,
				key: &'static str,
				expected: &'static str,
			) -> Result<Vec<SmolStr>, Whatever> {
				Ok(
					racer
						.get(key)
						.map(|el| {
							el.as_array()
								.map(|a| {
									a.borrow()
										.iter()
										.map(|l| l.as_string())
										.collect::<Option<Vec<SmolStr>>>()
								})
								.unwrap_or(Some(vec![]))
								.whatever_context(expected)
						})
						.transpose()?
						.unwrap_or_default(),
				)
			}
			let action = string_array(&racer, "ActionName", "expected action name to be a string")?;
			let action_cap = string_array(
				&racer,
				"ActionNameCap",
				"expected cap action name to be a string",
			)?;
			let material_codes = string_array(
				&racer,
				"MaterialCode",
				"expected material code to be a string",
			)?;
			let data = racer
				.get("DataArray")
				.whatever_context("data array is missing")?
				.as_array()
				.whatever_context("expected data array to be an array")?
				.into_borrow();

			let mut frames = Vec::with_capacity(data.len());
			for frame in data.iter() {
				let frame = frame
					.as_array()
					.whatever_context("frame data must be an array")?
					.into_borrow();
				ensure_whatever!(frame.len() >= 10, "frame's length must be ten or more");

				let translation = Vec3::new(
					frame[0]
						.as_f32()
						.whatever_context("translation.x is not an f32")?,
					frame[1]
						.as_f32()
						.whatever_context("translation.y is not an f32")?,
					frame[2]
						.as_f32()
						.whatever_context("translation.z is not an f32")?,
				);
				let rotation = Vec3::new(
					frame[3]
						.as_f32()
						.whatever_context("rotation.x is not an f32")?,
					frame[4]
						.as_f32()
						.whatever_context("rotation.y is not an f32")?,
					frame[5]
						.as_f32()
						.whatever_context("rotation.z is not an f32")?,
				);
				let animation = frame[6]
					.as_i32()
					.whatever_context("animation index is not an i32")?;
				let animation = action
					.get(animation as usize)
					.cloned()
					.whatever_context("animation index out of bounds")?;
				let animation_time = frame[7]
					.as_f32()
					.whatever_context("animation time is not an f32")?;
				let flags = frame[8].as_i32().whatever_context("")?;
				let flags = RacerFrameFlags::from_bits(flags as _);
				let material_code = frame[9].as_i32().whatever_context("")?;
				let material_code = material_codes
					.get(material_code as usize)
					.cloned()
					.whatever_context("material code not found")?;
				let (cap, effect) = frame
					.get(10)
					.map(|extras: &Element| {
						let extras = extras
							.as_dictionary()
							.whatever_context("extra hash was not a dictionary")?
							.into_borrow();

						let cap = extras
							.get("Cap")
							.and_then(|cap| cap.as_array())
							.map(|cap| {
								let cap = cap.into_borrow();
								ensure_whatever!(cap.len() >= 9, "frame's cap length must be nine or more");
								let translation = Vec3::new(
									cap[0]
										.as_f32()
										.whatever_context("cap translation x is not an f32")?,
									cap[1]
										.as_f32()
										.whatever_context("cap translation y is not an f32")?,
									cap[2]
										.as_f32()
										.whatever_context("cap translation z is not an f32")?,
								);
								let rotation = Vec3::new(
									cap[3]
										.as_f32()
										.whatever_context("cap rotation x is not an f32")?,
									cap[4]
										.as_f32()
										.whatever_context("cap rotation y is not an f32")?,
									cap[5]
										.as_f32()
										.whatever_context("cap rotation z is not an f32")?,
								);
								let animation = cap[6]
									.as_i32()
									.whatever_context("cap animation index is not an i32")?;
								let animation = action_cap
									.get(animation as usize)
									.cloned()
									.whatever_context("cap animation index out of bounds")?;
								let animation_time = cap[7]
									.as_f32()
									.whatever_context("cap animation time is not an f32")?;
								Ok(RacerFrameCap {
									translation,
									rotation,
									animation,
									animation_time,
								})
							})
							.transpose()?;
						let effect = extras
							.get("Effect")
							.and_then(|effect| effect.as_array())
							.map(|effect| {
								let effect = effect.into_borrow();
								ensure_whatever!(
									effect.len() >= 6,
									"frame effect's length must be six or more"
								);
								let translation = Vec3::new(
									effect[0]
										.as_f32()
										.whatever_context("effect translation x is not an f32")?,
									effect[1]
										.as_f32()
										.whatever_context("effect translation y is not an f32")?,
									effect[2]
										.as_f32()
										.whatever_context("effect translation z is not an f32")?,
								);
								let rotation = Vec3::new(
									effect[3]
										.as_f32()
										.whatever_context("effect rotation x is not an f32")?,
									effect[4]
										.as_f32()
										.whatever_context("effect rotation y is not an f32")?,
									effect[5]
										.as_f32()
										.whatever_context("effect rotation z is not an f32")?,
								);
								Ok(RacerFrameEffect {
									translation,
									rotation,
								})
							})
							.transpose()?;

						Ok((cap, effect))
					})
					.transpose()?
					.unzip();

				frames.push(RacerFrame {
					translation,
					rotation,
					animation,
					animation_time,
					flags,
					material_code,
					cap: cap.flatten(),
					effect: effect.flatten(),
				})
			}

			let racer = Racer {
				capture_name,
				frames,
			};

			scenarios
				.entry(scenario_no)
				.or_default()
				.entry(racer_index)
				.or_default()
				.insert(level, racer);
		}

		Ok(KoopaFreerunning { scenarios })
	}
}
