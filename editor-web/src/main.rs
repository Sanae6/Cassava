#![feature(error_generic_member_access)]

mod infra;
mod state;

use webkit6::{
	UserContentInjectedFrames, UserScript, UserScriptInjectionTime, WebView, gtk::{self, ApplicationWindow}, javascriptcore::Value, prelude::*
};

use crate::infra::{
	rpc::{Rpc, TestService},
	serde,
};

fn main() {
	let app = gtk::Application::new(Some("ca.sanae.crescent.app"), Default::default());
	app.connect_activate(move |app| {
		let window = ApplicationWindow::new(app);

		let webview = WebView::new();

		let window_title = window.clone();
		webview.connect_title_notify(move |webview| {
			let b = webview.title().unwrap();
			window_title.set_title(Some(b.as_str()));
		});
		let ucm = webview.user_content_manager().unwrap();
		Rpc::setup();
		Rpc::instance()
			.borrow_mut()
			.register_singleton("test", TestService);
		ucm.add_script(&UserScript::new(
			"window.rusty = true;",
			UserContentInjectedFrames::AllFrames,
			UserScriptInjectionTime::Start,
			&[],
			&[],
		));
		ucm.register_script_message_handler_with_reply("ui", None);
		let window2 = window.clone();
		ucm.connect_script_message_with_reply_received(Some("ui"), move |_, value, reply| {
			let result = std::panic::catch_unwind(|| {
				let message_type = value.object_get_property("__mty").unwrap().to_str();
				let rpc = Rpc::instance();
				let context = value.context().expect("how is there no context");
				match message_type.as_str() {
					"get_singleton" => {
						let name = value
							.object_get_property("name")
							.expect("no name was provided");
						let singleton = rpc.borrow().get_singleton(name.to_str().as_str());
						match singleton {
							Some(singleton) => {
								reply.return_value(&serde::serialize(&context, singleton).unwrap());
							}
							None => reply.return_value(&Value::new_null(&context)),
						}
					}
					"rpc" => {
						let handler = rpc
							.borrow()
							.get_object(
								serde::deserialize(
									value
										.object_get_property("__id")
										.expect("no name was provided"),
								)
								.expect("failed to extract id from message"),
							)
							.expect("handler does not exist for object");
						reply.return_value(&handler.borrow_mut().handle(value));
					}
					mty => unimplemented!("unimplemented message type: {mty}"),
				}
			});
			match result {
				Ok(_) => true,
				Err(error) => {
					eprintln!("{error:?}");
					reply.return_error_message(&format!("{:?}", error));
					window2.close();
					false
				}
			}
		});

		webview.load_uri("http://localhost:5173");
		window.set_child(Some(&webview));

		let settings = WebViewExt::settings(&webview).unwrap();
		settings.set_enable_developer_extras(true);

		let inspector = webview.inspector().unwrap();
		inspector.show();

		window.present();
	});
	app.run();
}
