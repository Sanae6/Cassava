use std::{env::args, path::Path};

use cassava_project::ALProject;
use editor_web_macro::handlers;

pub struct ProjectManager {
	active_project: Option<ALProject>,
}

impl ProjectManager {
	pub fn new() -> ProjectManager {
		let mut manager = ProjectManager {
			active_project: None,
		};

    manager
	}

	pub fn try_load_project_from_arg(&mut self) {
		let project_dir = args().nth(1);
		if let Some(project_dir) = project_dir {
			match ALProject::load(Path::new(&project_dir)) {
				Ok(project) => self.active_project = Some(project),
				Err(err) => {
					eprintln!("{err}");
					std::process::exit(1);
				}
			};
		}
	}
}

#[handlers]
impl ProjectManager {
  
}
