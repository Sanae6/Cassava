use std::{any::TypeId, collections::BTreeMap};

use gpui::{
	AnyView, App, AppContext, Context, DefiniteLength, Entity, EventEmitter, ParentElement, Render,
	Styled, Subscription, WeakEntity, Window, div, rgba,
};

use crate::components::{StyleExt, event::ContextExt, theme::ThemeExt};

pub struct Root {
	dialog_stack: Vec<AnyView>,
	root_view: AnyView,
}

impl Root {
	pub fn new(cx: &mut App, view: impl Into<AnyView>) -> Entity<Self> {
		cx.new(|cx| {
			cx.bus_subscribe(|root: &mut Self, event: &OpenDialog, _| {
				root.dialog_stack.push(event.0.clone());
			});
			cx.bus_subscribe(|root: &mut Self, event: &PopDialog, _| {
				root.dialog_stack.pop();
			});
			Self {
				dialog_stack: vec![],
				root_view: view.into(),
			}
		})
	}

	pub fn update_root<R>(
		window: &mut Window,
		cx: &mut App,
		func: impl FnOnce(&mut Self, &mut Window, &mut Context<Self>) -> R,
	) -> R {
		assert!(window.root_view().expect("root").entity_type() == TypeId::of::<Root>());
		unsafe { window.update_root::<Root, _>(cx, func) }
	}

	pub fn set_root_view(&mut self, root_view: impl Into<AnyView>) {
		self.root_view = root_view.into();
	}
}

impl Render for Root {
	fn render(
		&mut self,
		window: &mut gpui::Window,
		cx: &mut gpui::Context<Self>,
	) -> impl gpui::IntoElement {
		div()
			.size_full()
			.bg(cx.theme().bg)
			.text_color(rgba(0xFFFFFFFF))
			.child(self.root_view.clone())
			.child(
				div()
					.absolute()
					.children(self.dialog_stack.iter().cloned().map(|dialog| {
						let dialog = dialog.clone();
						div()
						.size_full()
							.child(
								div()
									.child(dialog)
									.h(DefiniteLength::Fraction(3.0 / 4.0))
									.w(DefiniteLength::Fraction(3.0 / 4.0)),
							)
							.inset_0()
							.bg(rgba(0x00000030))
							.v_flex()
							.items_center()
							.justify_center()
					}))
					.inset_0()
					.size_full(),
			)
	}
}

pub struct OpenDialog(AnyView);
impl OpenDialog {
	pub fn new(view: impl Into<AnyView>) -> Self {
		Self(view.into())
	}
}
pub struct PopDialog;
