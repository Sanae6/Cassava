use gpui::{App, Entity, Render, SharedString, div, prelude::*, px, rems};

use crate::{
	components::{
		StyleExt,
		button::Button,
		event::{AppEventExt, ContextExt},
		root::OpenDialog,
		tree::{TreeItem, TreeState, tree},
	},
	project::ProjectAccessor,
	workspace::{NewTab, ProjectUpdate, import_stage::ImportStage, stage_editor::StageEditor},
};

pub struct Explorer {
	tree: Entity<TreeState>,
}

impl Explorer {
	pub fn new(cx: &mut App) -> Entity<Explorer> {
		let mut tree = TreeState::new();

		{
			tree.set_items(Self::build_tree_state(cx));
			// root.push(TreeItem::new("text", "Text"));
			// root.push(TreeItem::new("objects", "Objects"));
		}
		cx.new::<Self>(|cx| {
			cx.bus_subscribe(|explorer, _: &ProjectUpdate, cx| {
				explorer
					.tree
					.update(cx, |tree, cx| tree.set_items(Self::build_tree_state(cx)))
			});

			Self {
				tree: tree.build(cx, rems(2.0)),
			}
		})
	}

	fn build_tree_state(cx: &mut App) -> impl Iterator<Item = TreeItem> {
		let project = cx.al_project();
		let mut root = vec![];
		root.push(
			TreeItem::new("stages", "Stages").children(
				project
					.stages
					.keys()
					.map(|stage_name| TreeItem::new(format!("stages/{stage_name}"), stage_name)),
			), // .expanded(true),
		);
		root.into_iter()
	}
}

impl Render for Explorer {
	fn render(
		&mut self,
		window: &mut gpui::Window,
		cx: &mut gpui::Context<Self>,
	) -> impl gpui::IntoElement {
		div()
			.v_flex()
			.size_full()
			.child(
				Button::new("import-stage")
					.label("Import stage")
					.on_click(|_, _, cx| {
						let import_stage = cx.new(|cx| ImportStage::new(cx));
						cx.bus_emit(OpenDialog::new("Import stage", import_stage));
					}),
			)
			.child(
				tree(&self.tree, |entry, _, _| {
					Button::new(entry.id())
						.pl(px(16.) * entry.depth() + px(12.))
						.child(entry.label())
				})
				.on_selected(|item, window, cx| {
					let id = item.id();
					let (root, other) = id.split_once("/").unwrap_or((&id, ""));
					match root {
						"stages" => {
							let stage_name: SharedString = other.into();
							let stage_editor = cx.new(|cx| StageEditor::new(&stage_name, window,cx));
							cx.bus_emit(NewTab::new(stage_editor, move |tab, _, _| {
								tab.title(stage_name.clone()).closeable(true)
							}));
						}
						_ => {}
					}
				}),
			)
	}
}
