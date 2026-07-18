use std::collections::HashMap;

use snafu::{ResultExt, Snafu};

use crate::util::{self, BinaryReader};

#[derive(Snafu, Debug)]
pub enum DictError<E: snafu::Error + 'static> {
	#[snafu(display("{source}"))]
	ReadError {
		#[snafu(source(false))]
		source: util::Error,
	},
	#[snafu(transparent)]
	ElementError {
		source: E,
	},
	InvalidMagic,
}

pub fn read_dict<T, E: snafu::Error + From<util::Error>>(
	reader: &mut BinaryReader,
	value_list: usize,
	dictionary_list: usize,
	mut element_reader: impl FnMut(&mut BinaryReader, &str) -> Result<T, E>,
) -> Result<HashMap<String, T>, DictError<E>> {
	reader
		.seek(dictionary_list)
		.map_err(|source| DictError::ReadError { source })?;
	let magic = reader
		.read_bytes::<4>()
		.map_err(|source| DictError::ReadError { source })?;
  println!("magic: {}", String::from_utf8_lossy(&magic));
	if magic != *b"_DIC" {
		return Err(DictError::InvalidMagic);
	}

	let count = reader
		.read_u32()
		.map_err(|source| DictError::ReadError { source })? as _;

	let mut map = HashMap::with_capacity(count);
	for i in 0..count {
		reader.seek(
			dictionary_list
				.checked_add(24 + i * 16 + 8)
				.ok_or(DictError::ReadError { source: util::Error::OutOfBoundsSeek })?,
		)		.map_err(|source| DictError::ReadError { source })?;
		let name = reader.read_u64()		.map_err(|source| DictError::ReadError { source })? as usize;
		let name = reader.scope(name, |reader| {
      let count = reader.read_u16()?;
			reader.read_string(count as usize).map(ToOwned::to_owned)
		})		.map_err(|source| DictError::ReadError { source })?;
		let offset = value_list
			.checked_add(i * 8)
			.ok_or(util::Error::OutOfBoundsSeek)		.map_err(|source| DictError::ReadError { source })?;
		let element = reader.scope(offset, |reader| {
			element_reader(reader, &name)
		})?;
		map.insert(name, element);
	}

	Ok(map)
}
