mod sarc;
mod stage;
pub mod detect_format;

use bevy::{ecs::system::entity_command, prelude::*};
use bevy_egui::{
	EguiContexts, EguiPrimaryContextPass,
	egui::{self, CentralPanel},
};
use egui_dock::{DockArea, DockState, SurfaceIndex, TabViewer};

use crate::{Project, UiStates, editor::stage::StageEditorTab};

pub fn plugin(app: &mut App) {
	app
		.insert_resource(Editor {
			dock_state: DockState::new(vec![]),
		})
		.add_systems(Update, (delete_standalone_entities, add_project_explorer))
		.add_systems(OnEnter(UiStates::Editor), on_editor_start)
		.add_systems(EguiPrimaryContextPass, tick_editor);
}

#[derive(Component, Clone, Default)]
struct StandaloneOnly;
fn on_editor_start(mut commands: Commands) {
	commands.spawn_scene(bsn! {
		Node {
			margin: px(10),
			align_self: AlignSelf::Center,
			justify_self: JustifySelf::Center,
		}
		Text::new("To get started, drag a file\nonto this window, or press Ctrl+O.")
		StandaloneOnly
	});
}

fn delete_standalone_entities(
	mut commands: Commands,
	query: Query<Entity, With<StandaloneOnly>>,
	_: Single<(), With<Project>>,
) {
	for entity in query {
		commands.entity(entity).despawn();
	}
}

struct ProjectExplorerTab;

enum Tabs {
	Explorer(ProjectExplorerTab),
	Stage(StageEditorTab),
}

struct TabsViewer;
impl TabViewer for TabsViewer {
	type Tab = Tabs;

	fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
		"todo".into()
	}

	fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
		match tab {
			Tabs::Explorer(project_explorer_tab) => {}
			Tabs::Stage(stage_editor_tab) => {}
		}
	}
  fn on_tab_button(&mut self, _tab: &mut Self::Tab, _response: &egui::Response) {
    
  }

	fn closeable(&mut self, _tab: &mut Self::Tab) -> bool {
		false
	}

	fn allowed_in_windows(&self, _tab: &mut Self::Tab) -> bool {
		false
	}
}

#[derive(Resource)]
struct Editor {
	dock_state: DockState<Tabs>,
}

fn add_project_explorer(mut editor: ResMut<Editor>, query: Single<(), Added<Project>>) {
	editor
		.dock_state
		.main_surface_mut()
		.push_to_first_leaf(Tabs::Explorer(ProjectExplorerTab));
}

fn tick_editor(mut contexts: EguiContexts, mut editor: ResMut<Editor>) -> Result {
	let ctx = contexts.ctx_mut()?;
	let mut viewport_ui = egui::Ui::new(
		ctx.clone(),
		"editor viewport".into(),
		egui::UiBuilder::new()
			.layer_id(egui::LayerId::background())
			.max_rect(ctx.viewport_rect()),
	);
	DockArea::new(&mut editor.dock_state).show_inside(&mut viewport_ui, &mut TabsViewer);
	ctx.input(|i| {
		for ele in &i.raw.dropped_files {
			info!("found dropped item {}", ele.name)
		}
	});

	Ok(())
}
