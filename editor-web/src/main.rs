#![feature(error_generic_member_access)]
#![feature(variant_count)]

mod infra;
mod state;

use snafu::{OptionExt, ResultExt, Whatever, whatever};
use webkit6::{
	UserContentInjectedFrames, UserScript, UserScriptInjectionTime, WebView,
	gtk::{self, ApplicationWindow},
	javascriptcore::Value,
	prelude::*,
};

use crate::{
	infra::{
		rpc::{Context, Rpc},
		serde,
	},
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
		state::register_singletons(window.clone());
		ucm.add_script(&UserScript::new(
			"window.rusty = true;",
			UserContentInjectedFrames::AllFrames,
			UserScriptInjectionTime::Start,
			&[],
			&[],
		));
		ucm.register_script_message_handler_with_reply("ui", None);
		let window2 = window.clone();
		let webview2 = webview.clone();
		ucm.connect_script_message_with_reply_received(Some("ui"), move |_, value, reply| {
			let webview2 = webview2.clone();
			let result = std::panic::catch_unwind(|| -> Result<Option<Value>, Whatever> {
				let message_type = value
					.object_get_property("__mty")
					.whatever_context("no message type was provided")?
					.to_str();
				let rpc = Rpc::instance();
				let context = value.context().expect("how is there no context");
				match message_type.as_str() {
					"get_singleton" => {
						let name = value
							.object_get_property("name")
							.whatever_context("no name was provided")?;
						let singleton = rpc.borrow().get_singleton(name.to_str().as_str());
						match singleton {
							Some(singleton) => Ok(Some(serde::serialize(&context, singleton).unwrap())),
							None => Ok(Some(Value::new_null(&context))),
						}
					}
					"rpc" => {
						let handler = rpc
							.borrow()
							.get_object(
								serde::deserialize(
									value
										.object_get_property("__id")
										.whatever_context("no name was provided")?,
								)
								.whatever_context("failed to extract id from message")?,
							)
							.whatever_context("handler does not exist for object")?;
						let progress_reporter = value
							.object_get_property("__progressUuid")
							.map(|value| value.to_str().as_str().to_owned());
						let context = Context {
							rc: handler.clone(),
							webview: webview2,
							progress_reporter: progress_reporter,
						};
						match handler.borrow_mut().handle(context, reply.clone(), value) {
							Ok(value) => Ok(value),
							Err(error) => {
								whatever!("{error}");
							}
						}
					}
					mty => {
						whatever!("unimplemented message type: {mty}");
					}
				}
			});
			match result {
				Ok(Ok(Some(value))) => {
					reply.return_value(&value);
					false
				}
				Ok(Ok(None)) => {
					// reply has been taken elsewhere, asynchronous reply
					true
				}
				Ok(Err(error)) => {
					reply.return_error_message(&error.to_string());
					false
				}
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
