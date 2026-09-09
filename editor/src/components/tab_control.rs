use gpui::{
	AnyView, App, AppContext, Context, ElementId, Entity, InteractiveElement, IntoElement,
	ParentElement, Point, Render, RenderOnce, ScrollHandle, ScrollWheelEvent, SharedString,
	StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder, px,
};

use crate::components::{StyleExt, button::Button, theme::ThemeExt};

pub struct TabControl {
	base_id: SharedString,
	tabs: Vec<TabEntry>,
	active_tab: usize,
	scroll_handle: ScrollHandle,
	next_unique_id: u32,
	live_ids: Vec<u32>,
	tab_placeholder: Entity<TabPlaceholder>,
}

struct TabEntry {
	unique_id: SharedString,
	entity: AnyView,
	configure: Box<dyn Fn(Tab, &mut Window, &mut App) -> Tab>,
}

#[derive(IntoElement)]
pub struct Tab {
	tab_control: Entity<TabControl>,
	index: usize,
	id: SharedString,
	active: bool,
	title: SharedString,
	closeable: bool,
}

impl Tab {
	fn new(tab_control: Entity<TabControl>, index: usize, id: SharedString, active: bool) -> Self {
		Self {
			tab_control,
			index,
			id,
			active,
			title: "".into(),
			closeable: true,
		}
	}

	pub fn title(mut self, title: impl Into<SharedString>) -> Self {
		self.title = title.into();
		self
	}

	pub fn closeable(mut self, closeable: bool) -> Self {
		self.closeable = closeable;
		self
	}
}

impl RenderOnce for Tab {
	fn render(self, _: &mut Window, cx: &mut App) -> impl gpui::IntoElement {
		let index = self.index;
		Button::new(self.id)
			.label(self.title)
			.bg(cx.theme().bg)
			.active(|div| div.bg(cx.theme().pane_bg))
			.when(self.active, |div| div.bg(cx.theme().pane_bg))
			.on_click(move |_, _, cx| {
				self
					.tab_control
					.update(cx, |tabs, cx| tabs.select_tab(index, cx));
			})
	}
}

impl TabControl {
	pub fn new(base_id: &str, cx: &mut App) -> Self {
		Self {
			base_id: base_id.into(),
			tabs: Vec::new(),
			active_tab: 0,
			scroll_handle: Default::default(),
			live_ids: Vec::new(),
			next_unique_id: 0,
			tab_placeholder: cx.new(|_| TabPlaceholder),
		}
	}

	pub fn add_tab(
		&mut self,
		tab_view: AnyView,
		configure: impl Fn(Tab, &mut Window, &mut App) -> Tab + 'static,
	) -> usize {
		let unique_id = loop {
			let unique_id = self.next_unique_id;
			self.next_unique_id = self.next_unique_id.wrapping_add(1);

			if !self.live_ids.contains(&unique_id) {
				break unique_id;
			}
		};
		self.live_ids.push(unique_id);

		self.tabs.push(TabEntry {
			unique_id: unique_id.to_string().into(),
			entity: tab_view.clone().into(),
			configure: Box::new(configure),
		});

		self.tabs.len() - 1
	}

	pub fn select_tab(&mut self, index: usize, cx: &mut Context<Self>) {
		self.active_tab = index;
		cx.notify();
	}
}

impl Render for TabControl {
	fn render(
		&mut self,
		window: &mut gpui::Window,
		cx: &mut gpui::Context<Self>,
	) -> impl gpui::IntoElement {
		div()
			.v_flex()
			.size_full()
			.child(
				div()
					.id(ElementId::Name(self.base_id.clone()))
					.h_flex()
					.w_full()
					.h_8()
					.gap_1()
					.track_scroll(&self.scroll_handle)
					.flex_nowrap()
					.on_scroll_wheel(window.listener_for(
						&cx.entity(),
						|view, scroll: &ScrollWheelEvent, _, cx| {
							let offset = view.scroll_handle.offset();
							let delta = scroll.delta.pixel_delta(px(10.0));
							let delta = delta.x + delta.y;
							view
								.scroll_handle
								.set_offset(offset + Point::new(delta, px(0.0)));
							cx.notify();
						},
					))
					.children(self.tabs.iter().enumerate().map(|(index, tab_entry)| {
						let tab = Tab::new(
							cx.entity(),
							index,
							tab_entry.unique_id.clone(),
							self.active_tab == index,
						);
						(tab_entry.configure)(tab, window, cx)
					})),
			)
			.child(
				self
					.tabs
					.get(self.active_tab)
					.map(|tab: &TabEntry| tab.entity.clone())
					.unwrap_or_else(|| self.tab_placeholder.clone().into()),
			)
	}
}

pub struct TabPlaceholder;
impl Render for TabPlaceholder {
	fn render(&mut self, _: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
		div().size_full().bg(cx.theme().pane_bg)
	}
}
