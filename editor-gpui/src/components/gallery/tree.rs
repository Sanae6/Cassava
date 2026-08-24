use gpui::{App, AppContext, Entity, ParentElement, Render, Styled, div, px};

use crate::components::{
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
			tree: cx.new(|_| {
				TreeState::new().items((1..=3).map(|i1| {
					TreeItem::new(i1.to_string(), i1.to_string()).children((1..=3).map(|i2| {
						TreeItem::new(format!("{i1}-{i2}"), i2.to_string())
							.children((1..=3).map(|i3| TreeItem::new(format!("{i1}-{i2}-{i3}"), i3.to_string())))
					}))
				}))
			}),
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
