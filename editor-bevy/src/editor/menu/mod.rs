use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};

use crate::{
	Project,
	editor::{ViewportUi, setup_viewport_ui, tick_editor},
};

pub fn plugin(app: &mut App) {
	app.add_systems(
		EguiPrimaryContextPass,
		(
			(menu).before(tick_editor).after(setup_viewport_ui),
			(import_stage_modal).after(menu),
		),
	);
	// app.add_systems(EguiPrimaryContextPass, (menu).after(view));
}

struct ImportStage {
  stage_name: String,
}

fn menu(
	mut contexts: EguiContexts,
	project: Option<ResMut<Project>>,
	mut viewport_ui: ResMut<ViewportUi>,
) -> Result {
	let ctx = contexts.ctx_mut()?;
	egui::MenuBar::new().ui(&mut viewport_ui.0, |ui| {
		if let Some(project) = &project {
			ui.menu_button("Project", |ui| if ui.button("Import a stage").clicked() {});
		}
	});

	Ok(())
}

fn import_stage_modal(mut contexts: EguiContexts, project: Option<ResMut<Project>>) -> Result {
	let ctx = contexts.ctx_mut()?;
	egui::Modal::new("import-stage".into()).show(ctx, |ui| {
		//
	});
	Ok(())
}
