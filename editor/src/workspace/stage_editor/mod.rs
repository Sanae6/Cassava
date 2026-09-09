// mod camera;
mod camera;
mod hierarchy;
mod view;

use std::{collections::HashMap, sync::Arc};

use cassava_project::stage::Stage;
use glam::Vec2;
use gpui::{
	App, Entity, FocusHandle, InteractiveElement, KeyDownEvent, KeyUpEvent, MouseButton,
	MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, Pixels, Point, Render,
	StatefulInteractiveElement, Styled, WgpuSurfaceHandle, Window, div, prelude::FluentBuilder, px,
	rgb, wgpu_surface,
};
use wgpu::TextureFormat;

use crate::{
	components::{StyleExt, theme::ThemeExt},
	project::ProjectAccessor,
	workspace::stage_editor::{hierarchy::StageHierarchy, view::StageView},
};

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
enum StageAction {
	MoveForward,
	MoveLeft,
	MoveBack,
	MoveRight,
	MoveUp,
	MoveDown,
	Select,
	Lock,
}
#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum ActionState {
	#[default]
	Released,
	Pressed,
	Held,
}

#[derive(Default)]
struct Actions {
	actions: HashMap<StageAction, ActionState>,
	last_mouse_position: Option<Point<Pixels>>,
	mouse_delta: Vec2,
}
impl Actions {
	fn match_key(key: &str) -> Option<StageAction> {
		match key {
			"w" => Some(StageAction::MoveForward),
			"a" => Some(StageAction::MoveLeft),
			"s" => Some(StageAction::MoveBack),
			"d" => Some(StageAction::MoveRight),
			"q" => Some(StageAction::MoveDown),
			"e" => Some(StageAction::MoveUp),
			_ => None,
		}
	}
	fn match_mouse(mouse_button: &MouseButton) -> Option<StageAction> {
		match mouse_button {
			MouseButton::Left => Some(StageAction::Select),
			MouseButton::Right => Some(StageAction::Lock),
			_ => None,
		}
	}
	fn update_action(&mut self, action: StageAction, down: bool) {
		*self.actions.entry(action).or_default() = match down {
			true => ActionState::Pressed,
			false => ActionState::Released,
		};
	}

	fn step(&mut self) -> bool {
		self.mouse_delta = Vec2::ZERO;
		let mut any_down = false;
		for (_, state) in self.actions.iter_mut() {
			if let ActionState::Pressed = state {
				*state = ActionState::Held;
			}

			if let ActionState::Held = state {
				any_down = true;
			}
		}
		any_down
	}

	fn pressed(&self, action: StageAction) -> bool {
		self.actions.get(&action).cloned().unwrap_or_default() == ActionState::Pressed
	}

	fn held(&self, action: StageAction) -> bool {
		self.actions.get(&action).cloned().unwrap_or_default() != ActionState::Released
	}
}

pub struct StageEditor {
	surface: WgpuSurfaceHandle,
	stage_name: String,
	view: Option<StageView>,
	actions: Actions,
	focus: FocusHandle,
	hierarchy: Entity<StageHierarchy>,
}

impl StageEditor {
	pub fn new(stage_name: &str, window: &mut Window, cx: &mut App) -> Self {
		let focus = cx.focus_handle();
		let hierarchy = StageHierarchy::new(stage_name, window, cx);
		Self {
			surface: window
				.create_wgpu_surface(800, 600, TextureFormat::Rgba8UnormSrgb)
				.expect("unable to create"),
			stage_name: stage_name.into(),
			view: None,
			actions: Actions::default(),
			focus,
			hierarchy,
		}
	}

	pub fn stage<'a>(&self, cx: &'a mut App) -> &'a mut Stage {
		cx.al_project_mut()
			.get_stage(&self.stage_name)
			.unwrap()
			.unwrap()
	}

	fn unwrap_view(&mut self) -> &mut StageView {
		self.view.as_mut().expect("view has not been initialised")
	}
}

impl Render for StageEditor {
	fn render(
		&mut self,
		window: &mut gpui::Window,
		cx: &mut gpui::Context<Self>,
	) -> impl gpui::IntoElement {
		if let Some((texture_view, (w, h))) = self.surface.back_view_with_size() {
			let stage = cx
				.al_project_mut()
				.get_stage(&self.stage_name)
				.unwrap()
				.unwrap();
			let view = self.view.get_or_insert_with(|| {
				let device = Arc::new(self.surface.device().clone());
				let queue = Arc::new(self.surface.queue().clone());
				let format = self.surface.format();
				StageView::new(device, queue, w, h, format, stage)
			});

			view.try_resize(w, h);

			view.render(texture_view);

			self.surface.swap_buffers();
		}
		cx.on_next_frame(window, |editor, window, cx| {
			if let Some(view) = &mut editor.view {
				view.update(&editor.actions);
			}
			if editor.actions.step() {
				cx.on_next_frame(window, |_, _, cx| cx.notify());
			}
		});

		let editor = cx.entity();
		window
			.on_focus_out(&self.focus, cx, move |_, _, cx| {
				editor.update(cx, |editor, _| {
					editor.actions.last_mouse_position = None;
				});
			})
			.detach();
		div()
			.h_flex()
			.child(div().h_full().w(px(255.0)).child(self.hierarchy.clone()))
			.child(wgpu_surface(self.surface.clone()).size_full())
			.bg(cx.theme().pane_bg)
			.size_full()
			.focusable()
			.track_focus(&self.focus)
			.on_key_up(cx.listener(|editor, up: &KeyUpEvent, window, _| {
				let Some(action) = Actions::match_key(&up.keystroke.key) else {
					return;
				};

				editor.actions.update_action(action, false);
				window.request_animation_frame();
			}))
			.on_key_down(cx.listener(|editor, down: &KeyDownEvent, window, _| {
				if down.is_held {
					return;
				}
				let Some(action) = Actions::match_key(&down.keystroke.key) else {
					return;
				};

				editor.actions.update_action(action, true);
				window.request_animation_frame();
			}))
			.map(|mut div| {
				div.interactivity().on_any_mouse_up(cx.listener(
					|editor, down: &MouseUpEvent, window, _| {
						let Some(action) = Actions::match_mouse(&down.button) else {
							return;
						};

						editor.actions.update_action(action, false);
						window.request_animation_frame();
					},
				));
				div
			})
			.on_any_mouse_down(cx.listener(|editor, down: &MouseDownEvent, window, _| {
				let Some(action) = Actions::match_mouse(&down.button) else {
					return;
				};

				editor.actions.update_action(action, true);
				window.request_animation_frame();
			}))
			.on_mouse_move(cx.listener(|editor, event: &MouseMoveEvent, window, _| {
				let last_position = editor.actions.last_mouse_position.replace(event.position);
				let Some(last_position) = last_position else {
					return;
				};

				let delta = last_position - event.position;
				editor.actions.mouse_delta = Vec2::new(delta.x.to_f32(), delta.y.to_f32());
				window.request_animation_frame();
			}))
	}
}
