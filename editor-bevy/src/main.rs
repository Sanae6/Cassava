mod config;
mod editor;
mod main_menu;

use std::sync::Arc;

use bevy::prelude::*;
use bevy_egui::{
	EguiContexts, EguiGlobalSettings, EguiPlugin, PrimaryEguiContext,
	egui::{FontData, FontDefinitions, FontFamily},
};
use cassava_project::ALProject;

fn main() {
	App::new()
		.add_plugins(DefaultPlugins)
		.add_plugins(EguiPlugin::default())
		.add_plugins(main_menu::plugin)
		.add_plugins(editor::plugin)
		.add_plugins(config::plugin)
		.add_systems(Startup, setup)
		.add_systems(Update, add_fonts)
		.insert_state(UiStates::MainMenu)
		.run();
}

fn setup(mut commands: Commands, mut egui_settings: ResMut<EguiGlobalSettings>) {
	egui_settings.auto_create_primary_context = false;
	commands.spawn((Camera2d, PrimaryEguiContext));
}

fn add_fonts(mut contexts: EguiContexts) {
	let ctx = contexts.ctx_mut().unwrap();
	let mut fonts = FontDefinitions::default();
	fonts.font_data.insert(
		"NotoSansCJKjp-Regular".to_owned(),
		Arc::new(FontData::from_static(include_bytes!(
			"../assets/fonts/NotoSansCJKjp-Regular.otf"
		))),
	);
	fonts.font_data.insert(
		"NotoSansMonoCJKjp-Regular".to_owned(),
		Arc::new(FontData::from_static(include_bytes!(
			"../assets/fonts/NotoSansMonoCJKjp-Regular.otf"
		))),
	);
	fonts
		.families
		.get_mut(&FontFamily::Proportional)
		.unwrap()
		.push("NotoSansCJKjp-Regular".to_owned());
	fonts
		.families
		.get_mut(&FontFamily::Monospace)
		.unwrap()
		.push("NotoSansMonoCJKjp-Regular".to_owned());
	ctx.set_fonts(fonts);
}

#[derive(States, Debug, Hash, PartialEq, Eq, Clone)]
pub enum UiStates {
	MainMenu,
	Editor,
}

#[derive(Resource)]
pub struct Project(ALProject);
