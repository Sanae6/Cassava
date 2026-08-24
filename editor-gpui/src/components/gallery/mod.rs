mod start;
mod tree;

use gpui::{App, AppContext, Entity, ParentElement, Render, Styled, div, rgb};

use crate::components::tab_control::TabControl;

pub struct Gallery {
	tab_control: Entity<TabControl>,
}

impl Gallery {
	pub fn new(cx: &mut App) -> Self {
		Self {
			tab_control: cx.new(|cx| {
				let mut control = TabControl::new("gallery-tabs", cx);
				let view = cx.new(|_| start::StartView { v: true });
				control.add_tab(&view, |tab, _, _| tab.title("Hello world"));
				let view = cx.new(|_| start::StartView { v: false });
				control.add_tab(&view, |tab, _, _| tab.title("Test"));
				let view = cx.new(|cx| tree::TreeTest::new(cx));
				control.add_tab(&view, |tab, _, _| tab.title("Tabs"));
				control
			}),
		}
	}
}

impl Render for Gallery {
	fn render(
		&mut self,
		_: &mut gpui::Window,
		_: &mut gpui::Context<Self>,
	) -> impl gpui::IntoElement {
		div()
			.flex()
			.flex_col()
			.size_full()
			.child(self.tab_control.clone())
			.text_color(rgb(0xFFFFFF))
	}
}
