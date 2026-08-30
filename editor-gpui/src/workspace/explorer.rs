use gpui::{App, Entity, Render, div, prelude::*, px, rems};

use crate::{
	components::{
		StyleExt,
		button::Button,
		event::{AppEventExt, ContextExt},
		root::OpenDialog,
		tree::{TreeItem, TreeState, tree},
	},
	project::ProjectAccessor,
	workspace::{ProjectUpdate, import_stage::ImportStage},
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
		_: &mut gpui::Window,
		_: &mut gpui::Context<Self>,
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
				}), // .context_menu(|_, entry, popup, _, _| match entry.item().id.as_str() {
				    // 	"stages" => popup.item(
				    // 		PopupMenuItem::new("Import stage").on_click(|_, window, cx| {
				    // 			let import_stage = cx.new(|cx| ImportStage::new(cx));
				    // 			window.open_dialog(cx, move |dialog, _, _| {
				    // 				dialog
				    // 					.title("Import a stage")
				    // 					.h(DefiniteLength::Fraction(3.0 / 4.0))
				    // 					.child(import_stage.clone())
				    // 			});
				    // 		}),
				    // 	),
				    // 	_ => popup,
				    // }),
			)
	}
}
