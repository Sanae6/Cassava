use indexmap::IndexMap;
use snafu::Snafu;

use crate::{
	nw::res_dict::{self, read_dict},
	util::BinaryReader,
};

#[derive(Snafu, Debug)]
pub enum UserDataError {
	#[snafu(transparent)]
	ReadError {
		source: crate::util::Error,
		backtrace: snafu::Backtrace,
	},
}

#[derive(Clone, Debug)]
pub enum UserData {
	S32(i32),
	F32(f32),
	String(String),
	EmbeddedFile(Vec<u8>),
}

pub fn read_gfx_user_data(
	reader: &mut BinaryReader,
	dict_magic: [u8; 4],
	value_list: usize,
	dictionary_list: usize,
) -> Result<IndexMap<String, UserData>, res_dict::DictError<UserDataError>> {
	if value_list == 0 || dictionary_list == 0 {
		return Ok(IndexMap::new());
	}

	read_dict(
		reader,
		dict_magic,
		value_list,
		dictionary_list,
		|_reader, _| todo!(),
	)
}
