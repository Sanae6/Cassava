use bevy::prelude::*;

use bevy_egui::egui::{self, Ui};
use cassava::{byaml, util::BinaryReader};

pub struct BymlEditorTab {
	name: String,
	byml: byaml::Byml,
}

impl BymlEditorTab {
	pub fn new(name: String, data: Vec<u8>) -> Result<Self> {
		let byml = byaml::Byml::read(&mut BinaryReader::new(&data))?;

		Ok(Self { name, byml })
	}
	pub fn title(&self) -> egui::WidgetText {
		self.name.clone().into()
	}
	pub fn show(&mut self, ui: &mut Ui) {
		let font = egui::TextStyle::Monospace.resolve(ui.style());
		let indent_width = ui.fonts_mut(|fonts| fonts.glyph_width(&font, ' '));
		ui.spacing_mut().indent = indent_width;
		ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);
		let root_element = match &self.byml.root_node {
			Some(container) => container.as_element(),
			None => byaml::Element::Null,
		};

		self.show_element(ui, "Root", &root_element)
	}

	// todo: custom collapsing element that draws from /editor-bevy/byml.png
	fn show_element(&self, ui: &mut Ui, key: &str, element: &byaml::Element) {
		match element {
			byaml::Element::Array(array) => {
				let array = array.borrow();
				ui.collapsing(format!("{key}: ["), |ui| {
					ui.visuals_mut().indent_has_left_vline = false;
					ui.indent("g", |ui| {
						ui.visuals_mut().indent_has_left_vline = true;
						for (index, element) in array.iter().enumerate() {
							self.show_element(ui, &index.to_string(), element);
						}
					});
				});
				// ui.indent(format!("water{key}"), |ui| {
				// 	ui.horizontal(|ui| {
				// 		ui.spacing_mut().item_spacing.x = 0.0;
				// 		ui.add_visible(false, egui::Label::new(format!("{key}: ")));
				// 	})
				// });
				ui.label("]");
			}
			byaml::Element::Dictionary(dict) => {
				let dict = dict.borrow();
				ui.collapsing(format!("{key}: {{"), |ui| {
					ui.visuals_mut().indent_has_left_vline = false;
					ui.indent("g", |ui| {
					ui.visuals_mut().indent_has_left_vline = true;
						for (key, element) in dict.iter() {
							self.show_element(ui, key, element);
						}
					});
				});
				// ui.indent(format!("water{key}"), |ui| {
				// 	ui.horizontal(|ui| {
				// 		ui.spacing_mut().item_spacing.x = 0.0;
				// 		ui.add_visible(false, egui::Label::new(format!("{key}: ")));
				// 		ui.label("}");
				// 	})
				// });
				ui.label("}");
			}
			byaml::Element::String(value) => {
				ui.label(format!("{key}: {value:?}"));
			}
			byaml::Element::Boolean(value) => {
				let mut value = *value;
				ui.horizontal(|ui| {
					ui.spacing_mut().item_spacing.x = 0.0;
					ui.label(format!("{key}: "));
					ui.add_enabled(false, egui::Checkbox::new(&mut value, ""));
				});
			}
			byaml::Element::I32(value) => {
				let mut value = *value;
				ui.horizontal(|ui| {
					ui.spacing_mut().item_spacing.x = 0.0;
					ui.label(format!("{key}: "));
					ui.add_enabled(false, egui::DragValue::new(&mut value));
				});
			}
			byaml::Element::F32(value) => {
				let mut value = *value;
				ui.horizontal(|ui| {
					ui.spacing_mut().item_spacing.x = 0.0;
					ui.label(format!("{key}: "));
					ui.add_enabled(false, egui::DragValue::new(&mut value));
				});
			}
			byaml::Element::U32(value) => {
				let mut value = *value;
				ui.horizontal(|ui| {
					ui.spacing_mut().item_spacing.x = 0.0;
					ui.label(format!("{key}: "));
					ui.add_enabled(false, egui::DragValue::new(&mut value));
				});
			}
			byaml::Element::I64(value) => {
				let mut value = *value;
				ui.horizontal(|ui| {
					ui.spacing_mut().item_spacing.x = 0.0;
					ui.label(format!("{key}: "));
					ui.add_enabled(false, egui::DragValue::new(&mut value));
				});
			}
			byaml::Element::U64(value) => {
				let mut value = *value;
				ui.horizontal(|ui| {
					ui.spacing_mut().item_spacing.x = 0.0;
					ui.label(format!("{key}: "));
					ui.add_enabled(false, egui::DragValue::new(&mut value));
				});
			}
			byaml::Element::F64(value) => {
				let mut value = *value;
				ui.horizontal(|ui| {
					ui.spacing_mut().item_spacing.x = 0.0;
					ui.label(format!("{key}: "));
					ui.add_enabled(false, egui::DragValue::new(&mut value));
				});
			}
			byaml::Element::Null => {
				ui.label(format!("{key}: Null"));
			}
		}
	}
}
