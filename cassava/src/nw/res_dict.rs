use std::convert::Infallible;

use indexmap::IndexMap;
use snafu::{OptionExt, Snafu};

use crate::util::{self, BinaryReader};

fn never_convert<E>(_: Infallible) -> E {
	unreachable!()
}
#[derive(Snafu, Debug)]
#[snafu(visibility)]
pub enum DictError<E: snafu::Error + snafu::ErrorCompat + 'static> {
	#[snafu(transparent)]
	ReadError {
		source: util::Error,
		backtrace: snafu::Backtrace,
	},
	#[snafu(transparent)]
	ElementError {
		#[snafu(source(from(Infallible, never_convert)))]
		#[snafu(backtrace)]
		source: E,
	},
	#[snafu(display("tried to seek past EOF"))]
	OutOfBoundsSeek {
		backtrace: snafu::Backtrace,
	},
	InvalidMagic,
}

pub type Dict<T> = IndexMap<String, T>;

pub fn read_dict<'a, T: 'a, E: snafu::Error + snafu::ErrorCompat + From<util::Error>>(
	reader: &mut BinaryReader<'a>,
	dict_magic: [u8; 4],
	value_list: usize,
	dictionary_list: usize,
	mut element_reader: impl FnMut(&mut BinaryReader<'a>, &str) -> Result<T, E>,
) -> Result<Dict<T>, DictError<E>> {
	reader.scope(dictionary_list, |reader| {
		reader.seek(dictionary_list)?;
		let magic = reader.read_bytes::<4>()?;
		if magic != dict_magic {
			return Err(DictError::InvalidMagic);
		}

		let count = reader.read_u32()? as _;

		let mut map = Dict::with_capacity(count);
		for i in 0..count {
			reader.seek(
				dictionary_list
					.checked_add(24 + i * 16 + 8)
					.context(OutOfBoundsSeekSnafu)?,
			)?;
			let name = reader.read_u64()? as usize;
			let name = reader.scope(name, |reader| {
				let count = reader.read_u16()?;
				reader.read_string(count as usize).map(ToOwned::to_owned)
			})?;
			let offset = value_list
				.checked_add(i * 8)
				.context(OutOfBoundsSeekSnafu)?;
			let element = reader
				.scope(offset, |reader| element_reader(reader, &name))
				.map_err(|source| DictError::ElementError { source })?;
			map.insert(name, element);
		}

		Ok(map)
	})
}
