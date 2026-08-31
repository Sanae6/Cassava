//! The stage file format, used to build [stage archives](https://smo.wiki/Stage_archive) for the final build.

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
	/// The map of all entities in the stage.
	pub entities: SlotMap<EntityKey, Entity>,
	/// The root of the stage hierarchy.
	pub root: Folder,
	/// The lone `DefaultTickets` camera and its suffix (generally set to "Default")
	pub default_camera: SuffixCamera,
	/// `StartTickets` cameras, used as the initial camera ticket before the player moves.
	pub entrance_cameras: Vec<SuffixCamera>,
	/// `Tickets` cameras that are not bound to an entity.
	pub global_cameras: Vec<SuffixCamera>,
	/// `Tickets` cameras that are bound to an entity that does not exist in the stage (import only)
	pub unused_cameras: IndexMap<SmolStr, (Option<SmolStr>, Camera)>,
	/// Koopa Freerunning
	pub koopa_freerunning: (),
	// todo: evaluate everything that goes into design archives (aamp/graphicsarea/graphicsquality/etc.)
	// this is currently a temporary solution storing for graphics files like cubemaps and
	// other aamp/byml files
	pub design_files: IndexMap<SmolStr, DiskRef<Vec<u8>>>,
}

/// A folder or an entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Node {
	Folder(Folder),
	Entity(EntityKey),
}

/// An organisational unit. Defines the scenarios in which entities will be visible.
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

/// A bit set of all scenarios that entities will show up on.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Scenarios(u16);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
	/// An entity's "unique" id. This can be used by the game in certain hardcoded situations,
	/// and is not actually unique. See [note 2](https://smo.wiki/w/index.php?title=Stage_archive&oldid=12386#cite_note-leftover-2)
	/// for more information about id uniqueness.
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
	/// Cameras attached to an entity, identified by their suffix.
	pub cameras: IndexMap<SmolStr, Camera>,
	/// The transform of the entity.
	pub transform: Transform,
	/// The display transform of the entity, originating from the base transform
	pub display_transform: Transform,
	/// Sets of linked objects identified by a link name.
	pub links: IndexMap<String, Vec<Linked>>,
	/// Extra parameters used by an actor.
	pub params: IndexMap<String, EntityParameter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transform {
	/// The translation of an entity from the origin.
	pub translation: Vec3,
	/// The rotation of an entity in Euler angles.
	pub rotation: Vec3,
	/// The scale of an entity.
	pub scale: Vec3,
}

/// A linked
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Linked {
	/// A linked entity.
	Object(EntityKey),
	/// A linked rail.
	Rail(Rail),
}

/// A user-defined curve for actors to sample points from.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Rail {
	/// A rail that uses linear interpolation to interpolate between points.
	Linear {
		/// Whether the rail will interpolate between the first and last points.
		is_closed: bool,
		/// The list of points used in the line.
		points: Vec<RailPoint>,
	},
	/// A rail that uses a bezier curve to interpolate between points.
	Bezier {
		/// Whether the rail will interpolate between the first and last points.
		is_closed: bool,
		/// The list of points and handles used in the curve.
		points: Vec<BezierRailPoint>,
	},
}

/// A point in a rail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RailPoint {
	/// The transform of the point.
	pub transform: Transform,
	/// Extra parameters used by actors.
	pub parameters: IndexMap<SmolStr, EntityParameter>,
}

/// A bezier curve point in a rail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BezierRailPoint {
	/// The point that the curve will go through.
	pub point: RailPoint,
	/// The handle used for interpolation between the previous and current points.
	pub incoming_handle: Vec3,
	/// The handle used for interpolation between the current and next points.
	pub outgoing_handle: Vec3,
}

/// A camera with a suffix.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuffixCamera {
	/// The `Suffix` in a camera's `Id`
	pub suffix: SmolStr,
	/// The camera.
	pub camera: Camera,
}

/// Partially defined camera id
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CameraId {
	/// The `ObjId` in a camera's `Id`
	object: Option<SmolStr>,
	/// The `Suffix` in a camera's `Id`
	suffix: Option<SmolStr>,
}

// TODO: statically parameterized cameras
/// A camera ticket, used for defining how cameras should behave.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Camera {
	/// The `Suffix` in a camera's `Id`
	pub factory_name: SmolStr,
	/// Extra parameters used by a camera poser.
	pub parameters: IndexMap<SmolStr, CameraParameter>,
}

/// Essential types and values of a camera parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CameraParameter {
	String(SmolStr),
	Boolean(bool),
	I32(i32),
	F32(f32),
	Dictionary(IndexMap<SmolStr, CameraParameter>),
	Array(Vec<CameraParameter>),
}

/// Essential types and values of an entity's parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EntityParameter {
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
