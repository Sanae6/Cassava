use std::{fs, path::PathBuf, sync::LazyLock};

use gpui::App;
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Whatever};

static PROJECT_DIR: LazyLock<directories::ProjectDirs> =
	LazyLock::new(|| directories::ProjectDirs::from("ca.sanae", "Aubrey", "Cassava").unwrap());

#[derive(Default, Serialize, Deserialize)]
pub struct Config {
	pub default_romfs_dir: Option<PathBuf>,
	pub project_dirs: Vec<PathBuf>,
}

impl Config {
	fn config_path() -> PathBuf {
		let _ = fs::create_dir_all(PROJECT_DIR.config_dir())
			.inspect_err(|error| eprintln!("failed to create config directory: {error}"));
		PROJECT_DIR.config_dir().join("config.toml")
	}

	pub fn register(cx: &mut App) {
		cx.set_global(match Config::load() {
			Ok(value) => value,
			Err(error) => {
				eprintln!("failed to load config: {error}");
				eprintln!("falling back to default config");
				Config::default()
			}
		});
	}

	fn load() -> Result<Self, Whatever> {
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
impl gpui::Global for Config {}

#[allow(unused)]
pub trait ConfigAccessor {
	fn config(&self) -> &Config;
	fn config_mut(&mut self) -> &mut Config;
}

impl ConfigAccessor for App {
	fn config(&self) -> &Config {
		self.global()
	}
	fn config_mut(&mut self) -> &mut Config {
		self.global_mut()
	}
}
