use gpui::{App, Rgba, rgb};

pub struct Theme {
	// primary background colour
	pub bg: Rgba,
	pub pane_bg: Rgba,
	pub button_normal: Rgba,
	pub button_hover: Rgba,
	pub button_active: Rgba,
	pub text_disabled: Rgba,
}

impl Theme {
	pub fn register(app: &mut App) {
		app.set_global(Self {
			bg: rgb(0x141414),
			pane_bg: rgb(0x292929),
			button_normal: rgb(0x303030),
			button_hover: rgb(0x404040),
			button_active: rgb(0x505050),
			text_disabled: rgb(0x808080),
		});
	}
}

impl gpui::Global for Theme {}

pub trait ThemeExt {
	fn theme(&self) -> &Theme;
}

impl ThemeExt for App {
	fn theme(&self) -> &Theme {
		self.global()
	}
}
