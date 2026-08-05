use std::{fs, path::Path, sync::Arc};

use cassava::{byaml, util::BinaryReader};
use glam::Vec3;
use indexmap::IndexMap;
use relative_path::RelativePath;
use serde::{Deserialize, Serialize};
use slotmap::{SlotMap, new_key_type};
use snafu::{OptionExt, ResultExt, Whatever, ensure_whatever, whatever};
use tracing::warn;

use crate::{DiskRef, ProjectAsset, util::load_szs};

new_key_type! { pub struct ObjectKey; }

#[derive(Debug, Serialize, Deserialize)]
pub struct Stage {
	pub objects: SlotMap<ObjectKey, Object>,
	pub map_lists: IndexMap<String, Node>,
	pub sound_lists: IndexMap<String, Node>,
	pub design_lists: IndexMap<String, Node>,
	pub layers: Vec<String>,
	pub design_files: IndexMap<String, DiskRef<Vec<u8>>>,
	pub entrance_cameras: Vec<Camera>,
	pub koopa_freerunning: (),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Node {
	Folder { children: Vec<Node> },
	Object(ObjectKey),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Object {
	pub id: String,
	pub model_name: String,
	pub display_name: String,
	pub comment: String,
	pub camera: Option<Camera>,
	pub transform: ObjectTransform,
	pub display_transform: ObjectTransform,
	pub links: IndexMap<String, Vec<LinkedObject>>,
	pub params: IndexMap<String, StageParameter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectTransform {
	/// The translation of an object from the origin.
	pub translation: Vec3,
	/// The rotation of an object in Euler angles.
	pub rotation: Vec3,
	/// The scale of an object.
	pub scale: Vec3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LinkedObject {
	Object(ObjectKey),
	Rail {
		// todo: what's in a rail in byml form?
	},
}

// TODO: statically parameterized cameras
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Camera {
	pub factory_name: String,
	pub parameters: IndexMap<String, StageParameter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StageParameter {
	String(String),
	I32(i32),
	F32(f32),
}

impl Stage {
	pub fn import(
		romfs_root: &Path,
		stage_name: &str,
		save_dir_rel: &RelativePath,
	) -> Result<Self, Whatever> {
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
					.get_file(&format!("{stage_name}.byml"))
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
					.get_file(&format!("{stage_name}.byml"))
					.whatever_context("unable to find stage byml in stage's design archive")?;
				Ok((
					byaml::Byml::read(&mut BinaryReader::new(&byml))
						.whatever_context("failed to read stage design byml")?,
					sarc,
				))
			})
			.transpose()?
			.unzip();
		let design_byml_name = format!("{stage_name}.byml");
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

		fn ensure_dictionary(byml: byaml::Byml) -> Result<byaml::Dictionary, Whatever> {
			let root_node = byml
				.root_node
				.whatever_context("no root node in byml file")?;
			match root_node {
				byaml::Container::Dictionary(value) => Ok(Arc::unwrap_or_clone(value).into_inner()),
				byaml::Container::Array(_) => whatever!("byml file's root node is an array"),
			}
		}
		let (map_byml, sound_byml, design_byml) = (
			ensure_dictionary(map_byml)?,
			sound_byml.map(ensure_dictionary).transpose()?,
			design_byml.map(ensure_dictionary).transpose()?,
		);

		let design_files = if let Some(archive) = design_archive {
			archive
				.files()
				.map(|(file_name, data)| {
					(
						file_name.to_owned(),
						DiskRef {
							path: save_dir_rel.join(file_name),
							value: Some(data.to_vec()),
						},
					)
				})
				.collect()
		} else {
			Default::default()
		};

		map_archive.get_file("CameraParam.byml");

		Ok(Self {
			objects: Default::default(),
			map_lists: Default::default(),
			sound_lists: Default::default(),
			design_lists: Default::default(),
			layers: Default::default(),
			design_files,
			entrance_cameras: Default::default(),
			koopa_freerunning: Default::default(),
		})
	}
}

impl ProjectAsset for Stage {
	fn load(path: &Path) -> Result<Self, Whatever> {
		let data =
			fs::read_to_string(path.join("stage.ron")).whatever_context("failed to read stage file")?;
		ron::from_str(&data).whatever_context("failed to load stage file")
	}

	fn save(&self, path: &Path) -> Result<(), Whatever> {
		let file = ron::to_string(self).whatever_context("failed to serialize stage file")?;
		fs::write(path.join("stage.ron"), file).whatever_context("failed to write stage file")?;

		for (file_name, ele) in &self.design_files {
			ele
				.save(path)
				.with_whatever_context(|_| format!("failed to write {file_name} design file"))?;
		}

		Ok(())
	}
}
