use std::ptr;

use snafu::Snafu;
use webkit6::{
	glib::{
		ffi::gpointer,
		translate::{ToGlibPtr, from_glib_full},
	},
	javascriptcore::{self, Context, Value},
};

pub mod de;
pub mod ser;
pub use {de::deserialize, ser::serialize};

#[derive(Snafu, Debug)]
pub enum Error {
	#[snafu(display("{message}"))]
	Custom {
		message: String,
	},
	#[snafu(display("key is not a string"))]
	KeyNotAString,
	#[snafu(display("missing low value in 64-bit integer"))]
	MissingLow,
	#[snafu(display("missing high value in 64-bit integer"))]
	MissingHigh,
	#[snafu(display("string is not a single character"))]
	StringIsNotOneCharacter,
	#[snafu(display("typed array does not contain an array buffer"))]
	NoBufferInTypedArray,
	#[snafu(display("expected bytes, no typed array or array buffer could be found"))]
	ExpectedBytes,
	NameMismatch,
	TooManyVariantKeys,
}

// #[derive(TS)]
// pub struct U64Struct {
// 	___l: u32,
// 	___u: u32,
// }

fn new_object(context: &Context) -> Value {
	unsafe {
		from_glib_full(javascriptcore::ffi::jsc_value_new_object(
			context.to_glib_none().0,
			ptr::null_mut(),
			ptr::null_mut(),
		))
	}
}

fn new_array_buffer(context: &Context, length: usize) -> Value {
	unsafe {
		unsafe extern "C" fn free(value: gpointer) {
			drop(unsafe { Box::from_raw(value) });
		}
		let data = Box::into_raw(Box::<[u8]>::new_zeroed_slice(length));
		from_glib_full(javascriptcore::ffi::jsc_value_new_array_buffer(
			context.to_glib_none().0,
			data.cast(),
			0,
			Some(free),
			data.cast(),
		))
	}
}

fn get_array_buffer_data<'a>(value: &'a Value) -> &'a mut [u8] {
	unsafe {
		let mut size = 0usize;
		let data =
			javascriptcore::ffi::jsc_value_array_buffer_get_data(value.to_glib_none().0, &mut size);
		std::slice::from_raw_parts_mut(data.cast(), size)
	}
}
