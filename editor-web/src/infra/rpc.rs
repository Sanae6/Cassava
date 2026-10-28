use std::{cell::RefCell, collections::HashMap, rc::Rc, sync::Once};

use linkme::distributed_slice;
use serde::Deserialize;
use ts_rs::{Config, Dependency, ExportError, TS};
use webkit6::javascriptcore::Value;

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

pub trait ObjectHandler {
	fn ts_dependencies(config: &Config) -> Vec<Dependency>
	where
		Self: Sized;
	fn ts_export(config: &Config) -> Result<(), ExportError>
	where
		Self: Sized;
	fn ts_types(config: &Config) -> String
	where
		Self: Sized;
	fn handle(&mut self, data: &Value) -> Value;
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

#[derive(TS, Deserialize)]
#[ts(export)]
struct Burgy {
	value: u32,
}

pub struct TestService;

#[editor_web_macro::handlers]
impl TestService {
	fn double(&self, value: Burgy) -> u32 {
		value.value * 2
	}
}

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
