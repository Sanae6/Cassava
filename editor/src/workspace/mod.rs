mod explorer;
mod import_stage;
pub mod stage_editor;

use std::cell::RefCell;

use gpui::{AnyView, App, Entity, Window, div, prelude::*, px};

use crate::{
	components::{
		StyleExt,
		event::ContextExt,
		tab_control::{Tab, TabControl},
	},
	workspace::explorer::Explorer,
};

struct ProjectUpdate;
struct NewTab(
	AnyView,
	RefCell<Option<Box<dyn Fn(Tab, &mut Window, &mut App) -> Tab + 'static>>>,
);
impl NewTab {
	pub fn new(
		view: Entity<impl Render>,
		func: impl Fn(Tab, &mut Window, &mut App) -> Tab + 'static,
	) -> Self {
		Self(view.into(), RefCell::new(Some(Box::new(func))))
	}
}

pub struct Workspace {
	explorer: Entity<Explorer>,
	tabs: Entity<TabControl>,
}

impl Workspace {
	pub fn new(cx: &mut App) -> Entity<Workspace> {
		cx.new(|cx| {
			cx.bus_subscribe(|workspace: &mut Self, event: &NewTab, cx| {
				workspace.tabs.update(cx, |tabs, _| {
					tabs.add_tab(
						event.0.clone(),
						event
							.1
							.borrow_mut()
							.take()
							.expect("tab configuration taken already"),
					)
				})
			});
			Self {
				explorer: Explorer::new(cx),
				tabs: cx.new(|cx| TabControl::new("workspace", cx)),
			}
		})
	}
}

impl Render for Workspace {
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
				.child(div().w(px(255.0)).h_full().child(self.explorer.clone()))
				.child(self.tabs.clone()),
		)
	}
}
