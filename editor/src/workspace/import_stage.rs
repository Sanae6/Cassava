use std::fs;

use cassava::{
	byaml::Byml,
	sarc,
	util::{ArcAtomicRefCellExt, BinaryReader},
	yaz0,
};
use gpui::{App, Entity, div, prelude::*, px};
use snafu::Report;

use crate::{
	components::{
		StyleExt, button::Button, event::AppEventExt, root::PopDialog, theme::ThemeExt, tree::{TreeItem, TreeState, tree}
	},
	config::ConfigAccessor,
	project::ProjectAccessor,
	workspace::ProjectUpdate,
};

pub struct ImportStage {
	tree_state: Entity<TreeState>,
}

impl ImportStage {
	pub fn new(cx: &mut App) -> Self {
		let mut items = vec![];
		let romfs = cx.config().default_romfs_dir.clone().unwrap();
		let a = fs::read(romfs.join("SystemData/WorldList.szs")).unwrap();
		let (a, _) = yaz0::decompress(&a).unwrap();
		let a = sarc::Reader::new(a).unwrap();
		let a = Byml::read(&mut BinaryReader::new(
			a.get_file("WorldListFromDb.byml").unwrap(),
		))
		.unwrap();
		let array = a.as_array().unwrap().into_borrow();
		// let array = a.as_array().unwrap();
		// let array = array.borrow();

		for kingdom in array.iter() {
			let kingdom = kingdom.as_dictionary().unwrap().into_borrow();
			let world_name = kingdom["WorldName"].as_string().unwrap();
			items.push(
				TreeItem::new(&*world_name, &*world_name).children(
					kingdom["StageList"]
						.as_array()
						.unwrap()
						.into_borrow()
						.iter()
						.map(|a| {
							let name = &*a.as_dictionary().unwrap().into_borrow()["name"]
								.as_string()
								.unwrap();

							TreeItem::new(format!("{world_name}/{name}"), name)
						}),
				),
			);
		}

		Self {
			tree_state: TreeState::new()
				.items(items.into_iter())
				.build(cx, px(32.0)),
		}
	}
}

impl Render for ImportStage {
	fn render(
		&mut self,
		window: &mut gpui::Window,
		cx: &mut gpui::Context<Self>,
	) -> impl gpui::IntoElement {
		let view = cx.entity();
		div()
			.v_flex()
			.size_full()
			.bg(cx.theme().pane_bg)
			.child(tree(&self.tree_state, |entry, _, _| {
				Button::new(entry.label())
					.pl(px(16.) * entry.depth() + px(12.))
					.child(entry.label())
			}))
			.child(
				div()
					.h_flex()
					.justify_end()
					.gap_2()
					.child(
						Button::new("close")
							.label("Cancel")
							.on_click(move |_, _, cx| {
								cx.bus_emit(PopDialog);
							}),
					)
					.child(
						Button::new("import")
							.label("Import")
							.disabled(
								self
									.tree_state
									.read(cx)
									.selected()
									.is_none_or(|entry| entry.depth() == 0),
							)
							.on_click(window.listener_for(&view, move |view, _, _, cx| {
								let Some(entry) = view.tree_state.read(cx).selected() else {
									return;
								};
								if entry.depth() == 0 {
									return;
								}
								let stage_name = entry.label();

								let romfs_dir = cx.config().default_romfs_dir.as_ref().unwrap().clone();
								println!("Importing stage {stage_name} from {romfs_dir:?}");
								if let Err(err) = cx.al_project_mut().import_stage(&romfs_dir, &stage_name) {
									let err = Report::from_error(err);
									eprintln!("{err}");
									panic!()
								};

								cx.bus_emit(ProjectUpdate);
								cx.bus_emit(PopDialog);
							})),
					),
			)
			// .border_2()
			// .border_color(red_400())
			.size_full()
	}
}
