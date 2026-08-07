use glib::Object;
use gtk::{Application, gio, glib};

glib::wrapper! {
	pub struct Window(ObjectSubclass<imp::WindowInner>)
		@extends adw::ApplicationWindow, gtk::ApplicationWindow, gtk::Window, gtk::Widget,
		@implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
			gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl Window {
	pub fn new(app: &Application) -> Self {
		// Create new window
		Object::builder().property("application", app).build()
	}
}

mod imp {
	use adw::{prelude::*, subclass::prelude::*};
	use gtk::{CompositeTemplate, gio, glib};
	use panel::prelude::*;

	// Object holding the state
	#[derive(CompositeTemplate, Default)]
	#[template(resource = "/ca/sanae/Cassava/Cassava.ui")]
	pub struct WindowInner {
		#[template_child]
		dock: TemplateChild<panel::Dock>,
		#[template_child]
		grid: TemplateChild<panel::Grid>,
	}

	// The central trait for subclassing a GObject
	#[glib::object_subclass]
	impl ObjectSubclass for WindowInner {
		// `NAME` needs to match `class` attribute of template
		const NAME: &'static str = "CassavaWindow";
		type Type = super::Window;
		type ParentType = adw::ApplicationWindow;

		fn class_init(klass: &mut Self::Class) {
			klass.bind_template();
			Self::Type::bind_template_callbacks(klass);
		}

		fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
			obj.init_template();
		}
	}

	#[gtk::template_callbacks]
	impl super::Window {
		#[template_callback]
		fn create_frame_cb(_: &panel::Grid, window: &Self) -> panel::Frame {
			let imp = window.imp();
			let frame = panel::Frame::new();
			frame
		}

		#[template_callback]
		fn egg() {
			println!("egg")
		}
	}

	// Trait shared by all GObjects
	impl ObjectImpl for WindowInner {
		fn constructed(&self) {
			// Call "constructed" on parent
			self.parent_constructed();

			// Connect to "clicked" signal of `button`
			// self.button.connect_clicked(move |button| {
			// 	// Set the label to "Hello World!" after the button has been clicked on
			// 	button.set_label("Hello World!");
			// });
		}
	}

	// Trait shared by all widgets
	impl WidgetImpl for WindowInner {}
	impl WindowImpl for WindowInner {}
	impl ApplicationWindowImpl for WindowInner {}
	impl AdwApplicationWindowImpl for WindowInner {}
}
