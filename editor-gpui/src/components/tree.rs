use gpui::{
	AnyElement, App, AppContext, ClickEvent, ElementId, Entity, EntityId, IntoElement, ParentElement, RenderOnce, Reservation, SharedString, StatefulInteractiveElement, StyleRefinement, Styled, Window, div, prelude::FluentBuilder, uniform_list
};

use crate::components::{StyleExt, button::Button};

pub struct TreeState {
	entries: Vec<TreeItem>,
	selected: Option<usize>,
}

#[derive(Clone)]
pub struct TreeItem {
	id: SharedString,
	label: SharedString,
	// flattened list of children
	children: Vec<TreeItem>,
	collapsed: bool,
	depth: usize,
}

impl TreeItem {
	pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
		Self {
			id: id.into(),
			label: label.into(),
			children: vec![],
			collapsed: false,
			depth: 0,
		}
	}

	pub fn children(mut self, iter: impl Iterator<Item = TreeItem>) -> Self {
		self.children.extend(iter.flat_map(|mut item| {
			let children = std::mem::take(&mut item.children);
			std::iter::once(item).chain(children).map(|mut item| {
				item.depth += 1;
				item
			})
		}));
		self
	}

	pub fn id(&self) -> SharedString {
		self.id.clone()
	}

	pub fn label(&self) -> SharedString {
		self.label.clone()
	}

	pub fn depth(&self) -> usize {
		self.depth
	}
}

impl TreeState {
	pub fn new() -> Self {
		Self {
			entries: Vec::new(),
			selected: None,
		}
	}

	pub fn items(mut self, items: impl Iterator<Item = TreeItem>) -> Self {
		self.set_items(items);
		self
	}

	pub fn set_items(&mut self, items: impl Iterator<Item = TreeItem>) {
		self.entries.clear();

		self.entries.extend(items.flat_map(|mut item| {
			let children = std::mem::take(&mut item.children);
			[item].into_iter().chain(children)
		}));
	}

	pub fn selected(&self) -> Option<&TreeItem> {
		self.entries.get(self.selected?)
	}
}

pub fn tree(
	state: &Entity<TreeState>,
	render: impl Fn(&TreeItem, &mut Window, &mut App) -> Button + 'static,
) -> Tree {
	Tree {
		id: ElementId::Name(format!("tree-{}", state.entity_id()).into()),
		state: state.clone(),
		render: Box::new(move |item, window, app| render(item, window, app).justify_start().w_full()),
		style: StyleRefinement::default(),
	}
}

#[derive(IntoElement)]
pub struct Tree {
	id: ElementId,
	state: Entity<TreeState>,
	render: Box<dyn Fn(&TreeItem, &mut Window, &mut App) -> Button>,
	style: StyleRefinement,
}

impl Styled for Tree {
	fn style(&mut self) -> &mut StyleRefinement {
		&mut self.style
	}
}

impl RenderOnce for Tree {
	fn render(self, _: &mut Window, cx: &mut App) -> impl gpui::IntoElement {
		let entry_count = self.state.read(cx).entries.len();
		uniform_list(self.id, entry_count, move |range, window, cx| {
			self.state.clone().update(cx, |state, cx| {
				let mut vec = Vec::with_capacity(range.len());
				vec.extend(
					state
						.entries
						.get(range)
						.into_iter()
						.flatten()
						.enumerate()
						.map(|(index, entry)| {
							(&self.render)(entry, window, cx)
							.when_some(state.selected.clone(), |div, selected| {
								div.when(selected == index, |div| div.state(super::button::ButtonState::Hovered))
							})
							.on_click(window.listener_for(
								&cx.entity(),
								move |state, _event: &ClickEvent, _, _| {
									state.selected = Some(index);
								},
							))
						}),
				);
				vec
			})
		})
		.size_full()
		// todo: escape, arrow keys actions
		.refine_style(self.style)
	}
}
