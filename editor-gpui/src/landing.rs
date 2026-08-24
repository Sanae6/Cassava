use std::{path::PathBuf, rc::Rc};

use cassava_project::ALProject;
use gpui::{
	AbsoluteLength, App, DefiniteLength, ListState, Pixels, Rems, SharedString, StyledText, div,
	list, prelude::*, px, rems,
};

use crate::{
	components::{StyleExt, button::Button, root::Root, theme::ThemeExt},
	config::{Config, ConfigAccessor},
	project::Project,
	workspace::Workspace,
};

pub struct LandingPage {
	project_list: ListState,
	projects: Vec<ExistingProject>,
}
#[derive(Debug)]
struct ExistingProject {
	name: SharedString,
	exists: bool,
	last_opened: SharedString,
	path_string: SharedString,
	path: PathBuf,
}

impl LandingPage {
	pub fn new(cx: &mut App) -> Self {
		let project_dirs = &cx.config().project_dirs;
		let project_list = ListState::new(project_dirs.len(), gpui::ListAlignment::Top, px(128.0));

		let projects = project_dirs
			.iter()
			.map(|project_dir| match ALProject::peek(project_dir) {
				Ok(peek) => ExistingProject {
					name: peek.name.into(),
					exists: true,
					last_opened: format!("Last opened: {}", peek.last_opened).into(),
					path: project_dir.clone(),
					path_string: project_dir.to_string_lossy().into(),
				},
				Err(error) => {
					eprintln!("failed to load project at {project_dir:?}");
					eprintln!("error: {error}");
					ExistingProject {
						name: project_dir
							.file_name()
							.map(ToOwned::to_owned)
							.unwrap_or_default()
							.to_string_lossy()
							.into(),
						exists: false,
						last_opened: "Last opened: unknown".into(),
						path: project_dir.clone(),
						path_string: project_dir.to_string_lossy().into(),
					}
				}
			})
			.collect();
		println!("projects: {projects:?}");

		Self {
			project_list,
			projects,
		}
	}
}

impl Render for LandingPage {
	fn render(&mut self, _: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
		let entity = cx.entity();
		div()
			.v_flex()
			.justify_center()
			.items_center()
			.bg(cx.theme().bg)
			.size_full()
			.child(
				div()
					.size(DefiniteLength::Fraction(2.0 / 3.0))
					.v_flex()
					.items_center()
					.bg(cx.theme().pane_bg)
					.rounded_md()
					// .shadow_md()
					.justify_center()
					.child(div().child("Cassava").text_2xl().p_4())
					.child(div().child("New project").on_click(|_, _, _| {}))
					.child(
						list(self.project_list.clone(), move |index, _, cx| {
							let p_entity = entity.clone();
							cx.read_entity(&entity, move |page, cx| {
								let project: &ExistingProject = page.projects.get(index).unwrap();
								div()
									.size_full()
									.p_4()
									.child(
										Button::new(project.name.clone())
											.child(
												div()
													.h_flex()
													.p_2()
													.justify_between()
													.items_start()
													.size_full()
													.child(
														div().v_flex().child(StyledText::new(&project.name)).child(
															div()
																.child(project.path_string.clone())
																.text_color(cx.theme().bg),
														),
													)
													.child(div().child("super mario").text_right()),
											)
											.on_click(move |_, window, cx| {
												let project = cx
													.read_entity(&p_entity, move |page, _| {
														let project: &ExistingProject = page.projects.get(index).unwrap();
														ALProject::load(&project.path)
													})
													.unwrap();
												cx.set_global(Project::new(project));
												Root::update_root(window, cx, |root, _, cx| {
													root.set_root_view(Workspace::new(cx))
												});
											})
											.size_full(),
									)
									.into_any_element()
							})
						})
						.size_full(),
					),
			)
	}
}
