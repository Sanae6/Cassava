use std::{
	any::Any,
	cell::{RefCell, RefMut},
	fs::read_dir,
	path::PathBuf,
	rc::Rc,
};

use editor_web_macro::handlers;
use webkit6::{
	gio::prelude::FileExt,
	gtk::{ApplicationWindow, FileDialog},
};

use crate::{
	infra::rpc::Context,
	state::{
		config::{Config, Version},
		hashes::{HashingReport, hash_romfs},
	},
};

mod config;
mod hashes;

pub struct Settings {
	config: Config,
	window: ApplicationWindow,
}

impl Settings {
	pub fn new(window: ApplicationWindow) -> Settings {
		let config = match Config::load() {
			Ok(value) => value,
			Err(error) => {
				eprintln!("failed to load config: {error}");
				eprintln!("falling back to default config");
				Config::default()
			}
		};

		Settings { config, window }
	}
}

#[handlers]
impl Settings {
	pub fn get_config(&self) -> Config {
		self.config.clone()
	}

	fn set_romfs_directory(&mut self, version: Version, directory: PathBuf) -> Result<(), String> {
		self.config.romfs_versions.insert(version, directory);
		self.config.save().map_err(|error| error.to_string())
	}

	pub async fn open_directory_dialog(call_context: Context<Self>) -> Option<PathBuf> {
		let dialog = FileDialog::new();
		let result = dialog
			.select_folder_future(Some(&call_context.rc.borrow().window))
			.await;
		match result {
			Ok(file) => {
				println!("found file {:?} {:?}", file.path(), file.path().unwrap());
				Some(file.path().unwrap())
			}
			Err(error) => {
				eprintln!("{error:?}");
				None
			}
		}
	}

	#[report_progress(processed: usize, total: usize)]
	pub async fn match_hashes(call_context: Context<Self>, dir: PathBuf) -> HashingReport {
		hash_romfs(&dir, |processed, total| {
			call_context.report_progress(format_args!(r#""{processed}", "{total}""#));
		})
		.await
	}
}
