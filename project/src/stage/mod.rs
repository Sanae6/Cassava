//! The stage file format, used to build [stage archives](https://smo.wiki/Stage_archive) for the final build.

pub mod database;
mod import;

use std::{fs, path::Path};

use bitfield_struct::bitfield;
use glam::{EulerRot, Mat4, Quat, Vec3};
use indexmap::{IndexMap, IndexSet};
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
	/// [Koopa Freerunning](https://smo.wiki/Koopa_Freerunning) racers' recordings.
	pub koopa_freerunning: KoopaFreerunning,
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
	pub name: SmolStr,
	/// The scenarios that child nodes will be able to show up in.
	/// If the parent folder specifies the set of scenarios its children are present in,
	/// the current folder can opt out of being in particular scenarios.
	/// ```
	/// Village in {1,3,5}
	/// Village/HouseB in {1,5}
	/// {1,5} ⊆ {1,3,5}
	/// ```
	/// If the scenario is not specified on a root folder, it will default to all scenarios.
	pub scenarios: Option<Scenarios>,
	pub children: Vec<Node>,
}

/// Type-safe wrapper around a scenario number.
#[derive(Clone, Copy, Debug, Hash, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Scenario(u8);
impl Scenario {
	pub fn new(index: u8) -> Self {
		assert!((1..16).contains(&index), "index must be in range 1..=15");
		Self(index)
	}
	pub fn value(&self) -> u8 {
		self.0
	}
}

/// The set of all scenarios that entities will show up on.
#[derive(Clone, Copy, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct Scenarios(u16);
impl std::fmt::Debug for Scenarios {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_tuple("Scenarios")
			.field(&self.iter().collect::<Vec<_>>())
			.finish()
	}
}
impl Scenarios {
	pub fn new(set: u16) -> Self {
		Self(set)
	}
	pub fn set(&mut self, index: Scenario) {
		self.0 |= 1 << (index.0 - 1);
	}
	pub fn test(&self, index: Scenario) -> bool {
		self.0 & 1 << (index.0 - 1) != 0
	}
	pub fn iter(&self) -> impl Iterator<Item = Scenario> {
		(1..16)
			.filter(|index| self.test(Scenario::new(*index)))
			.map(Scenario)
	}
	pub fn set_all_from(&mut self, other: Scenarios) {
		self.0 |= other.0;
	}
}

/// A single stage entity. This can correspond with an actor, area, rail, a map icon,
/// or more.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
	/// An entity's "unique" id. This can be used by the game in certain hardcoded situations,
	/// and is not actually unique. See [note 1](https://smo.wiki/w/index.php?title=Stage_archive&oldid=12386#cite_note-1)
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
	/// Cameras attached to an entity, with an optional suffix.
	pub cameras: Vec<(Option<SmolStr>, Camera)>,
	/// The transform of the entity.
	pub transform: Transform,
	/// The display transform of the entity, originating from the base transform
	pub display_transform: Transform,
	/// Sets of linked entities identified by a link name.
	pub links: IndexMap<String, IndexSet<LinkedEntity>>,
	/// Extra parameters used by an actor.
	pub params: IndexMap<String, Parameter>,
}

pub const EULER_ROT: EulerRot = glam::EulerRot::XYZEx;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Transform {
	/// The translation of an entity from the origin.
	pub translation: Vec3,
	/// The rotation of an entity in Euler angles.
	pub rotation: Vec3,
	/// The scale of an entity.
	pub scale: Vec3,
}

impl Transform {
	pub fn into_mat4(self) -> Mat4 {
		Mat4::from_scale_rotation_translation(
			self.scale,
			Quat::from_euler(
				EULER_ROT,
				self.rotation.x.to_radians(),
				self.rotation.y.to_radians(),
				self.rotation.z.to_radians(),
			),
			self.translation,
		)
	}
}

impl Default for Transform {
	fn default() -> Self {
		Self {
			translation: Default::default(),
			rotation: Default::default(),
			scale: Vec3::ONE,
		}
	}
}

#[derive(Debug, Clone, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub struct LinkedEntity {
	// if an entity has a more visible set than its parent,
	// it *should* be displayed as an issue! can be solved by duplicating
	// the linked object in scenarios where it gets put onto a list,
	// or solved by making the parent appear in every scenario that the
	// child can show up in.
	// ideally show as gray on both link source and linked entity with a little warning icon :3
	scenarios: Option<Scenarios>,
	entity: EntityKey,
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

/// A camera ticket, used for defining how cameras should behave.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Camera {
	/// The name used by a factory to instantiate the camera. (`Class/Name`)
	pub factory_name: SmolStr,
	/// Extra parameters used by a camera poser.
	pub parameters: IndexMap<SmolStr, Parameter>,
}

/// Describes an entity or camera's parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Parameter {
	String(SmolStr),
	Boolean(bool),
	I32(i32),
	F32(f32),
	Dictionary(IndexMap<SmolStr, Parameter>),
	Array(Vec<Parameter>),
	Null,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KoopaFreerunning {
	pub scenarios: IndexMap<Scenario, IndexMap<u8, IndexMap<RacerLevel, Racer>>>,
}

#[derive(Debug, Clone, Copy, Hash, Serialize, Deserialize, PartialEq, Eq)]
pub enum RacerLevel {
	/// A racer which is available on every difficulty
	Unspecified,
	/// A racer which is available on a specific difficulty
	Level(u8),
}

/// An individual KFR racer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Racer {
	pub capture_name: Option<String>,
	pub frames: Vec<RacerFrame>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RacerFrame {
	pub translation: Vec3,
	pub rotation: Vec3,
	pub animation: SmolStr,
	pub animation_time: f32,
	pub flags: RacerFrameFlags,
	pub material_code: SmolStr,
	pub cap: Option<RacerFrameCap>,
	pub effect: Option<RacerFrameEffect>,
}

#[bitfield(u16)]
#[derive(Serialize, Deserialize)]
pub struct RacerFrameFlags {
	#[bits(1)]
	_unused: u8,
	pub landing_hit_reaction: bool,
	pub running_land_hit_reaction: bool,
	#[bits(3)]
	pub unknown: u8,
	pub splash: bool,
	pub big_splash: bool,
	pub wet: bool,
	pub water: bool,
	pub puddle: bool,
	#[bits(5)]
	_unused: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RacerFrameCap {
	translation: Vec3,
	rotation: Vec3,
	animation: SmolStr,
	animation_time: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RacerFrameEffect {
	translation: Vec3,
	rotation: Vec3,
}

impl ProjectAsset for Stage {
	fn load(path: &Path) -> Result<Self, Whatever> {
		let data = fs::OpenOptions::new()
			.read(true)
			.open(path.join("stage.ron.zstd"))
			.whatever_context("failed to open stage file")?;
		let data = zstd::stream::read::Decoder::new(data)
			.whatever_context("failed to start decompressing stage file")?;
		ron::de::from_reader(data).whatever_context("failed to load stage file")
	}

	fn save(&self, path: &Path) -> Result<(), Whatever> {
		fs::create_dir_all(path).whatever_context("failed to ensure stage directory exists")?;

		let file = fs::OpenOptions::new()
			.create(true)
			.write(true)
			.open(path.join("stage.ron.zstd"))
			.whatever_context("failed to open stage file")?;
		let mut file = zstd::stream::Encoder::new(file, 0)
			.whatever_context("failed to compress stage file")?
			.auto_finish();
		ron::Options::default()
			.to_io_writer_pretty(&mut file, self, PrettyConfig::new().indentor("  "))
			.whatever_context("failed to serialize stage file")?;

		for (file_name, ele) in &self.design_files {
			ele
				.save(&path)
				.with_whatever_context(|_| format!("failed to write {file_name} design file"))?;
		}

		Ok(())
	}
}
