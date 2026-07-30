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
	objects: (),
	effects: (),
	layouts: (),
	event_flows: (),
	text: (),
	shaders: (),
	system: (),
}

impl ALProject {
	pub fn create(directory: PathBuf) -> Result<Self, Whatever> {
		fs::create_dir_all(&directory.join("stages"))
			.whatever_context("failed to create stages directory")?;

		Ok(Self {
			root: directory,
			stages: BTreeMap::new(),
			objects: (),
			effects: (),
			layouts: (),
			event_flows: (),
			text: (),
			shaders: (),
			system: (),
		})
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
