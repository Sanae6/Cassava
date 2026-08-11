#![feature(error_generic_member_access)]

use std::{backtrace::Backtrace, fs};

use cassava::{sarc, yaz0};
use godot::{
	meta::error::{CallError, CallOutcome, ErrorToGodot},
	prelude::*,
};
use snafu::{FromString, OptionExt, ResultExt};

mod detect_format;

struct MyExtension;

#[gdextension]
unsafe impl ExtensionLibrary for MyExtension {}

#[derive(GodotClass)]
#[class(no_init)]
struct ALFile {
	data: FileData,
}

#[godot_api]
impl ALFile {
	#[func]
	fn load(file_name: String) -> Result<Gd<Self>, GodotError> {
		let data = fs::read(file_name).whatever_context("failed to read file")?;
		let format = detect_format::detect_format(&data).whatever_context("failed to detect format")?;

		let data = match format {
			detect_format::Format::Szs => {
				let (data, _alignment) = yaz0::decompress(&data).whatever_context("failed to decompress file")?;
				let reader = sarc::Reader::new(data).whatever_context("failed to read sarc file")?;
				FileData::Sarc(reader)
			}
			detect_format::Format::Sarc => {
				let reader = sarc::Reader::new(data).whatever_context("failed to read sarc file")?;
				FileData::Sarc(reader)
			}
			detect_format::Format::Byml => todo!(),
			detect_format::Format::Bntx => todo!(),
			detect_format::Format::Kcl => todo!(),
		};

		Ok(Gd::from_object(Self { data }))
	}
}

enum FileData {
	Sarc(sarc::Reader),
}

struct GodotError {
	backtrace: Backtrace,
	message: String,
}
impl FromString for GodotError {
	type Source = Box<dyn snafu::Error>;

	fn without_source(message: String) -> Self {
		let backtrace = Backtrace::force_capture();
		GodotError { backtrace, message }
	}

	fn with_source(source: Self::Source, message: String) -> Self {
		let backtrace: Backtrace = core::error::request_value::<Backtrace>(source.as_ref())
			.unwrap_or_else(Backtrace::force_capture);

		GodotError { backtrace, message }
	}
}
impl<T: ToGodot> ErrorToGodot<T> for GodotError {
	type Mapped = T;

	fn result_to_godot(result: Result<T, Self>) -> CallOutcome<Self::Mapped> {
		match result {
			Ok(value) => CallOutcome::Return(value),
			Err(GodotError { backtrace, message }) => {
				CallOutcome::CallFailed(format!("{message}\n\nBacktrace:\n{backtrace}"))
			}
		}
	}
}
