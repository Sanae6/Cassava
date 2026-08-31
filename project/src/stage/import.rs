use std::{fs, path::Path};

use cassava::{
	byaml::{self, Element},
	util::{ArcAtomicRefCellExt, BinaryReader},
};
use snafu::{OptionExt, ResultExt, Whatever, ensure_whatever, whatever};
use tracing::warn;

use crate::{
	DiskRef,
	stage::{Camera, CameraId, CameraParameter, Folder, Stage, SuffixCamera},
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

		map_byml.into_borrow()

		Ok(Self {
			entities: Default::default(),
			root: Folder {
				name: stage_name.into(),
				scenarios: None,
				children: vec![],
			},
			design_files,
			default_camera,
			entrance_cameras,
			global_cameras,
			unused_cameras: Default::default(),
			koopa_freerunning: Default::default(),
		})
	}

	fn import_list() -> Result<(), ()> {
		Ok(())
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

		fn read_param(param: &Element) -> Result<CameraParameter, Whatever> {
			Ok(match param {
				Element::String(value) => CameraParameter::String(value.clone()),
				Element::Boolean(value) => CameraParameter::Boolean(*value),
				Element::I32(value) => CameraParameter::I32(*value),
				Element::F32(value) => CameraParameter::F32(*value),
				Element::Dictionary(value) => CameraParameter::Dictionary(
					value
						.borrow()
						.iter()
						.map(|(name, param)| Ok((name.clone(), read_param(param)?)))
						.collect::<Result<_, _>>()?,
				),
				Element::Array(value) => CameraParameter::Array(
					value
						.borrow()
						.iter()
						.map(read_param)
						.collect::<Result<_, _>>()?,
				),
				_ => whatever!("invalid camera parm type {:?}", param.data_type()),
			})
		}
		let parameters = match param {
			Element::Dictionary(params) => params
				.borrow()
				.iter()
				.map(|(name, param)| Ok((name.clone(), read_param(param)?)))
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
