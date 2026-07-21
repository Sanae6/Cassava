use std::{collections::HashMap, convert::Infallible, hint::unreachable_unchecked};

use snafu::Snafu;

use crate::util::{self, BinaryReader};

fn never_convert<E>(_: Infallible) -> E {
	unreachable!()
}
#[derive(Snafu, Debug)]
pub enum DictError<E: snafu::Error + 'static> {
	#[snafu(transparent)]
	ReadError {
		source: util::Error,
	},
	#[snafu(transparent)]
	ElementError {
		#[snafu(source(from(Infallible, never_convert)))]
		source: E,
	},
	InvalidMagic,
}

pub type Dict<T> = HashMap<String, T>;

pub fn read_dict<T, E: snafu::Error + From<util::Error>>(
	reader: &mut BinaryReader,
	dict_magic: [u8; 4],
	value_list: usize,
	dictionary_list: usize,
	mut element_reader: impl FnMut(&mut BinaryReader, &str) -> Result<T, E>,
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
					.ok_or(DictError::ReadError {
						source: util::Error::OutOfBoundsSeek,
					})?,
			)?;
			let name = reader.read_u64()? as usize;
			let name = reader.scope(name, |reader| {
				let count = reader.read_u16()?;
				reader.read_string(count as usize).map(ToOwned::to_owned)
			})?;
			let offset = value_list
				.checked_add(i * 8)
				.ok_or(util::Error::OutOfBoundsSeek)?;
			let element = reader
				.scope(offset, |reader| element_reader(reader, &name))
				.map_err(|source| DictError::ElementError { source })?;
			map.insert(name, element);
		}

		Ok(map)
	})
}
