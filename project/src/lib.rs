use std::{
	fmt::Debug,
	fs,
	path::{Path, PathBuf},
};

use indexmap::IndexMap;
use relative_path::RelativePathBuf;
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Whatever};

use crate::stage::Stage;

pub use disk_ref::{DiskRef, ProjectAsset};

mod disk_ref;
pub mod stage;
mod util;

#[derive(Debug, Serialize, Deserialize)]
pub struct ALProject {
	#[serde(skip)]
	root: PathBuf,
	pub stages: IndexMap<String, DiskRef<Stage>>,
	objects: IndexMap<String, ()>,
	effects: IndexMap<String, String>,
	layouts: IndexMap<String, String>,
	event_flows: IndexMap<String, String>,
	text: IndexMap<String, String>,
	shaders: IndexMap<String, String>,
	system: IndexMap<String, String>,
}

impl ALProject {
	const PROJECT_FILE: &str = "cassava-project.ron";

	pub fn create(directory: &Path) -> Result<Self, Whatever> {
		fs::create_dir_all(directory.join("stages"))
			.with_whatever_context(|err| format!("failed to create project: {err}"))?;

		let project = Self {
			root: directory.to_owned(),
			stages: IndexMap::new(),
			objects: IndexMap::new(),
			effects: IndexMap::new(),
			layouts: IndexMap::new(),
			event_flows: IndexMap::new(),
			text: IndexMap::new(),
			shaders: IndexMap::new(),
			system: IndexMap::new(),
		};
		fs::write(
			directory.join(Self::PROJECT_FILE),
			ron::to_string(&project).expect("serialization failure"),
		)
		.with_whatever_context(|err| format!("failed to create project file: {err}"))?;
		Ok(project)
	}

	pub fn load(directory: &Path) -> Result<ALProject, Whatever> {
		let data = fs::read_to_string(directory.join(Self::PROJECT_FILE))
			.with_whatever_context(|err| format!("failed to read project file: {err}"))?;

		ron::from_str(&data)
			.with_whatever_context(|err| format!("project file could not be read as toml: {err}"))
	}

	fn save_file<T: ProjectAsset>(&self, file_ref: &DiskRef<T>) -> Result<(), Whatever> {
		file_ref.save(&self.root)
	}

	pub fn import_stage(
		&mut self,
		romfs_root: &Path,
		stage_name: &str,
	) -> Result<&mut Stage, Whatever> {
		let stage_dir_rel: RelativePathBuf = format!("stages/{stage_name}").into();
		let stage = Stage::import(romfs_root, stage_name, &stage_dir_rel)?;

		let file_ref = DiskRef {
			path: stage_dir_rel,
			value: Some(stage),
		};

		self.save_file(&file_ref)?;

		self.stages.insert(stage_name.to_owned(), file_ref);

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
