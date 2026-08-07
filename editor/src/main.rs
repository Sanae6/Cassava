mod window;

use gtk::prelude::*;
use gtk::{Application, gio, glib};
use window::Window;

const APP_ID: &str = "ca.sanae.Cassava";

fn main() -> glib::ExitCode {
	gio::resources_register_include!("resources.gresource").expect("Failed to register resources.");
	let app = Application::builder().application_id(APP_ID).build();
	app.connect_activate(build_ui);
	app.run()
}

fn build_ui(app: &Application) {
	let window = Window::new(app);
	window.present();
}
