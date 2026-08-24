use gpui::{App, Rgba, rgba};

pub struct Theme {
	// primary background colour
	pub bg: Rgba,
	pub pane_bg: Rgba,
}

impl Theme {
	pub fn register(app: &mut App) {
		app.set_global(Self {
			bg: rgba(0x141414FF),
			pane_bg: rgba(0x292929FF),
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
