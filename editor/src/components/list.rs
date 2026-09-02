use std::{cell::RefCell, ops::Range, panic::Location, rc::Rc};

use gpui::{
	AbsoluteLength, AnyElement, App, AppContext, Element, ElementId, Entity, Hitbox, IntoElement,
	Length, Overflow, Pixels, Point, Refineable, ScrollWheelEvent, Size, Style, StyleRefinement,
	Styled, Window, px,
};

pub struct ListState {
	scroll_offset: f32,
	element_count: usize,
	element_height: AbsoluteLength,
}

impl ListState {
	pub fn new(
		cx: &mut App,
		element_count: usize,
		element_height: impl Into<AbsoluteLength>,
	) -> Entity<Self> {
		cx.new(|_| ListState {
			scroll_offset: 0.0,
			element_count,
			element_height: element_height.into(),
		})
	}
}

pub struct NaiveList {
	location: &'static Location<'static>,
	style: StyleRefinement,
	state: Entity<ListState>,
	render: Option<Box<dyn FnMut(Range<usize>, &mut Window, &mut App) -> Vec<AnyElement>>>,
}

#[track_caller]
pub fn list(
	state: &Entity<ListState>,
	render: impl FnMut(Range<usize>, &mut Window, &mut App) -> Vec<AnyElement> + 'static,
) -> NaiveList {
	let state = state.clone();
	NaiveList {
		location: Location::caller(),
		style: StyleRefinement::default(),
		state: state.clone(),
		render: Some(Box::new(render)),
	}
}

pub struct LayoutState {
	size: Size<Pixels>,
}
impl Element for NaiveList {
	type RequestLayoutState = Rc<RefCell<Option<Vec<AnyElement>>>>;

	type PrepaintState = Hitbox;

	fn id(&self) -> Option<ElementId> {
		Some(ElementId::View(self.state.entity_id()))
	}

	fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
		Some(self.location)
	}

	fn request_layout(
		&mut self,
		id: Option<&gpui::GlobalElementId>,
		_: Option<&gpui::InspectorElementId>,
		window: &mut Window,
		cx: &mut App,
	) -> (gpui::LayoutId, Self::RequestLayoutState) {
		let children_return = Rc::new(RefCell::new(None));
		let state = self.state.clone();
		let offset = state.read(cx).scroll_offset;
		let render = RefCell::new(self.render.take().unwrap());
		let children_return = children_return.clone();
		let layout = window.with_element_offset(Point::new(px(0.0), px(offset)), |window| {
			let size = window.with_element_state(id.unwrap(), |state: Option<LayoutState>, _| {
				let size = state.map(|state| state.size).unwrap_or_else(|| Size {
					width: px(0.0),
					height: px(0.0),
				});
				(size, LayoutState { size })
			});
			let state = state.read(cx);
			let element_height = state.element_height.to_pixels(window.rem_size());
			let scroll_offset = px(state.scroll_offset);
			let start_idx = (scroll_offset / element_height).floor() as usize;
			let end_idx = ((scroll_offset + size.height) / element_height).ceil() as usize;
			let visible_range = start_idx..end_idx.min(state.element_count);
			let mut children = render.borrow_mut()(visible_range.clone(), window, cx);

			let layouts = children
				.iter_mut()
				.into_iter()
				.map(|child| child.request_layout(window, cx))
				.collect::<Vec<_>>();
			let style = Style {
				size: Size::full(),
				overflow: Point {
					x: Overflow::Scroll,
					y: Overflow::Hidden,
				},
				..Style::default()
			};
			let layout = window.request_layout(style, layouts, cx);
			*children_return.borrow_mut() = Some(children);
			layout
		});
		(layout, children_return)
	}

	fn prepaint(
		&mut self,
		global_id: Option<&gpui::GlobalElementId>,
		_: Option<&gpui::InspectorElementId>,
		bounds: gpui::Bounds<gpui::Pixels>,
		children: &mut Self::RequestLayoutState,
		window: &mut Window,
		cx: &mut App,
	) -> Self::PrepaintState {
		window.with_element_state(global_id.unwrap(), |_: Option<LayoutState>, _| {
			(bounds.size, LayoutState { size: bounds.size })
		});
		let state = self.state.clone();
		let offset = state.read(cx).scroll_offset;

		let hitbox = window.insert_hitbox(bounds, gpui::HitboxBehavior::Normal);
		window.with_element_offset(Point::new(px(0.0), px(offset)), |window| {
			let mut children = children.borrow_mut();
			for child in children.as_mut().unwrap() {
				child.prepaint(window, cx);
			}
		});

		hitbox
	}

	fn paint(
		&mut self,
		_: Option<&gpui::GlobalElementId>,
		_: Option<&gpui::InspectorElementId>,
		bounds: gpui::Bounds<gpui::Pixels>,
		children: &mut Self::RequestLayoutState,
		hitbox: &mut Self::PrepaintState,
		window: &mut Window,
		cx: &mut App,
	) {
		let mut children = children.borrow_mut();
		let mut style = Style {
			size: bounds.size.map(Into::into).map(Length::Definite),
			overflow: Point {
				x: Overflow::Scroll,
				y: Overflow::Hidden,
			},
			..Style::default()
		};
		style.refine(&self.style);
		style.paint(bounds, window, cx, |window, cx| {
			window.with_content_mask(style.overflow_mask(bounds, window.rem_size()), |window| {
				for child in children.as_mut().unwrap() {
					child.paint(window, cx);
				}
			});
		});

		window.on_mouse_event({
			let hitbox = hitbox.clone();
			let state = self.state.read(cx);
			let total_height = (state.element_height.to_pixels(window.rem_size())
				* children.as_ref().unwrap().len() as f32
				- bounds.size.height)
				.max(px(0.0));
			let state = self.state.clone();

			move |scroll: &ScrollWheelEvent, phase, window, cx| {
				if hitbox.is_hovered(window) && phase.bubble() {
					let delta = scroll.delta.pixel_delta(window.rem_size());
					let delta = delta.x + delta.y;
					state.update(cx, |state, _| {
						state.scroll_offset += delta.to_f32();
						state.scroll_offset = state.scroll_offset.clamp(-total_height.to_f32(), 0.0);
					});
					cx.notify(state.entity_id());
					cx.stop_propagation();
				}
			}
		});
	}
}

impl IntoElement for NaiveList {
	type Element = Self;

	fn into_element(self) -> Self::Element {
		self
	}
}

impl Styled for NaiveList {
	fn style(&mut self) -> &mut StyleRefinement {
		&mut self.style
	}
}
