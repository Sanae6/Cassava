#![allow(semicolon_in_expressions_from_non_local_macros)]
use std::{
	fmt::Debug,
	fs,
	path::{Path, PathBuf},
};

use chrono::{DateTime, Local, Utc};
use indexmap::IndexMap;
use relative_path::RelativePathBuf;
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Whatever};

use crate::{stage::{Stage, database::EntityDatabase}, system::SystemData};

pub use disk_ref::{DiskRef, ProjectAsset};
pub use smol_str;

mod disk_ref;
pub mod stage;
mod util;
pub mod system;

#[derive(Debug, Serialize, Deserialize)]
pub struct ALProject {
	#[serde(skip)]
	root: PathBuf,
	pub name: String,
	pub last_opened: DateTime<Utc>,
	pub stages: IndexMap<String, DiskRef<Stage>>,
	pub entity_database: DiskRef<EntityDatabase>,
	objects: IndexMap<String, ()>,
	effects: IndexMap<String, ()>,
	layouts: IndexMap<String, ()>,
	event_flows: IndexMap<String, ()>,
	text: IndexMap<String, ()>,
	shaders: IndexMap<String, ()>,
	system: DiskRef<SystemData>,
}

impl ALProject {
	const PROJECT_FILE: &str = "cassava-project.ron";

	pub fn create(directory: &Path, name: &str) -> Result<Self, Whatever> {
		fs::create_dir_all(directory.join("stages"))
			.with_whatever_context(|err| format!("failed to create project: {err}"))?;

		let project = Self {
			name: name.to_owned(),
			last_opened: Local::now().to_utc(),
			root: directory.to_owned(),
			stages: IndexMap::new(),
			entity_database: DiskRef {
				path: RelativePathBuf::new(),
				value: Some(EntityDatabase::default()),
			},
			objects: IndexMap::new(),
			effects: IndexMap::new(),
			layouts: IndexMap::new(),
			event_flows: IndexMap::new(),
			text: IndexMap::new(),
			shaders: IndexMap::new(),
			system: DiskRef {
				path: RelativePathBuf::new(),
				value: Some(SystemData::default()),
			},
		};
		fs::write(
			directory.join(Self::PROJECT_FILE),
			ron::to_string(&project).expect("serialization failure"),
		)
		.with_whatever_context(|err| format!("failed to create project file: {err}"))?;
		Ok(project)
	}

	pub fn peek(directory: &Path) -> Result<ALProjectPeek, Whatever> {
		let data = fs::read_to_string(directory.join(Self::PROJECT_FILE))
			.with_whatever_context(|err| format!("failed to read project file: {err}"))?;

		ron::from_str(&data)
			.with_whatever_context(|err| format!("project file could not be read as ron: {err}"))
	}

	pub fn load(directory: &Path) -> Result<ALProject, Whatever> {
		let data = fs::read_to_string(directory.join(Self::PROJECT_FILE))
			.with_whatever_context(|err| format!("failed to read project file: {err}"))?;

		let mut project: ALProject = ron::from_str(&data)
			.with_whatever_context(|err| format!("project file could not be read as ron: {err}"))?;

		project.root = directory.to_owned();

		Ok(project)
	}

	pub fn save(&self) -> Result<(), Whatever> {
		fs::write(
			self.root.join(Self::PROJECT_FILE),
			ron::to_string(&self).expect("serialization failure"),
		)
		.with_whatever_context(|err| format!("failed to create project file: {err}"))?;

		Ok(())
	}

	fn save_file<T: ProjectAsset>(&self, file_ref: &DiskRef<T>) -> Result<(), Whatever> {
		file_ref.save(&self.root)
	}

	pub fn import_stage(
		&mut self,
		romfs_root: &Path,
		stage_name: &str,
	) -> Result<&mut Stage, Whatever> {
		let stage = Stage::import(romfs_root, stage_name)?;

		let file_ref = DiskRef {
			path: format!("stages/{stage_name}").into(),
			value: Some(stage),
		};

		self.save_file(&file_ref)?;

		self.stages.insert(stage_name.to_owned(), file_ref);

		self.save()?;

		Ok(
			self
				.stages
				.get_mut(stage_name)
				.unwrap()
				.value
				.as_mut()
				.unwrap(),
		)
	}
}

#[derive(Deserialize)]
pub struct ALProjectPeek {
	pub name: String,
	pub last_opened: DateTime<Utc>,
}
