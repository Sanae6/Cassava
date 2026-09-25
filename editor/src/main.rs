pub mod components;
mod config;
// mod landing;
mod landing;
mod project;
mod workspace;
// mod workspace;

use std::{
	env::args,
	path::Path,
	sync::atomic::{AtomicU32, Ordering},
};

use cassava_project::ALProject;
use gpui::{App, Application, WgpuOptions, Window, WindowOptions, prelude::*};
use wgpu::Features;

use crate::{
	components::{event::EventBus, gallery::Gallery, root::Root, theme::Theme},
	config::Config,
	landing::LandingPage,
	project::Project,
	workspace::Workspace,
};

// use crate::{config::Config, landing::LandingPage, project::Project, workspace::Workspace};
fn refresh(window: &mut Window, _: &mut App) {
	static REFRESH_COUNTER: AtomicU32 = AtomicU32::new(3);
	window.refresh();
	if REFRESH_COUNTER.fetch_sub(1, Ordering::AcqRel) > 0 {
		window.on_next_frame(refresh);
	}
}

fn main() {
	// Application::new().run(|cx| {
	// });
	Application::with_wgpu_options(WgpuOptions {
		additional_features: Features::PIPELINE_CACHE
			| Features::TEXTURE_COMPRESSION_BC,
	})
	.run(|cx| {
		Theme::register(cx);
		Config::register(cx);
		EventBus::register(cx);

		if false {
			cx.open_window(WindowOptions::default(), |window, cx| {
				let view = Gallery::new(cx);
				window.on_next_frame(refresh);
				Root::new(cx, view)
			})
			.expect("failed to open window");
		} else {
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
						window.on_next_frame(refresh);
						let view = Workspace::new(cx);
						Root::new(cx, view)
					})
					.expect("failed to open window");
				}
				None => {
					cx.open_window(WindowOptions::default(), |window, cx| {
						window.on_next_frame(refresh);
						let view = cx.new(|cx| LandingPage::new(cx));
						Root::new(cx, view)
					})
					.expect("failed to open window");
				}
			};
		}
	});
}

// #[derive(Debug)]
// struct RootWindow(pub AnyWindowHandle);
// impl gpui::Global for RootWindow {}
