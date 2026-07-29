use std::path::PathBuf;

use eframe::{
	App, CreationContext, NativeOptions,
	egui::{CentralPanel, MenuBar},
};
use egui_dock::{DockArea, DockState};

use crate::ui::{FileNode, State, TabType};

mod ui;

fn main() {
	eframe::run_native(
		"Cassava",
		NativeOptions::default(),
		Box::new(|ctx| Ok(Box::new(Cassava::new(ctx)))),
	)
	.unwrap();
}

struct Cassava {
	state: State,
	dock_state: DockState<TabType>,
}

impl Cassava {
	fn new(ctx: &CreationContext) -> Self {
		let dock_state = DockState::new(vec![TabType::FileTree {
			root: FileNode::new_directory(
				PathBuf::from("/home/aubrey/Projects/modding/smo/romfs")
					.canonicalize()
					.expect("hardcoded path is wrong Are you Stupid"),
			),
		}]);
		// let dock_state = DockState::new(vec![]);

		Self {
			state: State::new(ctx),
			dock_state,
		}
	}
}

impl App for Cassava {
	fn ui(&mut self, ui: &mut eframe::egui::Ui, frame: &mut eframe::Frame) {
		ui.ctx().clear_animations();
		ui.ctx().all_styles_mut(|s| s.animation_time = 0.0);

		MenuBar::new().ui(ui, |ui| {
			ui.menu_button("mreeow", |ui| {
				if ui.button("mrrrrp").clicked() {
					println!("maow");
				}
			});
		});

		CentralPanel::no_frame().show_inside(ui, |ui| {
			DockArea::new(&mut self.dock_state)
				.show_leaf_collapse_buttons(false)
				.show_leaf_close_all_buttons(false)
				.show_inside(ui, &mut self.state)
		});
	}
}
