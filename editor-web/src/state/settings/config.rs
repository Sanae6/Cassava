use std::{collections::HashMap, fs, path::PathBuf, sync::LazyLock};

use num_derive::FromPrimitive;
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Whatever};
use ts_rs::TS;

#[derive(TS, Serialize, Deserialize, Default, Clone)]
#[ts(export)]
pub struct Config {
	pub romfs_versions: HashMap<Version, PathBuf>,
	pub project_dirs: Vec<PathBuf>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash, TS, FromPrimitive)]
pub enum Version {
	#[serde(rename = "v100")]
	V100,
	#[serde(rename = "v101")]
	V101,
	#[serde(rename = "v110")]
	V110,
	#[serde(rename = "v120")]
	V120,
	#[serde(rename = "v130")]
	V130,
}
impl Version {
	pub const VERSION_COUNT: usize = std::mem::variant_count::<Self>();
}

static PROJECT_DIR: LazyLock<directories::ProjectDirs> =
	LazyLock::new(|| directories::ProjectDirs::from("ca.sanae", "Aubrey", "Cassava").unwrap());

impl Config {
	fn config_path() -> PathBuf {
		let _ = fs::create_dir_all(PROJECT_DIR.config_dir())
			.inspect_err(|error| eprintln!("failed to create config directory: {error}"));
		PROJECT_DIR.config_dir().join("config.toml")
	}
	pub fn load() -> Result<Self, Whatever> {
		let text =
			fs::read_to_string(Self::config_path()).whatever_context("failed to read config file")?;
		let config = toml::from_str(&text).whatever_context("failed to deserialize config")?;
		Ok(config)
	}

	#[allow(unused)]
	pub fn save(&mut self) -> Result<(), Whatever> {
		fs::write(
			Config::config_path(),
			toml::to_string_pretty(&self).whatever_context("failed to serialize config")?,
		)
		.whatever_context("failed to write config file")
	}
}
