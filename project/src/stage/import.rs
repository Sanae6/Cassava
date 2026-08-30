use std::{fs, path::Path};

use cassava::{
	byaml::{self, Element},
	util::{ArcAtomicRefCellExt, BinaryReader},
};
use snafu::{OptionExt, ResultExt, Whatever, ensure_whatever, whatever};
use tracing::warn;

use crate::{
	DiskRef,
	stage::{Camera, Stage, StageParameter},
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

		let (default_camera, entrance_cameras, object_cameras) =
			if let Some(camera_param) = map_archive.get_file("CameraParam.byml") {
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
				let default_camera = Self::import_camera(&default_tickets[0])?;
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
							.map(Self::import_camera)
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
				(default_camera, entrance_cameras, object_cameras)
			} else {
				(
					Camera {
						factory_name: "制限付きフォロー".into(),
						suffix: None,
						parameters: Default::default(),
					},
					Default::default(),
					Default::default(),
				)
			};

		Ok(Self {
			objects: Default::default(),
			map_lists: Default::default(),
			sound_lists: Default::default(),
			design_lists: Default::default(),
			design_files,
			default_camera,
			entrance_cameras,
			unassigned_cameras: Vec::new(),
			koopa_freerunning: Default::default(),
		})
	}

	fn import_camera(camera: &Element) -> Result<Camera, Whatever> {
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

		let suffix = camera
			.get("Class")
			.whatever_context("failed to find camera id")?
			.as_dictionary()
			.whatever_context("expected camera id to be a dictionary")?
			.into_borrow()
			.get("Name")
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
				.map(|(name, param)| {
					Ok((
						name.clone(),
						match param {
							Element::String(value) => StageParameter::String(value.clone()),
							Element::Boolean(value) => StageParameter::Boolean(*value),
							Element::I32(value) => StageParameter::I32(*value),
							Element::F32(value) => StageParameter::F32(*value),
							_ => whatever!("invalid type {:?} for param {name}", param.data_type()),
						},
					))
				})
				.collect::<Result<_, _>>()?,
			Element::Null => Default::default(),
			_ => whatever!("unexpected type for camera param element"),
		};

		Ok(Camera {
			factory_name,
			suffix,
			parameters,
		})
	}
}
