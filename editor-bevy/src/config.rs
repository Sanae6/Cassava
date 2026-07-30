use std::{fs, path::PathBuf, sync::LazyLock};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

static PROJECT_DIR: LazyLock<directories::ProjectDirs> =
	LazyLock::new(|| directories::ProjectDirs::from("ca.sanae", "Aubrey", "Cassava").unwrap());

pub fn plugin(app: &mut App) {
	let config = Config::load()
		.inspect_err(|error| error!("failed to load config: {error}"))
		.unwrap_or_default();
	app
		.add_message::<ConfigChanged>()
		.add_systems(Update, save_config.run_if(resource_exists::<Config>))
		.insert_resource(config);
}

#[derive(Resource, Default)]
pub struct Config {
	pub config_data: ConfigData,
}

#[derive(Default, Serialize, Deserialize)]
pub struct ConfigData {
	pub default_romfs_dir: Option<PathBuf>,
	pub project_dirs: Vec<PathBuf>,
}

impl Config {
	fn config_path() -> PathBuf {
		let _ = fs::create_dir_all(PROJECT_DIR.config_dir())
			.inspect_err(|error| error!("failed to create config directory: {error}"));
		PROJECT_DIR.config_dir().join("config.toml")
	}

	fn load() -> Result<Self> {
		let text = fs::read_to_string(Self::config_path())?;
		let config_data = toml::from_str(&text)?;
		Ok(Self { config_data })
	}
}

#[derive(Message)]
pub struct ConfigChanged;

fn save_config(config: Res<Config>, mut event: MessageReader<ConfigChanged>) -> Result {
	if !event.is_empty() {
		info!("saving config");
		event.clear();

		fs::write(
			Config::config_path(),
			toml::to_string_pretty(&config.into_inner().config_data)?,
		)?;
	}

	Ok(())
}
