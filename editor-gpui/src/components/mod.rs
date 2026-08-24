use gpui::{Refineable, StyleRefinement, Styled, rgb};

pub mod button;
pub mod event;
pub(crate) mod gallery;
pub mod root;
pub mod tab_control;
pub mod theme;
pub mod tree;

pub trait StyleExt: Styled {
	fn h_flex(self) -> Self {
		self.flex().flex_row()
	}

	fn v_flex(self) -> Self {
		self.flex().flex_col()
	}

	fn refine_style(mut self, style: StyleRefinement) -> Self {
		self.style().refine(&style);
		self
	}

	/// adds a red debug border for checking if elements are missized
	fn border_debug(self, shade: u8) -> Self {
		self.border_2().border_color(rgb((shade as u32) << 16))
	}
}

impl<T: Styled> StyleExt for T {}
