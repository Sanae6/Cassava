use std::ops::{Deref, DerefMut};

use cassava_project::ALProject;
use gpui::{App, Context, SharedString};

pub struct Project {
	project: ALProject,
	project_name: SharedString,
}

impl Project {
	pub fn new(project: ALProject) -> Self {
		Self {
			project_name: project.name.clone().into(),
			project: (project),
		}
	}

	pub fn project_name(&self) -> SharedString {
		self.project_name.clone()
	}

	pub fn al_project(&self) -> &ALProject {
		&self.project
	}

	pub fn al_project_mut(&mut self) -> &mut ALProject {
		&mut self.project
	}
}

impl gpui::Global for Project {}

pub trait ProjectAccessor {
	fn project(&self) -> &Project;
	fn al_project(&self) -> &ALProject;
	fn project_mut(&mut self) -> &mut Project;
	fn al_project_mut(&mut self) -> &mut ALProject;
}

impl ProjectAccessor for App {
	fn project(&self) -> &Project {
		self.global()
	}

	fn al_project(&self) -> &ALProject {
		self.project().al_project()
	}

	fn project_mut(&mut self) -> &mut Project {
		self.global_mut()
	}

	fn al_project_mut(&mut self) -> &mut ALProject {
		self.project_mut().al_project_mut()
	}
}
