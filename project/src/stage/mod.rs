mod import;

use std::{fs, path::Path};

use glam::Vec3;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use slotmap::{SlotMap, new_key_type};
use smol_str::SmolStr;
use snafu::{ResultExt, Whatever};

use crate::{DiskRef, ProjectAsset};

new_key_type! { pub struct ObjectKey; }

#[derive(Debug, Serialize, Deserialize)]
pub struct Stage {
	pub objects: SlotMap<ObjectKey, Object>,
	pub map_lists: IndexMap<SmolStr, Node>,
	pub sound_lists: IndexMap<SmolStr, Node>,
	pub design_lists: IndexMap<SmolStr, Node>,
	pub design_files: IndexMap<SmolStr, DiskRef<Vec<u8>>>,
	pub entrance_cameras: Vec<Camera>,
	pub default_camera: Camera,
	pub unassigned_cameras: Vec<(SmolStr, Camera)>,
	pub koopa_freerunning: (),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Node {
	Folder { name: String, children: Vec<Node> },
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
		// todo: what's in a rail?
	},
}

// TODO: statically parameterized cameras
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Camera {
	pub factory_name: SmolStr,
	pub suffix: Option<SmolStr>,
	pub parameters: IndexMap<SmolStr, StageParameter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StageParameter {
	String(SmolStr),
	Boolean(bool),
	I32(i32),
	F32(f32),
}

impl ProjectAsset for Stage {
	fn load(path: &Path) -> Result<Self, Whatever> {
		let data =
			fs::read_to_string(path.join("stage.ron")).whatever_context("failed to read stage file")?;
		ron::from_str(&data).whatever_context("failed to load stage file")
	}

	fn save(&self, path: &Path) -> Result<(), Whatever> {
		fs::create_dir_all(path).whatever_context("failed to ensure stage directory exists")?;
		let file = ron::to_string(self).whatever_context("failed to serialize stage file")?;
		println!("Save path: {path:?}");
		fs::write(path.join("stage.ron"), file).whatever_context("failed to write stage file")?;

		for (file_name, ele) in &self.design_files {
			ele
				.save(&path)
				.with_whatever_context(|_| format!("failed to write {file_name} design file"))?;
		}

		Ok(())
	}
}
