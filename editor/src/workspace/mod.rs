mod explorer;
mod import_stage;
mod project;
pub mod stage_editor;

use std::cell::RefCell;

use gpui::{AnyView, App, Entity, Window, div, prelude::*, px};

use crate::{
	components::{
		StyleExt,
		event::ContextExt,
		tab_control::{Tab, TabControl},
	},
	workspace::project::ProjectView,
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
	tabs: Entity<TabControl>,
}

impl Workspace {
	pub fn new(cx: &mut App) -> Entity<Workspace> {
		cx.new(|cx| {
			cx.bus_subscribe(|workspace: &mut Self, event: &NewTab, cx| {
				workspace.tabs.update(cx, |tabs, cx| {
					let tab_index = tabs.add_tab(
						event.0.clone(),
						event
							.1
							.borrow_mut()
							.take()
							.expect("tab configuration taken already"),
					);
					tabs.select_tab(tab_index, cx);
				})
			});

			Self {
				tabs: cx.new(|cx| {
					let mut tabs = TabControl::new("workspace", cx);
					let view = ProjectView::new(cx);
					tabs.add_tab(view.into(), |tab, _, _| tab.title("Project").closeable(false));
					tabs
				}),
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
		div()
			.v_flex()
			.size_full()
			.child(div().h_flex().gap_2().size_full().child(self.tabs.clone()))
	}
}
