use std::cell::Cell;

use gpui::{
	AbsoluteLength, App, AppContext, ClickEvent, Entity, IntoElement, RenderOnce, SharedString,
	StatefulInteractiveElement, StyleRefinement, Styled, Window, prelude::FluentBuilder,
};
use ouroboros::self_referencing;

use crate::components::{
	StyleExt,
	button::Button,
	list::{ListState, list},
};

#[self_referencing]
struct TreeItems {
	items: Vec<TreeItem>,
	#[covariant]
	#[borrows(items)]
	flat_items: Vec<&'this TreeItem>,
}

impl Default for TreeItems {
	fn default() -> Self {
		Self::new(Vec::new(), |_| Vec::new())
	}
}

pub struct TreeState {
	items: TreeItems,
	selected: Option<usize>,
	list_state: Option<Entity<ListState>>,
}

#[derive(Clone)]
pub struct TreeItem {
	id: SharedString,
	label: SharedString,
	children: Vec<TreeItem>,
	collapsed: Cell<bool>,
	depth: usize,
}

impl TreeItem {
	pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
		Self {
			id: id.into(),
			label: label.into(),
			children: vec![],
			collapsed: false.into(),
			depth: 0,
		}
	}

	pub fn children(mut self, iter: impl Iterator<Item = TreeItem>) -> Self {
		self.children.extend(iter.map(|mut item| {
			item.inc_depth();
			item
		}));
		self
	}

	fn inc_depth(&mut self) {
		self.depth += 1;
		for child in &mut self.children {
			child.inc_depth();
		}
	}

	pub fn collapsed(&self) -> bool {
		self.collapsed.get()
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

	fn render(&self, parent_collapsed: bool, func: &mut impl FnMut(bool, &TreeItem)) {
		func(parent_collapsed, self);
		for child in &self.children {
			child.render(parent_collapsed || self.collapsed(), func);
		}
	}
}

impl TreeState {
	pub fn new() -> Self {
		Self {
			items: TreeItems::default(),
			selected: None,
			list_state: None,
		}
	}

	pub fn items(mut self, items: impl Iterator<Item = TreeItem>) -> Self {
		self.set_items(items);
		self
	}

	pub fn set_items(&mut self, iter: impl Iterator<Item = TreeItem>) {
		let mut items = std::mem::replace(&mut self.items, TreeItems::default())
			.into_heads()
			.items;
		items.clear();
		items.extend(iter);

		self.items = TreeItems::new(items, |items| {
			fn traverse<'a>(list: &mut Vec<&'a TreeItem>, item: &'a TreeItem) {
				list.push(item);
				for child in &item.children {
					traverse(list, child);
				}
			}
			let mut vec = Vec::new();
			for item in items {
				traverse(&mut vec, item);
			}
			vec
		});
	}

	pub fn selected(&self) -> Option<&TreeItem> {
		self
			.items
			.with_flat_items(|items| items.get(self.selected?))
			.map(|item| *item)
	}

	pub fn build(mut self, cx: &mut App, button_height: impl Into<AbsoluteLength>) -> Entity<Self> {
		cx.new::<Self>(|cx| {
			let element_count = self.items.borrow_flat_items().len();
			self.list_state = Some(ListState::new(cx, element_count, button_height));
			self
		})
	}
}

pub fn tree(
	state: &Entity<TreeState>,
	render: impl Fn(&TreeItem, &mut Window, &mut App) -> Button + 'static,
) -> Tree {
	Tree {
		state: state.clone(),
		render: Box::new(move |item, window, app| render(item, window, app).justify_start().w_full()),
		style: StyleRefinement::default(),
	}
}

#[derive(IntoElement)]
pub struct Tree {
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
		let state = self.state.read(cx);
		let list_state = state.list_state.clone().unwrap();
		list(&list_state, move |range, window, cx| {
			self.state.clone().update(cx, |state, cx| {
				let mut vec = Vec::with_capacity(range.len());

				let mut next_index = 0;
				for item in state.items.borrow_items() {
					item.render(false, &mut |collapsed, item| {
						let index = next_index;
						next_index += 1;
						if collapsed {
							return;
						}
						vec.push(
							(&self.render)(item, window, cx)
								.when_some(state.selected.clone(), |div, selected| {
									div.when(selected == index, |div| {
										div.state(super::button::ButtonState::Hovered)
									})
								})
								.on_click(window.listener_for(
									&cx.entity(),
									move |state, _event: &ClickEvent, _, cx| {
										state.selected = Some(index);
										let item: &TreeItem = state
											.items
											.with_flat_items(|items| items.get(index))
											.expect("missing selected item");
										if item.collapsed() {
											item.collapsed.set(false);
										} else {
											item.collapsed.set(true);
										}
										cx.notify();
									},
								))
								.into_any_element(),
						)
					})
				}
				vec
			})
		})
		// todo: escape, arrow keys actions
		.refine_style(self.style)
	}
}
