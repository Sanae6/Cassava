/*
placement file
the object name directly maps to which placement files an entity will be sent into

 */

use std::{fs, path::Path};

use indexmap::IndexMap;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;
use snafu::{ResultExt, Whatever};

use crate::{ProjectAsset, stage::Entity};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlacementFile {
	Map,
	Sound,
	Design,
	All,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct EntityDatabase {
	/// The mapping of object names to
	object_name_to_location: IndexMap<String, PlacementLocation>,
	entity_templates: IndexMap<String, Entity>,
	camera_parameter_templates: (),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PlacementLocation {
	/// The list that the object is in.
	pub list_name: SmolStr,
	/// The placement file that the entity can be placed into.
	pub placement_file: PlacementFile,
}

impl ProjectAsset for EntityDatabase {
	fn load(path: &Path) -> Result<Self, Whatever> {
		let data = fs::read_to_string(path.join("entity_database.ron"))
			.whatever_context("failed to read entity database")?;
		ron::from_str(&data).whatever_context("failed to load entity database")
	}

	fn save(&self, path: &Path) -> Result<(), Whatever> {
		let file = ron::ser::to_string_pretty(self, PrettyConfig::new().indentor("  "))
			.whatever_context("failed to serialize entity database file")?;
		fs::write(path.join("entity_database.ron"), file)
			.whatever_context("failed to write entity database")?;
		Ok(())
	}
}
