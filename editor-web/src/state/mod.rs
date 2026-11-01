use webkit6::gtk::ApplicationWindow;

use crate::{infra::rpc::Rpc, state::{project::ProjectManager, settings::Settings}};

mod project;
mod settings;

pub fn register_singletons(window: ApplicationWindow) {
	let rpc = Rpc::instance();
	let mut rpc = rpc.borrow_mut();
	rpc.register_singleton("settings", Settings::new(window));
	rpc.register_singleton("project", ProjectManager::new());
}
