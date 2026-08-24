pub mod components;
mod config;
// mod landing;
mod landing;
mod project;
mod workspace;
// mod workspace;

use std::{env::args, path::Path};

use cassava_project::ALProject;
use gpui::{AnyWindowHandle, App, Application, Window, WindowId, WindowOptions, prelude::*};

use crate::{
	components::{event::EventBus, gallery::Gallery, root::Root, theme::Theme},
	config::Config,
	landing::LandingPage,
	project::Project,
	workspace::Workspace,
};

// use crate::{config::Config, landing::LandingPage, project::Project, workspace::Workspace};

fn main() {
	// Application::new().run(|cx| {
	// 	cx.open_window(WindowOptions::default(), |window, cx| {
	// 		Theme::register(cx);
	// 		cx.new(|cx| Gallery::new(cx))
	// 	}).expect("failed to open window");
	// });
	Application::new().run(|cx| {
		Theme::register(cx);
		Config::register(cx);
		EventBus::register(cx);
		cx.spawn(async move |cx| {
			let args = args().nth(1);
			match args {
				Some(project_dir) => {
					let project = match ALProject::load(Path::new(&project_dir)) {
						Ok(project) => project,
						Err(err) => {
							eprintln!("{err}");
							std::process::exit(1);
						}
					};
					cx.open_window(WindowOptions::default(), |window, cx| {
						cx.set_global(Project::new(project));
						let view = Workspace::new(cx);
						Root::new(cx, view)
					})
					.expect("failed to open window");
				}
				None => {
					cx.open_window(WindowOptions::default(), |window, cx| {
						let view = cx.new(|cx| LandingPage::new(cx));
						Root::new(cx, view)
					})
					.expect("failed to open window");
				}
			};
		})
		.detach()
	});
}

// #[derive(Debug)]
// struct RootWindow(pub AnyWindowHandle);
// impl gpui::Global for RootWindow {}
