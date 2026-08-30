mod explorer;
mod import_stage;

use gpui::{App, Entity, EventEmitter, StyledText, div, prelude::*, px};

use crate::{components::StyleExt, project::Project, workspace::explorer::Explorer};

struct ProjectUpdate;

pub struct Workspace {
	explorer: Entity<Explorer>,
}

impl Workspace {
	pub fn new(cx: &mut App) -> Entity<Workspace> {
		cx.new(|cx| Self {
			explorer: Explorer::new(cx),
		})
	}
}

impl Render for Workspace {
	fn render(
		&mut self,
		_: &mut gpui::Window,
		cx: &mut gpui::Context<Self>,
	) -> impl gpui::IntoElement {
		div().v_flex().size_full().child(
			div()
				.h_flex()
				.items_baseline()
				.gap_2()
				.size_full()
				.child(
					div()
						.w(px(255.0))
						.h_full()
						.child(StyledText::new(cx.global::<Project>().project_name()))
						.child(self.explorer.clone()),
				)
				.child("Main view"),
		)
	}
}

impl EventEmitter<ProjectUpdate> for Workspace {}
