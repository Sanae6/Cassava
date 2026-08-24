use gpui::{
	AnyElement, Div, InteractiveElement, Interactivity, IntoElement, ParentElement, RenderOnce,
	SharedString, Stateful, StatefulInteractiveElement, StyleRefinement, Styled, div,
	prelude::FluentBuilder, rgb,
};

use crate::components::StyleExt;

#[derive(IntoElement)]
pub struct Button {
	label: Option<SharedString>,
	base: Stateful<Div>,
	children: Vec<AnyElement>,
	forced_state: Option<ButtonState>,
}

impl Button {
	pub fn new(id: impl Into<SharedString>) -> Self {
		Self {
			label: None,
			base: div().id(id.into()),
			children: Vec::new(),
			forced_state: None,
		}
	}

	pub fn label(mut self, label: impl Into<SharedString>) -> Self {
		self.label = Some(label.into());
		self
	}

	pub fn state(mut self, forced_state: ButtonState) -> Self {
		self.forced_state = Some(forced_state);
		self
	}
}

pub enum ButtonState {
	Normal,
	Hovered,
	Clicked,
}

impl Styled for Button {
	fn style(&mut self) -> &mut StyleRefinement {
		self.base.style()
	}
}

impl ParentElement for Button {
	fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
		self.children.extend(elements);
	}
}

impl InteractiveElement for Button {
	fn interactivity(&mut self) -> &mut Interactivity {
		self.base.interactivity()
	}
}

impl StatefulInteractiveElement for Button {}

impl RenderOnce for Button {
	fn render(mut self, _: &mut gpui::Window, _: &mut gpui::App) -> impl IntoElement {
		let style = self.base.style().clone();
		self
			.base
			.flex()
			.flex_shrink_0()
			.items_center()
			.justify_center()
			.h_8()
			.px_2()
			// .border_1()
			// .border_color(rgb(0xC0C0C0))
			.bg(rgb(0x303030))
			.text_color(rgb(0xFFFFFF))
			.hover(|div| div.bg(rgb(0x404040)))
			.active(|div| div.bg(rgb(0x505050)))
			.map(|div| match self.forced_state {
				Some(ButtonState::Normal) => div.bg(rgb(0x303030)),
				Some(ButtonState::Hovered) => div.bg(rgb(0x404040)),
				Some(ButtonState::Clicked) => div.bg(rgb(0x505050)),
				None => div,
			})
			.when_some(self.label, |div, label| div.child(label))
			.children(self.children)
			.refine_style(style)
	}
}
