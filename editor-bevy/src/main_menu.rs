use bevy::prelude::*;
use bevy_egui::{
	EguiContexts, EguiPrimaryContextPass,
	egui::{self, Align2, Layout, Modal},
};
use cassava_project::ALProject;

use crate::{
	UiStates,
	config::{Config, ConfigChanged},
};

pub fn plugin(app: &mut App) {
	app
		.add_systems(OnEnter(UiStates::MainMenu), check_for_romfs)
		.add_systems(
			EguiPrimaryContextPass,
			(
				main_ui.run_if(in_state(MenuState::MainMenu)),
				no_romfs_ui.run_if(in_state(MenuState::NoRomfs)),
			),
		)
		.init_state::<MenuState>()
		.insert_resource(MainMenu::default());
}

#[derive(SubStates, Debug, Hash, PartialEq, Eq, Clone, Default)]
#[source(UiStates = UiStates::MainMenu)]
enum MenuState {
	#[default]
	NoRomfs,
	MainMenu,
}

#[derive(Resource, Default)]
struct MainMenu {
	invalid_romfs: Vec<&'static str>,
}

fn check_for_romfs(config: ResMut<Config>, mut commands: Commands) {
	if config.config_data.default_romfs_dir.is_none() {
		commands.set_state(MenuState::MainMenu);
	}

	commands.spawn_scene(bsn! {
		Node {
			margin: px(10),
		}
		Text::new("Move with arrow keys.\nPress SPACEBAR to pause.")
	});
}

fn no_romfs_ui(
	mut commands: Commands,
	mut contexts: EguiContexts,
	mut state: ResMut<MainMenu>,
	config: ResMut<Config>,
	mut config_updater: MessageWriter<ConfigChanged>,
) -> Result {
	let mut mark_config_changed = || config_updater.write(ConfigChanged);
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
					let dialog = rfd::FileDialog::new();
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
							state.invalid_romfs = missing_directories;
							error!("fail!");
						} else {
							info!("awesome!");
							config_data.default_romfs_dir = Some(dir);
							mark_config_changed();

							commands.set_state(MenuState::MainMenu);
						}
					}
				}

				if !state.invalid_romfs.is_empty() {
					ui.separator();
					ui.horizontal_wrapped(|ui| {
						ui.spacing_mut().item_spacing.x = 0.0;
						ui.label(
							"The provided folder is incorrectly dumped or not a RomFS dump. \
											  Missing subfolders: ",
						);
						for (index, string) in state.invalid_romfs.iter().enumerate() {
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

fn main_ui(
	mut contexts: EguiContexts,
	mut state: ResMut<MainMenu>,
	config: ResMut<Config>,
	mut config_updater: MessageWriter<ConfigChanged>,
) -> Result {
	let context = contexts.ctx_mut()?;
	egui::Window::new("Cassava")
		.anchor(Align2::CENTER_CENTER, (0.0, 0.0))
		.resizable(false)
		.collapsible(false)
		.show(context, |ui| {
			ui.vertical_centered(|ui| {
				if ui.button("New Project").clicked() {
					let dialog = rfd::FileDialog::new();
					if let Some(dir) = dialog.pick_folder() {}
				}
				// ui.label("Config test");
				// ui.label(
				// 	config_data
				// 		.default_romfs_dir
				// 		.as_ref()
				// 		.unwrap()
				// 		.to_string_lossy(),
				// );
			});
			ui.separator();
			ui.vertical_centered(|ui| {
				let github_emote = egui::special_emojis::GITHUB;
				ui.hyperlink_to(
					format!("{github_emote} Source code"),
					"https://github.com/Sanae6/Cassava",
				);
				ui.hyperlink_to("SMO Modding Hub", "https://discord.gg/dZX7sn2mER");
			})
		});
	Ok(())
}
