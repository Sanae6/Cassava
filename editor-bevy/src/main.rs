mod main_menu;
mod config;
mod editor;

use bevy::prelude::*;
use bevy_egui::{EguiPlugin, EguiPrimaryContextPass};

fn main() {
	App::new()
		.add_plugins(DefaultPlugins)
		.add_plugins(EguiPlugin::default())
		.add_plugins(main_menu::plugin)
		.add_plugins(config::plugin)
		.add_systems(Startup, setup)
		.insert_state(UiStates::MainMenu)
		.run();
}

fn setup(mut commands: Commands) {
	commands.spawn(Camera2d);
}

#[derive(States, Debug, Hash, PartialEq, Eq, Clone)]
pub enum UiStates {
	MainMenu,
	Editor,
}
