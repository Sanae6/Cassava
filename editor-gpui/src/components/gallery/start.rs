use gpui::{ParentElement, Render, Styled, div};

use crate::components::theme::ThemeExt;

pub struct StartView {
	pub v: bool,
}

impl Render for StartView {
	fn render(
		&mut self,
		_: &mut gpui::Window,
		cx: &mut gpui::Context<Self>,
	) -> impl gpui::IntoElement {
		div()
			.size_full()
			.child(if self.v { "hi :3" } else { "tee" })
			.bg(cx.theme().pane_bg)
	}
}
