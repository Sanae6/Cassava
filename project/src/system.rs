use std::{fs, path::Path};

use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Whatever};

use crate::ProjectAsset;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SystemData {}

impl ProjectAsset for SystemData {
	fn load(path: &Path) -> Result<Self, Whatever> {
		let data = fs::read_to_string(path.join("system_data.ron"))
			.whatever_context("failed to read system data")?;
		ron::from_str(&data).whatever_context("failed to load system data")
	}

	fn save(&self, path: &Path) -> Result<(), Whatever> {
		let file = ron::ser::to_string_pretty(self, PrettyConfig::new().indentor("  "))
			.whatever_context("failed to serialize system data file")?;
		fs::write(path.join("system_data.ron"), file)
			.whatever_context("failed to write system data")?;
		Ok(())
	}
}
