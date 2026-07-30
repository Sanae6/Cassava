use std::{
	collections::BTreeMap,
	fmt::Debug,
	fs,
	path::{Path, PathBuf},
	rc::Rc,
};

use cassava::{byaml, util::BinaryReader};
use relative_path::RelativePathBuf;
use serde::{Deserialize, Serialize};
use snafu::{OptionExt, ResultExt, Whatever, ensure_whatever, whatever};

use crate::{stage::Stage, util::load_szs};

pub use disk_ref::{DiskRef, ProjectAsset};

mod disk_ref;
pub mod stage;
mod util;

#[derive(Debug, Serialize, Deserialize)]
pub struct ALProject {
	#[serde(skip)]
	root: PathBuf,
	pub stages: BTreeMap<String, DiskRef<Stage>>,
	objects: BTreeMap<String, ()>,
	effects: BTreeMap<String, String>,
	layouts: BTreeMap<String, String>,
	event_flows: BTreeMap<String, String>,
	text: BTreeMap<String, String>,
	shaders: BTreeMap<String, String>,
	system: BTreeMap<String, String>,
}

impl ALProject {
	const PROJECT_FILE: &str = "cassava-project.toml";

	pub fn create(directory: &Path) -> Result<Self, Whatever> {
		fs::create_dir_all(directory.join("stages"))
			.with_whatever_context(|err| format!("failed to create project: {err}"))?;

		let project = Self {
			root: directory.to_owned(),
			stages: BTreeMap::new(),
			objects: BTreeMap::new(),
			effects: BTreeMap::new(),
			layouts: BTreeMap::new(),
			event_flows: BTreeMap::new(),
			text: BTreeMap::new(),
			shaders: BTreeMap::new(),
			system: BTreeMap::new(),
		};
		fs::write(
			directory.join(Self::PROJECT_FILE),
			toml::to_string_pretty(&project).expect("serialization failure"),
		)
		.with_whatever_context(|err| format!("failed to create project file: {err}"))?;
		Ok(project)
	}

	pub fn load(directory: &Path) -> Result<ALProject, Whatever> {
		let data = fs::read(directory.join(Self::PROJECT_FILE))
			.with_whatever_context(|err| format!("failed to read project file: {err}"))?;

		toml::from_slice(&data)
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
