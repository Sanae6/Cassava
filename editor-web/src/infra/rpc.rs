use std::{
	any::Any, cell::RefCell, collections::HashMap, fmt, hint::unreachable_unchecked, rc::Rc,
	sync::Once,
};

use linkme::distributed_slice;
use snafu::Snafu;
use ts_rs::{Config, Dependency, ExportError, TS};
use webkit6::{ScriptMessageReply, WebView, glib, javascriptcore::Value, prelude::WebViewExt};

slotmap::new_key_type! {
	#[derive(TS)]
	#[ts(type = "object")]
	pub struct ObjectId;
}
#[derive(Default)]
pub struct Rpc {
	objects: slotmap::SlotMap<ObjectId, Rc<RefCell<dyn ObjectHandler>>>,
	singletons: HashMap<String, ObjectId>,
}

thread_local! {
	static RPC_INSTANCE: RefCell<Option<Rc<RefCell<Rpc>>>> = None.into();
}
impl Rpc {
	pub fn setup() {
		if option_env!("TS_RS_EXPORT_DIR").is_some() {
			write_types();
		}
		RPC_INSTANCE.set(Some(Rc::new(RefCell::new(Rpc::default()))))
	}

	pub fn instance() -> Rc<RefCell<Rpc>> {
		RPC_INSTANCE.with_borrow(|value| {
			value
				.as_ref()
				.expect("rpc object not initialised on this thread")
				.clone()
		})
	}

	pub fn register<O: ObjectHandler + 'static>(&mut self, handler: O) -> (ObjectId, Rc<RefCell<O>>) {
		let handler = Rc::new(RefCell::new(handler));
		(self.objects.insert(handler.clone()), handler)
	}

	pub fn register_singleton<O: ObjectHandler + 'static>(
		&mut self,
		key: &str,
		handler: O,
	) -> (ObjectId, Rc<RefCell<O>>) {
		let (object_id, handler) = self.register(handler);
		self.singletons.insert(key.to_owned(), object_id);
		(object_id, handler)
	}

	pub fn get_singleton(&self, name: &str) -> Option<ObjectId> {
		self.singletons.get(name).cloned()
	}

	pub fn get_object(&self, object: ObjectId) -> Option<Rc<RefCell<dyn ObjectHandler>>> {
		self.objects.get(object).cloned()
	}
}

pub fn filter_export_failures(error: ExportError) -> Result<(), ExportError> {
	match error {
		ExportError::CannotBeExported(_) => Ok(()),
		error => Err(error),
	}
}

#[derive(Snafu, Debug)]
#[snafu(display("unknown message on {handler_name}: {type_string}"))]
pub struct HandlerError {
	pub type_string: String,
	pub handler_name: &'static str,
}
pub struct Context<T: ObjectHandler + ?Sized> {
	pub rc: Rc<RefCell<T>>,
	pub webview: WebView,
	pub progress_reporter: Option<String>,
}
impl Context<dyn ObjectHandler> {
	#[doc(hidden)]
	/// downcasts the dyn context into a context of a given type.
	///
	/// safety: the caller must call this function with the currently stored type.
	pub unsafe fn downcast_unchecked<T: ObjectHandler>(self) -> Context<T> {
		let rc = Rc::into_raw(self.rc);

		Context {
			// safety: the type is guaranteed to be correct by the caller, therefore
			// alignment and size are the same for the type.
			rc: unsafe { Rc::from_raw(rc.cast()) },
			webview: self.webview,
			progress_reporter: self.progress_reporter,
		}
	}
}
impl<T: ObjectHandler> Context<T> {
	pub fn report_progress(&self, args: fmt::Arguments) {
		let Some(progress_reporter) = &self.progress_reporter else {
			eprintln!("warning: attempting to report progress without progress reporter id");
			return;
		};
		let args = format!("__crescentEventManager.invokeProgressReporter(\"{progress_reporter}\", {args})");
		let webview = self.webview.clone();
		glib::MainContext::default().spawn_local(async move {
			let _ = webview.evaluate_javascript_future(&args, None, None).await;
		});
	}
}
pub trait ObjectHandler: Any {
	fn ts_dependencies(config: &Config) -> Vec<Dependency>
	where
		Self: Sized;
	fn ts_export(config: &Config) -> Result<(), ExportError>
	where
		Self: Sized;
	fn ts_types(config: &Config) -> String
	where
		Self: Sized;

	fn handle(
		&mut self,
		context: Context<dyn ObjectHandler>,
		reply: ScriptMessageReply,
		data: &Value,
	) -> Result<Option<Value>, HandlerError>;
}

#[distributed_slice]
pub static HANDLER_TYPE_WRITERS: [fn(&Config)];
fn write_types() {
	static WRITE_ONCE: Once = Once::new();
	WRITE_ONCE.call_once(|| {
		if let Ok(_) = std::env::var("TS_RS_EXPORT_DIR") {
			let config = ts_rs::Config::from_env();
			for type_writer in HANDLER_TYPE_WRITERS {
				type_writer(&config)
			}
		}
	});
}

// pub use rpc_promise::new_promise;
// mod rpc_promise {
// 	use webkit6::{
// 		glib::{
// 			self,
// 			translate::{ToGlibPtr, from_glib_borrow, from_glib_full},
// 		},
// 		javascriptcore::ffi,
// 		javascriptcore::{Context, Value},
// 	};

// 	pub fn new_promise<P: FnOnce(&Value, &Value)>(context: &Context, executor: P) -> Value {
// 		let mut executor_data: Option<P> = Some(executor);
// 		unsafe extern "C" fn executor_func<P: FnOnce(&Value, &Value)>(
// 			resolve: *mut ffi::JSCValue,
// 			reject: *mut ffi::JSCValue,
// 			user_data: glib::ffi::gpointer,
// 		) {
// 			unsafe {
// 				let resolve = from_glib_borrow(resolve);
// 				let reject = from_glib_borrow(reject);
// 				let callback = user_data as *mut Option<P>;
// 				if let Some(callback) = callback.as_mut_unchecked().take() {
// 					(callback)(&resolve, &reject)
// 				} else {
// 					eprintln!("callback called despite already being taken");
// 				}
// 			}
// 		}
// 		let executor = Some(executor_func::<P> as _);
// 		let super_callback0: &mut Option<P> = &mut executor_data;
// 		unsafe {
// 			from_glib_full(ffi::jsc_value_new_promise(
// 				context.to_glib_none().0,
// 				executor,
// 				super_callback0 as *mut _ as *mut _,
// 			))
// 		}
// 	}
// }

pub use ts_rs_vendored::generate_imports;
mod ts_rs_vendored {
	use std::{
		collections::{BTreeMap, BTreeSet},
		path::{Component, Path, PathBuf, absolute},
	};

	use ts_rs::{Config, Dependency, ExportError};

	pub fn generate_imports(
		cfg: &Config,
		path: &str,
		dependencies: Vec<Dependency>,
		out: &mut String,
	) -> Result<(), ExportError> {
		// let path = <T as TS>::output_path()
		// 	.ok_or_else(std::any::type_name::<T>)
		// 	.map(|x| cfg.out_dir().join(x))
		// 	.map_err(ExportError::CannotBeExported)?;
		let path = cfg.out_dir().join(path);

		// let deps = <T as TS>::dependencies(cfg);
		let deduplicated_deps = dependencies
			.iter()
			// .filter(|dep| dep.type_id != TypeId::of::<T>())
			.map(|dep| (&dep.ts_name, dep))
			.collect::<BTreeMap<_, _>>();

		let mut imports: BTreeMap<String, BTreeSet<&str>> = Default::default();

		for (_, dep) in deduplicated_deps {
			let dep_path = cfg.out_dir().join(&dep.output_path);
			let rel_path = import_path(cfg, &path, &dep_path)?;

			let is_same_file = path
				.file_name()
				.and_then(std::ffi::OsStr::to_str)
				.map(|x| x.trim_end_matches(".ts"))
				.map(|x| format!("./{x}"))
				.map(|x| x == rel_path.trim_end_matches(".js"))
				.unwrap_or(false);

			if is_same_file {
				continue;
			}

			imports.entry(rel_path).or_default().insert(&dep.ts_name);
		}

		use std::fmt::Write;
		for (path, types) in imports {
			write!(out, "import type {{ ")?;

			let mut types = types.iter().peekable();

			while let Some(ty) = types.next() {
				write!(out, "{ty}")?;

				if types.peek().is_some() {
					write!(out, ", ")?;
				}
			}

			writeln!(out, r#" }} from "{path}";"#)?;
		}

		writeln!(out)?;
		Ok(())
	}

	/// Returns the required import path for importing `import` from the file `from`
	fn import_path(cfg: &Config, from: &Path, import: &Path) -> Result<String, ExportError> {
		let rel_path = diff_paths(import, from.parent().unwrap())?;
		let str_path = match rel_path.components().next() {
			Some(Component::Normal(_)) => {
				format!("./{}", rel_path.to_string_lossy())
			}
			_ => rel_path.to_string_lossy().into(),
		};

		let path = if cfg!(target_os = "windows") {
			str_path.replace('\\', "/")
		} else {
			str_path
		};

		let mut path = path.trim_end_matches(".ts").to_owned();
		if let Some(ext) = cfg.import_extension() {
			path.push('.');
			path.push_str(ext);
		}

		Ok(path)
	}

	// Construct a relative path from a provided base directory path to the provided path.
	//
	// Copyright 2012-2015 The Rust Project Developers.
	//
	// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
	// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
	// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
	// option. This file may not be copied, modified, or distributed
	// except according to those terms.
	//
	// Adapted from rustc's path_relative_from
	// https://github.com/rust-lang/rust/blob/e1d0de82cc40b666b88d4a6d2c9dcbc81d7ed27f/src/librustc_back/rpath.rs#L116-L158
	pub(super) fn diff_paths<P, B>(path: P, base: B) -> Result<PathBuf, ts_rs::ExportError>
	where
		P: AsRef<Path>,
		B: AsRef<Path>,
	{
		use std::path::Component as C;
		let path = absolute(path)?;
		let base = absolute(base)?;

		let mut ita = path.components();
		let mut itb = base.components();
		let mut comps: Vec<C> = vec![];

		loop {
			match (ita.next(), itb.next()) {
				(Some(C::ParentDir | C::CurDir), _) | (_, Some(C::ParentDir | C::CurDir)) => {
					unreachable!("The paths have been cleaned, no no '.' or '..' components are present")
				}
				(None, None) => break,
				(Some(a), None) => {
					comps.push(a);
					comps.extend(ita.by_ref());
					break;
				}
				(None, _) => comps.push(C::ParentDir),
				(Some(a), Some(b)) if comps.is_empty() && a == b => (),
				(Some(a), Some(_)) => {
					comps.push(C::ParentDir);
					for _ in itb {
						comps.push(C::ParentDir);
					}
					comps.push(a);
					comps.extend(ita.by_ref());
					break;
				}
			}
		}

		Ok(comps.iter().map(|c| c.as_os_str()).collect())
	}
}
