use gpui::{App, AppContext, Entity, ParentElement, Render, Styled, div, px, rems};

use crate::components::{
	StyleExt,
	button::Button,
	theme::ThemeExt,
	tree::{TreeItem, TreeState, tree},
};

pub struct TreeTest {
	tree: Entity<TreeState>,
}
impl TreeTest {
	pub fn new(cx: &mut App) -> Self {
		Self {
			tree: TreeState::new()
				.items((1..=3).map(|i1| {
					TreeItem::new(i1.to_string(), format!("l1 - {i1}")).children((1..=3).map(|i2| {
						TreeItem::new(format!("{i1}-{i2}"), format!("l2 - {i2}")).children(
							(1..=3).map(|i3| TreeItem::new(format!("{i1}-{i2}-{i3}"), format!("l3 - {i3}"))),
						)
					}))
				}))
				.build(cx, rems(2.0)),
		}
	}
}

impl Render for TreeTest {
	fn render(
		&mut self,
		_: &mut gpui::Window,
		cx: &mut gpui::Context<Self>,
	) -> impl gpui::IntoElement {
		tree(&self.tree, |item, _, _| {
			Button::new(item.id())
				.label(item.label())
				.pl(px(16.) * item.depth() + px(12.))
		})
		.size_full()
		.bg(cx.theme().pane_bg)
	}
}
