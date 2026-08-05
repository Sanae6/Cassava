use std::{
	borrow::Cow,
	path::{Path, PathBuf},
};

use bevy::prelude::*;
use bevy_egui::{
	EguiContexts, EguiPrimaryContextPass,
	egui::{self, Align2, Grid, Layout, Modal, ScrollArea},
};
use cassava_project::ALProject;
use snafu::Whatever;

use crate::{
	Project, UiStates,
	config::{Config, ConfigChanged},
};

pub fn plugin(app: &mut App) {
	app
		.add_systems(OnEnter(UiStates::MainMenu), check_for_romfs)
		.add_systems(OnEnter(MenuStates::ProjectSetup), start_project_setup_flow)
		.add_systems(
			EguiPrimaryContextPass,
			(
				main_ui.run_if(in_state(MenuStates::MainMenu)),
				no_romfs_ui.run_if(in_state(MenuStates::NoRomfs)),
				project_setup.run_if(in_state(MenuStates::ProjectSetup)),
			)
				.run_if(in_state(UiStates::MainMenu)),
		)
		.init_state::<MenuStates>();
}

#[derive(SubStates, Debug, Hash, PartialEq, Eq, Clone, Default)]
#[source(UiStates = UiStates::MainMenu)]
enum MenuStates {
	#[default]
	NoRomfs,
	MainMenu,
	ProjectSetup,
}

#[derive(Resource, Default)]
struct InvalidRomfs(Vec<&'static str>);

fn check_for_romfs(config: ResMut<Config>, mut commands: Commands) {
	if let Some(romfs_dir) = &config.config_data.default_romfs_dir
		&& romfs_dir.exists()
	{
		commands.set_state(MenuStates::MainMenu);
	}

}

fn no_romfs_ui(
	mut commands: Commands,
	mut contexts: EguiContexts,
	invalid_romfs: Option<ResMut<InvalidRomfs>>,
	config: ResMut<Config>,
	mut config_updater: MessageWriter<ConfigChanged>,
) -> Result {
	let context = contexts.ctx_mut()?;
	egui::Window::new("Cassava")
		.anchor(Align2::CENTER_CENTER, (0.0, 0.0))
		.resizable(false)
		.collapsible(false)
		.show(context, |ui| {
			let config = config.into_inner();
			let config_data = &mut config.config_data;
			ui.with_layout(Layout::top_down(egui::Align::Center), |ui| {
				ui.horizontal_wrapped(|ui| {
					ui.spacing_mut().item_spacing.x = 0.0;
					ui.label(
						"To get started, you need to link a dump of Super Mario Odyssey. \
							 If you don't have a copy of the game, follow ",
					);
					ui.hyperlink_to(
						"this guide",
						"https://smo.wiki/Guide:Dumping_and_preparing_your_ROM",
					);
					ui.label(
						" on the SMO Wiki, and then select the folder where you placed your RomFS directory.",
					);
				});
				if ui.button("Select default RomFS folder").clicked() {
					let dialog = rfd::FileDialog::new().set_title("Select your RomFS dump");
					if let Some(dir) = dialog.pick_folder() {
						const REQUIRED_ROMFS_DIRS: &[&str] = &[
							"EffectData",
							"EventData",
							"LayoutData",
							"LocalizedData",
							"MovieData",
							"ObjectData",
							"ShaderData",
							"SoundData",
							"StageData",
							"SystemData",
						];
						let missing_directories = REQUIRED_ROMFS_DIRS
							.into_iter()
							.cloned()
							.filter(|romfs| !dir.join(romfs).exists())
							.collect::<Vec<_>>();
						if missing_directories.len() > 0 {
							commands.insert_resource(InvalidRomfs(missing_directories));
							error!("fail!");
						} else {
							info!("awesome!");
							config_data.default_romfs_dir = Some(dir);
							config_updater.write(ConfigChanged);

							commands.set_state(MenuStates::MainMenu);
						}
					}
				}

				if let Some(invalid_romfs) = invalid_romfs {
					ui.separator();
					ui.horizontal_wrapped(|ui| {
						ui.spacing_mut().item_spacing.x = 0.0;
						ui.label(
							"The provided folder is incorrectly dumped or not a RomFS dump. \
											  Missing subfolders: ",
						);
						for (index, string) in invalid_romfs.0.iter().enumerate() {
							if index != 0 {
								ui.label(", ");
							}
							ui.monospace(*string);
						}
					});
				}
			});
		});
	Ok(())
}

#[derive(Resource)]
struct ProjectLoadFailure(Whatever);

fn main_ui(
	mut commands: Commands,
	mut contexts: EguiContexts,
	load_failure: Option<ResMut<ProjectLoadFailure>>,
	config: ResMut<Config>,
	mut config_updater: MessageWriter<ConfigChanged>,
) -> Result {
	let config = config.into_inner();
	let config_data = &mut config.config_data;
	let context = contexts.ctx_mut()?;
	egui::Window::new("Cassava")
		.anchor(Align2::CENTER_CENTER, (0.0, 0.0))
		.resizable(false)
		.collapsible(false)
		.show(context, |ui| {
			if ui.button("Start without a project").clicked() {
				commands.set_state(UiStates::Editor);
			}
			if ui.button("New Project").clicked() {
				commands.set_state(MenuStates::ProjectSetup);
			}
			ScrollArea::vertical().show(ui, |ui| {
				enum AfterOp {
					Delete(usize),
					Reorder(usize),
				}
				let mut after_iter_op = None;
				for (index, project_dir) in config_data.project_dirs.iter().enumerate() {
					ui.horizontal(|ui| {
						if ui.button("X").clicked() {
							after_iter_op = Some(AfterOp::Delete(index));
						}
						let r = ui.button("Open");
						let r2 = ui.selectable_label(
							true,
							project_dir
								.file_name()
								.expect("path doesn't have a file name")
								.to_string_lossy(),
						);
						if r2.clicked() || r.clicked() {
							match ALProject::load(project_dir.as_path()) {
								Ok(project) => {
									info!("loaded!");
									commands.insert_resource(Project(project));
									commands.set_state(UiStates::Editor);
									after_iter_op = Some(AfterOp::Reorder(index));
								}
								Err(error) => {
									error!("not loaded...");
									commands.insert_resource(ProjectLoadFailure(error));
								}
							};
						}
					});
				}
				match after_iter_op {
					Some(AfterOp::Reorder(index)) => {
						let value = config_data.project_dirs.remove(index);
						config_data.project_dirs.insert(0, value);
						config_updater.write(ConfigChanged);
					}
					Some(AfterOp::Delete(index)) => {
						let _ = config_data.project_dirs.remove(index);
						config_updater.write(ConfigChanged);
					}
					None => {}
				}
			});
			ui.separator();
			ui.vertical_centered(|ui| {
				let github_emote = egui::special_emojis::GITHUB;
				ui.hyperlink_to(
					format!("{github_emote} Source code"),
					"https://https://mkwiiki.org/wiki/YAZ0_(File_Format)?github.com/Sanae6/Cassava",
				);
				ui.hyperlink_to("SMO Modding Hub", "https://discord.gg/dZX7sn2mER");
			})
		});
	if let Some(error) = &load_failure {
		let r = display_error(context, "create-error", &error.0);

		if r.should_close() {
			commands.remove_resource::<ProjectLoadFailure>();
		}
	}
	Ok(())
}

enum PathState {
	Ok,
	ParentDoesNotExist,
	ParentNotADirectory,
	NotADirectory,
	NotEmpty,
}
impl PathState {
	fn user_warning(&self) -> &str {
		match self {
			PathState::Ok => "✔ Good to go",
			PathState::ParentDoesNotExist => "❌ Does not exist",
			PathState::ParentNotADirectory => "❌ Parent is not a folder",
			PathState::NotADirectory => "❌ Not a folder",
			PathState::NotEmpty => "⚠ Not empty",
		}
	}
}
#[derive(Resource, Default)]
struct ProjectSetupData {
	path: String,
	path_state: Option<PathState>,
	create_error: Option<Whatever>,
}

impl ProjectSetupData {
	fn is_valid_path(&self) -> bool {
		match self.path_state {
			Some(PathState::Ok | PathState::NotEmpty) => true,
			_ => false,
		}
	}

	fn revalidate_path(&mut self) {
		let path = Path::new(&self.path);
		let parent = path.parent();
		let path_state = match parent {
			Some(parent) => {
				if !parent.exists() {
					PathState::ParentDoesNotExist
				} else if !parent.is_dir() {
					PathState::ParentNotADirectory
				} else if path.exists() {
					if !path.is_dir() {
						PathState::NotADirectory
					} else if path.read_dir().is_ok_and(|mut iter| iter.next().is_some()) {
						PathState::NotEmpty
					} else {
						PathState::Ok
					}
				} else {
					PathState::Ok
				}
			}
			None => {
				if !path.is_dir() {
					PathState::NotADirectory
				} else if path.read_dir().is_ok_and(|mut iter| iter.next().is_some()) {
					PathState::NotEmpty
				} else {
					PathState::Ok
				}
			}
		};

		self.path_state = Some(path_state);
	}
}

fn start_project_setup_flow(mut commands: Commands) {
	commands.insert_resource(ProjectSetupData::default());
}

fn project_setup(
	mut commands: Commands,
	mut contexts: EguiContexts,
	mut state: ResMut<ProjectSetupData>,
	mut config: ResMut<Config>,
	mut config_updater: MessageWriter<ConfigChanged>,
) -> Result {
	let context = contexts.ctx_mut()?;
	egui::Window::new("New Project")
		.anchor(Align2::CENTER_CENTER, (0.0, 0.0))
		.resizable(false)
		.collapsible(false)
		.show(context, |ui| {
			Grid::new("new-project").num_columns(2).show(ui, |ui| {
				ui.label("Project Folder");
				ui.horizontal(|ui| {
					if ui.text_edit_singleline(&mut state.path).changed() {
						state.revalidate_path();
					}
					if ui.button("Open").clicked() {
						let mut dialog = rfd::FileDialog::new()
							.set_can_create_directories(true)
							.set_title("Select empty project directory");
						if state.is_valid_path() {
							dialog = dialog.set_directory(&state.path);
						}

						if let Some(dir) = dialog.pick_folder() {
							let lossy = dir.to_string_lossy();
							state.path = lossy.into_owned();
							state.revalidate_path();
						}
					}
				});
				ui.end_row();
				if let Some(path_state) = &state.path_state {
					ui.label(""); // space filler
					ui.label(path_state.user_warning());
					ui.end_row();
				}
			});
			ui.separator();
			ui.horizontal(|ui| {
				if ui.button("Cancel").clicked() {
					commands.set_state(MenuStates::MainMenu);
				}
				let r = ui.add_enabled_ui(state.is_valid_path(), |ui| ui.button("Create"));
				if r.inner.clicked() {
					let path = Path::new(&state.path);
					match ALProject::create(path) {
						Ok(project) => {
							info!("success!");
							config.config_data.project_dirs.push(path.to_owned());
							config_updater.write(ConfigChanged);
							commands.insert_resource(Project(project));
							commands.set_state(UiStates::Editor);
						}
						Err(error) => {
							error!("failure...");
							state.create_error = Some(error);
						}
					}
				}
			});
		});

	if let Some(error) = &state.create_error {
		let r = display_error(context, "create-error", error);

		if r.should_close() {
			state.create_error = None;
		}
	}
	Ok(())
}

fn display_error(
	context: &mut egui::Context,
	id: &'static str,
	error: &Whatever,
) -> egui::ModalResponse<()> {
	egui::Modal::new(id.into()).show(context, |ui| {
		ui.heading("Failed to create project");
		ui.label(format!("Error: {error}"));
		if ui.button("Ok").clicked() {
			ui.close();
		}
		ui.collapsing("Stack trace", |ui| {
			ui.set_min_width(600.0);
			ui.set_min_height(600.0);
			if ui.button("Copy stack trace").clicked() {
				ui.copy_text(error.backtrace().to_string());
			}
			ScrollArea::new([false, true]).show(ui, |ui| {
				ui.monospace(error.backtrace().to_string());
			})
		});
	})
}
