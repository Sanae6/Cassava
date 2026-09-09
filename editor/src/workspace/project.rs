use gpui::{App, AppContext, Entity, ParentElement, Render, Styled, div, px};

use crate::{components::StyleExt, workspace::explorer::Explorer};

pub struct ProjectView {
	explorer: Entity<Explorer>,
}

impl ProjectView {
	pub fn new(cx: &mut App) -> Entity<Self> {
		cx.new(|cx| Self {
			explorer: Explorer::new(cx),
		})
	}
}
/*
show explorer tree
show last opened files
 */

impl Render for ProjectView {
	fn render(
		&mut self,
		_: &mut gpui::Window,
		_: &mut gpui::Context<Self>,
	) -> impl gpui::IntoElement {
		div().v_flex().size_full().child(
			div()
				.h_flex()
				.gap_2()
				.size_full()
				.child(div().w(px(255.0)).h_full().child(self.explorer.clone())),
		)
	}
}
