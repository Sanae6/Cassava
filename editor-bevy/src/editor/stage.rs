use bevy_egui::egui::{self, Ui};

pub struct StageEditorTab {
	stage_name: String,
}

impl StageEditorTab {
	pub fn title(&self) -> egui::WidgetText {
		(&self.stage_name).into()
	}
	pub fn show(&mut self, ui: &mut Ui) {}
}
