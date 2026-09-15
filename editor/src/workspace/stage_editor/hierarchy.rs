use std::sync::Arc;

use cassava_project::stage::{Folder, Node, Stage};
use gpui::{App, AppContext, Entity, Render, Styled, Window, px};

use crate::{
	components::{
		button::Button,
		tree::{TreeItem, TreeState, tree},
	},
	project::ProjectAccessor,
};

pub struct StageHierarchy {
	tree_view: Entity<TreeState>,
	stage_name: String,
}

impl StageHierarchy {
	pub fn new(stage_name: &str, window: &mut Window, cx: &mut App) -> Entity<StageHierarchy> {
		let mut state = TreeState::new();

		Self::rebuild_tree(
			&mut state,
			cx.al_project_mut().get_stage(stage_name).unwrap().unwrap(),
		);

		cx.new(|cx| Self {
			tree_view: state.build(cx, px(32.0)),
			stage_name: stage_name.into(),
		})
	}

	fn rebuild_tree(tree_state: &mut TreeState, stage: &Stage) {
		fn traverse(id_path: &str, folder: &Folder, stage: &Stage) -> TreeItem {
			TreeItem::new(id_path, &*folder.name).children(folder.children.iter().map(
				|child| match child {
					Node::Folder(folder) => {
						let id_path = format!("{id_path}/{}", folder.name);
						traverse(&id_path, folder, stage)
					}
					Node::Entity(entity_key) => {
						let path = format!("{id_path}/{entity_key:?}");
						let entity = stage.entities.get(*entity_key).unwrap();
						TreeItem::new(path, &*entity.object_name)
					}
				},
			))
		}
		let item = traverse("root", &stage.root, stage);

		tree_state.set_items([item].into_iter());
	}
}

impl Render for StageHierarchy {
	fn render(
		&mut self,
		_: &mut gpui::Window,
		_: &mut gpui::Context<Self>,
	) -> impl gpui::IntoElement {
		tree(&self.tree_view, |item, _, _| {
			Button::new(item.id())
				.label(item.label())
				.px(px(12.) + item.depth() * px(16.0))
		})
	}
}
