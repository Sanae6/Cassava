pub mod database;
mod import;

use std::{fs, path::Path};

use glam::Vec3;
use indexmap::IndexMap;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use slotmap::{SlotMap, new_key_type};
use smol_str::SmolStr;
use snafu::{ResultExt, Whatever};

use crate::{DiskRef, ProjectAsset};

new_key_type! { pub struct EntityKey; }

#[derive(Debug, Serialize, Deserialize)]
pub struct Stage {
	pub entities: SlotMap<EntityKey, Entity>,
	// pub map_lists: IndexMap<SmolStr, Vec<Node>>,
	// pub sound_lists: IndexMap<SmolStr, Vec<Node>>,
	// pub design_lists: IndexMap<SmolStr, Vec<Node>>,
	pub root: Folder,
	pub design_files: IndexMap<SmolStr, DiskRef<Vec<u8>>>,
	/// The lone `DefaultTickets` camera and its suffix (generally set to "Default")
	pub default_camera: SuffixCamera,
	// `StartTickets` cameras.
	pub entrance_cameras: Vec<SuffixCamera>,
	// `Tickets` cameras who are not bound to an entity
	pub global_cameras: Vec<SuffixCamera>,
	// `Tickets` cameras who are bound to an entity that does not exist in the stage (import only)
	pub unused_cameras: IndexMap<SmolStr, (Option<SmolStr>, Camera)>,
	pub koopa_freerunning: (),
	// todo: evaluate graphics stuff (aamp/graphicsarea/graphicsquality/etc.)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Node {
	Folder(Folder),
	Entity(EntityKey),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Folder {
	name: SmolStr,
	/// The scenarios that child nodes will be able to show up in.
	/// If the parent folder specifies the set of scenarios its children are present in,
	/// the current folder can opt out of being in particular scenarios.
	/// ```
	/// Village in {1,3,5}
	/// Village/HouseB in {1,5}
	/// {1,5} ⊆ {1,3,5}
	/// ```
	/// If the scenario is not specified on a root folder, it will default to all scenarios.
	scenarios: Option<Scenarios>,
	children: Vec<Node>,
}

/// A bit set of all scenarios that objects will show up on.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Scenarios(u16);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
	pub id: SmolStr,
	/// The name used by a factory to instantiate the entity. (`ParameterConfigName`)
	pub factory_name: SmolStr,
	/// The name of the object archive. This decides the placement file/list of the entity.
	/// This may be used or ignored by the actor. (`UnitConfigName`)
	pub object_name: SmolStr,
	/// The name of the model archive. This may be used or ignored by the actor. (`ModelName`)
	pub model_name: Option<SmolStr>,
	/// The name displayed to the user in the editor. (`DisplayName`)
	pub display_name: SmolStr,
	/// A comment displayed to the user in the editor.
	pub comment: SmolStr,
	/// Cameras attached to an object, identified by their suffix.
	pub cameras: IndexMap<SmolStr, Camera>,
	/// The transform of the object.
	pub transform: ObjectTransform,
	/// The display transform of the object, originating from the base transform
	pub display_transform: ObjectTransform,
	pub links: IndexMap<String, Vec<Linked>>,
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
pub enum Linked {
	/// A linked object.
	Object(EntityKey),
	/// A linked rail.
	Rail {
		// todo: what's in a rail?
	},
}

struct Rail {
	
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuffixCamera {
	/// The `Suffix` in a camera's `Id`
	pub suffix: SmolStr,
	pub camera: Camera,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CameraId {
	/// The `ObjId` in a camera's `Id`
	object: Option<SmolStr>,
	/// The `Suffix` in a camera's `Id`
	suffix: Option<SmolStr>,
}

// TODO: statically parameterized cameras
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Camera {
	/// The `Suffix` in a camera's `Id`
	pub factory_name: SmolStr,
	pub parameters: IndexMap<SmolStr, CameraParameter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CameraParameter {
	String(SmolStr),
	Boolean(bool),
	I32(i32),
	F32(f32),
	Dictionary(IndexMap<SmolStr, CameraParameter>),
	Array(Vec<CameraParameter>),
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

		let file = ron::ser::to_string_pretty(self, PrettyConfig::new().indentor("  "))
			.whatever_context("failed to serialize stage file")?;
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
