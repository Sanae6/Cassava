use bevy::prelude::*;
use bevy_egui::egui::{self, Ui};
use cassava::yaz0;

use crate::editor::{NewFileTab, TabContext};

pub struct SarcEditorTab {
	name: String,
	reader: cassava::sarc::Reader,
	alignment: u32,
}

impl SarcEditorTab {
	pub fn new(name: String, mut data: Vec<u8>, compressed: bool) -> Result<Self> {
		let mut alignment = 0x80;
		if compressed {
			(data, alignment) = yaz0::decompress(&data)?;
		}

		let reader = cassava::sarc::Reader::new(data)?;

		Ok(Self {
			name,
			reader,
			alignment,
		})
	}

	pub fn title(&self) -> egui::WidgetText {
		self.name.clone().into()
	}

	pub fn show(&mut self, ui: &mut Ui, viewer: &mut TabContext) {
		ui.horizontal_wrapped(|ui| {
			ui.spacing_mut().item_spacing.x = 0.0;
			ui.label("i'm a sarc editor for mr ");
			ui.monospace(&self.name);
			ui.label(" and these are his files")
		});

		ui.indent("byright", |ui| {
			for (name, data) in self.reader.files() {
				if ui
					.add(egui::Button::new(name).frame_when_inactive(false))
					.double_clicked()
				{
					viewer.commands.write_message(NewFileTab {
						name: name.to_owned(),
						data: data.to_vec(),
					});
				}
			}
		});
	}
}
