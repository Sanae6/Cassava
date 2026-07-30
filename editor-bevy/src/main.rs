mod config;
mod editor;
mod main_menu;

use bevy::prelude::*;
use bevy_egui::{EguiGlobalSettings, EguiPlugin, PrimaryEguiContext};
use cassava_project::ALProject;

fn main() {
	App::new()
		.add_plugins(DefaultPlugins)
		.add_plugins(EguiPlugin::default())
		.add_plugins(main_menu::plugin)
		.add_plugins(editor::plugin)
		.add_plugins(config::plugin)
		.add_systems(Startup, setup)
		.insert_state(UiStates::MainMenu)
		.run();
}

fn setup(mut commands: Commands, mut egui: ResMut<EguiGlobalSettings>) {
	egui.auto_create_primary_context = false;
	commands.spawn((Camera2d, PrimaryEguiContext));
}

#[derive(States, Debug, Hash, PartialEq, Eq, Clone)]
pub enum UiStates {
	MainMenu,
	Editor,
}

#[derive(Resource)]
pub struct Project(ALProject);
